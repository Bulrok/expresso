// SPDX-License-Identifier: GPL-3.0-or-later
//! Win32 side of Expresso: the bubble window, the text watcher, and the
//! message loop that ties them together. Everything runs on one UI thread.

mod overlay;
mod watcher;

use windows::core::w;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetCursorPos, GetMessageW, KillTimer, MessageBoxW, PostQuitMessage, SetTimer,
    TranslateMessage, MB_ICONINFORMATION, MB_OK, MSG, WM_TIMER,
};

use expresso::calc::find_and_evaluate;
use expresso::layout::Rect;

/// Sample lines for `--demo`, shown one after another beside the mouse pointer.
const DEMO: &[&str] = &[
    "total 12*4.5",
    "200 + 15%",
    "2pi =",
    "1,5 * 2",
    "sqrt(2)",
    "hello world",
];
const DEMO_TIMER: usize = 1;

pub fn run() {
    // Caret rectangles from UI Automation are in physical pixels; without
    // per-monitor awareness Windows would rescale our window behind our back.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let bubble = match overlay::Bubble::new() {
        Ok(b) => b,
        Err(e) => return fatal(&format!("Could not create the bubble window: {e}")),
    };

    if std::env::args().any(|a| a == "--demo") {
        demo(&bubble);
        return;
    }

    if let Err(e) = watcher::start(&bubble) {
        return fatal(&format!(
            "{e}\n\nThis build can show its bubble (run with --demo) but cannot read text fields yet."
        ));
    }
    message_loop(|_| {});
}

fn demo(bubble: &overlay::Bubble) {
    let mut step = 0usize;
    let show = |step: usize| {
        let mut p = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut p);
        }
        let caret = Rect {
            x: p.x,
            y: p.y,
            w: 1,
            h: 20,
        };
        match find_and_evaluate(DEMO[step]) {
            Some(ev) => bubble.show(&format!("= {}", ev.display), caret),
            None => bubble.hide(),
        }
    };
    show(step);
    unsafe {
        SetTimer(Some(bubble.hwnd()), DEMO_TIMER, 1500, None);
    }
    message_loop(|msg| {
        if msg.message == WM_TIMER && msg.wParam.0 == DEMO_TIMER {
            step += 1;
            if step == DEMO.len() {
                unsafe {
                    let _ = KillTimer(Some(bubble.hwnd()), DEMO_TIMER);
                    PostQuitMessage(0);
                }
            } else {
                show(step);
            }
        }
    });
}

fn message_loop(mut on_msg: impl FnMut(&MSG)) {
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            on_msg(&msg);
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn fatal(text: &str) {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    unsafe {
        MessageBoxW(
            None::<HWND>,
            windows::core::PCWSTR(wide.as_ptr()),
            w!("Expresso"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
