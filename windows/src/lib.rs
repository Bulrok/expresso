// SPDX-License-Identifier: GPL-3.0-or-later
//! Platform-independent core of Expresso for Windows: the evaluator and the
//! bubble placement rule. Everything here builds and tests on any OS; the
//! Win32 side lives in `src/win/` and only compiles on Windows.

pub mod calc;
pub mod layout;
