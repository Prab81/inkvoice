//! InkVoice shell — M1 vertical slice.
//!
//! Toggle hotkey (Ctrl+Alt+Space) -> WASAPI mic capture (cpal; PortAudio is
//! broken for BT LE Audio mics, see docs/CONTEXT.md) -> PCM streamed to the
//! Python ASR sidecar over localhost TCP -> partials printed to console,
//! final text typed into the focused window via SendInput (Unicode events,
//! so it works in terminals and plain Win32 controls alike).
//!
//! NOTE: live-typing partials into the focused document was tried (2026-07-06)
//! and ROLLED BACK — typing while the hotkey/PTT modifiers were physically
//! held caused the target app to interpret the injected keystrokes as
//! shortcuts (Notepad commands fired repeatedly). See CONTEXT.md before
//! re-attempting: any live-typing scheme must not inject while Ctrl/Shift
//! are physically down.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, VIRTUAL_KEY, VK_8, VK_9,
    VK_F8, VK_F9, VK_CONTROL, VK_OEM_3, VK_SHIFT, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, LLKHF_LOWER_IL_INJECTED, MSG, WH_KEYBOARD_LL, WM_APP,
    WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

const WM_APP_PTT_DOWN: u32 = WM_APP + 10;
const WM_APP_PTT_UP: u32 = WM_APP + 11;

/// Set inside the low-level keyboard hook, read from the same hook — tracks
/// whether the push-to-talk chord is currently held, so we post exactly one
/// "down" event per press (not once per repeat-key WM_KEYDOWN while held)
/// and exactly one "up" event on release. `extern "system" fn` hook
/// callbacks get no user-data slot, so this has to be a static rather than
/// captured state.
static PTT_KEY_DOWN: AtomicBool = AtomicBool::new(false);

