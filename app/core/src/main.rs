mod manager;
mod message;

use self::{
    manager::DeviceManager,
    message::{Route, Stdio},
};

use std::{
    io::{stderr, Stderr, Write},
    net::Ipv4Addr,
    sync::{
        mpsc::{channel, Sender},
        Arc,
    },
    thread,
};

use anyhow::{anyhow, Result};
use common::{MediaStreamDescription, Size};
use hylarana::{
    create_receiver, create_sender, shutdown, startup, AVFrameStreamPlayer,
    AVFrameStreamPlayerOptions, Capture, HylaranaReceiver, HylaranaReceiverOptions, HylaranaSender,
    HylaranaSenderOptions, MediaStreamObserver, VideoDecoderType, VideoRenderBackend,
    VideoRenderOptionsBuilder, VideoRenderSurfaceOptions,
};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Fullscreen, Window, WindowAttributes},
};

#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
enum Status {
    // The sender has been created and is sending.
    Sending,
    // The receiver has been created and is receiving.
    Receiving,
    #[default]
    Idle,
}

struct HylaranaObserver {
    events: Arc<EventLoopProxy<Events>>,
    is_sender: bool,
}

impl MediaStreamObserver for HylaranaObserver {
    fn close(&self) {
        // The sender or receiver has closed and needs to be notified of the external
        // message loop processing.
        let _ = self.events.send_event(if self.is_sender {
            Events::SenderClosed
        } else {
            Events::ReceiverClosed
        });
    }
}

#[derive(Debug, Clone)]
enum Events {
    SetName(String),
    CreateSender {
        // Because creation is asynchronous, this `tx` is a message channel for notification of the
        // creation result.
        tx: Sender<Result<()>>,
        targets: Vec<Ipv4Addr>,
        options: HylaranaSenderOptions,
    },
    CreateReceiver {
        // Because creation is asynchronous, this `tx` is a message channel for notification of the
        // creation result.
        tx: Sender<Result<()>>,
        options: HylaranaReceiverOptions,
        backend: VideoRenderBackend,
        description: MediaStreamDescription,
    },
    CloseSender,
    CloseReceiver,
    SenderClosed,
    ReceiverClosed,
}

type ISender = HylaranaSender<(), HylaranaObserver>;
type IReceiver = HylaranaReceiver<AVFrameStreamPlayer<'static>, HylaranaObserver>;

struct App {
    router: Arc<Route<Stdio>>,
    status: Arc<RwLock<Status>>,
    manager: Arc<DeviceManager>,
    events: Arc<EventLoopProxy<Events>>,
    window: Option<Arc<Window>>,
    receiver: Option<IReceiver>,
    sender: Option<ISender>,
    name: Option<String>,
}

impl App {
    fn new(events: Arc<EventLoopProxy<Events>>) -> Result<Self> {
        let router = Route::new(Stdio::default());
        let manager = Arc::new(DeviceManager::new()?);
        let status: Arc<RwLock<Status>> = Arc::new(RwLock::new(Status::Idle));

        router.on(
            "SetName",
            |events, name: String| Ok(events.send_event(Events::SetName(name))?),
            events.clone(),
        );

        router.on(
            "GetDevices",
            |manager, _: ()| Ok(manager.get_devices()),
            manager.clone(),
        );

        router.on(
            "GetCaptureSources",
            |_, kind| Ok(Capture::get_sources(kind)?),
            (),
        );

        router.on(
            "CreateSender",
            |events, (targets, options): (Vec<Ipv4Addr>, HylaranaSenderOptions)| {
                let (tx, rx) = channel();
                events.send_event(Events::CreateSender {
                    tx,
                    targets,
                    options,
                })?;

                Ok(rx.recv()??)
            },
            events.clone(),
        );

        router.on(
            "CloseSender",
            |events, _: ()| {
                events.send_event(Events::CloseSender)?;

                Ok(())
            },
            events.clone(),
        );

        router.on(
            "CreateReceiver",
            |events,
             (decoder, backend, description): (
                VideoDecoderType,
                VideoRenderBackend,
                MediaStreamDescription,
            )| {
                let (tx, rx) = channel();
                events.send_event(Events::CreateReceiver {
                    tx,
                    options: HylaranaReceiverOptions {
                        video_decoder: decoder,
                    },
                    backend,
                    description,
                })?;

                Ok(rx.recv()??)
            },
            events.clone(),
        );

        router.on(
            "CloseReceiver",
            |events, _: ()| {
                events.send_event(Events::CloseReceiver)?;

                Ok(())
            },
            events.clone(),
        );

        router.on(
            "GetStatus",
            |status, _: ()| Ok(status.read().clone()),
            status.clone(),
        );

        // This is a separate thread for notifying external when the device list is
        // updated.
        let router_ = Arc::downgrade(&router);
        let mut watcher = manager.get_watcher();
        thread::spawn(move || {
            while watcher.change() {
                if let Some(router) = router_.upgrade() {
                    let _ = router.send("DevicesChangeNotify");
                } else {
                    break;
                }
            }
        });

        // Notifies the external process that it is ready to process the request.
        router.send("ReadyNotify")?;

        Ok(Self {
            window: None,
            manager,
            receiver: None,
            sender: None,
            router,
            events,
            name: None,
            status,
        })
    }
}

