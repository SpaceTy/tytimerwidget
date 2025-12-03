use crate::audio::AudioPlayer;
use crate::tray::{TrayAction, TrayHandle};
use crate::ui::TimerWindow;
use gtk4::prelude::*;
use gtk4::Application;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone)]
struct TimerState {
    original: i64,
    remaining: i64,
    running: bool,
    alarm_triggered: bool,
}

pub struct TimerApp {
    app: Application,
    window: TimerWindow,
    tray: TrayHandle,
    state: Rc<RefCell<TimerState>>,
    audio: AudioPlayer,
    tick_source: RefCell<Option<glib::SourceId>>,
}

impl TimerApp {
    pub fn new(app: &Application, total_seconds: i64) -> Rc<Self> {
        let (action_tx, action_rx) = mpsc::channel::<TrayAction>();

        let tray = TrayHandle::spawn(action_tx.clone()).unwrap_or_else(|err| {
            eprintln!("Tray failed to start: {err}");
            TrayHandle::noop(action_tx.clone())
        });

        let window = TimerWindow::new(app, total_seconds);
        let audio = AudioPlayer::new();
        let state = Rc::new(RefCell::new(TimerState {
            original: total_seconds,
            remaining: total_seconds,
            running: true,
            alarm_triggered: false,
        }));

        let app_clone = app.clone();

        let this = Rc::new(Self {
            app: app_clone,
            window,
            tray,
            state: state.clone(),
            audio,
            tick_source: RefCell::new(None),
        });

        // Connect tray actions into GTK main loop
        {
            let app = Rc::clone(&this);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                while let Ok(action) = action_rx.try_recv() {
                    app.handle_action(action);
                }
                glib::ControlFlow::Continue
            });
        }

        // Connect window buttons
        {
            let app = Rc::clone(&this);
            this.window.connect_stop(move || {
                app.quit();
            });
        }
        for pct in [1_u64, 5, 10] {
            let app = Rc::clone(&this);
            this.window.connect_pause(pct, move || app.pause_for_percent(pct));
        }
        {
            let app = Rc::clone(&this);
            this.window.connect_close(move || app.window.hide());
        }

        this.window.set_remaining(total_seconds);

        this.update_tray();
        this
    }

    pub fn present(self: &Rc<Self>) {
        self.start_tick();
    }

    fn start_tick(self: &Rc<Self>) {
        if let Some(source) = self.tick_source.borrow_mut().take() {
            source.remove();
        }

        let app = Rc::clone(self);
        let source = glib::timeout_add_seconds_local(1, move || {
            app.on_tick();
            glib::ControlFlow::Continue
        });
        *self.tick_source.borrow_mut() = Some(source);
    }

    fn on_tick(self: &Rc<Self>) {
        let mut state = self.state.borrow_mut();
        if state.running {
            state.remaining -= 1;
            if state.remaining <= 0 && !state.alarm_triggered {
                self.window.show();
                self.audio.play_alarm();
                state.alarm_triggered = true;
            }
        }
        drop(state);
        self.window.set_remaining(self.state.borrow().remaining);
        self.update_tray();
    }

    fn update_tray(&self) {
        let state = self.state.borrow();
        let label = format_time(state.remaining);
        self.tray
            .update_state(&label, state.running)
            .unwrap_or_else(|err| eprintln!("Failed to update tray: {err}"));
    }

    fn pause_for_percent(&self, percent: u64) {
        let mut state = self.state.borrow_mut();
        let new_remaining = ((state.original as f64) * (percent as f64 / 100.0)).round() as i64;
        state.remaining = new_remaining.max(1);
        state.running = true;
        state.alarm_triggered = false;
        self.window.hide();
        drop(state);
        self.audio.stop();
        self.update_tray();
    }

    fn handle_action(&self, action: TrayAction) {
        match action {
            TrayAction::ToggleRunning => {
                let mut state = self.state.borrow_mut();
                state.running = !state.running;
            }
            TrayAction::ShowAlarm => {
                self.window.show();
            }
            TrayAction::Quit => {
                self.quit();
                return;
            }
        }
        self.update_tray();
    }

    fn quit(&self) {
        self.audio.stop();
        if let Some(source) = self.tick_source.borrow_mut().take() {
            source.remove();
        }
        self.tray.shutdown();
        self.app.quit();
    }
}

fn format_time(total_seconds: i64) -> String {
    let sign = total_seconds < 0;
    let abs = total_seconds.abs();
    let minutes = abs / 60;
    let seconds = abs % 60;
    if sign {
        format!("-{minutes}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}
