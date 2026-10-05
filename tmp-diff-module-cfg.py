#!/usr/bin/env python3
import gzip
from pathlib import Path

phone = gzip.open(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/phone-config.gz",
    "rt",
).read().splitlines()
build = Path("/home/mike/kernel-work/common-bd23337/.config").read_text().splitlines()
keys = [
    "CONFIG_ARCH_USES_CFI_TRAPS",
    "CONFIG_STACKTRACE_BUILD_ID",
    "CONFIG_ARCH_WANTS_MODULES_DATA_IN_VMALLOC",
    "CONFIG_MODULE_REL_CRCS",
    "CONFIG_HAVE_MOD_ARCH_SPECIFIC",
    "CONFIG_ANDROID_KABI_RESERVE",
    "CONFIG_ANDROID_VENDOR_OEM_DATA",
    "CONFIG_CFI_CLANG",
    "CONFIG_GENERIC_BUG",
    "CONFIG_KALLSYMS",
    "CONFIG_TRACEPOINTS",
    "CONFIG_EVENT_TRACING",
    "CONFIG_FTRACE",
    "CONFIG_BPF_EVENTS",
    "CONFIG_JUMP_LABEL",
    "CONFIG_FUNCTION_ERROR_INJECTION",
    "CONFIG_DYNAMIC_DEBUG",
    "CONFIG_CONSTRUCTORS",
    "CONFIG_MODULE_UNLOAD",
]


def get(lines, k):
    for ln in lines:
        if ln.startswith(k + "=") or ln.startswith("# " + k + " "):
            return ln
    return "MISSING"


print(f"{'key':45} {'phone':45} {'build'}")
for k in keys:
    p, b = get(phone, k), get(build, k)
    mark = " ***" if p != b else ""
    print(f"{k:45} {p[:45]:45} {b[:45]}{mark}")

t = Path("/home/mike/kernel-work/common-bd23337/include/linux/module.h").read_text()
# print ANDROID_KABI lines near end of struct
start = t.find("struct module {")
end = t.find("};", start)
body = t[start:end]
print("\n--- ANDROID/KABI lines in struct module ---")
for ln in body.splitlines():
    if "ANDROID" in ln or "KABI" in ln or "OEM_DATA" in ln or "cfi" in ln.lower():
        print(ln)
