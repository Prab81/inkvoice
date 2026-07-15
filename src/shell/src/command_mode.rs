//! Command Mode — STORY-009.
//!
//! A hands-free editing mode distinct from prose dictation, entered via its
//! own hotkey (not voice — "say command mode to enter" is unreliable to
//! detect and adds real complexity; a second hotkey is simple and certain).
//! When a Command Mode recording stops, the transcribed text is matched
//! against a small fixed set of command phrases (not a real grammar/intent
//! parser — that's a much bigger lift, tracked as a future refinement) and
//! dispatched as real key combos.
//!
//! Matching is deliberately simple substring checks, longest/most-specific
//! phrase first, same philosophy as cleanup.py's cue-phrase matching:
//! conservative rather than clever, since a wrong action (deleting the
//! wrong thing) is worse than no action.
//!
//! "New line" dispatches a real VK_RETURN keypress, not a Unicode LF
//! character — a lesson directly from STORY-015's terminal testing, where
//! a literal LF character doesn't submit/act the way a real Enter keypress
//! does in terminal-hosted apps.
//!
//! User-extensible commands (STORY-009 AC3): `custom_commands.json`, next to
//! the exe, lets a user bind their own phrase to an arbitrary key combo
//! without touching code or recompiling — e.g. `{"phrase": "save file",
//! "modifiers": ["ctrl"], "key": "s"}`. Checked BEFORE the built-in set, so
//! a user can override a built-in phrase if they want. Parsed with
//! `serde_json::Value` rather than a derived struct — this project already
//! depends on serde_json but not serde-with-derive, and hand-parsing a
//! handful of fields isn't worth a new dependency.

use std::path::PathBuf;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_0,
    VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_B, VK_BACK, VK_C, VK_CONTROL,
    VK_D, VK_DELETE, VK_DOWN, VK_E, VK_END, VK_ESCAPE, VK_F, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2,
    VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_G, VK_H, VK_HOME, VK_I, VK_J, VK_K, VK_L,
    VK_LEFT, VK_LWIN, VK_M, VK_MENU, VK_N, VK_O, VK_OEM_COMMA, VK_OEM_PERIOD, VK_P, VK_Q, VK_R,
    VK_RETURN, VK_RIGHT, VK_S, VK_SHIFT, VK_SPACE, VK_T, VK_TAB, VK_U, VK_UP, VK_V, VK_W, VK_X,
    VK_Y, VK_Z,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Undo,
    Redo,
    SelectWord,
    SelectSentence,
    SelectParagraph,
    DeleteWord,
    DeleteSentence,
    DeleteParagraph,
    NewLine,
    GoToStart,
    GoToEnd,
    /// A user-defined binding from custom_commands.json: (modifiers, key),
    /// resolved once at match time so dispatch doesn't need to re-parse.
    Custom(Vec<VIRTUAL_KEY>, VIRTUAL_KEY),
}

struct CustomCommand {
    phrase: String,
    modifiers: Vec<VIRTUAL_KEY>,
    key: VIRTUAL_KEY,
}

/// Virtual-key name -> code, for the keys a custom binding is likely to
/// need. Not exhaustive (no OEM punctuation beyond comma/period — see
/// main.rs's FOUND LIVE note on why OEM punctuation is layout-risky for
/// hotkeys; the same caution applies here, so it's deliberately omitted
/// rather than silently unreliable).
fn vk_from_name(name: &str) -> Option<VIRTUAL_KEY> {
    Some(match name.to_lowercase().as_str() {
        "a" => VK_A, "b" => VK_B, "c" => VK_C, "d" => VK_D, "e" => VK_E, "f" => VK_F,
        "g" => VK_G, "h" => VK_H, "i" => VK_I, "j" => VK_J, "k" => VK_K, "l" => VK_L,
        "m" => VK_M, "n" => VK_N, "o" => VK_O, "p" => VK_P, "q" => VK_Q, "r" => VK_R,
        "s" => VK_S, "t" => VK_T, "u" => VK_U, "v" => VK_V, "w" => VK_W, "x" => VK_X,
        "y" => VK_Y, "z" => VK_Z,
        "0" => VK_0, "1" => VK_1, "2" => VK_2, "3" => VK_3, "4" => VK_4,
        "5" => VK_5, "6" => VK_6, "7" => VK_7, "8" => VK_8, "9" => VK_9,
        "f1" => VK_F1, "f2" => VK_F2, "f3" => VK_F3, "f4" => VK_F4, "f5" => VK_F5,
        "f6" => VK_F6, "f7" => VK_F7, "f8" => VK_F8, "f9" => VK_F9, "f10" => VK_F10,
        "f11" => VK_F11, "f12" => VK_F12,
        "space" => VK_SPACE, "enter" | "return" => VK_RETURN, "tab" => VK_TAB,
        "escape" | "esc" => VK_ESCAPE, "backspace" => VK_BACK, "delete" | "del" => VK_DELETE,
        "home" => VK_HOME, "end" => VK_END,
        "up" => VK_UP, "down" => VK_DOWN, "left" => VK_LEFT, "right" => VK_RIGHT,
        "comma" => VK_OEM_COMMA, "period" | "dot" => VK_OEM_PERIOD,
        _ => return None,
    })
}

