//! Floating recording-indicator overlay.
//!
//! A borderless, always-on-top cream pill shown while dictating, with
//! three stacked regions:
//!   1. live waveform — three flowing sine ribbons (gold/indigo/azure),
//!      each a bright leading edge plus a fan of translucent trailing
//!      strands, amplitude driven by the smoothed mic level;
//!   2. the live partial transcript (two wrapping lines, showing the tail
//!      — the most recent words — when the utterance outgrows the space);
//!   3. a clickable mode chip cycling Dictation → Email → Message, which
//!      selects the sidecar's writing-mode formatting for the final text.
//!
//! Live-text history (STORY-002 AC2): v1 showed partials in the pill as a
//! cramped single line (user: not useful); v2 live-typed partials into the
//! focused document (rolled back — injected keystrokes while hotkey
//! modifiers were physically held fired app shortcuts); v3 (this) returns
//! the transcript to the pill with room to breathe — display-only, so the
//! entire injection failure class is structurally impossible here. The
//! pill grew (300x84 → 380x150) to fit; concept echoes FluidVoice's
//! adaptive live-preview overlay (github.com/altic-dev/FluidVoice).
//!
//! Because the mode chip must be clickable, the window is no longer
//! click-through (WS_EX_TRANSPARENT removed): clicks on the pill's opaque
//! pixels now land on us (layered windows hit-test per-pixel alpha).
//! WS_EX_NOACTIVATE is what actually matters for dictation and it stays —
//! clicking the chip never steals focus from the app receiving the text.
//!
//! FOUND LIVE (M2, v1): plain GDI + color-key transparency painted via raw
//! `GetDC`/`BitBlt` reported every individual call as successful yet
//! rendered as a solid black rectangle — the compositor never picked up
//! the drawn content. Switched to `UpdateLayeredWindow` with a real
//! per-pixel alpha buffer (see git history / CONTEXT.md for that finding).
//!
//! FOUND LIVE (M2, v2): the waveform looked noticeably rougher with real
//! microphone input than in the synthetic-sine-wave test used to validate
//! the rendering pipeline. Two fixes: (1) raw mic RMS is frame-to-frame
//! noisy, so amplitude is now temporally smoothed (EMA) before it ever
//! reaches the renderer; (2) waveform strokes are drawn with a soft
//! anti-aliased brush directly into the pixel buffer (not raw `Polyline`,
//! which is hard-edged/jagged) for a softer, more hand-drawn look.
//!
//! Lives on its own dedicated thread: window handles and message loops are
//! thread-affine in Win32, and this way the overlay's lifetime (spawned on
//! recording start, torn down on stop) is independent of both the Win32
//! hotkey message loop and the audio-owner thread.

use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::mpsc::{channel, Receiver, Sender};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW, GdiFlush,
    GetDC, ReleaseDC, SelectObject, SetBkMode, SetTextColor, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, DT_WORD_ELLIPSIS,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, GetWindowLongPtrW, PostThreadMessageW, RegisterClassW, SetTimer,
    SetWindowLongPtrW, ShowWindow, TranslateMessage, UpdateLayeredWindow, GWLP_USERDATA, MSG,
    SM_CXSCREEN, SM_CYSCREEN, SW_SHOWNOACTIVATE, ULW_ALPHA, WM_APP, WM_DESTROY, WM_LBUTTONDOWN,
    WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::Win32::Graphics::Gdi::{AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION};

use crate::modes::SharedMode;

const WIDTH: i32 = 380;
const HEIGHT: i32 = 150; // waveform + 2-line live transcript + mode chip
const PILL_RADIUS: i32 = 28;
const BOTTOM_MARGIN: i32 = 130;
const TIMER_ID: usize = 1;
const TIMER_MS: u32 = 33; // ~30fps
const HISTORY_LEN: usize = 90;
const AMP_SMOOTHING: f32 = 0.6; // EMA weight on the previous sample
const WM_APP_STOP: u32 = WM_APP + 1;

// Vertical layout bands.
const WAVE_TOP: i32 = 6;
const WAVE_BOTTOM: i32 = 68;
const TEXT_TOP: i32 = 70;
const TEXT_BOTTOM: i32 = 116;
const CHIP_RECT: (i32, i32, i32, i32) = (115, 120, 265, 144); // centered chip, generous hit area

const PILL_BG: (u8, u8, u8) = (245, 240, 227); // cream
const CHIP_BG: (u8, u8, u8) = (236, 229, 211); // cream-dark
const TEXT_COLOR: (u8, u8, u8) = (35, 33, 30); // ink
const HINT_COLOR: (u8, u8, u8) = (99, 94, 84); // ink-faint (post-contrast-fix shade)

