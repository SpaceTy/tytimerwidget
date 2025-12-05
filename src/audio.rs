use gstreamer::prelude::*;
use std::cell::RefCell;

// Embed the UA.mp3 file into the binary
static ALARM_SOUND: &[u8] = include_bytes!("../UA.mp3");

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
        self.stop();

        // Create a pipeline with appsrc for memory playback
        let pipeline = gstreamer::Pipeline::new();

        let appsrc = match gstreamer::ElementFactory::make("appsrc")
            .name("tytimer-src")
            .build()
        {
            Ok(element) => element,
            Err(_) => return,
        };

        let decodebin = match gstreamer::ElementFactory::make("decodebin")
            .name("tytimer-decoder")
            .build()
        {
            Ok(element) => element,
            Err(_) => return,
        };

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
            });

        let audio_sink = match audio_sink {
            Ok(sink) => sink,
            Err(_) => return,
        };

        // Add elements to pipeline
        if pipeline
            .add_many([&appsrc, &decodebin, &audio_sink])
            .is_err()
        {
            return;
        }

        // Link appsrc to decodebin
        if appsrc.link(&decodebin).is_err() {
            return;
        }

        // Configure appsrc
        let appsrc = appsrc.dynamic_cast::<gstreamer_app::AppSrc>().unwrap();
        appsrc.set_property("format", gstreamer::Format::Bytes);
        appsrc.set_property("is-live", false);

        // Connect decodebin pad-added signal to link to audio sink
        let audio_sink_weak = audio_sink.downgrade();
        decodebin.connect_pad_added(move |_, src_pad| {
            if let Some(sink) = audio_sink_weak.upgrade() {
                let sink_pad = sink.static_pad("sink").unwrap();
                if !sink_pad.is_linked() {
                    let _ = src_pad.link(&sink_pad);
                }
            }
        });

        // Feed the embedded MP3 data to appsrc
        let sound_data = ALARM_SOUND.to_vec();
        let mut buffer = gstreamer::Buffer::with_size(sound_data.len()).unwrap();
        {
            let buffer_ref = buffer.get_mut().unwrap();
            buffer_ref.copy_from_slice(0, &sound_data).unwrap();
        }

        let _ = appsrc.push_buffer(buffer);
        let _ = appsrc.end_of_stream();

        // Set up bus message handling
        if let Some(bus) = pipeline.bus() {
            let pipeline_weak = pipeline.downgrade();
            let _ = bus.add_watch_local(move |_, msg| {
                use gstreamer::MessageView;
                match msg.view() {
                    MessageView::Eos(_) => {
                        if let Some(pipeline) = pipeline_weak.upgrade() {
                            let _ = pipeline.set_state(gstreamer::State::Null);
                        }
                        glib::ControlFlow::Break
                    }
                    MessageView::Error(err) => {
                        eprintln!("GStreamer error: {:?}", err.error());
                        if let Some(pipeline) = pipeline_weak.upgrade() {
                            let _ = pipeline.set_state(gstreamer::State::Null);
                        }
                        glib::ControlFlow::Break
                    }
                    _ => glib::ControlFlow::Continue,
                }
            });
        }

        let _ = pipeline.set_state(gstreamer::State::Playing);
        *self.player.borrow_mut() = Some(pipeline.upcast());
    }

    pub fn stop(&self) {
        if let Some(player) = self.player.borrow_mut().take() {
            let _ = player.set_state(gstreamer::State::Null);
        }
    }
}
