#!/usr/bin/env python3
import argparse
import math
import os
import signal
import subprocess
import sys
from pathlib import Path

try:
	import gi  # type: ignore
except Exception as exc:
	print(f"❌ Failed to import gi (PyGObject). Ensure python-gobject is installed. Error: {exc}", file=sys.stderr)
	sys.exit(1)

gi.require_version('Gtk', '3.0')
gi.require_version('GLib', '2.0')
gi.require_version('Gdk', '3.0')
gi.require_version('Gst', '1.0')

# Ayatana AppIndicator (system tray / StatusNotifierItem)
try:
	gi.require_version('AyatanaAppIndicator3', '0.1')
	from gi.repository import AyatanaAppIndicator3 as AppIndicator3  # type: ignore
except Exception as exc:
	print("❌ AyatanaAppIndicator3 GI not available. Install libayatana-appindicator.", file=sys.stderr)
	print(f"Details: {exc}", file=sys.stderr)
	sys.exit(1)

# Layer shell to anchor window in top-right on Wayland
try:
	gi.require_version('GtkLayerShell', '0.1')
	from gi.repository import GtkLayerShell  # type: ignore
except Exception as exc:
	print("❌ GtkLayerShell GI not available. Install gtk-layer-shell.", file=sys.stderr)
	print(f"Details: {exc}", file=sys.stderr)
	sys.exit(1)

from gi.repository import Gtk, GLib, Gdk, Gst  # type: ignore


class TimerApp:
	def __init__(self, minutes: float) -> None:
		self.original_seconds: int = max(1, int(round(minutes * 60)))
		self.remaining_seconds: int = self.original_seconds
		self.running: bool = True
		self._tick_source_id: int | None = None
		self._gst_player = None  # GStreamer playbin instance

		self.indicator = self._create_indicator()
		self.window = self._create_top_right_window()

		self._start_tick()
		self._update_indicator_label()

	def _create_indicator(self) -> AppIndicator3.Indicator:
		indicator = AppIndicator3.Indicator.new(
			"tytimer",
			"alarm-symbolic",  # icon from theme
			AppIndicator3.IndicatorCategory.APPLICATION_STATUS,
		)
		indicator.set_status(AppIndicator3.IndicatorStatus.ACTIVE)

		menu = Gtk.Menu()

		pause_item = Gtk.MenuItem(label="Pause/Resume")
		pause_item.connect("activate", self._toggle_pause)
		menu.append(pause_item)

		show_item = Gtk.MenuItem(label="Show Alarm Window")
		show_item.connect("activate", lambda *_: self._show_alarm_window())
		menu.append(show_item)

		sep = Gtk.SeparatorMenuItem()
		menu.append(sep)

		quit_item = Gtk.MenuItem(label="Quit")
		quit_item.connect("activate", self._quit)
		menu.append(quit_item)

		menu.show_all()
		indicator.set_menu(menu)
		return indicator

	def _create_top_right_window(self) -> Gtk.Window:
		window = Gtk.Window.new(Gtk.WindowType.TOPLEVEL)
		window.set_title("tytimer")
		window.set_default_size(520, 220)
		window.set_resizable(False)
		window.set_deletable(False)
		window.set_keep_above(True)
		window.set_decorated(False)

		GtkLayerShell.init_for_window(window)
		GtkLayerShell.set_layer(window, GtkLayerShell.Layer.TOP)
		GtkLayerShell.set_anchor(window, GtkLayerShell.Edge.TOP, True)
		GtkLayerShell.set_anchor(window, GtkLayerShell.Edge.RIGHT, True)
		GtkLayerShell.set_margin(window, GtkLayerShell.Edge.TOP, 16)
		GtkLayerShell.set_margin(window, GtkLayerShell.Edge.RIGHT, 16)

		root_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
		root_box.set_border_width(18)

		title = Gtk.Label()
		title.set_xalign(0)
		title.set_markup("<span size='20000' weight='bold'>Time's up</span>")
		root_box.pack_start(title, False, False, 0)

		sublabel = Gtk.Label()
		sublabel.set_xalign(0)
		sublabel.set_markup(
			f"<span size='12000'>Original: {self._format_seconds(self.original_seconds)}</span>"
		)
		root_box.pack_start(sublabel, False, False, 0)

		button_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)

		stop_btn = Gtk.Button.new_with_label("Stop")
		stop_btn.get_style_context().add_class("suggested-action")
		stop_btn.connect("clicked", lambda *_: self._quit())
		button_row.pack_start(stop_btn, True, True, 0)

		for pct in (1, 5, 10):
			btn = Gtk.Button.new_with_label(f"Pause {pct}%")
			btn.connect("clicked", self._on_pause_pct_clicked, pct)
			button_row.pack_start(btn, True, True, 0)

		root_box.pack_start(button_row, False, False, 0)

		frame = Gtk.Frame()
		frame.add(root_box)
		frame.get_style_context().add_class("view")
		window.add(frame)

		# Hide initially; shown on expiry
		window.connect("delete-event", lambda *args: True)  # prevent close
		window.hide()
		return window

	def _on_pause_pct_clicked(self, _button: Gtk.Button, pct: int) -> None:
		self.remaining_seconds = max(1, int(round(self.original_seconds * (pct / 100.0))))
		self.running = True
		self.window.hide()
		self._update_indicator_label()

	def _toggle_pause(self, *_args) -> None:
		self.running = not self.running
		self._update_indicator_label()

	def _show_alarm_window(self) -> None:
		# Make sure it's visible on the current monitor
		try:
			display = Gdk.Display.get_default()
			if display is not None:
				monitor = display.get_primary_monitor()
				if monitor is not None:
					GtkLayerShell.set_monitor(self.window, monitor)
		except Exception:
			pass
		self.window.show_all()

	def _hide_alarm_window(self) -> None:
		self.window.hide()

	def _start_tick(self) -> None:
		if self._tick_source_id is not None:
			GLib.source_remove(self._tick_source_id)
		self._tick_source_id = GLib.timeout_add_seconds(1, self._on_tick)

	def _on_tick(self) -> bool:
		if self.running and self.remaining_seconds > 0:
			self.remaining_seconds -= 1
			if self.remaining_seconds == 0:
				self._show_alarm_window()
				self._play_alarm_sound()
		self._update_indicator_label()
		return True  # keep ticking

	def _update_indicator_label(self) -> None:
		label = self._format_seconds(self.remaining_seconds)
		try:
			# Not all trays show labels; harmless if ignored
			self.indicator.set_title(f"{label}")
		except Exception:
			pass

		# Update tooltip by swapping menu item labels (workaround)
		# Also reflect paused state in icon attention
		try:
			if self.running:
				self.indicator.set_status(AppIndicator3.IndicatorStatus.ACTIVE)
			else:
				self.indicator.set_status(AppIndicator3.IndicatorStatus.ATTENTION)
		except Exception:
			pass

	def _quit(self, *_args) -> None:
		self._stop_alarm_sound()
		Gtk.main_quit()

	@staticmethod
	def _format_seconds(total_seconds: int) -> str:
		minutes = total_seconds // 60
		seconds = total_seconds % 60
		return f"{minutes}:{seconds:02d}"

	# --- Sound (GStreamer) ---
	def _play_alarm_sound(self) -> None:
		try:
			path = Path('/home/st/Videos/UA.mp4').expanduser()
			if not path.exists():
				return
			# Stop any previous playback
			self._stop_alarm_sound()
			player = Gst.ElementFactory.make('playbin', 'tytimer-player')
			if player is None:
				return
			# Prefer PipeWire; fallback to PulseAudio; else auto
			audio_sink = (
				Gst.ElementFactory.make('pipewiresink', 'tytimer-pw-sink')
				or Gst.ElementFactory.make('pulsesink', 'tytimer-pa-sink')
				or Gst.ElementFactory.make('autoaudiosink', 'tytimer-auto-sink')
			)
			if audio_sink is not None:
				player.set_property('audio-sink', audio_sink)
			# Force no video window
			videosink = Gst.ElementFactory.make('fakesink', 'tytimer-fakesink')
			if videosink is not None:
				player.set_property('video-sink', videosink)
			player.set_property('uri', path.resolve().as_uri())
			player.set_property('volume', 1.0)
			bus = player.get_bus()
			if bus is not None:
				bus.add_signal_watch()
				bus.connect('message::eos', self._on_gst_eos)
				bus.connect('message::error', self._on_gst_error)
			player.set_state(Gst.State.PLAYING)
			self._gst_player = player
		except Exception as exc:
			print(f"❌ Failed to play sound: {exc}", file=sys.stderr)

	def _stop_alarm_sound(self) -> None:
		try:
			if self._gst_player is not None:
				self._gst_player.set_state(Gst.State.NULL)
				self._gst_player = None
		except Exception:
			pass

	def _on_gst_eos(self, _bus, _msg) -> None:
		self._stop_alarm_sound()

	def _on_gst_error(self, _bus, msg) -> None:
		try:
			err, debug = msg.parse_error()
			print(f"❌ GStreamer error: {err} | {debug}", file=sys.stderr)
		except Exception:
			print("❌ GStreamer error", file=sys.stderr)
		self._stop_alarm_sound()


