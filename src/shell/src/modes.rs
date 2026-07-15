//! Writing modes — how the sidecar formats the finalized transcript
//! (dictation = untouched, email = greeting/sign-off structure, message =
//! casual chat style, rewrite = local-LLM grammar/fluency pass via Ollama).
//! See src/sidecar/mode_format.py and ai_rewrite.py for the actual rules.
//! Rewrite adds real latency (a few hundred ms to a couple seconds even
//! warm) before text lands, since it's a network call to a local model —
//! accepted because it's opt-in, not the default mode.
//!
//! Selected by clicking the mode chip on the recording pill. Shared as an
//! atomic (not a Mutex) because the overlay's wnd_proc reads/cycles it from
//! the Win32 message loop thread while the main thread reads it when
//! sending the "end" message — a lock held across a render would be easy
//! to misuse there, an atomic can't deadlock.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WritingMode {
    Dictation = 0,
    Email = 1,
    Message = 2,
    Rewrite = 3,
}

impl WritingMode {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => WritingMode::Email,
            2 => WritingMode::Message,
            3 => WritingMode::Rewrite,
            _ => WritingMode::Dictation,
        }
    }

    pub fn next(self) -> Self {
        match self {
            WritingMode::Dictation => WritingMode::Email,
            WritingMode::Email => WritingMode::Message,
            WritingMode::Message => WritingMode::Rewrite,
            WritingMode::Rewrite => WritingMode::Dictation,
        }
    }

    /// Label shown on the pill's mode chip.
    pub fn label(self) -> &'static str {
        match self {
            WritingMode::Dictation => "Dictation",
            WritingMode::Email => "Email",
            WritingMode::Message => "Message",
            WritingMode::Rewrite => "Rewrite",
        }
    }

    /// Value sent to the sidecar in the "end" message.
    pub fn wire(self) -> &'static str {
        match self {
            WritingMode::Dictation => "dictation",
            WritingMode::Email => "email",
            WritingMode::Message => "message",
            WritingMode::Rewrite => "rewrite",
        }
    }
}

/// Cheaply cloneable handle to the current writing mode.
#[derive(Clone)]
pub struct SharedMode(Arc<AtomicU8>);

impl SharedMode {
    pub fn new() -> Self {
        SharedMode(Arc::new(AtomicU8::new(WritingMode::Dictation as u8)))
    }

    pub fn get(&self) -> WritingMode {
        WritingMode::from_u8(self.0.load(Ordering::Relaxed))
    }

    pub fn cycle(&self) -> WritingMode {
        let next = self.get().next();
        self.0.store(next as u8, Ordering::Relaxed);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_covers_all_modes_and_wraps() {
        let m = SharedMode::new();
        assert_eq!(m.get(), WritingMode::Dictation);
        assert_eq!(m.cycle(), WritingMode::Email);
        assert_eq!(m.cycle(), WritingMode::Message);
        assert_eq!(m.cycle(), WritingMode::Rewrite);
        assert_eq!(m.cycle(), WritingMode::Dictation);
    }

    #[test]
    fn wire_values_match_sidecar_mode_format_py_modes_tuple() {
        assert_eq!(WritingMode::Dictation.wire(), "dictation");
        assert_eq!(WritingMode::Email.wire(), "email");
        assert_eq!(WritingMode::Message.wire(), "message");
        assert_eq!(WritingMode::Rewrite.wire(), "rewrite");
    }
}
