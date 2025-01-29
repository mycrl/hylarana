use crate::{CaptureHandler, FrameArrived, Source, VideoCaptureSourceDescription};

use common::frame::VideoFrame;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CameraCaptureError {}

#[derive(Default)]
pub struct CameraCapture;

impl CaptureHandler for CameraCapture {
    type Frame = VideoFrame;
    type Error = CameraCaptureError;
    type CaptureOptions = VideoCaptureSourceDescription;

    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        Ok(Vec::new())
    }

    fn start<S: FrameArrived<Frame = Self::Frame> + 'static>(
        &self,
        _options: Self::CaptureOptions,
        _arrived: S,
    ) -> Result<(), Self::Error> {
        unimplemented!("camera capture is not supported on linux")
    }

    fn stop(&self) -> Result<(), Self::Error> {
        unimplemented!("camera capture is not supported on linux")
    }
}
