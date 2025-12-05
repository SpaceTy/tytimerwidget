use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, CssProvider, DrawingArea, Frame, Label,
    Orientation, Align, STYLE_PROVIDER_PRIORITY_APPLICATION, Scale,
};
use gtk4::{glib, prelude::WidgetExt};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::rc::Rc;
use std::process::Command;

pub struct TimerWindow {
    pub window: ApplicationWindow,
    pause_buttons: Vec<(u64, Button)>,
    stop_button: Button,
    close_button: Button,
    remaining_label: Label,
    original_seconds: i64,
}

impl TimerWindow {
    pub fn new(app: &Application, original_seconds: i64) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("tytimer")
            .default_width(520)
            .default_height(220)
            .resizable(false)
            .decorated(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 16);
        window.set_margin(Edge::Right, 16);

        let provider = CssProvider::new();
        let _ = provider.load_from_data(
            "window { background-color: rgba(0, 0, 0, 0.88); color: white; }
             button { border-radius: 6px; padding: 12px 14px; font-weight: 600; }
             button.suggested-action { background: #3b82f6; color: white; }
             button.suggested-action:hover { background: #4b8ffc; }
             frame { border-radius: 10px; }
             label.title { font-size: 24px; font-weight: 800; }
             label.subtitle { opacity: 0.7; }",
        );

        let display = WidgetExt::display(&window);
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let root_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(16)
            .margin_top(18)
            .margin_bottom(18)
            .margin_start(18)
            .margin_end(18)
            .build();

        let header = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .build();

        let flag = DrawingArea::new();
        // 30x20 is a clean 3:2 ratio.
        flag.set_content_width(30);
        flag.set_content_height(20);
        flag.set_hexpand(false);
        flag.set_vexpand(false);
        flag.set_halign(Align::Start);
        flag.set_valign(Align::Center);
        flag.set_draw_func(|_, cr, width, height| {
            let w = width as f64;
            let h = height as f64;
            cr.set_source_rgb(0.0, 0.34, 0.72);
            let half = h / 2.0;
            cr.rectangle(0.0, 0.0, w, half);
            let _ = cr.fill();
            cr.set_source_rgb(1.0, 0.84, 0.0);
            cr.rectangle(0.0, half, w, half);
            let _ = cr.fill();
        });
        header.append(&flag);

        let title = Label::builder()
            .label("TyTimer")
            .css_classes(vec!["title"])
            .xalign(0.0)
            .yalign(0.5)
            .build();
        header.append(&title);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);

        let close_button = Button::builder()
            .icon_name("window-close")
            .css_classes(vec!["flat"])
            .build();
        header.append(&close_button);
        root_box.append(&header);

        let remaining_label = Label::builder()
            .label(&format!(
                "Remaining: --:-- / Original: {}",
                format_seconds(original_seconds)
            ))
            .css_classes(vec!["subtitle"])
            .xalign(0.0)
            .yalign(0.5)
            .build();
        root_box.append(&remaining_label);

        let button_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .build();

        let stop_button = Button::builder()
            .label("Stop")
            .css_classes(vec!["suggested-action"])
            .hexpand(true)
            .build();
        button_row.append(&stop_button);

        let mut pause_buttons = Vec::new();
        for pct in [1_u64, 5, 10] {
            let btn = Button::builder()
                .label(&format!("Pause {pct}%"))
                .hexpand(true)
                .build();
            button_row.append(&btn);
            pause_buttons.push((pct, btn));
        }

        root_box.append(&button_row);

        let frame = Frame::new(None);
        frame.set_child(Some(&root_box));
        window.set_child(Some(&frame));

        window.connect_close_request(|_| glib::Propagation::Stop);
        window.set_visible(false);

        Self {
            window,
            pause_buttons,
            stop_button,
            close_button,
            remaining_label,
            original_seconds,
        }
    }

    pub fn show(&self) {
        self.window.present();
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    pub fn connect_stop<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.stop_button.connect_clicked(move |_| handler());
    }

    pub fn connect_pause<F>(&self, percent: u64, handler: F)
    where
        F: Fn() + 'static,
    {
        if let Some((_, button)) = self
            .pause_buttons
            .iter()
            .find(|(pct, _)| *pct == percent)
        {
            button.connect_clicked(move |_| handler());
        }
    }

    pub fn connect_close<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.close_button.connect_clicked(move |_| handler());
    }

    pub fn set_remaining(&self, seconds: i64) {
        self.remaining_label.set_label(&format!(
            "Remaining: {} / Original: {}",
            format_seconds(seconds),
            format_seconds(self.original_seconds)
        ));
    }
}