/// (color, phase offset, cycles across the pill, travel speed) per ribbon.
/// Distinct freq/speed/phase per ribbon so they weave through each other
/// like the reference instead of moving in lockstep; one travels backwards.
const RIBBONS: [((u8, u8, u8), f32, f32, f32); 3] = [
    ((212, 158, 52), 0.0, 1.9, 1.0),   // gold
    ((76, 64, 185), 2.1, 2.4, -0.7),   // indigo
    ((41, 160, 218), 4.2, 1.5, 1.5),   // azure
];
const STRANDS: usize = 9; // leading edge + trailing fan per ribbon
const AMP_FLOOR: f32 = 0.12; // keeps ribbons alive during silence

/// FOUND LIVE (2026-07-07): normal speaking volume barely moved the
/// waveform — only shouting did. Root cause: mic RMS for ordinary speech
/// is small (roughly 0.02-0.08 on typical input gain) and was fed straight
/// into a linear 0..1 amplitude, so most conversational volume sat at or
/// below AMP_FLOOR and only got clipped to the floor value. Fixed with (1)
/// a gain multiplier so moderate levels reach a useful range before
/// clamping, and (2) a perceptual (sub-linear) curve — human loudness
/// perception and mic RMS are roughly logarithmic, so a sqrt-like curve
/// (exponent < 1) stretches the quiet-to-moderate range where speech
/// actually lives, instead of only the loud end.
const AMP_GAIN: f32 = 6.0;
const AMP_CURVE: f32 = 0.5; // exponent; <1 boosts quiet/moderate levels

fn colorref((r, g, b): (u8, u8, u8)) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

/// Standard rounded-rect containment test: distance from the point to the
/// nearest corner-circle-center, clamped into the rect's interior band.
fn point_in_pill(x: i32, y: i32) -> bool {
    let r = PILL_RADIUS;
    let cx = x.clamp(r, WIDTH - r);
    let cy = y.clamp(r, HEIGHT - r);
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= r * r
}

fn point_in_chip(x: i32, y: i32) -> bool {
    let (l, t, r, b) = CHIP_RECT;
    x >= l && x < r && y >= t && y < b
}

struct State {
    amp_rx: Receiver<f32>,
    text_rx: Receiver<String>,
    history: VecDeque<f32>,
    partial_text: String,
    mode: SharedMode,
    frame: u64, // drives the ribbons' horizontal travel
}

pub struct OverlayHandle {
    thread_id: u32,
    join: Option<std::thread::JoinHandle<()>>,
}

