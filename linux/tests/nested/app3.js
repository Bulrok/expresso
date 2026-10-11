// GTK3 test window: entry + textview.
imports.gi.versions.Gtk = '3.0';
const {Gtk} = imports.gi;
Gtk.init(null);
const w = new Gtk.Window({title: 'expresso-test', default_width: 700, default_height: 400});
const box = new Gtk.Box({orientation: Gtk.Orientation.VERTICAL, spacing: 8, margin: 12});
box.pack_start(new Gtk.Entry(), false, false, 0);
box.pack_start(new Gtk.Entry({visibility: false, input_purpose: Gtk.InputPurpose.PASSWORD}), false, false, 0);
const tv = new Gtk.TextView();
box.pack_start(tv, true, true, 0);
w.add(box);
w.connect('destroy', () => Gtk.main_quit());
w.show_all();
Gtk.main();
