//! Text insertion into the focused window.
//!
//! Two mechanisms:
//! - `type_text`: synthetic Unicode keystrokes, char by char. Works nearly
//!   everywhere, but long sustained bursts can outrun the target app's
//!   input handling (see the FOUND LIVE notes below and on `insert_text`).
//! - `paste_text`: put the text on the clipboard and send one Ctrl+V —
//!   atomic from the app's point of view, so there is no per-character
//!   race at all. The previous clipboard text is saved and restored.
//!
//! `insert_text` picks between them by length. Shared between the main
//! shell and `src/bin/sendinput_test.rs` (a standalone compatibility-
//! testing harness — see STORY-015).

use std::time::Duration;

use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
};

/// Force-release every modifier key before typing starts.
///
/// Found live: dictating right after the Ctrl+Alt+` hotkey combo
/// occasionally caused an accidental Ctrl+N (new window) partway through
/// typing — Notepad windows appeared mid-dictation, each holding part of
/// the typed text, i.e. exactly the symptom of Ctrl being interpreted as
/// still held right as a character-with-an-N landed. Rather than chase the
/// exact timing race, explicitly send KEYUP for every modifier key up front
/// — cheap, and removes the whole class of "hotkey's modifiers bled into
/// the typed text" bug regardless of the precise mechanism.
/// Block until no modifier key is physically held (or a 3s deadline).
///
/// FOUND LIVE (2026-07-06, twice): typing right after the stop hotkey lands
/// while the user's Ctrl/Shift are still physically down. The synthetic
/// keyups from `release_all_modifiers` clear the modifier state only until
/// the still-held physical key's auto-repeat re-asserts it (~30ms later) —
/// so injected characters intermittently become accelerator chords in the
/// target app. Concretely: dictated commas opened Notepad's Settings
/// (Ctrl+,) on stop, and the (rolled-back) live-typing experiment fired
/// shortcuts continuously for the same reason. Synthetic keyups cannot fix
/// a physically held key; the only reliable move is to wait for the real
/// release. The deadline is a hang-guard (e.g. a genuinely stuck key);
/// `release_all_modifiers` still runs after as belt-and-braces.
pub fn wait_for_physical_modifier_release() {
    const DEADLINE: Duration = Duration::from_secs(3);
    const POLL: Duration = Duration::from_millis(15);
    let mods = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN];
    let start = std::time::Instant::now();
    while start.elapsed() < DEADLINE {
        let any_held = mods
            .iter()
            .any(|&vk| unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0);
        if !any_held {
            return;
        }
        std::thread::sleep(POLL);
    }
    eprintln!("type_text: modifier still held after 3s — typing anyway");
}

fn release_all_modifiers() {
    let mods = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN];
    let inputs: Vec<INPUT> = mods
        .iter()
        .map(|&vk| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        })
        .collect();
    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Type text into the focused window as Unicode keyboard events.
///
/// Found live (M1): sending the whole utterance as one large `SendInput`
/// batch can outrun the target window's message queue and drop/scramble
/// characters (seen once out of several runs; not every app, not every
/// time — a race, not a deterministic failure). Mitigated the standard way:
/// send a few characters at a time with a short sleep between groups, which
/// gives the receiving window's message pump time to drain between bursts.
///
/// FOUND LIVE AGAIN, longer dictation: a ~550-character utterance arrived
/// in Notepad cut off mid-sentence, even though `SendInput` itself reported
/// success for every single group (no dropped-event warning logged) — the
/// drop happened downstream, in the app's own processing, not at the OS
/// injection layer. Same race as above, just needing a longer burst to
/// actually surface. Widened the safety margin (smaller groups, longer
/// delay) since 6 chars/4ms was only ever validated against short test
/// utterances, not several hundred characters of sustained typing.
///
/// NOTE (STORY-015 compatibility testing): every character, including `\n`,
/// goes through as a Unicode *character* event (KEYEVENTF_UNICODE), never a
/// real VK_RETURN key event. Text editors treat a Unicode LF as a line
/// break, but terminal emulators generally expect an actual Enter keypress
/// to submit a command — a literal `\n` character may just insert a blank
/// line instead of running anything. Verified with `sendinput_test`; see
/// docs/CONTEXT.md for the result and whether a VK_RETURN special case was
/// needed.
pub fn type_text(text: &str) {
    const GROUP_SIZE: usize = 4; // characters per SendInput call
    const GROUP_DELAY: Duration = Duration::from_millis(8);

    wait_for_physical_modifier_release();
    release_all_modifiers();
    let units: Vec<u16> = text.encode_utf16().collect();
    for group in units.chunks(GROUP_SIZE) {
        let inputs: Vec<INPUT> = group
            .iter()
            .flat_map(|&cu| {
                let mk = |flags| INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: cu,
                            dwFlags: flags,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                [mk(KEYEVENTF_UNICODE), mk(KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)]
            })
            .collect();
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent as usize != inputs.len() {
            let err = unsafe { windows::Win32::Foundation::GetLastError() };
            eprintln!(
                "type_text: SendInput accepted {sent}/{} events in this group (GetLastError={:?}) — target window may have dropped input",
                inputs.len(), err
            );
        }
        std::thread::sleep(GROUP_DELAY);
    }
}

