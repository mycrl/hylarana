use std::sync::Arc;

use anyhow::{anyhow, Result};
use hylarana::{
    create_receiver, AVFrameObserver, AVFrameStreamPlayer, AVFrameStreamPlayerOptions,
    HylaranaReceiver, HylaranaReceiverOptions, Size, VideoRenderOptionsBuilder,
    VideoRenderSurfaceOptions,
};

use winit::{
    event_loop::ActiveEventLoop,
    window::{Fullscreen, Window, WindowAttributes},
};

use super::{Events, EventsManager, WindowHandler, WindowId};

struct Receiver {
    window: Arc<Window>,
    receiver: HylaranaReceiver<AVFrameStreamPlayer<'static, Player>>,
}

pub struct ScreenWindow {
    events_manager: EventsManager,
    receiver: Option<Receiver>,
}

impl ScreenWindow {
    pub fn new(events_manager: EventsManager) -> Self {
        Self {
            receiver: None,
            events_manager,
        }
    }
}

impl WindowHandler for ScreenWindow {
    fn id(&self) -> WindowId {
        WindowId::Screen
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: &Events) -> Result<()> {
        match event {
            Events::CreateReceiver {
                description,
                decoder,
                backend,
            } => {
                let mut func = || {
                    if self.receiver.is_none() {
                        let mut attr = WindowAttributes::default();
                        {
                            attr.fullscreen = Some(Fullscreen::Borderless(None));
                            attr.resizable = false;
                            attr.maximized = false;
                        }

                        let window = Arc::new(event_loop.create_window(attr)?);
                        let receiver = {
                            let options = HylaranaReceiverOptions {
                                video_decoder: *decoder,
                            };

                            create_receiver(
                                description,
                                &options,
                                AVFrameStreamPlayer::new(
                                    AVFrameStreamPlayerOptions::All(
                                        VideoRenderOptionsBuilder::new(VideoRenderSurfaceOptions {
                                            size: {
                                                let size = window.inner_size();
                                                Size {
                                                    width: size.width,
                                                    height: size.height,
                                                }
                                            },
                                            window: window.clone(),
                                        })
                                        .set_backend(*backend)
                                        .from_receiver(&description, &options)
                                        .build(),
                                    ),
                                    Player {
                                        events_manager: self.events_manager.clone(),
                                    },
                                )?,
                            )?
                        };

                        self.receiver.replace(Receiver { receiver, window });

                        Ok(())
                    } else {
                        Err(anyhow!("receiver is exists"))
                    }
                };

                self.events_manager
                    .send(WindowId::Main, Events::CreateReceiverResult(func().is_ok()));
            }
            Events::DisableWindow => {
                drop(self.receiver.take());
            }
            _ => (),
        }

        Ok(())
    }
}

struct Player {
    events_manager: EventsManager,
}

impl AVFrameObserver for Player {
    fn close(&self) {
        self.events_manager
            .send(WindowId::Screen, Events::DisableWindow);
    }
}
