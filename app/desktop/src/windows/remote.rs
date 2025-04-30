use std::sync::Arc;

use anyhow::Result;
use common::Size;
use winit::{
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::events::{EventChannel, EventTarget, UserEvents};

pub struct Remote {
    window: Option<Arc<Window>>,
    events: Arc<EventChannel>,
}

impl Remote {
    pub fn new(events: Arc<EventChannel>) -> Self {
        Self {
            window: None,
            events,
        }
    }

    pub fn create(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window = Arc::new(
            event_loop.create_window(
                WindowAttributes::default()
                    .with_title("Hylarana Remote View")
                    .with_visible(false),
            )?,
        );

        self.events.send(
            EventTarget::Frontend,
            UserEvents::OnRemoteWindowView(window.clone()),
        );

        self.window.replace(window);
        Ok(())
    }

    pub fn user_event(&mut self, event: &UserEvents) -> Result<()> {
        match event {
            UserEvents::SetRemoteWindowVisible(visible) => {
                if let Some(window) = &self.window {
                    window.set_visible(*visible);

                    if *visible {
                        window.focus_window();
                    }
                }
            }
            _ => (),
        }

        Ok(())
    }

    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|window| window.id())
    }

    pub fn window_event(&mut self, _event_loop: &ActiveEventLoop, event: &WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.events
                    .send(EventTarget::Frontend, UserEvents::OnRemoteWindowClose);
            }
            WindowEvent::Resized(size) => {
                self.events.send(
                    EventTarget::Frontend,
                    UserEvents::OnRemoteWindowResized(Size {
                        width: size.width,
                        height: size.height,
                    }),
                );
            }
            _ => (),
        }
    }
}
