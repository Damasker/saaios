from pathlib import Path

t = Path("/tmp/pcmanfm-strings.txt").read_text(errors="replace")
keys = (
    "focusPath",
    "ShowFilter",
    "FilterBar",
    "PathEdit",
    "--filter",
    "search://",
    "transientFilter",
    "setFocus",
    "QLineEdit",
    "inputMethod",
    "filter-string",
)
for line in t.splitlines():
    low = line.lower()
    if any(k.lower() in low for k in keys):
        print(line)
