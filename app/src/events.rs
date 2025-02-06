use std::sync::Arc;

use common::MediaStreamDescription;
use hylarana::{VideoDecoderType, VideoRenderBackend};
use winit::event_loop::EventLoopProxy;

use crate::window::WindowId;

#[derive(Debug, Clone)]
pub enum Events {
    EnableWindow,
    DisableWindow,
    CloseRequested,
    CreateReceiver {
        description: MediaStreamDescription,
        decoder: VideoDecoderType,
        backend: VideoRenderBackend,
    },
    CreateReceiverResult(bool),
}

#[derive(Clone)]
pub struct EventsManager(Arc<EventLoopProxy<(WindowId, Events)>>);

impl EventsManager {
    pub fn new(event_loop: EventLoopProxy<(WindowId, Events)>) -> Self {
        Self(Arc::new(event_loop))
    }

    pub fn broadcast(&self, event: Events) {
        for it in WindowId::all() {
            self.send(*it, event.clone());
        }
    }

    pub fn send(&self, id: WindowId, event: Events) {
        if let Err(e) = self.0.send_event((id, event)) {
            log::error!("failed to send event in manager, error={:?}", e);
        }
    }
}
