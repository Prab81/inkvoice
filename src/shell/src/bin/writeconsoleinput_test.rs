//! Diagnostic: does WriteConsoleInput reach a Windows-Terminal-hosted shell?
//!
//! WindowsTerminal.exe (the GUI) typically does NOT itself own a console —
//! the actual console/pseudoconsole belongs to the child shell process
//! (powershell.exe, cmd.exe, wsl.exe, OpenConsole.exe) it hosts. This probe
//! finds the foreground window's process, walks its descendants, and tries
//! AttachConsole + WriteConsoleInput against each candidate PID, reporting
//! which one (if any) actually accepts it.
//!
//! Usage: cargo run --release --bin writeconsoleinput_test
//! (focus the target terminal window first, then run this within ~3s)

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Console::{
    AttachConsole, FreeConsole, GetStdHandle, WriteConsoleInputW, INPUT_RECORD, INPUT_RECORD_0,
    KEY_EVENT, KEY_EVENT_RECORD, KEY_EVENT_RECORD_0, STD_INPUT_HANDLE,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

fn all_processes() -> Vec<(u32, u32, String)> {
    // (pid, parent_pid, exe_name)
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(
                    &entry.szExeFile[..entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(0)],
                );
                out.push((entry.th32ProcessID, entry.th32ParentProcessID, name));
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

fn descendants(root_pid: u32, procs: &[(u32, u32, String)]) -> Vec<(u32, String)> {
    let mut result = vec![];
    let mut frontier = vec![root_pid];
    while let Some(pid) = frontier.pop() {
        for (cpid, ppid, name) in procs {
            if *ppid == pid {
                result.push((*cpid, name.clone()));
                frontier.push(*cpid);
            }
        }
    }
    result
}

fn try_write_console_input(pid: u32) -> Result<(), String> {
    unsafe {
        let _ = FreeConsole();
        AttachConsole(pid).map_err(|e| format!("AttachConsole failed: {e}"))?;
        let stdin = GetStdHandle(STD_INPUT_HANDLE).map_err(|e| format!("GetStdHandle: {e}"))?;

        let mut record = INPUT_RECORD {
            EventType: KEY_EVENT as u16,
            Event: INPUT_RECORD_0 {
                KeyEvent: KEY_EVENT_RECORD {
                    bKeyDown: true.into(),
                    wRepeatCount: 1,
                    wVirtualKeyCode: 0,
                    wVirtualScanCode: 0,
                    uChar: KEY_EVENT_RECORD_0 { UnicodeChar: 'X' as u16 },
                    dwControlKeyState: 0,
                },
            },
        };
        let mut written = 0u32;
        let ok = WriteConsoleInputW(stdin, &[record], &mut written);
        record.Event.KeyEvent.bKeyDown = false.into();
        let _ = WriteConsoleInputW(stdin, &[record], &mut written);

        let result = if ok.is_ok() && written > 0 {
            Ok(())
        } else {
            Err(format!("WriteConsoleInputW wrote {written} records, ok={:?}", ok))
        };
        let _ = FreeConsole();
        result
    }
}

fn main() {
    println!("Focus your target terminal window NOW — probing in 3s...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let fg = unsafe { GetForegroundWindow() };
    let mut fg_pid = 0u32;
    unsafe { GetWindowThreadProcessId(fg, Some(&mut fg_pid)) };
    println!("foreground window's process id: {fg_pid}");

    let procs = all_processes();
    let fg_name = procs
        .iter()
        .find(|(pid, _, _)| *pid == fg_pid)
        .map(|(_, _, n)| n.clone())
        .unwrap_or_else(|| "?".into());
    println!("foreground process: {fg_name} (pid {fg_pid})");

    let mut candidates = vec![(fg_pid, fg_name)];
    candidates.extend(descendants(fg_pid, &procs));

    for (pid, name) in &candidates {
        match try_write_console_input(*pid) {
            Ok(()) => println!("PID {pid} ({name}): SUCCESS — check the terminal for an 'X'"),
            Err(e) => println!("PID {pid} ({name}): failed — {e}"),
        }
    }
}
