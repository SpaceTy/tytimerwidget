mod app;
mod audio;
mod tray;
mod ui;

use clap::Parser;
use std::process;
use gtk4::prelude::*;
use gtk4::gio::ApplicationFlags;

#[derive(Parser, Debug)]
#[command(name = "tytimer", about = "Hyprland-friendly timer rewritten in Rust", version)]
struct Args {
    /// Timer length in minutes (decimals allowed). If omitted, opens GUI to set duration.
    minutes: Option<f64>,
    /// Run in foreground without daemonizing
    #[arg(long)]
    no_daemon: bool,
}

fn main() {
    let args = Args::parse();

    // Handle case where no minutes argument is provided
    if args.minutes.is_none() {
        run_setter_gui();
        return;
    }

    let minutes = args.minutes.unwrap();
    if minutes <= 0.0 {
        eprintln!("Minutes must be positive.");
        process::exit(2);
    }

    if !args.no_daemon {
        match daemonize(minutes) {
            Ok(()) => {
                println!("✅ Timer started in background.");
                return;
            }
            Err(err) => {
                eprintln!("Failed to start timer in background: {err}");
                process::exit(1);
            }
        }
    }

    if let Err(err) = gstreamer::init() {
        eprintln!("Failed to init GStreamer (sound will be disabled): {err}");
    }

    let total_seconds = (minutes * 60.0).round().max(1.0) as i64;
    run_timer(total_seconds);
}

fn run_setter_gui() {
    if let Err(err) = gstreamer::init() {
        eprintln!("Failed to init GStreamer (sound will be disabled): {err}");
    }

    let app = gtk4::Application::builder()
        .application_id("dev.ty.timers.setter")
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |gtk_app| {
        let setter = ui::SetterWindow::new(gtk_app);
        setter.present();
    });

    app.run_with_args(&Vec::<&str>::new());
}

fn run_timer(total_seconds: i64) {
    let app = gtk4::Application::builder()
        .application_id("dev.ty.timers")
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |gtk_app| {
        let app_state = app::TimerApp::new(gtk_app, total_seconds);
        app_state.present();
    });

    // Pass no args to GTK so our custom flags don't trigger "Unknown option"
    app.run_with_args(&Vec::<&str>::new());
}

fn daemonize(minutes: f64) -> anyhow::Result<()> {
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg("--no-daemon")
        .arg(minutes.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    cmd.spawn()?;
    Ok(())
}