impl ApplicationHandler<Events> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // The window created by default is not visible.
        {
            self.window.replace(Arc::new(
                event_loop
                    .create_window(
                        WindowAttributes::default()
                            .with_fullscreen(Some(Fullscreen::Borderless(None)))
                            .with_visible(false),
                    )
                    .unwrap(),
            ));
        }

        startup().unwrap();
    }

    fn window_event(
        &mut self,
        _: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                let _ = self.events.send_event(Events::CloseReceiver);
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: Events) {
        match event {
            Events::SetName(name) => {
                self.name.replace(name.clone());
                self.manager.send_info(Vec::new(), name, None);
            }
            Events::CreateSender {
                tx,
                targets,
                options,
            } => {
                let func = || {
                    if self.status.read().clone() == Status::Idle && self.name.is_some() {
                        let sender = create_sender(
                            &options,
                            (),
                            HylaranaObserver {
                                events: self.events.clone(),
                                is_sender: true,
                            },
                        )?;

                        if let Some(name) = &self.name {
                            self.manager.send_info(
                                targets,
                                name.clone(),
                                Some(sender.get_description().clone()),
                            );
                        }

                        self.sender.replace(sender);
                        *self.status.write() = Status::Sending;
                        self.router.send("StatusChangeNotify")?;

                        Ok::<_, anyhow::Error>(())
                    } else {
                        Err(anyhow!("sender has been created"))
                    }
                };

                tx.send(func()).unwrap();
            }
            Events::CreateReceiver {
                tx,
                options,
                backend,
                description,
            } => {
                let mut func = || {
                    if self.status.read().clone() == Status::Idle && self.name.is_some() {
                        let window = self.window.as_ref().unwrap().clone();
                        let receiver = create_receiver(
                            &description,
                            &options,
                            AVFrameStreamPlayer::new(AVFrameStreamPlayerOptions::All(
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
                                .set_backend(backend)
                                .from_receiver(&description, &options)
                                .build(),
                            ))?,
                            HylaranaObserver {
                                events: self.events.clone(),
                                is_sender: false,
                            },
                        )?;

                        // The window also needs to display the notification created on the
                        // receiving end.
                        window.set_visible(true);

                        self.receiver.replace(receiver);
                        *self.status.write() = Status::Receiving;
                        self.router.send("StatusChangeNotify")?;

                        Ok::<_, anyhow::Error>(())
                    } else {
                        Err(anyhow!("receiver has been created"))
                    }
                };

                tx.send(func()).unwrap();
            }
            Events::CloseSender => {
                drop(self.sender.take());
            }
            Events::CloseReceiver => {
                drop(self.receiver.take());
            }
            Events::SenderClosed => {
                drop(self.sender.take());

                if let Some(name) = &self.name {
                    self.manager.send_info(Vec::new(), name.clone(), None);
                }

                *self.status.write() = Status::Idle;
                let _ = self.router.send("StatusChangeNotify");
            }
            Events::ReceiverClosed => {
                drop(self.receiver.take());

                // The receiver is closed and the window needs to be hidden.
                if let Some(window) = self.window.as_ref() {
                    window.set_visible(false);
                }

                *self.status.write() = Status::Idle;
                let _ = self.router.send("StatusChangeNotify");
            }
        }
    }
}

struct Logger(Stderr);

impl Default for Logger {
    fn default() -> Self {
        Self(stderr())
    }
}

impl log::Log for Logger {
    fn flush(&self) {}

    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        self.0
            .lock()
            .write_all(format!("{} - {}\n", record.level(), record.args()).as_bytes())
            .unwrap();
    }
}

fn main() -> Result<()> {
    log::set_max_level(log::LevelFilter::Info);
    log::set_boxed_logger(Box::new(Logger::default()))?;

    let mut event_loop_builder = EventLoop::<Events>::with_user_event();

    #[cfg(target_os = "macos")]
    event_loop_builder.with_activation_policy(ActivationPolicy::Prohibited);

    let event_loop = event_loop_builder.build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let events = Arc::new(event_loop.create_proxy());
    event_loop.run_app(&mut App::new(events)?)?;

    shutdown()?;
    Ok(())
}
