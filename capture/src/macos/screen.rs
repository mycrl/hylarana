use std::{ops::DerefMut, sync::atomic::AtomicBool};

use crate::{CaptureHandler, FrameArrived, Source, SourceType, VideoCaptureSourceDescription};

use thiserror::Error;

use common::{
    atomic::EasyAtomic,
    frame::{VideoFormat, VideoFrame, VideoSubFormat},
};

use core_foundation::{base::TCFType, error::CFError};
use core_media_rs::cm_time::CMTime;
use parking_lot::Mutex;
use screencapturekit::{
    output::CMSampleBuffer,
    shareable_content::SCShareableContent,
    stream::{
        configuration::{pixel_format::PixelFormat, SCStreamConfiguration},
        content_filter::SCContentFilter,
        output_trait::SCStreamOutputTrait,
        output_type::SCStreamOutputType,
        SCStream,
    },
};

#[derive(Error, Debug)]
pub enum ScreenCaptureError {
    #[error("{0}")]
    CoreFoundationError(String),
    #[error("not found capture source device")]
    NotFoundDevice,
}

impl From<CFError> for ScreenCaptureError {
    fn from(value: CFError) -> Self {
        Self::CoreFoundationError(format!("{}", value.description()))
    }
}

#[derive(Default)]
pub struct ScreenCapture(Mutex<Option<SCStream>>);

impl CaptureHandler for ScreenCapture {
    type Frame = VideoFrame;
    type Error = ScreenCaptureError;
    type CaptureOptions = VideoCaptureSourceDescription;

    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        Ok(SCShareableContent::with_options()
            .on_screen_windows_only()
            .get()?
            .displays()
            .into_iter()
            .map(|it| {
                let id = it.display_id();

                Source {
                    kind: SourceType::Screen,
                    index: id as usize,
                    is_default: id == 1,
                    id: id.to_string(),
                    name: format!("{} {}x{}", id, it.width(), it.height()),
                }
            })
            .collect())
    }

    fn start<S: FrameArrived<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureOptions,
        arrived: S,
    ) -> Result<(), Self::Error> {
        let display = SCShareableContent::with_options()
            .on_screen_windows_only()
            .get()?
            .displays()
            .into_iter()
            .find(|it| it.display_id() == options.source.index as u32)
            .ok_or_else(|| ScreenCaptureError::NotFoundDevice)?;

        let mut frame = VideoFrame::default();
        frame.sub_format = VideoSubFormat::CvPixelBufferRef;
        frame.format = VideoFormat::BGRA;
        frame.width = options.size.width;
        frame.height = options.size.height;
        frame.linesize = [frame.width as usize * 4, 0, 0];

        let mut stream = SCStream::new(
            &SCContentFilter::new().with_display_excluding_windows(&display, &[]),
            &SCStreamConfiguration::default()
                .set_captures_audio(false)?
                .set_width(frame.width)?
                .set_height(frame.height)?
                .set_pixel_format(PixelFormat::BGRA)?
                .set_minimum_frame_interval(&CMTime {
                    value: 1,
                    timescale: options.fps as i32,
                    flags: 0,
                    epoch: 0,
                })?,
        );

        stream.add_output_handler(
            Capture {
                ctx: Mutex::new(CaptureContext { arrived, frame }),
                status: AtomicBool::new(true),
            },
            SCStreamOutputType::Screen,
        );

        stream.start_capture()?;
        self.0.lock().replace(stream);

        Ok(())
    }

    fn stop(&self) -> Result<(), Self::Error> {
        if let Some(stream) = self.0.lock().take() {
            stream.stop_capture()?;
        }

        Ok(())
    }
}

struct CaptureContext<S: FrameArrived<Frame = VideoFrame> + 'static> {
    arrived: S,
    frame: VideoFrame,
}

struct Capture<S: FrameArrived<Frame = VideoFrame> + 'static> {
    ctx: Mutex<CaptureContext<S>>,
    status: AtomicBool,
}

impl<S> SCStreamOutputTrait for Capture<S>
where
    S: FrameArrived<Frame = VideoFrame> + 'static,
{
    fn did_output_sample_buffer(&self, buffer: CMSampleBuffer, _: SCStreamOutputType) {
        if !self.status.get() {
            return;
        }

        if !{
            if let Ok(buffer) = buffer.get_pixel_buffer() {
                let mut lock = self.ctx.lock();
                let CaptureContext { arrived, frame } = lock.deref_mut();

                frame.data[0] = buffer.as_concrete_TypeRef() as _;
                arrived.sink(&frame)
            } else {
                false
            }
        } {
            self.status.update(false);
        }
    }
}