/// FOUND LIVE (2026-07-08): a ~470-char final typed via `type_text` arrived
/// in the target app with a ~75-char stretch replaced by a run of a single
/// repeated character ("yyyy…") — the sidecar's own dump proved the text
/// was clean before injection, so the corruption happened in the app's
/// handling of the sustained synthetic-keystroke burst. Third live incident
/// in this class (drop, truncation, now repetition); pacing tweaks only
/// shrink the window. For long text the class is avoided entirely by not
/// simulating typing at all: put the text on the clipboard and send one
/// Ctrl+V. Below this threshold, typing is kept — it works in more places
/// (no paste-shortcut assumption) and never disturbs the clipboard.
const PASTE_THRESHOLD_CHARS: usize = 100;

/// Insert a finalized transcript into the focused window, choosing the
/// mechanism by length (see PASTE_THRESHOLD_CHARS).
pub fn insert_text(text: &str) {
    if text.chars().count() >= PASTE_THRESHOLD_CHARS {
        paste_text(text);
    } else {
        type_text(text);
    }
}

/// Read the clipboard's current text, if any (so it can be restored).
fn clipboard_get_text() -> Option<String> {
    unsafe {
        if OpenClipboard(None).is_err() {
            return None;
        }
        let result = GetClipboardData(CF_UNICODETEXT.0 as u32).ok().and_then(|handle| {
            let hglobal = HGLOBAL(handle.0);
            let ptr = GlobalLock(hglobal) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let mut len = 0;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
            let _ = GlobalUnlock(hglobal);
            Some(text)
        });
        let _ = CloseClipboard();
        result
    }
}

/// Put `text` on the clipboard as CF_UNICODETEXT. Returns false on any
/// failure (caller falls back to keystroke typing).
fn clipboard_set_text(text: &str) -> bool {
    unsafe {
        if OpenClipboard(None).is_err() {
            return false;
        }
        let ok = (|| {
            EmptyClipboard().ok()?;
            let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = units.len() * 2;
            let hglobal = GlobalAlloc(GMEM_MOVEABLE, bytes).ok()?;
            let ptr = GlobalLock(hglobal) as *mut u16;
            if ptr.is_null() {
                return None;
            }
            std::ptr::copy_nonoverlapping(units.as_ptr(), ptr, units.len());
            let _ = GlobalUnlock(hglobal);
            // On success the system owns the memory — do not free it.
            SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(hglobal.0)).ok()?;
            Some(())
        })()
        .is_some();
        let _ = CloseClipboard();
        ok
    }
}

/// Insert text by clipboard paste: save current clipboard text, set ours,
/// send Ctrl+V, wait for the target to process the paste, then restore
/// what was there before. Known limitation: non-text clipboard content
/// (images, files) is not preserved — acceptable for now; a full
/// multi-format save/restore is significant extra surface.
pub fn paste_text(text: &str) {
    wait_for_physical_modifier_release();

    let saved = clipboard_get_text();
    if !clipboard_set_text(text) {
        eprintln!("paste_text: clipboard unavailable — falling back to typed insertion");
        type_text(text);
        return;
    }

    release_all_modifiers();
    let mk = |vk: VIRTUAL_KEY, flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [
        mk(VK_CONTROL, Default::default()),
        mk(VK_V, Default::default()),
        mk(VK_V, KEYEVENTF_KEYUP),
        mk(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        eprintln!("paste_text: SendInput accepted {sent}/{} events", inputs.len());
    }

    // Give the target time to handle WM_PASTE before the clipboard changes
    // back under it. Most apps read synchronously on the keystroke; 300ms
    // covers slow/Electron apps comfortably.
    std::thread::sleep(Duration::from_millis(300));
    if let Some(prev) = saved {
        let _ = clipboard_set_text(&prev);
    }
}