def parse_args(argv: list[str]) -> argparse.Namespace:
	parser = argparse.ArgumentParser(
		description="Simple Hyprland-friendly timer with tray and top-right alarm window.",
	)
	parser.add_argument(
		"minutes",
		type=float,
		help="Timer length in minutes (decimals allowed)",
	)
	parser.add_argument(
		"--no-daemon",
		action="store_true",
		help="Run in foreground (do not detach)",
	)
	return parser.parse_args(argv)


def daemonize_and_exec(minutes: float) -> None:
	python = sys.executable
	cmd = [python, str(Path(__file__).resolve()), "--no-daemon", str(minutes)]
	# Detach from terminal/session
	subprocess.Popen(
		cmd,
		stdin=subprocess.DEVNULL,
		stdout=subprocess.DEVNULL,
		stderr=subprocess.DEVNULL,
		start_new_session=True,
	)
	print("✅ Timer started in background.")


def main(argv: list[str]) -> int:
	args = parse_args(argv)
	minutes = args.minutes
	if minutes <= 0:
		print("❌ Minutes must be positive.", file=sys.stderr)
		return 2

	if not args.no_daemon:
		daemonize_and_exec(minutes)
		return 0

	# Foreground process: setup app
	Gst.init(None)
	app = TimerApp(minutes)

	# Signal handling for graceful quit
	for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
		try:
			GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, sig, Gtk.main_quit)
		except Exception:
			# Fallback for non-unix environments
			signal.signal(sig, lambda *_: Gtk.main_quit())

	Gtk.main()
	return 0


if __name__ == "__main__":
	sys.exit(main(sys.argv[1:]))


