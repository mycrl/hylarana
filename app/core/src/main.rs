mod devices;
mod message;

use std::{
    io::{stderr, Stderr, Write},
    sync::{
        mpsc::{channel, Sender},
        Arc, LazyLock,
    },
    thread,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use common::{MediaStreamDescription, Size};
use hylarana::{
    create_receiver, create_sender, shutdown, startup, AVFrameStreamPlayer,
    AVFrameStreamPlayerOptions, Capture, HylaranaReceiver, HylaranaReceiverOptions, HylaranaSender,
    HylaranaSenderOptions, MediaStreamObserver, VideoRenderBackend, VideoRenderOptionsBuilder,
    VideoRenderSurfaceOptions,
};

use message::Stdio;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes},
};

use self::{devices::DevicesManager, message::Route};

static RUNTIME: LazyLock<Runtime> =
    LazyLock::new(|| Runtime::new().expect("failed to create tokio runtime, this is a bug"));

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
enum Status {
    Sending,
    Receiving,
    #[default]
    Idle,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    name: String,
}

struct HylaranaObserver {
    events: Arc<EventLoopProxy<Events>>,
    is_sender: bool,
}

impl MediaStreamObserver for HylaranaObserver {
    fn close(&self) {
        let _ = self.events.send_event(if self.is_sender {
            Events::SenderClosed
        } else {
            Events::ReceiverClosed
        });
    }
}

#[derive(Debug, Clone)]
enum Events {
    CreateSender {
        tx: Sender<Result<()>>,
        names: Vec<String>,
        options: HylaranaSenderOptions,
    },
    CreateReceiver {
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
    window: Option<Arc<Window>>,
    router: Arc<Route<Stdio>>,
    devices_manager: Arc<DevicesManager>,
    events: Arc<EventLoopProxy<Events>>,
    sender: Arc<Mutex<Option<ISender>>>,
    receiver: Arc<Mutex<Option<IReceiver>>>,
}

impl App {
    fn new(events: Arc<EventLoopProxy<Events>>) -> Result<Self> {
        let args = Args::parse();
        let router = Route::new(Stdio::default());
        let devices_manager = Arc::new(DevicesManager::new(args.name.clone())?);

        let sender: Arc<Mutex<Option<ISender>>> = Default::default();
        let receiver: Arc<Mutex<Option<IReceiver>>> = Default::default();
        {
            router.on(
                "GetDevices",
                |devices_manager, _: ()| Ok(devices_manager.get_devices()),
                devices_manager.clone(),
            );

            router.on(
                "GetCaptureSources",
                |_, kind| Ok(Capture::get_sources(kind)?),
                (),
            );

            router.on(
                "CreateSender",
                |(events, sender, receiver),
                 (names, options): (Vec<String>, HylaranaSenderOptions)| {
                    if sender.lock().is_none() && receiver.lock().is_none() {
                        let (tx, rx) = channel();
                        events.send_event(Events::CreateSender { tx, names, options })?;

                        Ok(rx.recv()??)
                    } else {
                        Err(anyhow!("sender has been created"))
                    }
                },
                (events.clone(), sender.clone(), receiver.clone()),
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
                |(events, sender, receiver),
                 (options, backend, description): (
                    HylaranaReceiverOptions,
                    VideoRenderBackend,
                    MediaStreamDescription,
                )| {
                    if receiver.lock().is_none() && sender.lock().is_none() {
                        let (tx, rx) = channel();
                        events.send_event(Events::CreateReceiver {
                            tx,
                            options,
                            backend,
                            description,
                        })?;

                        Ok(rx.recv()??)
                    } else {
                        Err(anyhow!("receiver has been created"))
                    }
                },
                (events.clone(), sender.clone(), receiver.clone()),
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
                |(sender, receiver), _: ()| {
                    Ok(if sender.lock().is_some() {
                        Status::Sending
                    } else if receiver.lock().is_some() {
                        Status::Receiving
                    } else {
                        Status::Idle
                    })
                },
                (sender.clone(), receiver.clone()),
            );
        }

        let router_ = Arc::downgrade(&router);
        let mut watcher = devices_manager.get_watcher();
        thread::spawn(move || {
            while watcher.change() {
                if let Some(router) = router_.upgrade() {
                    router.send_event("DevicesChangeNotify");
                } else {
                    break;
                }
            }
        });

        router.send_event("ReadyNotify");

        Ok(Self {
            window: None,
            devices_manager,
            receiver,
            sender,
            router,
            events,
        })
    }
}

impl ApplicationHandler<Events> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        {
            self.window.replace(Arc::new(
                event_loop
                    .create_window(
                        WindowAttributes::default()
                            .with_inner_size(PhysicalSize::new(1280, 720))
                            .with_visible(false),
                    )
                    .unwrap(),
            ));
        }

        startup().unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                shutdown().unwrap();
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: Events) {
        match event {
            Events::CreateSender { tx, names, options } => {
                let func = || {
                    let sender = create_sender(
                        &options,
                        (),
                        HylaranaObserver {
                            events: self.events.clone(),
                            is_sender: true,
                        },
                    )?;

                    self.devices_manager
                        .set_description(names, sender.get_description().clone())?;

                    self.sender.lock().replace(sender);
                    self.router.send_event("SenderCreatedNotify");

                    Ok::<_, anyhow::Error>(())
                };

                tx.send(func()).unwrap();
            }
            Events::CreateReceiver {
                tx,
                options,
                backend,
                description,
            } => {
                let func = || {
                    let window = self.window.as_ref().unwrap().clone();

                    window.set_visible(true);

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
                                window,
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

                    self.receiver.lock().replace(receiver);
                    self.router.send_event("ReceiverCreatedNotify");

                    Ok::<_, anyhow::Error>(())
                };

                tx.send(func()).unwrap();
            }
            Events::CloseSender => {
                let _ = self.sender.lock().take();
            }
            Events::CloseReceiver => {
                let _ = self.receiver.lock().take();
            }
            Events::SenderClosed => {
                self.router.send_event("SenderClosedNotify");
            }
            Events::ReceiverClosed => {
                if let Some(window) = self.window.as_ref() {
                    window.set_visible(false);
                }

                self.router.send_event("ReceiverClosedNotify");
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

    let event_loop = EventLoop::<Events>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let events = Arc::new(event_loop.create_proxy());
    event_loop.run_app(&mut App::new(events)?)?;
    Ok(())
}
