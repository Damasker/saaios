#!/usr/bin/env python3
"""APP-02 host probe: one GTK4 window against saai-displayd.

Opens a Wayland toplevel, presents a label, then quits. The cargo
test watches displayd logs for xdg_toplevel + a hashed shm commit.
Not a panther frame (Alpine 4.14.4 musl still waits).
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
    win.set_child(Gtk.Label(label="hello"))
    win.connect("close-request", lambda *_: (loop.quit(), False)[1])
    win.present()
    GLib.timeout_add(2000, lambda: (loop.quit(), False)[1])
    loop.run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
