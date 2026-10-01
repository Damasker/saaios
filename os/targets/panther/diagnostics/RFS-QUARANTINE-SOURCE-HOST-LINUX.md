# RFS quarantine: synthetic verified-fd source

`rfs-quarantine-source-host-linux.*` is Linux x86-64 **host-only**. It cannot
compile into an ARM phone binary and has no phone path, RFS channel or writer.
It models one part of the future source boundary: a caller-opened read-only
regular fd must be owned by the caller, mode 0600, link count one, exactly
524288 bytes, stable across the read, and match a SHA-256 pin supplied
independently of that fd. Failure zeroes the output. It never reads or trusts
an adjacent checksum sidecar.

The synthetic test writes a temporary generated file under `/tmp`, opens it
read-only, and checks success plus refusal for a wrong pin, writable fd,
permissive mode, hard link and wrong length. It deletes only the exact file
whose inode it created. CI runs normal and ASan/UBSan variants.

This is **not provenance proof** for the Pixel. The future phone adapter
must separately establish the allowlisted userdata path with `O_NOFOLLOW`,
verify its parent directory and fd identity, obtain a trusted expected hash
from outside the mutable copy/sidecar pair, and create an isolated candidate
before acknowledging RFS command 7. Do not open original EFS NV paths or
send a phone-side RFS response based on this fixture alone.
