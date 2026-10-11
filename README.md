<p align="center"><img src="assets/expresso.svg" width="112" alt=""></p>

<h1 align="center">Expresso</h1>
<p align="center"><em>a shot of eval, anywhere</em></p>

Expression + espresso. Type some math in any text field and the answer
appears in a small bubble right of the caret, while you're still typing:

```
rent 1850*12|  ( = 22,200 )
```

No hotkey, no window to open and no copy-paste. The bubble stays out of the
way: it never takes focus and never intercepts a click.

| Platform | Status |
|---|---|
| **Linux** (GNOME 50, Wayland) | Works. GNOME Shell extension. |
| **Windows** 10/11 | Early. The bubble and evaluator work (`expresso.exe --demo`); reading text fields is next. |

## What it understands

- **Arithmetic:** `+ - * / ^ **`, `× ÷ ·`, and `x` between numbers.
- **Typing-friendly:** unclosed parentheses (`2*(3+4` → 14), implicit
  multiplication (`2pi`, `3(4+1)`) and a trailing `=`.
- **Percentages the calculator way:** `200 + 15%` → 230, `80 - 25%` → 60,
  `15% * 200` → 30.
- **Other operators:** `5!`, `10 mod 4`, `7 % 3`.
- **Functions:** `sqrt √ cbrt abs exp ln log log2 sin cos tan asin acos atan sinh cosh tanh round floor ceil`
  (trig in radians).
- **Constants:** `pi π e tau τ`.
- **Number formats:** `1.5`, `1e3`, decimal commas (`1,5 * 2` → 3, and the
  answer comes back with a comma too) and grouping commas (`1,000,000`).

It evaluates the longest valid expression ending at the caret on the
current line, so in `the total is 12*4.5` only `12*4.5` counts.

It stays quiet for bare numbers, dates (`2026-10-06`, `06/10/2026`), phone
numbers, password and PIN fields, and while text is selected. Enter and
Escape dismiss the bubble.

## Install

### Linux (GNOME Shell 50)

```bash
git clone https://github.com/Bulrok/expresso.git && cd expresso
make install          # or: linux/install.sh
```

Log out and back in once: GNOME Shell only discovers new extensions at login.
**Expresso** in the app grid then toggles it on and off. `make uninstall`
removes it.

**Coverage:** native Wayland apps that support text-input-v3 surrounding
text. That includes GTK3/4, Firefox, Qt6 and LibreOffice.
- **Chromium/Electron apps** need `--enable-wayland-ime --wayland-text-input-version=3`.
- **XWayland apps** (Wine, older Electron) aren't covered.

### Windows

Grab `expresso-windows-x86_64.exe` from Releases. For now it only runs
`--demo`, which shows the bubble beside your mouse pointer for a few sample
expressions. See [Roadmap](#roadmap).

## How it works

Both versions do the same three things:

1. **Read** the text before the caret and the caret's position on screen.
   - **Linux:** from GNOME Shell's input method. Apps already report both to
     the compositor for IMEs, so nothing is polled.
   - **Windows (planned):** from UI Automation.
2. **Find and evaluate** the expression with a small hand-written
   recursive-descent parser. There's no `eval()`; a full pass takes about 20 µs.
3. **Draw** the bubble.
   - **Linux:** a shell actor above all windows.
   - **Windows:** a click-through, always-on-top popup that never takes focus.

On Linux, keypress to bubble takes about 3 ms in GTK apps and 6–8 ms in
Firefox. [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) has the details,
including the GNOME quirks it works around.

## Repository layout

```
spec/cases.json           behaviour spec shared by both implementations
linux/extension/…         the GNOME Shell extension (GJS)
linux/tests/              spec runner + headless nested-shell end-to-end test
windows/src/calc.rs       Rust port of the evaluator
windows/src/layout.rs     bubble placement (portable, unit-tested)
windows/src/win/          Win32: bubble window, text watcher, message loop
assets/                   icon
```

The evaluator exists twice, once in JavaScript and once in Rust, because each
platform needs its own runtime. Both must pass every case in
`spec/cases.json`: **add or change a case there first**, then make both
implementations pass it. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Development

```bash
make test          # spec + unit tests, JS and Rust (Rust tests run on Linux too)
make test-nested   # end-to-end: headless GNOME Shell, a GTK4 app, a virtual keyboard
make lint          # cargo fmt + clippy
make dist          # extension zip + Windows .exe (cross-compiled with MinGW)
```

On Fedora, cross-compiling for Windows needs `mingw64-gcc` and
`rust-std-static-x86_64-pc-windows-gnu`.

## Roadmap

- **Windows text watcher** (UI Automation): focus and text-change events,
  `TextPattern2` caret, password detection, and a `GetGUIThreadInfo`
  fallback for classic Win32 apps. The plan is in `windows/src/win/watcher.rs`.
- Windows tray icon, autostart and installer.
- Publish the extension on extensions.gnome.org.
- Linux coverage for XWayland apps.
- Insert the result with a keystroke.

## License

[GPL-3.0-or-later](LICENSE).
