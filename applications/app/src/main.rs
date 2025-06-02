mod config;
mod events;
mod windows;

#[cfg(target_os = "macos")]
mod delegate;

use std::sync::{Arc, LazyLock};

use anyhow::Result;
use common::logger;
use events::MainEvents;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};

use self::{
    config::AppConfig,
    events::{EventChannel, EventTarget, Events, UserEvents},
    windows::WindowManager,
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| AppConfig::default());

struct App {
    initialized: bool,
    window_manager: WindowManager,
    events: Arc<EventChannel>,
}

impl ApplicationHandler<(EventTarget, Events)> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.initialized {
            self.initialized = true;

            self.window_manager.create(event_loop).unwrap();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.window_manager.window_event(event_loop, id, event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, (target, event): (EventTarget, Events)) {
        match event {
            Events::UserEvents(event) => {
                if let Err(e) = self.window_manager.user_event(target, event) {
                    log::error!("Unable to process user event, error={:?}", e);
                }
            }
            Events::InnerEvent(event) => {
                self.events.inner_event(event);
            }
            Events::MainEvents(event) => match event {
                MainEvents::Shutdown => {
                    event_loop.exit();
                }
            },
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.events
            .send(EventTarget::Frontend, UserEvents::OnMessagePumpPoll);
    }
}

impl App {
    fn new(events: Arc<EventChannel>) -> Result<Self> {
        Ok(Self {
            window_manager: WindowManager::new(events.clone())?,
            initialized: false,
            events,
        })
    }
}

fn main() -> Result<()> {
    logger::init_logger(
        log::LevelFilter::Info,
        Some(&format!("{}/logs/", &APP_CONFIG.cache_path)),
    )?;

    log::info!("app config = {:?}", *APP_CONFIG);

    let event_loop = EventLoop::<(EventTarget, Events)>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    // fix cef send event handle for winit 0.29
    #[cfg(target_os = "macos")]
    unsafe {
        delegate::inject_delegate();
    }

    let event_channel = EventChannel::new(event_loop.create_proxy());
    event_loop.run_app(&mut App::new(event_channel)?)?;
    Ok(())
}
