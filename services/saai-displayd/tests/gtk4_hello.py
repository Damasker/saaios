#!/usr/bin/env python3
"""APP-02/APP-04 host probe: GTK4 window + focused Entry against saai-displayd.

Presents a label and a text field, grabs focus so GDK should
zwp_text_input_v3::enable (ADR-322). The cargo test watches displayd
logs for xdg_toplevel, a hashed shm commit, and text-input-v3 enable.
Not a panther frame.
"""
import os
import sys

os.environ.setdefault("GDK_BACKEND", "wayland")
os.environ.setdefault("GSK_RENDERER", "cairo")
os.environ.setdefault("GTK_A11Y", "none")
os.environ.setdefault("NO_AT_BRIDGE", "1")

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk


def main() -> int:
    loop = GLib.MainLoop()
    win = Gtk.Window()
    win.set_title("saaios-gtk4-probe")
    win.set_default_size(320, 240)
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
    box.append(Gtk.Label(label="hello"))
    entry = Gtk.Entry()
    entry.set_placeholder_text("local field")
    box.append(entry)
    win.set_child(box)
    win.connect("close-request", lambda *_: (loop.quit(), False)[1])
    win.present()
    GLib.idle_add(entry.grab_focus)
    if os.environ.get("GTK4_PROBE_HOLD"):
        def on_text(*_args):
            print(f"GTK_ENTRY_TEXT={entry.get_text()}", flush=True)

        entry.connect("notify::text", on_text)
        loop.run()
        return 0
    GLib.timeout_add(2000, lambda: (loop.quit(), False)[1])
    loop.run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
