use anyhow::{anyhow, Result};
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{MenuItem, StandardItem};
use ksni::{Category, Status, ToolTip, Tray};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    ToggleRunning,
    ShowAlarm,
    Quit,
}

pub struct TrayHandle {
    handle: Option<Handle<TimerTray>>,
}

impl TrayHandle {
    pub fn spawn(action_tx: Sender<TrayAction>) -> Result<Self> {
        let tray = TimerTray::new(action_tx);
        let handle = tray.spawn()?;
        Ok(Self {
            handle: Some(handle),
        })
    }

    pub fn noop(_action_tx: Sender<TrayAction>) -> Self {
        Self { handle: None }
    }

    pub fn update_state(&self, label: &str, running: bool) -> Result<()> {
        if let Some(handle) = &self.handle {
            let result = handle.update(|tray| {
                tray.remaining_label = label.to_string();
                tray.running = running;
            });
            if result.is_none() {
                return Err(anyhow!("tray service already stopped"));
            }
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        if let Some(handle) = &self.handle {
            handle.shutdown().wait();
        }
    }
}

#[derive(Clone)]
struct TimerTray {
    remaining_label: String,
    running: bool,
    action_tx: Sender<TrayAction>,
}

impl TimerTray {
    fn new(action_tx: Sender<TrayAction>) -> Self {
        Self {
            remaining_label: "--:--".into(),
            running: true,
            action_tx,
        }
    }
}

impl Tray for TimerTray {
    const MENU_ON_ACTIVATE: bool = false;

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    fn id(&self) -> String {
        "tytimer".into()
    }

    fn title(&self) -> String {
        format!("tytimer ({})", self.remaining_label)
    }

    fn icon_name(&self) -> String {
        "alarm-symbolic".into()
    }

    fn status(&self) -> Status {
        if self.running {
            Status::Active
        } else {
            Status::NeedsAttention
        }
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            icon_name: self.icon_name(),
            title: "tytimer".into(),
            description: format!("Remaining {}", self.remaining_label),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.action_tx.send(TrayAction::ShowAlarm);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: if self.running { "Pause" } else { "Resume" }.into(),
                activate: Box::new(|this: &mut Self| {
                    this.running = !this.running;
                    let _ = this.action_tx.send(TrayAction::ToggleRunning);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Show Alarm Window".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.action_tx.send(TrayAction::ShowAlarm);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.action_tx.send(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
