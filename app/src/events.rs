use std::sync::Arc;

use anyhow::Result;
use winit::event_loop::EventLoopProxy;

use crate::window::WindowId;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Events {
    EnableWindow,
    DisableWindow,
    CloseRequested,
}

#[derive(Clone)]
pub struct EventsManager(Arc<EventLoopProxy<(WindowId, Events)>>);

impl EventsManager {
    pub fn new(event_loop: Arc<EventLoopProxy<(WindowId, Events)>>) -> Self {
        Self(event_loop)
    }

    pub fn broadcast(&self, event: Events) -> Result<()> {
        for it in WindowId::all() {
            self.send(*it, event)?;
        }

        Ok(())
    }

    pub fn send(&self, id: WindowId, event: Events) -> Result<()> {
        self.0.send_event((id, event))?;

        log::info!("event manager, id={:?} event={:?}", id, event);

        Ok(())
    }
}
