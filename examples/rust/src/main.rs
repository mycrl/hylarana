use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;
use clap::Parser;
use hylarana::{
    create_receiver, create_sender, shutdown, startup, AVFrameObserver, AVFrameStreamPlayer,
    AVFrameStreamPlayerOptions, AudioOptions, Capture, DiscoveryService, HylaranaReceiver,
    HylaranaReceiverOptions, HylaranaSender, HylaranaSenderMediaOptions, HylaranaSenderOptions,
    HylaranaSenderTrackOptions, MediaStreamDescription, Size, SourceType, TransportOptions,
    TransportStrategy, VideoDecoderType, VideoEncoderType, VideoOptions, VideoRenderBackend,
    VideoRenderOptionsBuilder, VideoRenderSurfaceOptions,
};

use parking_lot::Mutex;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

trait GetSize {
    fn size(&self) -> Size;
}

impl GetSize for Window {
    fn size(&self) -> Size {
        let size = self.inner_size();
        Size {
            width: size.width,
            height: size.height,
        }
    }
}

struct ViewObserver;

impl AVFrameObserver for ViewObserver {
    fn close(&self) {
        println!("view is closed");
    }
}

#[allow(unused)]
struct Sender {
    sender: HylaranaSender<AVFrameStreamPlayer<'static, ViewObserver>>,
    discovery: DiscoveryService,
}

impl Sender {
    fn new(configure: &Configure, window: Arc<Window>) -> Result<Self> {
        let video_options = configure.get_video_options();

        // Get the first screen that can be captured.
        let mut video = None;
        if let Some(source) = Capture::get_sources(SourceType::Screen)?.get(0) {
            video = Some(HylaranaSenderTrackOptions {
                options: video_options.clone(),
                source: source.clone(),
            });
        }

        // Get the first audio input device that can be captured.
        let mut audio = None;
        if !cfg!(target_os = "macos") {
            if let Some(source) = Capture::get_sources(SourceType::Audio)?.get(0) {
                audio = Some(HylaranaSenderTrackOptions {
                    source: source.clone(),
                    options: AudioOptions {
                        sample_rate: 48000,
                        bit_rate: 64000,
                    },
                });
            }
        }

        let options = HylaranaSenderOptions {
            media: HylaranaSenderMediaOptions { video, audio },
            transport: TransportOptions {
                strategy: configure.get_strategy().unwrap(),
                mtu: 1500,
            },
        };

        let sender = create_sender(
            &options,
            AVFrameStreamPlayer::new(
                AVFrameStreamPlayerOptions::OnlyVideo(
                    VideoRenderOptionsBuilder::new(VideoRenderSurfaceOptions {
                        size: window.size(),
                        window,
                    })
                    .set_backend(configure.backend)
                    .from_sender(&options)
                    .build(),
                ),
                ViewObserver,
            )?,
        )?;

        // Register the current sender's information with the LAN discovery service so
        // that other receivers can know that the sender has been created and can access
        // the sender's information.
        let discovery = DiscoveryService::register(3456, sender.get_description())?;

        Ok(Self { discovery, sender })
    }
}

#[allow(unused)]
struct Receiver {
    receiver: Arc<Mutex<Option<HylaranaReceiver<AVFrameStreamPlayer<'static, ViewObserver>>>>>,
    discovery: DiscoveryService,
}

