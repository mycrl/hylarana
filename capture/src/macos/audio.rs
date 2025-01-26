use std::{ops::DerefMut, sync::atomic::AtomicBool};

use crate::{AudioCaptureSourceDescription, CaptureHandler, FrameArrived, Source, SourceType};

use common::{atomic::EasyAtomic, frame::AudioFrame, macos::get_format_description_info};
use core_foundation::{base::TCFType, error::CFError};
use parking_lot::Mutex;
use resample::AudioResampler;
use screencapturekit::{
    output::CMSampleBuffer,
    shareable_content::SCShareableContent,
    stream::{
        configuration::SCStreamConfiguration, content_filter::SCContentFilter,
        output_trait::SCStreamOutputTrait, output_type::SCStreamOutputType, SCStream,
    },
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioCaptureError {
    #[error("{0}")]
    CoreFoundationError(String),
}

impl From<CFError> for AudioCaptureError {
    fn from(value: CFError) -> Self {
        Self::CoreFoundationError(format!("{}", value.description()))
    }
}

#[derive(Default)]
pub struct AudioCapture(Mutex<Option<SCStream>>);

unsafe impl Send for AudioCapture {}
unsafe impl Sync for AudioCapture {}

impl CaptureHandler for AudioCapture {
    type Frame = AudioFrame;
    type Error = AudioCaptureError;
    type CaptureOptions = AudioCaptureSourceDescription;

    // Get the default input device. In theory, all microphones will be listed here.
    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        Ok(vec![Source {
            id: "screen audio".to_string(),
            name: "screen audio".to_string(),
            kind: SourceType::Audio,
            is_default: true,
            index: 0,
        }])
    }

    fn start<S: crate::FrameArrived<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureOptions,
        arrived: S,
    ) -> Result<(), Self::Error> {
        let mut stream = SCStream::new(
            &SCContentFilter::new().with_display_excluding_windows(
                &SCShareableContent::get()?.displays().remove(0),
                &[],
            ),
            &SCStreamConfiguration::new()
                .set_captures_audio(true)?
                .set_channel_count(2)?,
        );

        let mut frame = AudioFrame::default();
        frame.sample_rate = options.sample_rate;

        stream.add_output_handler(
            Capture {
                ctx: Mutex::new(CaptureContext {
                    arrived,
                    frame,
                    resampler: None,
                }),
                status: AtomicBool::new(true),
            },
            SCStreamOutputType::Audio,
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

struct CaptureContext<S: FrameArrived<Frame = AudioFrame> + 'static> {
    arrived: S,
    frame: AudioFrame,
    resampler: Option<AudioResampler>,
}

struct Capture<S: FrameArrived<Frame = AudioFrame> + 'static> {
    ctx: Mutex<CaptureContext<S>>,
    status: AtomicBool,
}

impl<S> SCStreamOutputTrait for Capture<S>
where
    S: FrameArrived<Frame = AudioFrame> + 'static,
{
    fn did_output_sample_buffer(&self, buffer: CMSampleBuffer, _: SCStreamOutputType) {
        if !self.status.get() {
            log::warn!("macos screen audio capture stops because sink returns false");

            return;
        }

        if buffer.make_data_ready().is_ok() {
            let mut lock = self.ctx.lock();
            let CaptureContext {
                arrived,
                frame,
                resampler,
            } = lock.deref_mut();

            if resampler.is_none() {
                let info = if let Some(it) = buffer
                    .get_format_description()
                    .ok()
                    .map(|it| get_format_description_info(it.as_concrete_TypeRef() as _))
                    .flatten()
                {
                    it
                } else {
                    self.status.update(false);

                    return;
                };

                if let Ok(it) = AudioResampler::new(
                    info.mSampleRate,
                    frame.sample_rate as f64,
                    info.mFramesPerPacket as usize,
                ) {
                    resampler.replace(it);
                } else {
                    self.status.update(false);

                    return;
                }
            }

            if let Ok(buffer) = buffer.get_audio_buffer_list() {
                if let Some(resampler) = resampler {
                    for buffer in buffer.buffers() {
                        let sample = resampler
                            .resample(unsafe { std::mem::transmute(buffer.data()) }, 2)
                            .unwrap();

                        frame.data = sample.as_ptr();

                        if !arrived.sink(frame) {
                            self.status.update(false);

                            return;
                        }
                    }
                }
            }
        }
    }
}
