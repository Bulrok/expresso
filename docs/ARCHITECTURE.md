# Architecture

Expresso runs the same pipeline on every platform:

```
text field ──► (1) capture text + caret ──► (2) coalesce ──► (3) find + evaluate ──► (4) draw bubble
```

Only stages 1, 2 and 4 are platform code. Stage 3 is the evaluator,
implemented once per runtime and pinned to identical behaviour by
`spec/cases.json`.

## 1. Capture (Linux)

Expresso never sees keystrokes as such. It relies on the input-method plumbing
that every Wayland toolkit already speaks, so CJK input methods can work.

- Apps use the **text-input-v3** protocol. On each change they send the
  compositor the *surrounding text* (usually the current paragraph), the
  caret offset within it, and the *caret rectangle*.
- Mutter converts the rectangle to stage (screen) coordinates and passes
  everything to GNOME Shell's input method, `Main.inputMethod`.
- The extension listens to `surrounding-text-set` and
  `cursor-location-changed`. Apps also declare the field's purpose, so
  password and PIN fields are skipped.

Consequence: only clients that speak text-input-v3 *with surrounding text*
are covered. Chromium/Electron need `--enable-wayland-ime
--wayland-text-input-version=3`; XWayland clients bypass this path entirely.

AT-SPI was rejected for Linux because it means synchronous D-Bus calls into
other apps from inside the compositor: one hung app would stall the desktop.

## 2. Coalesce

Text and caret rectangle arrive as separate signals for the same client
commit. Instead of two updates, one is scheduled with
`Meta.LaterType.BEFORE_REDRAW`, so the bubble always matches the frame being
drawn.

Hiding needs extra hooks, because some clients go quiet at exactly the wrong
moment:

| Situation | Hook |
|---|---|
| Focus moves to another window | `global.display` `notify::focus-window` |
| Focus leaves the text field | wrapped `focus_out` on the shell's IBus context |
| Enter / Escape | wrapped `process_key_event_async` on the IBus context; the bubble stays hidden until the text actually changes |
| Overview opens | `Main.overview` `showing` |

The shell sees no key events for Wayland windows except via its IBus context,
hence the wrapping. It patches the private `Main.inputMethod._context` on the
instance only, re-checks it on every update (the context is replaced if
ibus-daemon restarts), and is removed on `disable()`. Re-verify it whenever
`shell-version` is bumped.

Quirk: GTK4's `TextView` commits its caret rectangle one keystroke late, and
sends nothing at all after Return. The 12 px gap absorbs the lag; the
Enter/Escape rule handles the silence.

## 3. Find and evaluate

`calc.js` / `calc.rs`, `findAndEvaluate()` / `find_and_evaluate()`:

1. **Current line only:** text after the last newline before the caret,
   capped at 300 characters. A trailing `=` is ignored.
2. **Tokenize** into numbers (`1.5`, `1,5`, `1,000,000`, `1e3`), operators,
   parentheses, functions, constants, and **words** (anything unrecognised).
3. **Left boundary:** an expression can't reach back past a word.
4. **Longest valid suffix:** try each start position left to right; the first
   that parses cleanly to the caret wins. A leading sign is only allowed at
   the boundary, so `5-3` isn't also read as `-3`.
5. **Filter:** no result for zero operations (`2026`, `-5`), non-finite values
   (`1/0`), or date and phone-number shapes.

The parser is recursive descent, computing values directly (no AST):

```
expr    := term (('+'|'-') term)*        a ± b%  means  a * (1 ± b/100)
term    := unary (('*'|'/'|mod) unary | <implicit> unary)*
unary   := ('-'|'+') unary | power
power   := postfix ('^' unary)?            right-assoc, so -2^2 = -4
postfix := atom ('!' | '%')*
atom    := num | const | fn atom | fn '(' expr ')' | '(' expr ')'
```

Unclosed parentheses at the end are accepted, because the user is mid-typing.
Implicit multiplication never joins two bare numbers, so `v1.2.3` is not a
product.

Formatting rounds to 12 significant digits (`0.1+0.2` → `0.3`), groups
thousands for 5+ digit integers, switches to exponent form outside
[1e-9, 1e15), and mirrors a decimal comma if the input used one. The Rust
port reproduces JavaScript's number-to-string rules (half-up `toPrecision`,
`toFixed` for tiny values); the spec has cases that pin those edges.

## 4. Draw

- **Linux:** an `St.Label` added to `Main.layoutManager.uiGroup`, above all
  windows, `reactive: false`.
- **Windows:** a `WS_POPUP` with `WS_EX_LAYERED | WS_EX_TRANSPARENT |
  WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`, painted with GDI and
  rounded with a window region. The process is per-monitor DPI aware (v2), so
  caret coordinates and the window share physical pixels.

Placement is the same rule on both: right of the caret, vertically centred,
flipped left at the monitor edge, clamped to the work area. On Windows it is
the portable `layout::place_bubble`, unit-tested on any OS.

## Windows capture (planned)

UI Automation from the same process. No code is injected into other apps.

1. `CUIAutomation8`; `AddFocusChangedEventHandler`. Keep the focused element if
   it supports `TextPattern2` and is not a password field.
2. Subscribe to `Text_TextChanged` and `Text_TextSelectionChanged` on it.
3. `GetCaretRange` → expand a clone to the line, cut at the caret →
   `GetText(300)`; `GetBoundingRectangles` → caret rect.
4. Fallback rect: `GetGUIThreadInfo().rcCaret` for classic Win32 carets.
5. Marshal to the UI thread with `PostMessageW`, coalescing like stage 2.
6. Enter/Escape via a `WH_KEYBOARD_LL` hook.

Known trade-offs: UIA calls are cross-process (≈10–50 ms vs ≈3 ms on Linux),
Chromium builds its accessibility tree on demand when a UIA client asks, and
elevated apps are invisible to a non-elevated process. If coverage proves
insufficient, the fallback is a TSF text service (the true analogue of the
Linux approach), at the cost of a DLL loaded into every process.

## Testing

| Layer | Command | What it proves |
|---|---|---|
| Spec | `make test` | JS and Rust evaluators agree on every case in `spec/cases.json` |
| Layout | `cargo test` | Bubble placement, flipping, clamping, multi-monitor offsets |
| End-to-end (Linux) | `make test-nested` | Headless GNOME Shell + GTK4 app + virtual keyboard: placement, hiding rules, re-enable cycles, latency |
| Windows bubble | `expresso.exe --demo` in a Windows VM | Window style, painting, DPI |

The nested test runs in a temp XDG/dconf/runtime dir and never touches the
live session. Other targets:

```bash
EXPRESSO_TAG=gtk3 EXPRESSO_APP="gjs $PWD/linux/tests/nested/app3.js" linux/tests/nested/run.sh
```

The same runner launches Firefox with a throwaway `--profile`.

Under Wine in the nested shell, the Windows build's bubble is created,
positioned beside the pointer, sized to its text and repainted for each demo
step (`WINEDEBUG=+win,+msg`). However, the headless screenshot does not show
Wine's small popup windows, while Notepad does show. Visual checks of the
Windows bubble belong on real Windows.
