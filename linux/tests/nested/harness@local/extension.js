// Test-only: drives the GTK test app via a virtual keyboard inside a headless
// nested gnome-shell and reports what the Expresso bubble does.
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const OUT = GLib.getenv('EXPRESSO_OUT');
const sleep = ms => new Promise(r => GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => r() ?? false));
const say = s => printerr(`HARNESS: ${s}`);

export default class Harness extends Extension {
    enable() {
        Main.extensionManager.enableExtension('expresso@bulrok');
        this._run().catch(e => say(`ERROR ${e}\n${e.stack}`));
    }
    disable() {}

    async _run() {
        // Screenshot-only mode: for apps we can't type into (e.g. the Windows
        // build under Wine with --demo).
        const shots = Number(GLib.getenv('EXPRESSO_SHOTS') ?? 0);
        if (shots) {
            Main.overview.hide();
            await sleep(Number(GLib.getenv('EXPRESSO_SHOTS_DELAY') ?? 3000));
            for (let i = 0; i < shots; i++) {
                await this.shot(`shot-${i}`);
                for (const a of global.get_window_actors()) {
                    const w = a.meta_window;
                    const r = w.get_frame_rect();
                    say(`  window ${JSON.stringify(w.title)} type=${w.window_type} ${r.x},${r.y} ${r.width}x${r.height} visible=${a.visible} opacity=${a.opacity} minimized=${w.minimized}`);
                }
                await sleep(Number(GLib.getenv('EXPRESSO_SHOTS_EVERY') ?? 1500));
            }
            say('DONE');
            return;
        }
        const seat = Clutter.get_default_backend().get_default_seat();
        this.kbd = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
        let win = null;
        for (let i = 0; i < 300 && !win; i++) {
            await sleep(100);
            win = global.display.list_all_windows().find(w => w.title?.startsWith('expresso-test'));
        }
        if (!win) { say('no test window'); return; }
        await sleep(800);
        Main.overview.hide();
        win.activate(global.get_current_time());
        await sleep(500);
        const fr = win.get_frame_rect();
        say(`window frame ${fr.x},${fr.y} ${fr.width}x${fr.height}`);
        const ext = Main.extensionManager.lookup('expresso@bulrok');
        say(`expresso state ${ext?.state} err=${ext?.error}`);
        const bubble = () => ext.stateObj._bubble;

        const check = async (label) => {
            await sleep(150);
            const b = bubble();
            const r = ext.stateObj._rect;
            const [st] = Main.inputMethod.getSurroundingText();
            say(`[${label}] surrounding=${JSON.stringify(st)} rect=${JSON.stringify(r)} bubble=${b.visible ? JSON.stringify(b.text) + ' @' + b.x + ',' + b.y : 'hidden'}`);
        };

        // Entry 1 (focused first)
        await this.type('total 12*4.5');
        await check('entry');
        await this.shot(`${GLib.getenv('EXPRESSO_TAG') ?? 'x'}-entry`);
        await this.type(' apples');
        await check('entry-words-after');
        await this.key(Clutter.KEY_Tab);
        // Password entry
        await this.type('2+2');
        await check('password');
        await this.key(Clutter.KEY_Tab);
        // TextView
        await this.type('first line 3*3');
        await this.key(Clutter.KEY_Return);
        await this.type('second: (1+2)^3 =');
        await check('textview');
        await this.shot(`${GLib.getenv('EXPRESSO_TAG') ?? 'x'}-textview`);
        await this.key(Clutter.KEY_Return);
        await sleep(300);
        await check('textview-newline (expect hidden)');
        await this.type('5');
        await check('newline-then-5 (expect hidden)');
        await this.type('+5');
        await check('newline 5+5 (expect = 10)');
        await this.key(Clutter.KEY_Escape);
        await check('escape (expect hidden)');
        await this.key(Clutter.KEY_BackSpace);
        await this.key(Clutter.KEY_BackSpace);
        await check('backspace-x2');
        // latency: keypress -> bubble text change, several samples
        await this.type(' 7*6');
        await sleep(200);
        const lat = [];
        for (const d of '123456789') {
            const before = bubble().text;
            const t0 = GLib.get_monotonic_time();
            this.press(d);
            while (bubble().text === before && GLib.get_monotonic_time() - t0 < 500000)
                await sleep(1);
            lat.push((GLib.get_monotonic_time() - t0) / 1000);
            await sleep(60);
        }
        say(`latency keypress->bubble ms: ${lat.map(x => x.toFixed(1)).join(' ')} (final ${bubble().text})`);
        const t = GLib.get_monotonic_time();
        this.kbd.notify_keyval(t, Clutter.KEY_Control_L, Clutter.KeyState.PRESSED);
        this.kbd.notify_keyval(t, Clutter.KEY_Tab, Clutter.KeyState.PRESSED);
        this.kbd.notify_keyval(t, Clutter.KEY_Tab, Clutter.KeyState.RELEASED);
        this.kbd.notify_keyval(t, Clutter.KEY_Control_L, Clutter.KeyState.RELEASED);
        await check('ctrl+tab to button (expect hidden)');
        // lock/unlock cycle
        await this.key(Clutter.KEY_Tab);   // back to the entry (wraps)
        for (let i = 0; i < 3; i++) {
            Main.extensionManager.disableExtension('expresso@bulrok');
            await sleep(100);
            Main.extensionManager.enableExtension('expresso@bulrok');
            await sleep(100);
        }
        const orphans = Main.layoutManager.uiGroup.get_children().filter(c => c.style_class === 'expresso-bubble').length;
        say(`bubbles in uiGroup after 3 cycles: ${orphans}; ctx hooked=${Object.hasOwn(Main.inputMethod._context, 'process_key_event_async')}`);
        await this.type('1+1');
        await check('after re-enable');
        say('DONE');
    }

    press(ch) {
        const kv = Clutter.unicode_to_keysym(ch.codePointAt(0));
        const t = GLib.get_monotonic_time();
        this.kbd.notify_keyval(t, kv, Clutter.KeyState.PRESSED);
        this.kbd.notify_keyval(t, kv, Clutter.KeyState.RELEASED);
    }
    async type(s) {
        for (const ch of s) { this.press(ch); await sleep(15); }
    }
    async key(kv) {
        const t = GLib.get_monotonic_time();
        this.kbd.notify_keyval(t, kv, Clutter.KeyState.PRESSED);
        this.kbd.notify_keyval(t, kv, Clutter.KeyState.RELEASED);
        await sleep(100);
    }
    async shot(name) {
        await sleep(100);
        const file = Gio.File.new_for_path(`${OUT}/${name}.png`);
        const stream = file.replace(null, false, Gio.FileCreateFlags.NONE, null);
        await new Shell.Screenshot().screenshot(false, stream);
        stream.close(null);
        say(`shot ${name}`);
    }
}