impl OverlayHandle {
    pub fn stop(mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_APP_STOP, WPARAM(0), LPARAM(0));
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Start the overlay on its own thread. Returns senders for the current
/// recording turn: one amplitude sample per audio callback, and the live
/// partial transcript whenever it updates. `mode` is shared with the main
/// thread — clicking the pill's chip cycles it.
pub fn spawn(mode: SharedMode) -> (OverlayHandle, Sender<f32>, Sender<String>) {
    let (amp_tx, amp_rx) = channel::<f32>();
    let (text_tx, text_rx) = channel::<String>();
    let (ready_tx, ready_rx) = channel::<u32>();

    let join = std::thread::spawn(move || {
        let hwnd = create_window();
        let state = Box::new(State {
            amp_rx,
            text_rx,
            history: VecDeque::with_capacity(HISTORY_LEN),
            partial_text: String::new(),
            mode,
            frame: 0,
        });
        let mode_label = state.mode.get().label();
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            SetTimer(hwnd, TIMER_ID, TIMER_MS, None);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        render(hwnd, &VecDeque::new(), 0.0, "", mode_label); // first frame immediately

        let _ = ready_tx.send(unsafe { GetCurrentThreadId() });

        let mut msg = MSG::default();
        loop {
            let ok = unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool();
            if !ok {
                break;
            }
            // Thread-posted messages (PostThreadMessageW) carry a NULL hwnd
            // and are never routed to a window procedure — handle here.
            if msg.hwnd.0.is_null() && msg.message == WM_APP_STOP {
                break;
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    });

    let thread_id = ready_rx.recv().unwrap_or(0);
    (
        OverlayHandle {
            thread_id,
            join: Some(join),
        },
        amp_tx,
        text_tx,
    )
}

fn create_window() -> HWND {
    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();
        let class_name = windows::core::w!("InkVoiceOverlay");

        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc); // ignore "already registered" on repeat spawns

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - WIDTH) / 2;
        let y = screen_h - HEIGHT - BOTTOM_MARGIN;

        // NOTE: no WS_EX_TRANSPARENT — the mode chip needs real clicks.
        // WS_EX_NOACTIVATE keeps focus in the app receiving dictation.
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name,
            PCWSTR::null(),
            WS_POPUP,
            x,
            y,
            WIDTH,
            HEIGHT,
            None,
            None,
            hinstance,
            None,
        )
        .unwrap()
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
            if let Some(state) = state_ptr.as_mut() {
                while let Ok(v) = state.amp_rx.try_recv() {
                    // Temporal smoothing: raw mic RMS is noisy frame-to-frame,
                    // which made the waveform look jittery with real speech
                    // even though a clean synthetic test looked smooth.
                    // Gain + perceptual curve applied to the raw RMS before
                    // smoothing, so the EMA settles on the boosted signal
                    // rather than smoothing a value that's already been
                    // clamped to the floor (see AMP_GAIN/AMP_CURVE docs).
                    let boosted = (v * AMP_GAIN).clamp(0.0, 1.0).powf(AMP_CURVE);
                    let smoothed = match state.history.back() {
                        Some(&last) => last * AMP_SMOOTHING + boosted * (1.0 - AMP_SMOOTHING),
                        None => boosted,
                    };
                    if state.history.len() >= HISTORY_LEN {
                        state.history.pop_front();
                    }
                    state.history.push_back(smoothed);
                }
                while let Ok(t) = state.text_rx.try_recv() {
                    state.partial_text = t;
                }
                state.frame += 1;
                let t = state.frame as f32 * 0.055; // radians per ~33ms tick
                render(hwnd, &state.history, t, &state.partial_text, state.mode.get().label());
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            if point_in_chip(x, y) {
                let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
                if let Some(state) = state_ptr.as_mut() {
                    let new_mode = state.mode.cycle();
                    // Immediate render so the chip label updates on click,
                    // not on the next 33ms timer tick.
                    render(hwnd, &state.history, state.frame as f32 * 0.055, &state.partial_text, new_mode.label());
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
            if !state_ptr.is_null() {
                drop(Box::from_raw(state_ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Build the frame in a 32bpp premultiplied-alpha DIB section and push it
/// to the compositor via `UpdateLayeredWindow`. No `WM_PAINT` involved —
/// this is the whole render path, called directly from the timer tick (and
/// once immediately at spawn so the pill doesn't wait for the first tick).
fn render(hwnd: HWND, history: &VecDeque<f32>, t: f32, partial_text: &str, mode_label: &str) {
    let (w, h) = (WIDTH, HEIGHT);
    unsafe {
        let screen_dc = GetDC(None);
        let mem_dc = CreateCompatibleDC(screen_dc);

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // negative = top-down, matches our y-down math
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let dib = match CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) {
            Ok(b) if !bits_ptr.is_null() => b,
            _ => {
                eprintln!("overlay: CreateDIBSection failed, skipping frame");
                let _ = DeleteDC(mem_dc);
                ReleaseDC(None, screen_dc);
                return;
            }
        };
        let old_bitmap = SelectObject(mem_dc, dib);

        let pixel_count = (w * h) as usize;
        let pixels = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, pixel_count);

        // 1. Base fill + alpha, driven by our own geometry test rather than
        //    "did GDI happen to touch this pixel" — gives exact, reliable
        //    control over which pixels are opaque cream vs. transparent.
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                pixels[idx] = if point_in_pill(x, y) {
                    pack_bgra(PILL_BG, 255)
                } else {
                    0
                };
            }
        }

        // 2. Waveform in its band, soft-brush anti-aliased, drawn straight
        //    into the pixel buffer (clipped to the pill via point_in_pill
        //    inside blend_pixel).
        draw_waveform(pixels, w, h, history, t);

        // 3. Mode chip background (flat rounded rect in the pixel buffer;
        //    its label is GDI text in step 4).
        fill_rounded_rect(pixels, w, h, CHIP_RECT, 11, CHIP_BG);

        // 4. GDI text: live partial transcript + mode chip label.
        draw_texts(mem_dc, partial_text, mode_label);

        // 5. FOUND LIVE (2026-07-08): GDI text output ZEROES the alpha byte
        //    of every pixel it touches — on a per-pixel-alpha layered window
        //    that turns each glyph into a transparent hole, so the "text"
        //    was actually whatever sat behind the pill (white app → white
        //    text, dark app → dark text; earlier versions only ever looked
        //    right by luck of the background). Restore full opacity across
        //    the pill interior after all GDI drawing. GdiFlush() first: GDI
        //    ops are batched and may not have hit the DIB bits yet.
        let _ = GdiFlush();
        for y in 0..h {
            for x in 0..w {
                if point_in_pill(x, y) {
                    pixels[(y * w + x) as usize] |= 0xFF00_0000;
                }
            }
        }

        let size = SIZE { cx: w, cy: h };
        let src_pt = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            screen_dc,
            None,
            Some(&size),
            mem_dc,
            Some(&src_pt),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(dib);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);
    }
}

fn pack_bgra((r, g, b): (u8, u8, u8), a: u8) -> u32 {
    (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | (b as u32)
}

/// Alpha-blend `color` into the pixel at (x, y) with the given coverage
/// (0.0..1.0), clipped to the pill. Alpha stays 255 throughout the pill's
/// interior by construction (set in the base fill pass) — only RGB needs
/// blending here.
fn blend_pixel(pixels: &mut [u32], w: i32, h: i32, x: i32, y: i32, color: (u8, u8, u8), coverage: f32) {
    if x < 0 || y < 0 || x >= w || y >= h || !point_in_pill(x, y) {
        return;
    }
    let c = coverage.clamp(0.0, 1.0);
    if c <= 0.0 {
        return;
    }
    let idx = (y * w + x) as usize;
    let old = pixels[idx];
    let old_b = (old & 0xFF) as f32;
    let old_g = ((old >> 8) & 0xFF) as f32;
    let old_r = ((old >> 16) & 0xFF) as f32;
    let nr = old_r * (1.0 - c) + color.0 as f32 * c;
    let ng = old_g * (1.0 - c) + color.1 as f32 * c;
    let nb = old_b * (1.0 - c) + color.2 as f32 * c;
    pixels[idx] = 0xFF00_0000 | ((nr as u32) << 16) | ((ng as u32) << 8) | (nb as u32);
}

/// Flat rounded rectangle blended into the pixel buffer (used for the mode
/// chip). Anti-aliased edge via 1px coverage falloff on the radius test.
fn fill_rounded_rect(pixels: &mut [u32], w: i32, h: i32, rect: (i32, i32, i32, i32), radius: i32, color: (u8, u8, u8)) {
    let (l, t, r, b) = rect;
    for y in t..b {
        for x in l..r {
            let cx = x.clamp(l + radius, r - 1 - radius);
            let cy = y.clamp(t + radius, b - 1 - radius);
            let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
            let dist = (dx * dx + dy * dy).sqrt();
            let coverage = ((radius as f32 + 0.5) - dist).clamp(0.0, 1.0);
            if coverage > 0.0 {
                blend_pixel(pixels, w, h, x, y, color, coverage);
            }
        }
    }
}

/// Keep the tail of the transcript when it outgrows the two-line text
/// area: the newest words are what the speaker needs to see. Cut at a
/// word boundary and mark the truncation with a leading ellipsis.
fn tail_for_display(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let tail: String = text
        .chars()
        .rev()
        .take(max_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    match tail.find(' ') {
        Some(sp) => format!("…{}", &tail[sp..]),
        None => format!("…{tail}"),
    }
}

fn draw_texts(dc: windows::Win32::Graphics::Gdi::HDC, partial_text: &str, mode_label: &str) {
    unsafe {
        let make_font = |height: i32, weight: i32| {
            CreateFontW(
                height, 0, 0, 0, weight, 0, 0, 0,
                windows::Win32::Graphics::Gdi::DEFAULT_CHARSET.0 as u32,
                windows::Win32::Graphics::Gdi::OUT_DEFAULT_PRECIS.0 as u32,
                windows::Win32::Graphics::Gdi::CLIP_DEFAULT_PRECIS.0 as u32,
                windows::Win32::Graphics::Gdi::ANTIALIASED_QUALITY.0 as u32,
                0,
                windows::core::w!("Segoe UI"),
            )
        };
        SetBkMode(dc, TRANSPARENT);

        // Live partial transcript — two wrapping lines, tail-trimmed.
        let text_font = make_font(19, 400);
        let old_font = SelectObject(dc, text_font);
        let (display, color) = if partial_text.is_empty() {
            ("Listening…".to_string(), HINT_COLOR)
        } else {
            (tail_for_display(partial_text, 96), TEXT_COLOR)
        };
        SetTextColor(dc, colorref(color));
        let mut rect = RECT {
            left: 26,
            top: TEXT_TOP,
            right: WIDTH - 26,
            bottom: TEXT_BOTTOM,
        };
        let mut wide: Vec<u16> = display.encode_utf16().collect();
        DrawTextW(dc, &mut wide, &mut rect, DT_CENTER | DT_WORDBREAK | DT_WORD_ELLIPSIS | DT_NOPREFIX);
        SelectObject(dc, old_font);
        let _ = DeleteObject(text_font);

        // Mode chip label.
        let chip_font = make_font(15, 600);
        let old_font = SelectObject(dc, chip_font);
        SetTextColor(dc, colorref(HINT_COLOR));
        let (l, t, r, b) = CHIP_RECT;
        let mut chip_rect = RECT { left: l, top: t, right: r, bottom: b };
        let label = format!("Mode: {mode_label}  ▸");
        let mut wide: Vec<u16> = label.encode_utf16().collect();
        DrawTextW(dc, &mut wide, &mut chip_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(dc, old_font);
        let _ = DeleteObject(chip_font);
    }
}

/// Three flowing sine ribbons in the style of the reference video: for
/// each ribbon, two edge curves are computed (the second phase-shifted and
/// damped relative to the first) and a fan of strands is interpolated
/// between them — a bright, slightly thicker leading edge plus fine
/// translucent trailing strands, which together read as a swept surface.
///
/// The mic level shapes a spatial amplitude envelope: the (smoothed)
/// history is sampled across x, floored at AMP_FLOOR so silence still
/// shows gentle motion, and multiplied by a sin(πu) window so the ribbons
/// pinch toward both ends of the pill like the reference. Time `t` slides
/// each ribbon's phase at its own speed (one backwards) so they weave.
/// Confined to the WAVE_TOP..WAVE_BOTTOM band now that the pill also holds
/// text below.
fn draw_waveform(pixels: &mut [u32], w: i32, h: i32, history: &VecDeque<f32>, t: f32) {
    use std::f32::consts::{PI, TAU};

    let band_h = (WAVE_BOTTOM - WAVE_TOP) as f32;
    let mid_y = WAVE_TOP as f32 + band_h / 2.0;
    let amp_scale = band_h * 0.42;
    let margin_x = (w as f32) * 0.08;
    let usable_w = w as f32 - 2.0 * margin_x;

    let n = history.len();
    let amp_at = |u: f32| -> f32 {
        if n < 2 {
            return AMP_FLOOR;
        }
        let pos = u * (n - 1) as f32;
        let j = pos.floor() as usize;
        let frac = pos - j as f32;
        let a = history[j];
        let b = history[(j + 1).min(n - 1)];
        (a + (b - a) * frac).clamp(AMP_FLOOR, 1.0)
    };

    const STEPS: usize = 56;
    for &(color, phase, cycles, speed) in RIBBONS.iter() {
        // Trailing strands first, leading edge last so it stays on top.
        for k in (0..STRANDS).rev() {
            let e = k as f32 / (STRANDS - 1) as f32; // 0 = leading edge
            let (radius, opacity) = if k == 0 {
                (1.7, 0.85)
            } else {
                (0.9, 0.05 + 0.22 * (1.0 - e))
            };
            let mut prev: Option<(f32, f32)> = None;
            for i in 0..=STEPS {
                let u = i as f32 / STEPS as f32;
                let x = margin_x + u * usable_w;
                let envelope = (PI * u).sin();
                let amp = amp_at(u) * amp_scale * envelope * (1.0 - 0.45 * e);
                let theta = u * cycles * TAU + phase + t * speed + 0.8 * e;
                let y = mid_y + amp * theta.sin();
                if let Some((px, py)) = prev {
                    draw_soft_segment(pixels, w, h, px, py, x, y, color, radius, opacity);
                }
                prev = Some((x, y));
            }
        }
    }
}

/// Soft circular-brush stroke from (x0,y0) to (x1,y1) — denser and softer
/// than a hard 1px `Polyline`, closer to the reference's hand-drawn look.
fn draw_soft_segment(pixels: &mut [u32], w: i32, h: i32, x0: f32, y0: f32, x1: f32, y1: f32, color: (u8, u8, u8), radius: f32, opacity: f32) {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let steps = (len * 1.5).ceil() as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let (x, y) = (x0 + dx * t, y0 + dy * t);
        let x_min = (x - radius).floor() as i32;
        let x_max = (x + radius).ceil() as i32;
        let y_min = (y - radius).floor() as i32;
        let y_max = (y + radius).ceil() as i32;
        for py in y_min..=y_max {
            for px in x_min..=x_max {
                let (ddx, ddy) = (px as f32 + 0.5 - x, py as f32 + 0.5 - y);
                let dist = (ddx * ddx + ddy * ddy).sqrt();
                let coverage = (1.0 - dist / radius).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    blend_pixel(pixels, w, h, px, py, color, coverage * opacity);
                }
            }
        }
    }
}
