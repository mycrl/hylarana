mod main;
mod screen;

use std::sync::Arc;

use anyhow::Result;
use webview::Webview;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};

use crate::events::{Events, EventsManager};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WindowId {
    Main,
    Screen,
}

impl WindowId {
    pub const fn all() -> &'static [WindowId] {
        &[Self::Main, Self::Screen]
    }
}

pub trait WindowHandler: Send {
    fn id(&self) -> WindowId;

    #[allow(unused_variables)]
    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) -> Result<()> {
        Ok(())
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: &Events) -> Result<()>;
}

pub struct WindowsManager(Vec<Box<dyn WindowHandler + 'static>>);

impl WindowsManager {
    pub fn new(events_manager: EventsManager, webview: Arc<Webview>) -> Self {
        Self(vec![Box::new(main::MainWindow::new(
            events_manager,
            webview,
        ))])
    }
}

impl WindowsManager {
    pub fn window_event(&mut self, id: WindowId, event_loop: &ActiveEventLoop, event: WindowEvent) {
        for it in &mut self.0 {
            if it.id() == id {
                if let Err(e) = it.window_event(event_loop, &event) {
                    log::error!("failed to send window event, error={:?}", e);
                }
            }
        }
    }

    pub fn user_event(&mut self, id: WindowId, event_loop: &ActiveEventLoop, event: Events) {
        for it in &mut self.0 {
            if it.id() == id {
                if let Err(e) = it.user_event(event_loop, &event) {
                    log::error!("failed to send window event, error={:?}", e);
                }
            }
        }
    }
}
