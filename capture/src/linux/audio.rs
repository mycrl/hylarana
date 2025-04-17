use crate::{AudioCaptureSourceDescription, CaptureHandler, Source};

use common::frame::AudioFrame;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioCaptureError {}

#[derive(Default)]
pub struct AudioCapture;

impl CaptureHandler for AudioCapture {
    type Frame = AudioFrame;
    type Error = AudioCaptureError;
    type CaptureOptions = AudioCaptureSourceDescription;

    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        Ok(Vec::new())
    }

    fn start<S: crate::FrameConsumer<Frame = Self::Frame> + 'static>(
        &self,
        _options: Self::CaptureOptions,
        _consumer: S,
    ) -> Result<(), Self::Error> {
        unimplemented!("audio capture is not supported on linux")
    }

    fn stop(&self) -> Result<(), Self::Error> {
        unimplemented!("audio capture is not supported on linux")
    }
}
