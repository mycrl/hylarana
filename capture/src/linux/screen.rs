use crate::{CaptureHandler, FrameConsumer, Source, VideoCaptureSourceDescription};

use common::frame::VideoFrame;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScreenCaptureError {}

#[derive(Default)]
pub struct ScreenCapture;

impl CaptureHandler for ScreenCapture {
    type Frame = VideoFrame;
    type Error = ScreenCaptureError;
    type CaptureOptions = VideoCaptureSourceDescription;

    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        Ok(Vec::new())
    }

    fn start<S: FrameConsumer<Frame = Self::Frame> + 'static>(
        &self,
        _options: Self::CaptureOptions,
        _consumer: S,
    ) -> Result<(), Self::Error> {
        unimplemented!("screen capture is not supported on linux")
    }

    fn stop(&self) -> Result<(), Self::Error> {
        unimplemented!("screen capture is not supported on linux")
    }
}
