// SPDX-License-Identifier: GPL-3.0-or-later
//! The result bubble: a borderless popup that is always on top, never takes
//! focus and lets clicks fall through to the app underneath. Colours match
//! linux/extension/expresso@bulrok/stylesheet.css.

use std::cell::RefCell;

use windows::core::{w, Result};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, DrawTextW,
    EndPaint, FillRect, FrameRgn, GetDC, GetMonitorInfoW, MonitorFromRect, ReleaseDC, SelectObject,
    SetBkMode, SetTextColor, SetWindowRgn, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DT_CALCRECT, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FF_MODERN, FW_BOLD, HFONT,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, OUT_DEFAULT_PRECIS, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos,
    ShowWindow, HWND_TOPMOST, LWA_ALPHA, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WM_PAINT,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_POPUP,
};

use expresso::layout::{place_bubble, Rect, GAP};

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}
const BACKGROUND: COLORREF = rgb(0x2b, 0x1d, 0x16); // espresso
const BORDER: COLORREF = rgb(0x8a, 0x5a, 0x3c);
const TEXT: COLORREF = rgb(0xf3, 0xd9, 0xb1); // crema
const OPACITY: u8 = 240;

thread_local! {
    /// Text currently shown, as UTF-16 for GDI. Read by the window procedure.
    static LABEL: RefCell<Vec<u16>> = const { RefCell::new(Vec::new()) };
}

pub struct Bubble {
    hwnd: HWND,
}

impl Bubble {
    pub fn new() -> Result<Self> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                lpszClassName: w!("ExpressoBubble"),
                ..Default::default()
            };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                w!("ExpressoBubble"),
                w!("Expresso"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance.into()),
                None,
            )?;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), OPACITY, LWA_ALPHA)?;
            Ok(Self { hwnd })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Show `text` beside `caret` (screen coordinates, physical pixels).
    pub fn show(&self, text: &str, caret: Rect) {
        LABEL.with(|l| *l.borrow_mut() = text.encode_utf16().collect());
        unsafe {
            let scale = |px: i32| px * GetDpiForWindow(self.hwnd) as i32 / 96;
            let (tw, th) = self.measure();
            let size = (tw + scale(16), th + scale(4));

            let caret_rc = RECT {
                left: caret.x,
                top: caret.y,
                right: caret.x + caret.w.max(1),
                bottom: caret.y + caret.h.max(1),
            };
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(
                MonitorFromRect(&caret_rc, MONITOR_DEFAULTTONEAREST),
                &mut info,
            );
            let wa = info.rcWork;
            let area = Rect {
                x: wa.left,
                y: wa.top,
                w: wa.right - wa.left,
                h: wa.bottom - wa.top,
            };

            let (x, y) = place_bubble(caret, size, area, scale(GAP));
            let radius = scale(18);
            let rgn = CreateRoundRectRgn(0, 0, size.0 + 1, size.1 + 1, radius, radius);
            SetWindowRgn(self.hwnd, Some(rgn), false); // the system owns rgn now
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                size.0,
                size.1,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(self.hwnd), None, true);
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    /// Size of the current label in the bubble's font.
    fn measure(&self) -> (i32, i32) {
        unsafe {
            let hdc = GetDC(Some(self.hwnd));
            let font = font_for(self.hwnd);
            let old = SelectObject(hdc, font.into());
            let mut rc = RECT::default();
            LABEL.with(|l| {
                DrawTextW(
                    hdc,
                    &mut l.borrow_mut(),
                    &mut rc,
                    DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
                );
            });
            SelectObject(hdc, old);
            let _ = DeleteObject(font.into());
            ReleaseDC(Some(self.hwnd), hdc);
            (rc.right - rc.left, rc.bottom - rc.top)
        }
    }
}

/// Bold monospace at 11 pt, scaled for the window's monitor.
unsafe fn font_for(hwnd: HWND) -> HFONT {
    let height = -(11 * GetDpiForWindow(hwnd) as i32 / 72);
    CreateFontW(
        height,
        0,
        0,
        0,
        FW_BOLD.0 as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        FF_MODERN.0 as u32,
        w!("Consolas"),
    )
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg != WM_PAINT {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rc = RECT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rc);

    let bg = CreateSolidBrush(BACKGROUND);
    FillRect(hdc, &rc, bg);
    let _ = DeleteObject(bg.into());

    let radius = 18 * GetDpiForWindow(hwnd) as i32 / 96;
    let outline = CreateRoundRectRgn(0, 0, rc.right + 1, rc.bottom + 1, radius, radius);
    let border = CreateSolidBrush(BORDER);
    let _ = FrameRgn(hdc, outline, border, 1, 1);
    let _ = DeleteObject(border.into());
    let _ = DeleteObject(outline.into());

    let font = font_for(hwnd);
    let old = SelectObject(hdc, font.into());
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, TEXT);
    LABEL.with(|l| {
        DrawTextW(
            hdc,
            &mut l.borrow_mut(),
            &mut rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    });
    SelectObject(hdc, old);
    let _ = DeleteObject(font.into());
    let _ = EndPaint(hwnd, &ps);
    LRESULT(0)
}