impl Receiver {
    fn new(configure: Configure, window: Arc<Window>) -> Result<Self> {
        let video_decoder = configure.decoder;

        let receiver = Arc::new(Mutex::new(None));
        let receiver_ = Arc::downgrade(&receiver);

        // Find published senders through the LAN discovery service.
        let discovery =
            DiscoveryService::query(move |addrs, mut description: MediaStreamDescription| {
                if let Some(receiver) = receiver_.upgrade() {
                    let window = window.clone();

                    // If the sender has already been created, no further sender postings are
                    // processed.
                    if receiver.lock().is_some() {
                        return;
                    }

                    // The sender, if using passthrough, will need to replace the ip in the publish
                    // address by replacing the ip address with the sender's ip.
                    if let TransportStrategy::Direct(addr) = &mut description.transport.strategy {
                        addr.set_ip(IpAddr::V4(addrs[0]));
                    }

                    let options = HylaranaReceiverOptions { video_decoder };
                    receiver.lock().replace(
                        create_receiver(
                            &description,
                            &options,
                            AVFrameStreamPlayer::new(
                                AVFrameStreamPlayerOptions::All(
                                    VideoRenderOptionsBuilder::new(VideoRenderSurfaceOptions {
                                        size: window.size(),
                                        window,
                                    })
                                    .set_backend(configure.backend)
                                    .from_receiver(&description, &options)
                                    .build(),
                                ),
                                ViewObserver,
                            )
                            .unwrap(),
                        )
                        .unwrap(),
                    );
                }
            })?;

        Ok(Self {
            discovery,
            receiver,
        })
    }
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    receiver: Option<Receiver>,
    sender: Option<Sender>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        (|| {
            let mut attr = Window::default_attributes();
            attr.title = "hylarana example".to_string();
            attr.active = true;
            attr.inner_size = Some(winit::dpi::Size::Physical(PhysicalSize::new(1280, 720)));

            self.window
                .replace(Arc::new(event_loop.create_window(attr)?));
            startup()?;

            Ok::<_, anyhow::Error>(())
        })()
        .unwrap()
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            // The user closes the window, and we close the sender and receiver, in that order, and
            // release the renderer and hylarana instances, and finally stop the message loop.
            WindowEvent::CloseRequested => {
                drop(self.sender.take());
                drop(self.receiver.take());

                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if !event.repeat && event.state == ElementState::Released {
                    if let PhysicalKey::Code(key) = event.physical_key {
                        match key {
                            // When the S key is pressed, the sender is created, but check to see if
                            // the sender has already been created between sender creation to avoid
                            // duplicate creation.
                            //
                            // The receiving end is the same.
                            KeyCode::KeyS => {
                                if let (None, Some(window)) = (&self.sender, &self.window) {
                                    self.sender.replace(
                                        Sender::new(&Configure::parse(), window.clone()).unwrap(),
                                    );
                                }
                            }
                            KeyCode::KeyR => {
                                if let (None, Some(window)) = (&self.receiver, &self.window) {
                                    self.receiver.replace(
                                        Receiver::new(Configure::parse(), window.clone()).unwrap(),
                                    );
                                }
                            }
                            // When the S key is pressed, either the transmitter or the receiver
                            // needs to be turned off. No distinction is made here; both the
                            // transmitter and the receiver are turned off.
                            KeyCode::KeyK => {
                                drop(self.receiver.take());
                                drop(self.sender.take());
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

#[derive(Parser)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
struct Configure {
    /// The address to which the hylarana service is bound, indicating how to
    /// connect to the hylarana service.
    #[arg(long)]
    address: Option<SocketAddr>,
    /// direct, relay, multicast
    #[arg(long)]
    strategy: Option<String>,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
    #[arg(long, default_value_t = 24)]
    fps: u8,
    /// Each sender and receiver need to be bound to a channel, and the receiver
    /// can only receive the cast screen within the channel.
    #[arg(long, default_value_t = 0)]
    id: u32,
    #[arg(
        long,
        value_parser = clap::value_parser!(VideoEncoderType),
        default_value_t = Self::DEFAULT_ENCODER,
    )]
    encoder: VideoEncoderType,
    #[arg(
        long,
        value_parser = clap::value_parser!(VideoDecoderType),
        default_value_t = Self::DEFAULT_DECODER,
    )]
    decoder: VideoDecoderType,
    #[arg(
        long,
        value_parser = clap::value_parser!(VideoRenderBackend),
        default_value_t = Self::DEFAULT_BACKEND,
    )]
    backend: VideoRenderBackend,
}

impl Configure {
    #[cfg(target_os = "macos")]
    const DEFAULT_ENCODER: VideoEncoderType = VideoEncoderType::VideoToolBox;

    #[cfg(target_os = "windows")]
    const DEFAULT_ENCODER: VideoEncoderType = VideoEncoderType::Qsv;

    #[cfg(target_os = "linux")]
    const DEFAULT_ENCODER: VideoEncoderType = VideoEncoderType::X264;

    #[cfg(target_os = "macos")]
    const DEFAULT_DECODER: VideoDecoderType = VideoDecoderType::VideoToolBox;

    #[cfg(target_os = "windows")]
    const DEFAULT_DECODER: VideoDecoderType = VideoDecoderType::D3D11;

    #[cfg(target_os = "linux")]
    const DEFAULT_DECODER: VideoDecoderType = VideoDecoderType::H264;

    const DEFAULT_BACKEND: VideoRenderBackend = VideoRenderBackend::WebGPU;

    fn get_strategy(&self) -> Option<TransportStrategy> {
        Some(match self.strategy.as_ref()?.as_str() {
            "direct" => TransportStrategy::Direct(self.address?),
            "relay" => TransportStrategy::Relay(self.address?),
            "multicast" => TransportStrategy::Multicast(self.address?),
            _ => unreachable!(),
        })
    }

    fn get_video_options(&self) -> VideoOptions {
        VideoOptions {
            codec: self.encoder,
            frame_rate: self.fps,
            width: self.width,
            height: self.height,
            bit_rate: 10000000,
            key_frame_interval: 21,
        }
    }
}

fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    Configure::parse();

    // Creates a message loop, which is used to create the main window.
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut App::default())?;

    // When exiting the application, the environment of hylarana should be cleaned
    // up.
    shutdown()?;
    Ok(())
}
