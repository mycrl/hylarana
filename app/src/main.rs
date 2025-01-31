mod events;
mod window;

use std::sync::Arc;

use anyhow::Result;
use events::EventsManager;
use hylarana::{shutdown, startup};
use image::{DynamicImage, ImageFormat};
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};
use webview::{execute_subprocess, is_subprocess, Webview, WebviewOptions};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
};

use self::{
    events::Events,
    window::{WindowId, WindowsManager},
};

static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

struct App {
    webview: Arc<Webview>,
    windows_manager: WindowsManager,
    events_manager: EventsManager,
    tray: Option<TrayIcon>,
}

impl App {
    async fn new(event_loop: Arc<EventLoopProxy<(WindowId, Events)>>) -> Result<Self> {
        let webview = Webview::new(&WebviewOptions::default()).await?;

        Ok(Self {
            windows_manager: WindowsManager::new(webview.clone()),
            events_manager: EventsManager::new(event_loop),
            tray: None,
            webview,
        })
    }
}

impl ApplicationHandler<(WindowId, Events)> for App {
    fn resumed(&mut self, _: &ActiveEventLoop) {
        startup().unwrap();

        let webview = self.webview.clone();
        let events_manager = self.events_manager.clone();
        RUNTIME.spawn(async move {
            webview.wait_exit().await;
            events_manager.broadcast(Events::CloseRequested).unwrap();
        });

        self.tray.replace(
            TrayIconBuilder::new()
                .with_tooltip("hylarana")
                .with_icon({
                    match image::load_from_memory_with_format(
                        include_bytes!("../../logo.ico"),
                        ImageFormat::Ico,
                    )
                    .unwrap()
                    {
                        DynamicImage::ImageRgba8(it) => {
                            let width = it.width();
                            let height = it.height();

                            Icon::from_rgba(it.into_vec(), width, height).unwrap()
                        }
                        it => {
                            unimplemented!("unsupports logo format={:?}", it);
                        }
                    }
                })
                .build()
                .unwrap(),
        );
    }

    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match event {
                TrayIconEvent::DoubleClick { button, .. } => {
                    if button == MouseButton::Left {
                        self.events_manager
                            .send(WindowId::Main, Events::EnableWindow)
                            .unwrap();
                    }
                }
                _ => (),
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.events_manager
                    .broadcast(Events::CloseRequested)
                    .unwrap();
            }
            _ => (),
        }

        self.windows_manager
            .window_event(WindowId::Screen, event_loop, event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, (id, event): (WindowId, Events)) {
        match event {
            Events::CloseRequested => {
                event_loop.exit();
            }
            _ => (),
        }

        self.windows_manager.user_event(id, event_loop, event);
    }
}

fn main() -> Result<()> {
    if is_subprocess() {
        execute_subprocess();
    }

    simple_logger::init_with_level(log::Level::Info)?;

    let event_loop = EventLoop::<(WindowId, Events)>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = Arc::new(event_loop.create_proxy());
    event_loop.run_app(&mut RUNTIME.block_on(App::new(proxy))?)?;

    shutdown()?;
    Ok(())
}
