// GTK4 test window: entry, password entry, textview.
import Gtk from 'gi://Gtk?version=4.0';
const app = new Gtk.Application({application_id: 'local.expresso.test'});
app.connect('activate', () => {
    const w = new Gtk.ApplicationWindow({application: app, title: 'expresso-test', default_width: 700, default_height: 400});
    const box = new Gtk.Box({orientation: Gtk.Orientation.VERTICAL, spacing: 8, margin_top: 12, margin_start: 12, margin_end: 12, margin_bottom: 12});
    box.append(new Gtk.Entry());
    box.append(new Gtk.PasswordEntry());
    const tv = new Gtk.TextView({vexpand: true});
    box.append(tv);
    tv.buffer.connect('changed', b => printerr('HARNESS: APP buffer=' + JSON.stringify(b.text)));
    box.append(new Gtk.Button({label: 'button'}));
    w.set_child(box);
    w.present();
});
app.run([]);