fn format_seconds(total_seconds: i64) -> String {
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

pub struct SetterWindow {
    window: ApplicationWindow,
    app: Application,
}

impl SetterWindow {
    pub fn new(app: &Application) -> Rc<Self> {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("tytimer")
            .default_width(520)
            .default_height(180)
            .resizable(false)
            .decorated(false)
            .build();

        // Use layer-shell to anchor to top-right, matching TimerWindow
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 16);
        window.set_margin(Edge::Right, 16);

        let provider = CssProvider::new();
        let _ = provider.load_from_data(
            "window { background-color: rgba(0, 0, 0, 0.88); color: white; }
             button { border-radius: 6px; padding: 12px 14px; font-weight: 600; }
             button.suggested-action { background: #3b82f6; color: white; }
             button.suggested-action:hover { background: #4b8ffc; }
             frame { border-radius: 10px; }
             label.title { font-size: 24px; font-weight: 800; }
             label.subtitle { opacity: 0.7; }
             label.time-display { font-size: 32px; font-weight: 800; padding: 8px 16px; border: 2px solid rgba(59, 130, 246, 0.6); border-radius: 8px; background: rgba(59, 130, 246, 0.1); }
             scale { min-height: 50px; }
             scale trough { min-height: 6px; background: rgba(255, 255, 255, 0.2); border-radius: 3px; }
             scale highlight { background: #3b82f6; border-radius: 3px; }
             scale slider { min-width: 20px; min-height: 20px; margin: -7px; background: white; border-radius: 10px; }",
        );

        let display = WidgetExt::display(&window);
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let root_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(18)
            .margin_bottom(18)
            .margin_start(18)
            .margin_end(18)
            .build();

        // Header with flag and title (matching TimerWindow)
        let header = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .build();

        let flag = DrawingArea::new();
        flag.set_content_width(30);
        flag.set_content_height(20);
        flag.set_hexpand(false);
        flag.set_vexpand(false);
        flag.set_halign(Align::Start);
        flag.set_valign(Align::Center);
        flag.set_draw_func(|_, cr, width, height| {
            let w = width as f64;
            let h = height as f64;
            cr.set_source_rgb(0.0, 0.34, 0.72);
            let half = h / 2.0;
            cr.rectangle(0.0, 0.0, w, half);
            let _ = cr.fill();
            cr.set_source_rgb(1.0, 0.84, 0.0);
            cr.rectangle(0.0, half, w, half);
            let _ = cr.fill();
        });
        header.append(&flag);

        let title = Label::builder()
            .label("TyTimer")
            .css_classes(vec!["title"])
            .xalign(0.0)
            .yalign(0.5)
            .build();
        header.append(&title);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);

        // Time display in header
        let time_label = Label::builder()
            .label("5:00")
            .css_classes(vec!["time-display"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();
        header.append(&time_label);

        let close_button = Button::builder()
            .icon_name("window-close")
            .css_classes(vec!["flat"])
            .margin_start(12)
            .build();
        header.append(&close_button);
        root_box.append(&header);

        // Scale from 0.5 to 120 minutes (0.5 min to 2 hours)
        let scale = Scale::with_range(Orientation::Horizontal, 0.5, 120.0, 0.5);
        scale.set_value(5.0);
        scale.set_draw_value(false);
        scale.set_hexpand(true);
        root_box.append(&scale);

        // Start button
        let start_button = Button::builder()
            .label("Start Timer")
            .css_classes(vec!["suggested-action"])
            .hexpand(true)
            .build();
        root_box.append(&start_button);

        let frame = Frame::new(None);
        frame.set_child(Some(&root_box));
        window.set_child(Some(&frame));

        window.connect_close_request(|_| glib::Propagation::Stop);

        let this = Rc::new(Self {
            window: window.clone(),
            app: app.clone(),
        });

        // Connect close button
        {
            let this_clone = Rc::clone(&this);
            close_button.connect_clicked(move |_| {
                this_clone.window.close();
                this_clone.app.quit();
            });
        }

        // Update time label when scale changes
        let time_label_clone = time_label.clone();
        scale.connect_value_changed(move |scale| {
            let minutes = scale.value();
            let total_seconds = (minutes * 60.0).round() as i64;
            time_label_clone.set_label(&format_seconds(total_seconds));
        });

        // Handle start button click
        {
            let this_clone = Rc::clone(&this);
            let scale_clone = scale.clone();
            start_button.connect_clicked(move |_| {
                let minutes = scale_clone.value();
                this_clone.start_timer(minutes);
            });
        }

        this
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn start_timer(&self, minutes: f64) {
        // Close the setter window
        self.window.close();

        // Start the timer in the background using the --no-daemon flag
        // We need to spawn a new process with the timer
        let exe = std::env::current_exe().unwrap_or_else(|_| {
            eprintln!("Failed to get current executable path");
            std::path::PathBuf::from("tytimers")
        });

        let result = Command::new(exe)
            .arg("--no-daemon")
            .arg(minutes.to_string())
            .spawn();

        match result {
            Ok(_) => {
                // Quit this GUI application
                self.app.quit();
            }
            Err(err) => {
                eprintln!("Failed to start timer: {err}");
                self.app.quit();
            }
        }
    }
}
