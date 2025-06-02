pub mod frontend;
pub mod remote;

use std::sync::Arc;

use anyhow::Result;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowId};

use self::{frontend::Frontend, remote::Remote};

use crate::events::{EventChannel, EventTarget, UserEvents};

pub struct WindowManager {
    frontend: Frontend,
    remote: Remote,
}

impl WindowManager {
    pub fn new(events: Arc<EventChannel>) -> Result<Self> {
        Ok(Self {
            remote: Remote::new(events.clone()),
            frontend: Frontend::new(events)?,
        })
    }

    pub fn create(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        self.remote.create(event_loop)?;
        self.frontend.create(event_loop)?;

        Ok(())
    }

    pub fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self
            .frontend
            .window_id()
            .map(|it| it == id)
            .unwrap_or(false)
        {
            self.frontend.window_event(event_loop, &event);
        }

        if self.remote.window_id().map(|it| it == id).unwrap_or(false) {
            self.remote.window_event(event_loop, &event);
        }
    }

    pub fn user_event(&mut self, target: EventTarget, event: UserEvents) -> Result<()> {
        match target {
            EventTarget::Frontend => {
                self.frontend.user_event(&event)?;
            }
            EventTarget::Remote => {
                self.remote.user_event(&event)?;
            }
            _ => (),
        }

        Ok(())
    }
}
