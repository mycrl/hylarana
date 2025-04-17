use std::sync::Arc;

use anyhow::{Result, anyhow};
use hylarana::{
    AVFrameStreamPlayer, AVFrameStreamPlayerOptions, Capture, HylaranaReceiver,
    HylaranaReceiverOptions, HylaranaSender, HylaranaSenderOptions, MediaStreamDescription,
    MediaStreamObserver, Size, Source, SourceType, VideoRenderBackend, VideoRenderOptionsBuilder,
    VideoRenderSurfaceOptions, shutdown, startup,
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use winit::window::Window;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Sending,
    Receiving,
    #[default]
    Idle,
}

pub struct CoreService {
    status: Arc<Mutex<Status>>,
    sender: Mutex<Option<HylaranaSender<(), StreamObserver>>>,
    receiver: Mutex<Option<HylaranaReceiver<AVFrameStreamPlayer<'static>, StreamObserver>>>,
}

impl CoreService {
    pub fn get_sources(kind: SourceType) -> Result<Vec<Source>> {
        Ok(Capture::get_sources(kind)?)
    }
}

impl CoreService {
    pub fn init() -> Result<()> {
        startup()?;

        Ok(())
    }

    pub fn new() -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            status: Arc::new(Mutex::new(Status::default())),
            receiver: Default::default(),
            sender: Default::default(),
        }))
    }

    pub fn create_sender<T>(
        &self,
        options: &HylaranaSenderOptions,
        callback: T,
    ) -> Result<MediaStreamDescription>
    where
        T: Fn() + Send + Sync + 'static,
    {
        let mut status = self.status.lock();
        if *status != Status::Idle {
            return Err(anyhow!("The current status does not allow this."));
        }

        let callback = Arc::new(callback);
        let sender = hylarana::create_sender(
            options,
            (),
            StreamObserver {
                status: self.status.clone(),
                callback,
            },
        )?;

        let description = sender.get_description().clone();

        *status = Status::Sending;
        self.sender.lock().replace(sender);

        Ok(description)
    }

    pub fn close_sender(&self) {
        drop(self.sender.lock().take());
    }

    pub fn create_receiver<T>(
        &self,
        description: &MediaStreamDescription,
        options: &HylaranaReceiverOptions,
        window: Arc<Window>,
        backend: VideoRenderBackend,
        callback: T,
    ) -> Result<()>
    where
        T: Fn() + Send + Sync + 'static,
    {
        let mut status = self.status.lock();
        if *status != Status::Idle {
            return Err(anyhow!("The current status does not allow this."));
        }

        let callback = Arc::new(callback);
        let receiver = hylarana::create_receiver(
            description,
            options,
            AVFrameStreamPlayer::new(AVFrameStreamPlayerOptions::All(
                VideoRenderOptionsBuilder::new(VideoRenderSurfaceOptions {
                    size: {
                        let size = window.inner_size();
                        Size {
                            width: size.width,
                            height: size.height,
                        }
                    },
                    window,
                })
                .set_backend(backend)
                .from_receiver(&description, &options)
                .build(),
            ))?,
            StreamObserver {
                status: self.status.clone(),
                callback,
            },
        )?;

        *status = Status::Receiving;
        self.receiver.lock().replace(receiver);

        Ok(())
    }

    pub fn close_receiver(&self) {
        drop(self.receiver.lock().take());
    }

    pub fn get_status(&self) -> Status {
        self.status.lock().to_owned()
    }
}

impl Drop for CoreService {
    fn drop(&mut self) {
        let _ = shutdown();
    }
}

struct StreamObserver {
    status: Arc<Mutex<Status>>,
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl MediaStreamObserver for StreamObserver {
    fn close(&self) {
        *self.status.lock() = Status::Idle;
        (self.callback)();
    }
}
