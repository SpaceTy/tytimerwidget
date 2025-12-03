## tytimer — Hyprland-friendly timer with tray and top-right alarm

Minimal CLI timer for Wayland/Hyprland:
- runs in background with a system tray icon (Ayatana AppIndicator)
- accepts decimal minutes
- on expiry, shows a top-right anchored GUI window with Stop and Pause 1%/5%/10%

### Dependencies (Arch / Hyprland)

Install runtime deps:

```bash
sudo pacman -S python python-gobject libayatana-appindicator gtk-layer-shell gst-libav gst-plugins-good gst-plugin-pipewire
```

If your theme lacks `alarm-symbolic`, install an icon theme like `adwaita-icon-theme`.

### Usage

From this directory:

```bash
./tytimer.py 25       # starts 25-minute timer in background
./tytimer.py 0.5      # starts 30 seconds
./tytimer.py --no-daemon 10  # foreground (debug)
```

You should see a tray icon (StatusNotifierItem). When time is up, a large window appears at the top-right with buttons:
- Stop: quit
- Pause 1% / 5% / 10%: snooze based on original time
- Plays sound from `/home/st/Videos/US.mp4` (ensure file exists)

### Notes

- The app uses `gtk-layer-shell` to anchor on Wayland, tested with Hyprland.
- Tray visibility depends on your panel/status bar supporting SNI (e.g., Waybar with tray module).
- Logging prints only on `--no-daemon`. In background, it’s silent.

### Development
Quick test that your GStreamer + PipeWire audio works:

```bash
gst-launch-1.0 audiotestsrc ! audioconvert ! audioresample ! pipewiresink
```


Run in foreground with extra prints:

```bash
G_MESSAGES_DEBUG=all ./tytimer.py --no-daemon 0.1
```