fn modifier_from_name(name: &str) -> Option<VIRTUAL_KEY> {
    Some(match name.to_lowercase().as_str() {
        "ctrl" | "control" => VK_CONTROL,
        "shift" => VK_SHIFT,
        "alt" => VK_MENU,
        "win" | "windows" | "meta" => VK_LWIN,
        _ => return None,
    })
}

/// Directory to look for custom_commands.json in: next to the exe, so it
/// travels with the portable package regardless of what directory the app
/// happens to be launched from.
fn config_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
}

/// Load and validate custom_commands.json. Missing file = no custom
/// commands (not an error — this is an opt-in feature). Malformed entries
/// are skipped individually with a warning rather than discarding the
/// whole file, so one typo doesn't silently disable everything else.
fn load_custom_commands() -> Vec<CustomCommand> {
    let path = config_dir().join("custom_commands.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        eprintln!("custom_commands.json: invalid JSON, ignoring (path: {})", path.display());
        return Vec::new();
    };
    let Some(entries) = json.get("commands").and_then(|c| c.as_array()) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in entries {
        let Some(phrase) = entry.get("phrase").and_then(|p| p.as_str()) else {
            eprintln!("custom_commands.json: entry missing \"phrase\", skipping: {entry}");
            continue;
        };
        let Some(key_name) = entry.get("key").and_then(|k| k.as_str()) else {
            eprintln!("custom_commands.json: entry \"{phrase}\" missing \"key\", skipping");
            continue;
        };
        let Some(key) = vk_from_name(key_name) else {
            eprintln!("custom_commands.json: entry \"{phrase}\" has unknown key \"{key_name}\", skipping");
            continue;
        };
        let modifiers: Vec<VIRTUAL_KEY> = entry
            .get("modifiers")
            .and_then(|m| m.as_array())
            .map(|arr| arr.iter().filter_map(|m| m.as_str().and_then(modifier_from_name)).collect())
            .unwrap_or_default();
        out.push(CustomCommand { phrase: phrase.to_lowercase(), modifiers, key });
    }
    // Longest phrase first, same discipline as the built-in set, so a
    // specific user phrase isn't shadowed by a shorter one that happens to
    // be a substring of it.
    out.sort_by_key(|c| std::cmp::Reverse(c.phrase.len()));
    out
}

/// Match transcribed text against custom commands first, then the built-in
/// set. Returns None for anything unrecognized — no-op is always safer
/// than guessing.
pub fn match_command(text: &str) -> Option<Action> {
    let t = text.to_lowercase();

    for custom in load_custom_commands() {
        if t.contains(&custom.phrase) {
            return Some(Action::Custom(custom.modifiers, custom.key));
        }
    }

    // Longest/most-specific phrases first so e.g. "select sentence" isn't
    // shadowed by a looser "select" + "word" check matching first.
    if t.contains("select") && t.contains("paragraph") {
        Some(Action::SelectParagraph)
    } else if t.contains("select") && t.contains("sentence") {
        Some(Action::SelectSentence)
    } else if t.contains("select") && t.contains("word") {
        Some(Action::SelectWord)
    } else if t.contains("delete") && t.contains("paragraph") {
        Some(Action::DeleteParagraph)
    } else if t.contains("delete") && t.contains("sentence") {
        Some(Action::DeleteSentence)
    } else if t.contains("delete") && (t.contains("word") || t.contains("that")) {
        Some(Action::DeleteWord)
    } else if t.contains("undo") {
        Some(Action::Undo)
    } else if t.contains("redo") {
        Some(Action::Redo)
    } else if t.contains("new line") || t.contains("newline") {
        Some(Action::NewLine)
    } else if t.contains("go to start") || t.contains("go to beginning") || t.contains("beginning") {
        Some(Action::GoToStart)
    } else if t.contains("go to end") {
        Some(Action::GoToEnd)
    } else {
        None
    }
}

