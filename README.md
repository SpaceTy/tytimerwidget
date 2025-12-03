# tytimer — Hyprland-friendly timer (Rust/GTK4)

Rust rewrite of the original Python script that keeps the Hyprland-friendly behavior (tray + anchored window) while daemonizing by default.

Features:
- accepts decimal minutes; validates minutes > 0
- daemonizes unless `--no-daemon` is passed
- StatusNotifierItem (tray) with pause/resume + show alarm + quit
- alarm window anchored top-right via gtk-layer-shell with Stop + Pause 1%/5%/10%
- optional alarm sound: plays `/home/st/Videos/UA.mp4` via GStreamer (PipeWire/Pulse fallback)

The original Python version lives in `tytimer.py` for reference.

## Requirements

- System GTK4, gtk-layer-shell, and GStreamer with a PipeWire or Pulse sink available
- A bar/panel that supports SNI (e.g., Waybar) to see the tray icon
- Rust toolchain with Cargo

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run -- 25            # start a 25-minute timer in background
cargo run -- --no-daemon 5 # foreground for debugging
```

The alarm window is kept invisible until expiry; use the tray menu to reopen it if closed.
