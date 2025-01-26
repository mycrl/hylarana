use crate::{CaptureHandler, FrameArrived, Source, SourceType, VideoCaptureSourceDescription};

use std::{
    ops::Deref,
    sync::{atomic::AtomicBool, Arc},
    thread::{self, sleep},
    time::Duration,
};

use common::{
    atomic::EasyAtomic,
    frame::{VideoFormat, VideoFrame, VideoSubFormat},
};

use scrap::{Capturer, Display};
use thiserror::Error;
use yuv::{
    bgra_to_yuv_nv12, YuvBiPlanarImageMut, YuvChromaSubsampling, YuvRange, YuvStandardMatrix,
};

#[derive(Error, Debug)]
pub enum ScreenCaptureError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Default)]
pub struct ScreenCapture(Arc<AtomicBool>);

impl CaptureHandler for ScreenCapture {
    type Frame = VideoFrame;
    type Error = ScreenCaptureError;
    type CaptureOptions = VideoCaptureSourceDescription;

    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        Ok(vec![Source {
            index: 0,
            is_default: true,
            kind: SourceType::Screen,
            id: ":0.0".to_string(),
            name: "primary display".to_string(),
        }])
    }

    fn start<S: FrameArrived<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureOptions,
        mut arrived: S,
    ) -> Result<(), Self::Error> {
        let status = Arc::downgrade(&self.0);
        self.0.update(true);

        thread::Builder::new()
            .name("LinuxScreenCaptureThread".to_string())
            .spawn(move || {
                let mut capture =
                    Capturer::new(Display::primary().expect("not found primary screen"))
                        .expect("failed to create x11 capturer");

                let mut frame = VideoFrame::default();
                frame.width = capture.width() as u32;
                frame.height = capture.height() as u32;
                frame.sub_format = VideoSubFormat::SW;
                frame.format = VideoFormat::NV12;

                let mut planar_image = YuvBiPlanarImageMut::<u8>::alloc(
                    frame.width,
                    frame.height,
                    YuvChromaSubsampling::Yuv420,
                );

                while let Ok(avframe) = capture.frame() {
                    if let Some(status) = status.upgrade() {
                        if !status.get() {
                            break;
                        }
                    } else {
                        break;
                    }

                    if bgra_to_yuv_nv12(
                        &mut planar_image,
                        avframe.deref(),
                        frame.width * 4,
                        YuvRange::Limited,
                        YuvStandardMatrix::Bt601,
                    )
                    .is_err()
                    {
                        break;
                    }

                    frame.data[0] = planar_image.y_plane.borrow().as_ptr() as *const _;
                    frame.linesize[0] = planar_image.y_stride as usize;

                    frame.data[1] = planar_image.uv_plane.borrow().as_ptr() as *const _;
                    frame.linesize[1] = planar_image.uv_stride as usize;

                    if !arrived.sink(&frame) {
                        break;
                    }

                    sleep(Duration::from_millis(1000 / options.fps as u64));
                }
            })?;

        Ok(())
    }

    fn stop(&self) -> Result<(), Self::Error> {
        self.0.update(false);
        Ok(())
    }
}
