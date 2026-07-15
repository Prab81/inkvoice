//! STORY-015: standalone SendInput compatibility test.
//!
//! Launches a target app, waits for it to gain focus, then runs the exact
//! same `type_text` used by the real dictation pipeline. Screenshot the
//! result afterward to check what actually landed — this is meant to be
//! run once per target app, not automated end-to-end.
//!
//! Usage: cargo run --release --bin sendinput_test -- <app-exe> [args...]
//! Example: cargo run --release --bin sendinput_test -- wt.exe

#[path = "../text_insert.rs"]
#[allow(dead_code)] // this harness exercises the typed path only, not clipboard paste
mod text_insert;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: sendinput_test <app-exe> [args...]");
        std::process::exit(1);
    }

    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    println!("launching: {} {:?} (forcing a new console window)", args[0], &args[1..]);
    let _child = {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(&args[0])
            .args(&args[1..])
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .expect("failed to launch target app")
    };

    println!("waiting 2.5s for it to open and gain focus...");
    std::thread::sleep(std::time::Duration::from_millis(2500));

    // A realistic dictated-then-cleaned string: mixed case, punctuation,
    // and a trailing newline — the interesting case for terminals (see
    // text_insert.rs doc comment: \n is a Unicode LF character event, not
    // a VK_RETURN keypress).
    let test_text = "echo InkVoiceTest123\n";
    println!("typing: {:?}", test_text);
    text_insert::type_text(test_text);

    // Follow-up diagnostic: is the block specific to KEYEVENTF_UNICODE
    // character injection, or does SendInput fail for ANY event type
    // against this target (which would rule out a clipboard+Ctrl+V-paste
    // workaround, since that also goes through SendInput)?
    println!("testing a real VK-based keypress (VK_A, no KEYEVENTF_UNICODE)...");
    send_vk_keypress();

    println!("done — inspect the target window now");
    std::thread::sleep(std::time::Duration::from_millis(500));
}

fn send_vk_keypress() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };
    let vk = VIRTUAL_KEY(0x41); // VK_A
    let mk = |flags| INPUT {
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
    let inputs = [mk(Default::default()), mk(KEYEVENTF_KEYUP)];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        let err = unsafe { windows::Win32::Foundation::GetLastError() };
        println!("VK_A keypress: accepted {sent}/2 (GetLastError={:?})", err);
    } else {
        println!("VK_A keypress: accepted {sent}/2 (success)");
    }
}
