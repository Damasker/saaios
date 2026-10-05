from pathlib import Path

pairs = [
    ("/tmp/t-displayd-main.rs", "/tmp/saaios-b2/services/saai-displayd/src/main.rs"),
    ("/tmp/t-pcmanfm-frame.rs", "/tmp/saaios-b2/services/saai-displayd/tests/pcmanfm_frame.rs"),
    ("/tmp/t-falkon-frame.rs", "/tmp/saaios-b2/services/saai-displayd/tests/falkon_frame.rs"),
    ("/tmp/t-qt5-ime.rs", "/tmp/saaios-b2/services/saai-displayd/tests/qt5_ime.rs"),
    ("/tmp/t-text-ime.rs", "/tmp/saaios-b2/services/saai-displayd/src/text_ime.rs"),
    ("/tmp/t-gtk4-ime.rs", "/tmp/saaios-b2/services/saai-displayd/tests/gtk4_ime.rs"),
    ("/tmp/t-gtk414-entry.c", "/tmp/saaios-b2/services/saai-displayd/tests/gtk414_entry.c"),
    ("/tmp/t-gtk414-popover.c", "/tmp/saaios-b2/services/saai-displayd/tests/gtk414_popover.c"),
    ("/tmp/t-gtk414-combobox.c", "/tmp/saaios-b2/services/saai-displayd/tests/gtk414_combobox.c"),
    ("/tmp/t-gtk414-dropdown.c", "/tmp/saaios-b2/services/saai-displayd/tests/gtk414_dropdown.c"),
    ("/tmp/t-qt5-lineedit.cpp", "/tmp/saaios-b2/services/saai-displayd/tests/qt5_lineedit.cpp"),
]
for src, dst in pairs:
    p = Path(src)
    if not p.is_file():
        continue
    t = p.read_bytes().replace(b"\r\n", b"\n").replace(b"\r", b"\n")
    Path(dst).write_bytes(t)
    print("wrote", dst, len(t))
