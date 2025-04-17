use crate::{AudioCaptureSourceDescription, CaptureHandler, FrameConsumer, Source, SourceType};

use std::sync::LazyLock;

use common::frame::AudioFrame;
use cpal::{Host, Stream, StreamConfig, traits::*};
use parking_lot::Mutex;
use resample::{
    AudioResampler, AudioResamplerError, AudioResamplerOutput, AudioSampleDescription,
    AudioSampleFormat,
};

use thiserror::Error;

// Just use a default audio port globally.
static HOST: LazyLock<Host> = LazyLock::new(|| cpal::default_host());

#[derive(Error, Debug)]
pub enum AudioCaptureError {
    #[error("not found the audio source")]
    NotFoundAudioSource,
    #[error(transparent)]
    DeviceError(#[from] cpal::DevicesError),
    #[error(transparent)]
    DeviceNameError(#[from] cpal::DeviceNameError),
    #[error(transparent)]
    DefaultStreamConfigError(#[from] cpal::DefaultStreamConfigError),
    #[error(transparent)]
    BuildStreamError(#[from] cpal::BuildStreamError),
    #[error(transparent)]
    PlayStreamError(#[from] cpal::PlayStreamError),
    #[error(transparent)]
    PauseStreamError(#[from] cpal::PauseStreamError),
    #[error(transparent)]
    AudioResamplerError(#[from] AudioResamplerError),
}

enum DeviceKind {
    Input,
    Output,
}

#[derive(Default)]
pub struct AudioCapture(Mutex<Option<Stream>>);

unsafe impl Send for AudioCapture {}
unsafe impl Sync for AudioCapture {}

impl CaptureHandler for AudioCapture {
    type Frame = AudioFrame;
    type Error = AudioCaptureError;
    type CaptureOptions = AudioCaptureSourceDescription;

    // Get the default input device. In theory, all microphones will be listed here.
    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        let default_name = HOST
            .default_output_device()
            .map(|it| it.name().ok())
            .flatten();

        // If you ever need to switch back to recording, you just need to capture the
        // output device, which is really funny, but very simple and worth mentioning!
        let mut sources = Vec::with_capacity(20);
        for (index, device) in HOST
            .output_devices()?
            .chain(HOST.input_devices()?)
            .enumerate()
        {
            sources.push(Source {
                id: device.name()?,
                name: device.name()?,
                kind: SourceType::Audio,
                is_default: device.name().ok() == default_name,
                index,
            });
        }

        Ok(sources)
    }

    fn start<S: FrameConsumer<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureOptions,
        consumer: S,
    ) -> Result<(), Self::Error> {
        // Find devices with matching names
        let (device, kind) = HOST
            .output_devices()?
            .map(|it| (it, DeviceKind::Output))
            .chain(HOST.input_devices()?.map(|it| (it, DeviceKind::Input)))
            .find(|(it, _)| {
                it.name()
                    .map(|name| name == options.source.name)
                    .unwrap_or(false)
            })
            .ok_or_else(|| AudioCaptureError::NotFoundAudioSource)?;

        let mut config: StreamConfig = match kind {
            DeviceKind::Input => device.default_input_config()?.into(),
            DeviceKind::Output => device.default_output_config()?.into(),
        };

        config.channels = 2;

        let mut frame = AudioFrame::default();
        frame.sample_rate = options.sample_rate;

        let mut resampler = AudioResampler::new(
            // config.sample_rate.0 as f64,
            AudioSampleDescription {
                sample_bits: AudioSampleFormat::I16,
                sample_rate: config.sample_rate.0,
                channels: 2,
            },
            // options.sample_rate as f64,
            AudioSampleDescription {
                sample_bits: AudioSampleFormat::I16,
                sample_rate: options.sample_rate,
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
        )?;

        let mut playing = true;
        let stream = device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                // When any problem occurs in the process, you should not continue processing.
                // If the cpal bottom layer continues to push audio samples, it should be
                // ignored here and the process should not continue.
                if !playing {
                    return;
                }

                if resampler.resample(data).is_err() {
                    playing = false;
                }
            },
            |e| {
                // An error has occurred, but there is nothing you can do at this moment except
                // output the error log.
                log::error!("audio capture callback error={:?}", e);
            },
            None,
        )?;

        stream.play()?;

        // If there is a previous stream, end it first.
        // Normally, a Capture instance is only used once, but here a defensive process
        // is done to avoid multiple calls due to external errors.
        if let Some(stream) = self.0.lock().replace(stream) {
            stream.pause()?;
        }

        Ok(())
    }

    fn stop(&self) -> Result<(), Self::Error> {
        if let Some(stream) = self.0.lock().take() {
            stream.pause()?;
        }

        Ok(())
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
