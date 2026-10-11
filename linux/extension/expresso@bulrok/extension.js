// SPDX-License-Identifier: GPL-3.0-or-later
// Expresso: evaluates the math expression just before the text cursor in any
// text field and shows the result in a bubble to the right of the caret.
//
// Wayland clients (GTK, Firefox, Qt6, ...) report their surrounding text and
// caret rectangle to the compositor through text-input-v3; mutter hands both
// to GNOME Shell's input method (Main.inputMethod), already in stage
// coordinates. That path needs no accessibility and costs nothing while idle.

import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import IBus from 'gi://IBus';
import Meta from 'gi://Meta';
import Mtk from 'gi://Mtk';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {findAndEvaluate} from './calc.js';

// px between caret and bubble. GTK4 TextView reports its caret rect one
// keystroke late, so leave room for roughly one glyph of lag.
const GAP = 12;

// Keys after which the client often sends nothing new (GTK4 TextView on a
// fresh empty line, entries that submit and clear) so the old state would
// linger: hide on these and stay hidden until the text actually changes.
const DISMISS_KEYS = new Set([
    Clutter.KEY_Return, Clutter.KEY_KP_Enter, Clutter.KEY_ISO_Enter, Clutter.KEY_Escape,
]);

const PRIVATE_PURPOSES = [
    Clutter.InputContentPurpose.PASSWORD,
    Clutter.InputContentPurpose.PIN,
];

export default class ExpressoExtension extends Extension {
    enable() {
        this._rect = null;
        this._pending = false;
        this._staleText = null;
        this._hookedCtx = null;

        this._bubble = new St.Label({
            style_class: 'expresso-bubble',
            reactive: false,
            visible: false,
        });
        this._bubble.clutter_text.set({single_line_mode: true, ellipsize: 0});
        Main.layoutManager.uiGroup.add_child(this._bubble);
        // The shell destroys its UI at shutdown, before our IBus hooks stop
        // firing; never touch the actor after that.
        this._bubble.connect('destroy', () => (this._bubble = null));

        const im = Main.inputMethod;
        im.connectObject(
            'surrounding-text-set', () => this._queueUpdate(),
            'cursor-location-changed', (_im, rect) => {
                this._rect = {x: rect.get_x(), y: rect.get_y(), w: rect.get_width(), h: rect.get_height()};
                this._queueUpdate();
            },
            this);

        global.display.connectObject('notify::focus-window', () => this._hide(), this);
        Main.overview.connectObject('showing', () => this._hide(), this);
        this._hookKeys();
    }

    disable() {
        this._unhookKeys();
        Main.inputMethod.disconnectObject(this);
        global.display.disconnectObject(this);
        Main.overview.disconnectObject(this);
        this._bubble?.destroy();
        this._bubble = null;
        this._rect = null;
    }

    // The shell sees no key events for Wayland windows, except through its
    // input method, which forwards every key in a text field to its IBus
    // context; it also tells that context when a text field loses focus, which
    // is otherwise invisible to extensions. Wrap those two instance methods.
    // The context is replaced when ibus-daemon restarts, so this re-checks on
    // each update.
    _hookKeys() {
        const ctx = Main.inputMethod._context;
        if (!ctx || ctx === this._hookedCtx)
            return;
        this._unhookKeys();
        const origKey = ctx.process_key_event_async;
        const origFocusOut = ctx.focus_out;
        ctx.process_key_event_async = (keyval, keycode, state, ...rest) => {
            if (!(state & IBus.ModifierType.RELEASE_MASK) && DISMISS_KEYS.has(keyval))
                this._dismiss();
            return origKey.call(ctx, keyval, keycode, state, ...rest);
        };
        ctx.focus_out = (...args) => {
            this._hide();
            return origFocusOut.call(ctx, ...args);
        };
        this._hookedCtx = ctx;
    }

    _unhookKeys() {
        if (this._hookedCtx) {
            delete this._hookedCtx.process_key_event_async;
            delete this._hookedCtx.focus_out;
        }
        this._hookedCtx = null;
    }

    _dismiss() {
        const [text] = Main.inputMethod.getSurroundingText();
        this._staleText = text ?? '';
        this._hide();
    }

    // Text and caret position arrive as separate calls for the same client
    // commit; coalesce them into one update right before the next frame.
    _queueUpdate() {
        if (this._pending)
            return;
        this._pending = true;
        global.compositor.get_laters().add(Meta.LaterType.BEFORE_REDRAW, () => {
            this._pending = false;
            this._update();
            return GLib.SOURCE_REMOVE;
        });
    }

    _update() {
        if (!this._bubble)
            return;
        this._hookKeys();
        const im = Main.inputMethod;
        if (!im.currentFocus || !this._rect || this._isPrivate(im)) {
            this._hide();
            return;
        }
        const [text, cursor, anchor] = im.getSurroundingText();
        // No result while a selection is active; the caret is ambiguous then.
        if (!text || cursor !== anchor) {
            this._hide();
            return;
        }
        if (this._staleText !== null) {
            if (text === this._staleText) {
                this._hide();
                return;
            }
            this._staleText = null;
        }
        const res = findAndEvaluate(GLib.utf8_substring(text, 0, cursor));
        if (!res) {
            this._hide();
            return;
        }
        this._show(`= ${res.display}`);
    }

    _isPrivate(im) {
        if (PRIVATE_PURPOSES.includes(im.content_purpose))
            return true;
        const hints = im.content_hints;
        return (hints & (Clutter.InputContentHintFlags.SENSITIVE_DATA |
                         Clutter.InputContentHintFlags.HIDDEN_TEXT)) !== 0;
    }

    _show(label) {
        const b = this._bubble;
        const r = this._rect;
        b.text = label;
        const [, natW] = b.get_preferred_width(-1);
        const [, natH] = b.get_preferred_height(-1);

        let x = r.x + r.w + GAP;
        let y = r.y + (r.h - natH) / 2;

        const mon = Main.layoutManager.monitors[
            global.display.get_monitor_index_for_rect(
                new Mtk.Rectangle({x: r.x, y: r.y, width: Math.max(r.w, 1), height: Math.max(r.h, 1)}))] ??
            Main.layoutManager.primaryMonitor;
        if (x + natW > mon.x + mon.width)          // no room on the right: go left
            x = r.x - GAP - natW;
        x = Math.max(mon.x, x);
        y = Math.min(Math.max(mon.y, y), mon.y + mon.height - natH);

        b.set_position(Math.round(x), Math.round(y));
        if (!b.visible) {
            Main.layoutManager.uiGroup.set_child_above_sibling(b, null);
            b.show();
        }
    }

    _hide() {
        this._bubble?.hide();
    }
}
