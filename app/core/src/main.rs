mod devices;
mod message;

use std::{
    io::{stderr, Stderr, Write},
    sync::{Arc, LazyLock},
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
use parking_lot::{Mutex, RwLock};
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

#[derive(Debug, Clone, Copy)]
enum Events {
    CloseSender,
    CloseReceiver,
    SenderClosed,
    ReceiverClosed,
}

type ISender = HylaranaSender<(), HylaranaObserver>;
type IReceiver = HylaranaReceiver<AVFrameStreamPlayer<'static>, HylaranaObserver>;

struct App {
    router: Arc<Route<Stdio>>,
    window: Arc<RwLock<Option<Arc<Window>>>>,
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
        let window: Arc<RwLock<Option<Arc<Window>>>> = Default::default();
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
                |(events, sender, receiver, devices_manager),
                 (name_list, options): (Vec<String>, HylaranaSenderOptions)| {
                    let mut sender = sender.lock();
                    if sender.is_none() && receiver.lock().is_none() {
                        sender.replace({
                            let sender = create_sender(
                                &options,
                                (),
                                HylaranaObserver {
                                    is_sender: true,
                                    events,
                                },
                            )?;

                            devices_manager
                                .set_description(name_list, sender.get_description().clone())?;
                            sender
                        });

                        Ok(())
                    } else {
                        Err(anyhow!("sender has been created"))
                    }
                },
                (
                    events.clone(),
                    sender.clone(),
                    receiver.clone(),
                    devices_manager.clone(),
                ),
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
                |(events, window, sender, receiver),
                 (options, backend, description): (
                    HylaranaReceiverOptions,
                    VideoRenderBackend,
                    MediaStreamDescription,
                )| {
                    let window = if let Some(it) = window.read().as_ref() {
                        it.clone()
                    } else {
                        return Err(anyhow!("window not created"));
                    };

                    let mut receiver = receiver.lock();
                    if receiver.is_none() && sender.lock().is_none() {
                        receiver.replace(create_receiver(
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
                                is_sender: false,
                                events,
                            },
                        )?);

                        Ok(())
                    } else {
                        Err(anyhow!("receiver has been created"))
                    }
                },
                (
                    events.clone(),
                    window.clone(),
                    sender.clone(),
                    receiver.clone(),
                ),
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
                    RUNTIME.spawn(async move {
                        let _ = router.call::<(), ()>("DevicesChangeNotify", ()).await;
                    });
                } else {
                    break;
                }
            }
        });

        {
            let router_ = router.clone();
            RUNTIME.spawn(async move {
                let _ = router_.call::<(), ()>("ReadyNotify", ()).await;
            });
        }

        Ok(Self {
            receiver,
            sender,
            window,
            router,
        })
    }
}

impl ApplicationHandler<Events> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        {
            self.window.write().replace(Arc::new(
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
            Events::CloseSender => {
                let _ = self.sender.lock().take();
            }
            Events::CloseReceiver => {
                let _ = self.receiver.lock().take();
            }
            Events::SenderClosed => {
                let router = self.router.clone();
                RUNTIME.spawn(async move {
                    let _ = router.call::<_, ()>("SenderClosedNotify", ()).await;
                });
            }
            Events::ReceiverClosed => {
                let router = self.router.clone();
                RUNTIME.spawn(async move {
                    let _ = router.call::<_, ()>("ReceiverClosedNotify", ()).await;
                });
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
