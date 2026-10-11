// SPDX-License-Identifier: GPL-3.0-or-later
// Expresso for Windows: a shot of eval, anywhere.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod win;

#[cfg(windows)]
fn main() {
    win::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "This binary is the Windows app. On Linux, install the GNOME Shell extension in linux/."
    );
    std::process::exit(1);
}
