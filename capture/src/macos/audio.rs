use std::{slice::from_raw_parts, sync::atomic::AtomicBool};

use crate::{AudioCaptureSourceDescription, CaptureHandler, FrameConsumer, Source, SourceType};

use thiserror::Error;

use common::{atomic::EasyAtomic, frame::AudioFrame};
use core_foundation::error::CFError;
use parking_lot::Mutex;
use resample::{
    AudioResampler, AudioResamplerError, AudioResamplerOutput, AudioSampleDescription,
    AudioSampleFormat,
};

use screencapturekit::{
    output::CMSampleBuffer,
    shareable_content::SCShareableContent,
    stream::{
        SCStream, configuration::SCStreamConfiguration, content_filter::SCContentFilter,
        output_trait::SCStreamOutputTrait, output_type::SCStreamOutputType,
    },
};

#[derive(Error, Debug)]
pub enum AudioCaptureError {
    #[error("{0}")]
    CoreFoundationError(String),
    #[error("not found capture source device")]
    NotFoundDevice,
    #[error(transparent)]
    AudioResamplerError(#[from] AudioResamplerError),
}

impl From<CFError> for AudioCaptureError {
    fn from(value: CFError) -> Self {
        Self::CoreFoundationError(format!("{}", value.description()))
    }
}

#[derive(Default)]
pub struct AudioCapture(Mutex<Option<SCStream>>);

impl CaptureHandler for AudioCapture {
    type Frame = AudioFrame;
    type Error = AudioCaptureError;
    type CaptureOptions = AudioCaptureSourceDescription;

    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        Ok(vec![Source {
            name: "screen audio".to_string(),
            id: "screen audio".to_string(),
            kind: SourceType::Audio,
            is_default: true,
            index: 0,
        }])
    }

    fn start<S: FrameConsumer<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureOptions,
        consumer: S,
    ) -> Result<(), Self::Error> {
        let mut stream = SCStream::new(
            &SCContentFilter::new().with_display_excluding_windows(
                &SCShareableContent::get()?.displays().remove(0),
                &[],
            ),
            &SCStreamConfiguration::default()
                .set_captures_audio(true)?
                .set_channel_count(1)?,
        );

        stream.add_output_handler(
            Capture {
                status: AtomicBool::new(true),
                resampler: Mutex::new(AudioResampler::new(
                    AudioSampleDescription {
                        sample_bits: AudioSampleFormat::F32,
                        sample_rate: 48000,
                        channels: 1,
                    },
                    AudioSampleDescription {
                        sample_rate: options.sample_rate,
                        sample_bits: AudioSampleFormat::I16,
                        channels: 2,
                    },
                    Output {
                        consumer,
                        frame: {
                            let mut frame = AudioFrame::default();
                            frame.sample_rate = options.sample_rate;

                            frame
                        },
                    },
                )?),
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

struct Capture {
    resampler: Mutex<AudioResampler<f32, i16>>,
    status: AtomicBool,
}

impl SCStreamOutputTrait for Capture {
    fn did_output_sample_buffer(&self, buffer: CMSampleBuffer, _: SCStreamOutputType) {
        if !self.status.get() {
            log::warn!("macos screen audio capture stops because sink returns false");

            return;
        }

        if buffer.make_data_ready().is_ok() {
            if let Ok(list) = buffer.get_audio_buffer_list() {
                let mut resampler = self.resampler.lock();

                if let Some(buffer) = list.buffers().first() {
                    if let Err(e) = resampler.resample(unsafe {
                        from_raw_parts(buffer.data().as_ptr() as _, buffer.data().len() / 4)
                    }) {
                        log::error!("resample audio buffer error={:?}", e);

                        self.status.set(false);
                        return;
                    }
                }
            }
        }
    }
}

struct Output<S> {
    consumer: S,
    frame: AudioFrame,
}

impl<S> AudioResamplerOutput<i16> for Output<S>
where
    S: FrameConsumer<Frame = AudioFrame> + 'static,
{
    fn output(&mut self, buffer: &[i16], frames: u32) -> bool {
        self.frame.data = buffer.as_ptr();
        self.frame.frames = frames;

        self.consumer.sink(&self.frame)
    }
}
