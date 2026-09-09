# `saai-displayd`

The package keeps the S02 headless proof and adds the bounded Pixel 7 backend
for Sprint S03. It builds two independent executables:

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

On `panther`, `saai-displayd --backend panther` opens only `/dev/dri/card0` and
`/dev/input/touchscreen`, requires the connected DSI mode `1080x2400x60`, and
scales the S02 `64x48` `ARGB8888` frame into the physically verified Pixel 7
BGRX packing. The active fullscreen surface alone receives pointer events.

The phone image also contains a small C `saai-display-supervisor`. It keeps
`drm-splash` visible until PID 1 enables the trial after USB startup, requires
compositor readiness, starts the demo client, watches a 250 ms heartbeat, and
returns to `drm-splash` after no-ready, compositor exit, client exit, or a stale
heartbeat. The supervisor does not own or restart USB services.

All Wayland sockets and liveness files remain under `/run`. The stack does not
open the network or read user data. Supported protocol objects remain limited
to the subset recorded in ADR-007; shell, GPU and toolkit support are outside
S03.
