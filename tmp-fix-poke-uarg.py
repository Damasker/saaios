from pathlib import Path
p = Path("/home/mike/kernel-work/s5300-pantah/bootdump_io_device.c")
t = p.read_text()
old = "\tcase IOCTL_SAAIOS_CP_POKE_BYTE: {\n\t\tstruct saaios_cp_poke poke;\n"
new = "\tcase IOCTL_SAAIOS_CP_POKE_BYTE: {\n\t\tvoid __user *uarg = (void __user *)arg;\n\t\tstruct saaios_cp_poke poke;\n"
if old not in t:
    raise SystemExit("case block missing")
t = t.replace(old, new, 1)
t = t.replace("\tstruct mem_link_device *mld;\n", "", 1)
t = t.replace("\tmld = to_mem_link_device(ld);\n", "\t(void)ld;\n", 1)
# keep ld use
t = t.replace("\t(void)ld;\n\n\tatu_off", "\n\tatu_off", 1)
p.write_text(t)
print("fixed uarg")
