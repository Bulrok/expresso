// SPDX-License-Identifier: GPL-3.0-or-later
//! Finds the focused text field and reports the text before its caret plus
//! the caret's screen rectangle. The Windows counterpart of the GNOME
//! extension's `Main.inputMethod` signals.
//!
//! Plan (UI Automation, out of process, no injection):
//!  1. `CoCreateInstance(CUIAutomation8)` on the UI thread (STA).
//!  2. `AddFocusChangedEventHandler`: on focus, keep the element if it
//!     supports `TextPattern2` and `UIA_IsPasswordPropertyId` is false;
//!     otherwise hide the bubble.
//!  3. On that element, `AddAutomationEventHandler` for
//!     `UIA_Text_TextChangedEventId` and `UIA_Text_TextSelectionChangedEventId`.
//!  4. On each event: `TextPattern2::GetCaretRange` gives the caret. Expand a
//!     clone to `TextUnit_Line`, move its end to the caret's start, and
//!     `GetText(300)` is the line before the caret. `GetBoundingRectangles`
//!     on the caret range gives the rectangle (physical pixels).
//!  5. Fall back to `GetGUIThreadInfo().rcCaret` for the rectangle in classic
//!     Win32 apps that draw a system caret but report no UIA bounds.
//!  6. Events arrive on UIA's worker threads: marshal results to the UI
//!     thread with `PostMessageW` and coalesce to one update per message,
//!     as the extension does with `Meta.LaterType.BEFORE_REDRAW`.
//!  7. Dismiss on Enter/Escape via a `WH_KEYBOARD_LL` hook, as the extension
//!     does through its IBus wrapper.

use super::overlay::Bubble;

pub fn start(_bubble: &Bubble) -> Result<(), &'static str> {
    Err("The UI Automation text watcher is not implemented yet.")
}
