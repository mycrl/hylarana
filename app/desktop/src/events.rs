use std::sync::Arc;

use common::Size;
use winit::{event_loop::EventLoopProxy, window::Window};

#[derive(Clone)]
pub enum UserEvents {
    OnRemoteWindowView(Arc<Window>),
    SetRemoteWindowVisible(bool),
    OnRemoteWindowClose,
    OnWebviewAppContextInitialized,
    OnMessagePumpPoll,
    OnRemoteWindowResized(Size),
}

pub enum MainEvents {
    Shutdown,
}

pub enum InnerEvent {
    RunOnMainThread(Box<dyn FnOnce() + Send + Sync + 'static>),
}

pub enum Events {
    InnerEvent(InnerEvent),
    UserEvents(UserEvents),
    MainEvents(MainEvents),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTarget {
    Frontend,
    Remote,
    Main,
}

pub struct EventChannel {
    events: EventLoopProxy<(EventTarget, Events)>,
}

impl EventChannel {
    pub fn new(events: EventLoopProxy<(EventTarget, Events)>) -> Arc<Self> {
        Arc::new(Self { events })
    }

    pub fn send(&self, target: EventTarget, event: UserEvents) {
        if let Err(_) = self.events.send_event((target, Events::UserEvents(event))) {
            log::error!("send event to channel failed.");
        }
    }

    pub fn send_to_main(&self, event: MainEvents) {
        if let Err(_) = self
            .events
            .send_event((EventTarget::Main, Events::MainEvents(event)))
        {
            log::error!("send main event to channel failed.");
        }
    }

    pub fn inner_event(&self, event: InnerEvent) {
        match event {
            InnerEvent::RunOnMainThread(handle) => {
                (handle)();
            }
        }
    }

    pub fn run_in_main_thread<T>(&self, handle: T)
    where
        T: FnOnce() + Send + Sync + 'static,
    {
        let _ = self.events.send_event((
            EventTarget::Main,
            Events::InnerEvent(InnerEvent::RunOnMainThread(Box::new(handle))),
        ));
    }
}
