use std::sync::Arc;

use anyhow::{Result, anyhow};
use winit::{
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
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
        let primary_monitor = event_loop
            .primary_monitor()
            .ok_or_else(|| anyhow!("not found a primary_monitor"))?;

        let window = Arc::new(
            event_loop.create_window(
                WindowAttributes::default()
                    .with_title("Hylarana Remote View")
                    .with_inner_size(primary_monitor.size())
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

    pub fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        id: &winit::window::WindowId,
        event: &WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(window) = &self.window {
                    if id == &window.id() {
                        self.events
                            .send(EventTarget::Frontend, UserEvents::OnRemoteWindowClose);
                    }
                }
            }
            _ => (),
        }
    }
}
