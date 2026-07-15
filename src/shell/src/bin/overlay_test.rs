//! Standalone harness to visually debug the overlay without needing the
//! mic/hotkey pipeline running. Shows the overlay with a synthetic sine-wave
//! amplitude feed for a few seconds, then exits.
//!
//! Run: cargo run --release --bin overlay_test

#[path = "../modes.rs"]
#[allow(dead_code)] // this harness never sends the mode to a sidecar, so wire() is unused here
mod modes;
#[path = "../overlay.rs"]
mod overlay;

fn main() {
    let (handle, amp_tx, text_tx) = overlay::spawn(modes::SharedMode::new());
    println!("overlay shown — synthetic waveform + growing transcript for ~6s");
    println!("click the mode chip to cycle Dictation → Email → Message");
    let words = [
        "testing", "testing the", "testing the live", "testing the live transcript",
        "testing the live transcript area", "testing the live transcript area with",
        "testing the live transcript area with a", "testing the live transcript area with a longer",
        "testing the live transcript area with a longer utterance that should wrap and then tail-trim",
    ];
    for i in 0..180 {
        let phase = i as f32 * 0.15;
        let amp = (phase.sin() * 0.5 + 0.5).abs();
        let _ = amp_tx.send(amp);
        let _ = text_tx.send(words[(i / 20).min(words.len() - 1)].to_string());
        std::thread::sleep(std::time::Duration::from_millis(33));
    }
    println!("stopping overlay");
    handle.stop();
}
