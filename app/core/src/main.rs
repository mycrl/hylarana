mod devices;
mod message;

use std::sync::Arc;

use anyhow::{anyhow, Result};
use clap::Parser;
use common::{MediaStreamDescription, Size};
use hylarana::{
    create_receiver, create_sender, shutdown, startup, AVFrameStreamPlayer,
    AVFrameStreamPlayerOptions, Capture, HylaranaReceiver, HylaranaReceiverOptions, HylaranaSender,
    HylaranaSenderOptions, MediaStreamObserver, VideoRenderBackend, VideoRenderOptionsBuilder,
    VideoRenderSurfaceOptions,
};

use tokio::{
    runtime::Handle,
    sync::{Mutex, RwLock},
};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes},
};

use self::{devices::DevicesManager, message::Route};

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
    handle: Handle,
    router: Arc<Route>,
    window: Arc<RwLock<Option<Arc<Window>>>>,
    sender: Arc<Mutex<Option<ISender>>>,
    receiver: Arc<Mutex<Option<IReceiver>>>,
}

impl App {
    async fn new(events: Arc<EventLoopProxy<Events>>) -> Result<Self> {
        let args = Args::parse();
        let router = Route::new().await;
        let devices_manager = Arc::new(DevicesManager::new(args.name.clone()).await?);

        let sender: Arc<Mutex<Option<ISender>>> = Default::default();
        let receiver: Arc<Mutex<Option<IReceiver>>> = Default::default();
        let window: Arc<RwLock<Option<Arc<Window>>>> = Default::default();
        {
            router
                .on(
                    "GetDevices",
                    |devices_manager, _: ()| async move { Ok(devices_manager.get_devices().await) },
                    devices_manager.clone(),
                )
                .await;

            router
                .on(
                    "GetCaptureSources",
                    |_, kind| async move {
                        Ok(Handle::current()
                            .spawn_blocking(move || Capture::get_sources(kind))
                            .await??)
                    },
                    (),
                )
                .await;

            router
                .on(
                    "CreateSender",
                    |(events, sender, devices_manager), (name_list, options): (Vec<String>, HylaranaSenderOptions)| async move {
                        let mut sender = sender.lock().await;
                        if sender.is_none() {
                            sender.replace({
                                let sender = create_sender(&options, (),HylaranaObserver {
                                    is_sender: true,
                                    events,
                                })?;

                                devices_manager.set_description(name_list, sender.get_description().clone()).await;
                                sender
                            });

                            Ok(())
                        } else {
                            Err(anyhow!("sender has been created"))
                        }
                    },
                    (events.clone(), sender.clone(), devices_manager.clone()),
                )
                .await;

            router
                .on(
                    "CloseSender",
                    |events, _: ()| async move {
                        events.send_event(Events::CloseSender)?;

                        Ok(())
                    },
                    events.clone(),
                )
                .await;

            router
                .on(
                    "CreateReceiver",
                    |(events, window, receiver),
                     (options, backend, description): (
                        HylaranaReceiverOptions,
                        VideoRenderBackend,
                        MediaStreamDescription,
                    )| async move {
                        let window = if let Some(it) = window.read().await.as_ref() {
                            it.clone()
                        } else {
                            return Err(anyhow!("window not created"));
                        };

                        let mut receiver = receiver.lock().await;
                        if !receiver.is_none() {
                            return Err(anyhow!("receiver has been created"));
                        }

                        receiver.replace({
                            create_receiver(
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
                            )?
                        });

                        Ok(())
                    },
                    (events.clone(), window.clone(), receiver.clone()),
                )
                .await;

            router
                .on(
                    "CloseReceiver",
                    |events, _: ()| async move {
                        events.send_event(Events::CloseReceiver)?;

                        Ok(())
                    },
                    events.clone(),
                )
                .await;
        }

        let router_ = Arc::downgrade(&router);
        let mut watcher = devices_manager.get_watcher();
        tokio::spawn(async move {
            while watcher.change().await {
                if let Some(router) = router_.upgrade() {
                    let _ = router.call::<(), ()>("DevicesChangeNotify", ()).await;
                } else {
                    break;
                }
            }
        });

        let _ = router.call::<(), ()>("ReadyNotify", ()).await;

        Ok(Self {
            handle: Handle::current(),
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
            let window = Arc::new(
                event_loop
                    .create_window(
                        WindowAttributes::default()
                            .with_inner_size(PhysicalSize::new(1280, 720))
                            .with_visible(false),
                    )
                    .unwrap(),
            );

            let opt = self.window.clone();
            self.handle.spawn(async move {
                opt.write().await.replace(window);
            });
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
                drop(self.sender.blocking_lock().take());
            }
            Events::CloseReceiver => {
                drop(self.receiver.blocking_lock().take());
            }
            Events::SenderClosed => {
                let _ = self
                    .handle
                    .block_on(self.router.call::<_, ()>("SenderClosedNotify", ()));
            }
            Events::ReceiverClosed => {
                let _ = self
                    .handle
                    .block_on(self.router.call::<_, ()>("ReceiverClosedNotify", ()));
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let event_loop = EventLoop::<Events>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let events = Arc::new(event_loop.create_proxy());
    event_loop.run_app(&mut App::new(events).await?)?;
    Ok(())
}
