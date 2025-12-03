use gstreamer::prelude::*;
use std::cell::RefCell;
use std::path::Path;

pub struct AudioPlayer {
    player: RefCell<Option<gstreamer::Element>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            player: RefCell::new(None),
        }
    }

    pub fn play_alarm(&self) {
        let path = Path::new("/home/st/Videos/UA.mp4");
        if !path.exists() {
            return;
        }

        self.stop();

        let uri = match path.canonicalize() {
            Ok(p) => format!("file://{}", p.to_string_lossy()),
            Err(_) => return,
        };

        let player = match gstreamer::ElementFactory::make("playbin")
            .name("tytimer-player")
            .build()
        {
            Ok(element) => element,
            Err(_) => return,
        };

        player.set_property("uri", uri);

        let audio_sink = gstreamer::ElementFactory::make("pipewiresink")
            .name("tytimer-pw-sink")
            .build()
            .or_else(|_| {
                gstreamer::ElementFactory::make("pulsesink")
                    .name("tytimer-pa-sink")
                    .build()
            })
            .or_else(|_| {
                gstreamer::ElementFactory::make("autoaudiosink")
                    .name("tytimer-auto-sink")
                    .build()
            })
            .ok();

        if let Some(audio_sink) = audio_sink {
            player.set_property("audio-sink", audio_sink);
        }

        if let Ok(fake_sink) = gstreamer::ElementFactory::make("fakesink")
            .name("tytimer-fakesink")
            .build()
        {
            player.set_property("video-sink", fake_sink);
        }

        if let Some(bus) = player.bus() {
            let player_weak = player.downgrade();
            let _ = bus.add_watch_local(move |_, msg| {
                use gstreamer::MessageView;
                match msg.view() {
                    MessageView::Eos(_) => {
                        if let Some(player) = player_weak.upgrade() {
                            let _ = player.set_state(gstreamer::State::Null);
                        }
                        glib::ControlFlow::Break
                    }
                    MessageView::Error(err) => {
                        eprintln!("GStreamer error: {:?}", err.error());
                        if let Some(player) = player_weak.upgrade() {
                            let _ = player.set_state(gstreamer::State::Null);
                        }
                        glib::ControlFlow::Break
                    }
                    _ => glib::ControlFlow::Continue,
                }
            });
        }

        let _ = player.set_state(gstreamer::State::Playing);
        *self.player.borrow_mut() = Some(player);
    }

    pub fn stop(&self) {
        if let Some(player) = self.player.borrow_mut().take() {
            let _ = player.set_state(gstreamer::State::Null);
        }
    }
}
