# `saai-displayd` host vertical slice

This package is the Sprint S02 headless proof, not the Pixel 7 display server.
It builds two independent executables:

- `saai-displayd`: a bounded Wayland server;
- `saai-demo-surface`: a client that performs configure/ack, presents one
  deterministic `wl_shm` frame, receives focused pointer input and exits on
  `xdg_toplevel.close`.

The integration test also connects an observer and a malformed peer. The
observer must receive no focused input; the malformed peer must be disconnected
without preventing the valid surface from completing.

Run on Linux:

```sh
cargo test -p saai-displayd --all-targets -- --nocapture
```

The server uses only a private Unix socket under `XDG_RUNTIME_DIR`. It does not
open the network, read user data, access DRM/KMS or enter the Pixel 7 image.
Supported protocol objects are deliberately limited to the S02 subset recorded
in ADR-007.
