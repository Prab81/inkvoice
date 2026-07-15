//! One-off diagnostic: try registering a batch of candidate global hotkeys
//! and report which ones are actually free on this machine right now.
//! Run: cargo run --release --bin hotkey_probe

use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT,
    MOD_WIN, VK_0, VK_9, VK_F13, VK_F19, VK_OEM_3, VK_SPACE, VK_TAB,
};

fn main() {
    let candidates: Vec<(&str, HOT_KEY_MODIFIERS, u32)> = vec![
        ("Ctrl+Alt+Tab", MOD_CONTROL | MOD_ALT, VK_TAB.0 as u32),
        ("Ctrl+Alt+Space", MOD_CONTROL | MOD_ALT, VK_SPACE.0 as u32),
        ("Ctrl+Alt+`", MOD_CONTROL | MOD_ALT, VK_OEM_3.0 as u32),
        ("Ctrl+Shift+Space", MOD_CONTROL | MOD_SHIFT, VK_SPACE.0 as u32),
        ("Ctrl+Alt+9", MOD_CONTROL | MOD_ALT, VK_9.0 as u32),
        ("Ctrl+Alt+0", MOD_CONTROL | MOD_ALT, VK_0.0 as u32),
        ("Win+Alt+Space", MOD_WIN | MOD_ALT, VK_SPACE.0 as u32),
        ("Win+Space", MOD_WIN, VK_SPACE.0 as u32),
        ("Alt+Space", MOD_ALT, VK_SPACE.0 as u32),
        ("F13 (no modifier)", HOT_KEY_MODIFIERS(0), VK_F13.0 as u32),
        ("F19 (no modifier)", HOT_KEY_MODIFIERS(0), VK_F19.0 as u32),
        ("Ctrl+Alt+F13", MOD_CONTROL | MOD_ALT, VK_F13.0 as u32),
    ];

    for (i, (name, mods, vk)) in candidates.iter().enumerate() {
        let id = i as i32 + 1;
        let ok = unsafe { RegisterHotKey(None, id, *mods, *vk) }.is_ok();
        println!("{:<22} {}", name, if ok { "FREE" } else { "TAKEN / reserved" });
        if ok {
            unsafe {
                let _ = UnregisterHotKey(None, id);
            }
        }
    }
}
