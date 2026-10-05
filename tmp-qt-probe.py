#!/usr/bin/env python3
import importlib
for m in [
    "PyQt5.QtWidgets",
    "PySide2.QtWidgets",
    "PyQt6.QtWidgets",
    "PySide6.QtWidgets",
]:
    try:
        importlib.import_module(m)
        print("ok", m)
    except Exception as e:
        print("no", m, type(e).__name__)