/// Push-to-talk: hold Ctrl+Shift+Space to record, release to stop.
///
/// `RegisterHotKey`/`WM_HOTKEY` (used for the toggle hotkeys below) only
/// ever fires once per press — Windows gives no matching "key released"
/// hotkey message, so hold-to-talk needs a different mechanism: a
/// low-level keyboard hook, which sees real key-down/key-up pairs for any
/// key system-wide. Installed on the main thread — `SetWindowsHookExW`
/// requires a message loop pumping on the installing thread to actually
/// receive callbacks, and this thread already runs one (the `GetMessageW`
/// loop below), so no extra thread is needed.
///
/// Swallows the Space key (returns non-zero instead of calling
/// `CallNextHookEx`) whenever Ctrl+Shift are held, for the same reason the
/// toggle hotkeys use a dedicated key rather than a letter: without this,
/// holding the chord would leak a literal space keystroke into whatever
/// app is focused, and some apps bind Ctrl+Shift+Space to their own
/// shortcut (e.g. paste-without-formatting).
///
/// FOUND LIVE: a long dictation's text arrived incomplete in the target
/// app (Notepad) even though the sidecar's own log showed the full,
/// correct final text — the drop happened somewhere between `type_text`'s
/// `SendInput` calls (which all reported success) and the app actually
/// rendering every character. A low-level keyboard hook receives a
/// callback for *every* keyboard event system-wide, including events
/// `type_text` itself injects via `SendInput` (a space character alone is
/// roughly 1 in 6 characters of ordinary English text) — every one of
/// those was an extra synchronous hop through this hook, on top of
/// whatever the target app's own message pump was already straining to
/// keep up with on a long typing burst. Real physical key holds carry no
/// `LLKHF_INJECTED`/`LLKHF_LOWER_IL_INJECTED` flag; synthetic ones (from
/// `type_text`, or any other app's `SendInput`) do — bailing out
/// immediately for injected events means push-to-talk (which should only
/// ever react to an actual physical hold) skips all of that per-character
/// work for our own typing traffic, rather than paying hook overhead on
/// every synthetic character of every dictation.
unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let injected = (kb.flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED)).0 != 0;
        if !injected && kb.vkCode == VK_SPACE.0 as u32 {
            let msg = wparam.0 as u32;
            let is_keydown = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let is_keyup = msg == WM_KEYUP || msg == WM_SYSKEYUP;
            let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let shift_down = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;

            if is_keydown && ctrl_down && shift_down {
                if !PTT_KEY_DOWN.swap(true, Ordering::SeqCst) {
                    let _ = PostThreadMessageW(
                        GetCurrentThreadId(),
                        WM_APP_PTT_DOWN,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
                return LRESULT(1);
            }
            if is_keyup && PTT_KEY_DOWN.swap(false, Ordering::SeqCst) {
                let _ =
                    PostThreadMessageW(GetCurrentThreadId(), WM_APP_PTT_UP, WPARAM(0), LPARAM(0));
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

mod command_mode;
mod modes;
mod overlay;
mod text_insert;
use modes::SharedMode;
use text_insert::insert_text;

const SIDECAR_ADDR: &str = "127.0.0.1:43917";
const TARGET_RATE: u32 = 16000;
const HOTKEY_ID: i32 = 1;
const HOTKEY_ID_CMD: i32 = 2;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Prose,
    Command,
}

enum AudioCmd {
    Start(Sender<f32>), // amplitude sender for the recording overlay
    Stop,
}

enum NetCmd {
    Begin,
    Audio(Vec<f32>),
    End { mode: &'static str }, // writing mode for the sidecar's formatting pass
}

/// Downmix interleaved frames to mono and linearly resample to 16 kHz.
fn to_mono_16k(input: &[f32], channels: usize, src_rate: u32) -> Vec<f32> {
    let mono: Vec<f32> = input
        .chunks(channels)
        .map(|f| f.iter().sum::<f32>() / channels as f32)
        .collect();
    if src_rate == TARGET_RATE {
        return mono;
    }
    let ratio = src_rate as f64 / TARGET_RATE as f64;
    let out_len = (mono.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let j = pos as usize;
            let frac = (pos - j as f64) as f32;
            let a = mono[j.min(mono.len() - 1)];
            let b = mono[(j + 1).min(mono.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

/// Register the first hotkey in `candidates` that isn't already taken by
/// another app, returning its display name. Used for both the dictation
/// and Command Mode hotkeys.
fn register_first_free<'a>(id: i32, candidates: &[(&'a str, u32, VIRTUAL_KEY)]) -> &'a str {
    candidates
        .iter()
        .find(|(name, mods, vk)| {
            let ok = unsafe {
                RegisterHotKey(
                    None,
                    id,
                    windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS(*mods),
                    vk.0 as u32,
                )
            }
            .is_ok();
            if !ok {
                eprintln!("{name} unavailable, trying next…");
            }
            ok
        })
        .map(|(name, _, _)| *name)
        .expect("no candidate hotkey could be registered")
}

/// Network thread: frames PCM as JSON lines to the sidecar.
fn net_thread(rx: std::sync::mpsc::Receiver<NetCmd>, mut stream: TcpStream) {
    let b64 = base64::engine::general_purpose::STANDARD;
    for cmd in rx {
        let line = match cmd {
            NetCmd::Begin => serde_json::json!({"type": "begin"}).to_string(),
            NetCmd::End { mode } => serde_json::json!({"type": "end", "mode": mode}).to_string(),
            NetCmd::Audio(samples) => {
                let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                serde_json::json!({"type": "audio", "pcm": b64.encode(&bytes)}).to_string()
            }
        };
        if stream.write_all((line + "\n").as_bytes()).is_err() {
            eprintln!("sidecar connection lost");
            return;
        }
    }
}

/// Normalize a raw streaming partial for pill display: the streaming model
/// has no casing of its own, so lowercase everything and capitalize the
/// first letter — the cleaned final replaces it anyway.
fn normalize_partial(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => lower,
    }
}

/// Reader thread: prints partials to the console and forwards them to the
/// recording pill's live-transcript area (display-only — nothing is typed
/// until the final). On a final transcript, either types it (Prose mode)
/// or matches it against the command set and dispatches the resulting key
/// combo (Command mode) — see `command_mode.rs`.
fn reader_thread(
    stream: TcpStream,
    mode: Arc<Mutex<Mode>>,
    overlay_text_tx: Arc<Mutex<Option<Sender<String>>>>,
) {
    let reader = BufReader::new(stream);
    for line in reader.lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        match msg["type"].as_str() {
            Some("ready") => println!("sidecar ready — Ctrl+Alt+Space to toggle dictation"),
            Some("partial") => {
                let text = msg["text"].as_str().unwrap_or("");
                print!("\r\x1b[2K  … {text}");
                let _ = std::io::stdout().flush();
                if let Some(tx) = overlay_text_tx.lock().unwrap().as_ref() {
                    let _ = tx.send(normalize_partial(text));
                }
            }
            Some("final") => {
                let text = msg["text"].as_str().unwrap_or("");
                println!("\r\x1b[2K  ✓ {text}");
                if text.is_empty() {
                    continue;
                }
                match *mode.lock().unwrap() {
                    Mode::Prose => insert_text(text),
                    Mode::Command => match command_mode::match_command(text) {
                        Some(action) => {
                            println!("[command: {action:?}]");
                            command_mode::dispatch(action);
                        }
                        None => println!("[command not recognized: {text:?}]"),
                    },
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let stream = TcpStream::connect(SIDECAR_ADDR)
        .expect("cannot reach ASR sidecar — start src/sidecar/asr_server.py first");
    let (tx, rx) = channel::<NetCmd>();
    {
        let s = stream.try_clone().expect("clone stream");
        std::thread::spawn(move || net_thread(rx, s));
    }
    let mode = Arc::new(Mutex::new(Mode::Prose));
    let writing_mode = SharedMode::new();
    let overlay_text_tx: Arc<Mutex<Option<Sender<String>>>> = Arc::new(Mutex::new(None));
    {
        let mode = mode.clone();
        let overlay_text_tx = overlay_text_tx.clone();
        std::thread::spawn(move || reader_thread(stream, mode, overlay_text_tx));
    }

    // Audio capture.
    //
    // cpal::Stream wraps a COM object and is deliberately !Send, so it can
    // never move between threads — it must be created, held, and dropped on
    // one dedicated thread. That thread owns capture for the app's lifetime;
    // the message-loop thread only ever sends it lightweight Start/Stop
    // signals over a channel.
    //
    // That owning thread also self-heals: it retries opening the stream
    // whenever `want_recording` is true and it doesn't currently have one —
    // which covers three real failure modes found live during M1:
    //   1. Opening the stream at app startup raced Windows' Bluetooth
    //      A2DP->HFP profile switch and died within ~1s with no visible
    //      symptom besides an empty transcript.
    //   2. The OS default input device can be stale/invalid (e.g. a virtual
    //      "Broadcast" mic with no real source behind it).
    //   3. A Bluetooth mic can drop and reconnect mid-recording (out of
    //      range, a phone call stealing the HFP profile). The stream's error
    //      callback reports this back over `died_tx`, the owning thread
    //      drops the dead stream, and its normal retry loop reopens fresh
    //      against whatever the current default device is — so repeated
    //      off/on cycles within one recording all recover on their own.
    let recording = Arc::new(AtomicBool::new(false));

    enum AudioEvent {
        StreamDied,
    }

    fn build_stream_once(
        tx: Sender<NetCmd>,
        died_tx: Sender<AudioEvent>,
        amp_tx: Sender<f32>,
    ) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("no default input device")?;
        println!(
            "input device: {}",
            device.name().unwrap_or_else(|_| "?".into())
        );
        let config = device.default_input_config().map_err(|e| e.to_string())?;
        let channels = config.channels() as usize;
        let src_rate = config.sample_rate().0;
        println!("capture: {src_rate} Hz, {channels} ch -> 16 kHz mono");
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    let rms = (data.iter().map(|s| s * s).sum::<f32>() / data.len().max(1) as f32)
                        .sqrt();
                    let _ = amp_tx.send(rms);
                    let _ = tx.send(NetCmd::Audio(to_mono_16k(data, channels, src_rate)));
                },
                move |e| {
                    eprintln!("audio stream error: {e}");
                    let _ = died_tx.send(AudioEvent::StreamDied);
                },
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(stream)
    }

    let audio_cmd_tx = {
        let (audio_cmd_tx, audio_cmd_rx) = channel::<AudioCmd>();
        let tx = tx.clone();
        std::thread::spawn(move || {
            let (died_tx, died_rx) = channel::<AudioEvent>();
            let mut current: Option<cpal::Stream> = None;
            let mut want_recording = false;
            let mut current_amp_tx: Option<Sender<f32>> = None;
            let mut last_attempt = std::time::Instant::now() - Duration::from_secs(10);
            const RETRY_INTERVAL: Duration = Duration::from_millis(400);

            loop {
                match audio_cmd_rx.recv_timeout(RETRY_INTERVAL) {
                    Ok(AudioCmd::Start(amp_tx)) => {
                        want_recording = true;
                        current_amp_tx = Some(amp_tx);
                        current = None; // force an immediate fresh open below
                        last_attempt = std::time::Instant::now() - Duration::from_secs(10);
                    }
                    Ok(AudioCmd::Stop) => {
                        want_recording = false;
                        current_amp_tx = None;
                        current = None; // dropping the Stream stops capture
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }

                if current.is_some() && died_rx.try_recv().is_ok() {
                    println!("\n[mic disconnected — reconnecting…]");
                    current = None;
                }

                if want_recording && current.is_none() && last_attempt.elapsed() >= RETRY_INTERVAL
                {
                    last_attempt = std::time::Instant::now();
                    if let Some(amp_tx) = current_amp_tx.clone() {
                        match build_stream_once(tx.clone(), died_tx.clone(), amp_tx) {
                            Ok(s) => {
                                current = Some(s);
                                println!("[mic ready]");
                            }
                            Err(e) => eprintln!("capture open failed, retrying: {e}"),
                        }
                    }
                }
            }
        });
        audio_cmd_tx
    };

    // Register the first available hotkey from a candidate list — other
    // apps (IMEs, PowerToys, other dictation tools) commonly hold these.
    //
    // FOUND LIVE: Ctrl+Alt+I let the plain "I" leak through to the focused
    // app (many rich-text editors bind Ctrl+I to italics). Moved to
    // Ctrl+Alt+` next — but on non-US keyboard layouts, Ctrl+Alt is
    // interpreted as AltGr, and the OS can reinterpret the whole combo as
    // an entirely different character/shortcut (observed: pressing
    // Ctrl+Alt+` opened Notepad's Settings — modern Windows apps commonly
    // bind Ctrl+, to Settings, a plausible AltGr reinterpretation).
    //
    // FOUND LIVE AGAIN (2026-07-08): Ctrl+Shift+` — the AltGr-safe
    // replacement above — still leaked straight into Notepad on a machine
    // with two keyboard layouts installed (en-AU and en-GB) and Windows'
    // default "different input method per app window" enabled. Confirmed
    // via screenshot: Notepad showed its Alt-key KeyTip badges (F/E/V over
    // File/Edit/View, etc.) — proof a raw Alt keystroke reached Notepad
    // even though our combo is Ctrl+Shift-only. RegisterHotKey's
    // virtual-key matching for OEM punctuation keys is resolved against
    // the ACTIVE LAYOUT OF THE FOREGROUND THREAD at the moment of the
    // keypress, not the layout active when we registered — a
    // long-documented Windows quirk. US and UK layouts map the
    // grave/backtick key differently enough that producing it apparently
    // synthesizes an extra Alt component on this layout; that no longer
    // matches our exact Ctrl+Shift+VK_OEM_3 registration, so Windows lets
    // the raw keystroke fall through uncaught, immediately, before any of
    // our code ever ran. Root fix: never register a hotkey on an OEM
    // punctuation key again.
    // Function keys (VK_F9 etc.) have identical virtual-key codes on every
    // keyboard layout — there's no character to remap, so this whole bug
    // class is structurally impossible. Backtick combos kept as fallback
    // only for the rare case F9 itself is taken by something else.
    //
    // FOUND LIVE (2026-07-09, again): F9 works but most laptop keyboards
    // route bare F-keys to hardware/media functions by default, requiring
    // Fn held too — an extra key every single time is a real ergonomic
    // cost the layout-safety fix shouldn't have imposed. Digits don't have
    // that problem (no Fn row) and are just as layout-stable as function
    // keys for THIS user's installed layouts (en-AU/en-GB are both
    // standard QWERTY — the digit row is identical between them, unlike
    // the OEM punctuation that caused the original bug). Moved to
    // Ctrl+Shift+8 primary; F9 kept as the first fallback since it's still
    // the more universally layout-safe choice if 8 is ever taken.
    //
    // Ctrl+Alt+* is unreliable on non-US layouts (AltGr) — prefer
    // Ctrl+Shift combos there. Ctrl+Shift+Space is deliberately excluded
    // here — it's reserved for the push-to-talk chord (hold to record)
    // below, and having it as a toggle fallback too would make the two
    // modes ambiguous if it were ever actually selected as the registered
    // toggle hotkey.
    let candidates: [(&str, u32, VIRTUAL_KEY); 5] = [
        ("Ctrl+Shift+8", (MOD_CONTROL | MOD_SHIFT).0, VK_8),
        ("Ctrl+Shift+F9", (MOD_CONTROL | MOD_SHIFT).0, VK_F9),
        ("Ctrl+Shift+`", (MOD_CONTROL | MOD_SHIFT).0, VK_OEM_3),
        ("Ctrl+Alt+`", (MOD_CONTROL | MOD_ALT).0, VK_OEM_3),
        ("Ctrl+Alt+Space", (MOD_CONTROL | MOD_ALT).0, VK_SPACE),
    ];
    let hotkey_name = register_first_free(HOTKEY_ID, &candidates);
    println!("hotkey: {hotkey_name} (toggle dictation)");

    // STORY-009 MVP: Command Mode gets its own hotkey rather than a voice
    // wake-phrase — "say command mode to enter" is unreliable to detect
    // and adds real complexity; a second hotkey is simple and certain.
    // Same AltGr caution as above: Ctrl+Shift preferred over Ctrl+Alt.
    // Digit keys are far less layout-risky than OEM punctuation (see the
    // dictation hotkey's FOUND LIVE AGAIN note), but F8 costs nothing as a
    // belt-and-braces fallback that's fully layout-independent. (F10 is
    // deliberately avoided as a candidate — plain F10 is itself the
    // Windows "activate menu bar" key, and reusing that number felt like
    // asking for exactly the bug this fallback exists to avoid.)
    let cmd_candidates: [(&str, u32, VIRTUAL_KEY); 3] = [
        ("Ctrl+Shift+9", (MOD_CONTROL | MOD_SHIFT).0, VK_9),
        ("Ctrl+Alt+9", (MOD_CONTROL | MOD_ALT).0, VK_9),
        ("Ctrl+Shift+F8", (MOD_CONTROL | MOD_SHIFT).0, VK_F8),
    ];
    let cmd_hotkey_name = register_first_free(HOTKEY_ID_CMD, &cmd_candidates);
    println!("hotkey: {cmd_hotkey_name} (toggle command mode)");

    // STORY-001 AC1: push-to-talk alongside the toggle hotkeys above, not
    // replacing them — both stay available. See `keyboard_hook_proc` for
    // why this needs a low-level hook rather than `RegisterHotKey`.
    let ptt_hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0) };
    match &ptt_hook {
        Ok(_) => println!("hotkey: Ctrl+Shift+Space (hold to talk, Prose mode)"),
        Err(e) => eprintln!("push-to-talk hook failed to install: {e} — hold-to-talk unavailable, toggle hotkeys still work"),
    }

    let mut overlay_handle: Option<overlay::OverlayHandle> = None;
    let mut active_mode: Option<Mode> = None;
    let mut ptt_session = false; // true while the current recording was started by push-to-talk

    let mut msg = MSG::default();
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        // Thread-posted messages (PostThreadMessageW, from the keyboard
        // hook) carry a NULL hwnd, same as overlay.rs's WM_APP_STOP.
        let is_thread_posted = msg.hwnd.0.is_null();

        if is_thread_posted && msg.message == WM_APP_PTT_DOWN {
            if active_mode.is_none() {
                start_recording(
                    Mode::Prose, &mode, &tx, &recording, &audio_cmd_tx, &mut overlay_handle,
                    &overlay_text_tx, &writing_mode,
                );
                active_mode = Some(Mode::Prose);
                ptt_session = true;
                println!("[recording (Prose, push-to-talk) — release to stop]");
            }
            continue;
        }
        if is_thread_posted && msg.message == WM_APP_PTT_UP {
            if ptt_session {
                stop_recording(&tx, &recording, &audio_cmd_tx, &mut overlay_handle, &overlay_text_tx, &mode, &writing_mode);
                active_mode = None;
                ptt_session = false;
                println!("\n[stopped]");
            }
            continue;
        }
        if msg.message != WM_HOTKEY {
            continue;
        }
        let pressed_id = msg.wParam.0 as i32;
        let pressed_mode = if pressed_id == HOTKEY_ID {
            Mode::Prose
        } else if pressed_id == HOTKEY_ID_CMD {
            Mode::Command
        } else {
            continue;
        };

        match active_mode {
            Some(current) if current == pressed_mode => {
                if ptt_session {
                    println!("[currently held via push-to-talk — release Ctrl+Shift+Space to stop, not this hotkey]");
                    continue;
                }
                stop_recording(&tx, &recording, &audio_cmd_tx, &mut overlay_handle, &overlay_text_tx, &mode, &writing_mode);
                active_mode = None;
                println!("\n[stopped]");
            }
            Some(other) => {
                println!("[already recording in {other:?} mode — ignoring]");
            }
            None => {
                start_recording(
                    pressed_mode, &mode, &tx, &recording, &audio_cmd_tx, &mut overlay_handle,
                    &overlay_text_tx, &writing_mode,
                );
                active_mode = Some(pressed_mode);
                let stop_key = match pressed_mode {
                    Mode::Prose => hotkey_name,
                    Mode::Command => cmd_hotkey_name,
                };
                println!("[recording ({pressed_mode:?}) — {stop_key} to stop]");
            }
        }
    }

    if let Ok(hook) = ptt_hook {
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
    }
}

#[allow(clippy::too_many_arguments)] // plain plumbing fan-out from one call site
fn start_recording(
    target_mode: Mode,
    mode: &Arc<Mutex<Mode>>,
    tx: &Sender<NetCmd>,
    recording: &Arc<AtomicBool>,
    audio_cmd_tx: &Sender<AudioCmd>,
    overlay_handle: &mut Option<overlay::OverlayHandle>,
    overlay_text_tx: &Arc<Mutex<Option<Sender<String>>>>,
    writing_mode: &SharedMode,
) {
    *mode.lock().unwrap() = target_mode;
    let _ = tx.send(NetCmd::Begin);
    recording.store(true, Ordering::Relaxed);
    let (handle, amp_tx, text_tx) = overlay::spawn(writing_mode.clone());
    *overlay_handle = Some(handle);
    *overlay_text_tx.lock().unwrap() = Some(text_tx);
    let _ = audio_cmd_tx.send(AudioCmd::Start(amp_tx));
}

fn stop_recording(
    tx: &Sender<NetCmd>,
    recording: &Arc<AtomicBool>,
    audio_cmd_tx: &Sender<AudioCmd>,
    overlay_handle: &mut Option<overlay::OverlayHandle>,
    overlay_text_tx: &Arc<Mutex<Option<Sender<String>>>>,
    mode: &Arc<Mutex<Mode>>,
    writing_mode: &SharedMode,
) {
    recording.store(false, Ordering::Relaxed);
    let _ = audio_cmd_tx.send(AudioCmd::Stop);
    // Command-mode utterances are instructions matched against the command
    // grammar — writing-mode formatting (e.g. email greeting structure)
    // would corrupt them before matching, so they always go as dictation.
    let wire = match *mode.lock().unwrap() {
        Mode::Prose => writing_mode.get().wire(),
        Mode::Command => "dictation",
    };
    let _ = tx.send(NetCmd::End { mode: wire });
    *overlay_text_tx.lock().unwrap() = None;
    if let Some(h) = overlay_handle.take() {
        h.stop();
    }
}