pub fn dispatch(action: Action) {
    match action {
        Action::Undo => send_combo(&[VK_CONTROL], VK_Z),
        Action::Redo => send_combo(&[VK_CONTROL], VK_Y),
        Action::SelectWord => send_combo(&[VK_CONTROL, VK_SHIFT], VK_LEFT),
        Action::DeleteWord => send_combo(&[VK_CONTROL], VK_BACK),
        Action::NewLine => send_combo(&[], VK_RETURN),
        Action::GoToStart => send_combo(&[VK_CONTROL], VK_HOME),
        Action::GoToEnd => send_combo(&[VK_CONTROL], VK_END),
        // These are two-step approximations (no real "sentence" concept at
        // the OS text-editing level): jump to line start, then
        // shift-select/delete to line end. Good enough for a single-line
        // sentence; won't span multiple lines correctly — known limitation.
        Action::SelectSentence => {
            send_combo(&[], VK_HOME);
            send_combo(&[VK_SHIFT], VK_END);
        }
        Action::DeleteSentence => {
            send_combo(&[], VK_HOME);
            send_combo(&[VK_SHIFT], VK_END);
            send_combo(&[], VK_BACK);
        }
        // Paragraph nav has no OS-universal concept either; Ctrl+Up/Down is
        // the closest common ground (Word, VS Code, modern Notepad, many
        // other editors treat it as "jump to blank-line-delimited
        // paragraph boundary"). Won't work in apps that bind Ctrl+Up/Down
        // to something else (e.g. some browsers/terminals) — same class of
        // known limitation as the sentence approximation above.
        Action::SelectParagraph => {
            send_combo(&[VK_CONTROL], VK_UP);
            send_combo(&[VK_CONTROL, VK_SHIFT], VK_DOWN);
        }
        Action::DeleteParagraph => {
            send_combo(&[VK_CONTROL], VK_UP);
            send_combo(&[VK_CONTROL, VK_SHIFT], VK_DOWN);
            send_combo(&[], VK_BACK);
        }
        Action::Custom(modifiers, key) => send_combo(&modifiers, key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(text: &str) -> Option<&'static str> {
        match match_command(text)? {
            Action::Undo => Some("undo"),
            Action::Redo => Some("redo"),
            Action::SelectWord => Some("select_word"),
            Action::SelectSentence => Some("select_sentence"),
            Action::SelectParagraph => Some("select_paragraph"),
            Action::DeleteWord => Some("delete_word"),
            Action::DeleteSentence => Some("delete_sentence"),
            Action::DeleteParagraph => Some("delete_paragraph"),
            Action::NewLine => Some("new_line"),
            Action::GoToStart => Some("go_to_start"),
            Action::GoToEnd => Some("go_to_end"),
            Action::Custom(..) => Some("custom"),
        }
    }

    #[test]
    fn matches_core_verbs() {
        assert_eq!(kind("undo"), Some("undo"));
        assert_eq!(kind("Undo."), Some("undo"));
        assert_eq!(kind("redo that"), Some("redo"));
        assert_eq!(kind("select word"), Some("select_word"));
        assert_eq!(kind("select the last word"), Some("select_word"));
        assert_eq!(kind("select sentence"), Some("select_sentence"));
        assert_eq!(kind("delete that"), Some("delete_word"));
        assert_eq!(kind("delete the word"), Some("delete_word"));
        assert_eq!(kind("delete sentence"), Some("delete_sentence"));
        assert_eq!(kind("new line"), Some("new_line"));
        assert_eq!(kind("newline"), Some("new_line"));
        assert_eq!(kind("go to start"), Some("go_to_start"));
        assert_eq!(kind("go to the beginning"), Some("go_to_start"));
        assert_eq!(kind("go to end"), Some("go_to_end"));
    }

    #[test]
    fn sentence_beats_word_when_both_present() {
        // "select sentence" must not be shadowed by a looser word-level match.
        assert_eq!(kind("select this whole sentence, not just a word"), Some("select_sentence"));
    }

    #[test]
    fn matches_paragraph_commands() {
        assert_eq!(kind("select paragraph"), Some("select_paragraph"));
        assert_eq!(kind("delete this paragraph"), Some("delete_paragraph"));
    }

    #[test]
    fn no_custom_commands_file_falls_back_to_builtins() {
        // No custom_commands.json exists in the test working directory, so
        // this must resolve via the built-in set, not silently match None.
        assert_eq!(kind("undo"), Some("undo"));
    }

    #[test]
    fn unrecognized_text_is_none() {
        assert_eq!(match_command("hello my name is Vihaan"), None);
        assert_eq!(match_command(""), None);
        assert_eq!(match_command("this is just ordinary dictation"), None);
    }
}

/// Press modifiers down, tap `key`, release modifiers — one SendInput batch.
///
/// Waits for the user's own modifier keys to be physically released first:
/// a still-held Shift from the stop hotkey would silently turn e.g. the
/// dispatched Ctrl+Z into Ctrl+Shift+Z — same failure class as dictated
/// commas becoming Ctrl+, (see `wait_for_physical_modifier_release`).
fn send_combo(modifiers: &[VIRTUAL_KEY], key: VIRTUAL_KEY) {
    crate::text_insert::wait_for_physical_modifier_release();
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
    let mut inputs = Vec::new();
    for &m in modifiers {
        inputs.push(mk(m, Default::default()));
    }
    inputs.push(mk(key, Default::default()));
    inputs.push(mk(key, KEYEVENTF_KEYUP));
    for &m in modifiers.iter().rev() {
        inputs.push(mk(m, KEYEVENTF_KEYUP));
    }
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        eprintln!(
            "command_mode: SendInput accepted {sent}/{} events for {:?}+{:?}",
            inputs.len(), modifiers, key
        );
    }
}
