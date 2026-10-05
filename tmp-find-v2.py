from pathlib import Path

p = Path(
    "/home/mike/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/smithay-0.7.0/src/wayland/text_input/text_input_handle.rs"
)
print(p.read_text()[:12000])
print("==== SEARCH plasma ====")
root = Path("/home/mike/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f")
for d in root.iterdir():
    n = d.name
    if "plasma" in n or "wayland-protocols-misc" in n or n.startswith("wayland-protocols"):
        print("crate", n)
        for f in d.rglob("*text_input*"):
            print(" ", f)
