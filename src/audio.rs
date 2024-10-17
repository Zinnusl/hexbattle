use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use cpal::{FromSample, SizedSample};
use std::borrow::{Borrow, BorrowMut};
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

pub struct Handle {
    pub stream: Stream,
}

pub fn beep(freq: Arc<Mutex<crate::FreqWrapper>>) -> Handle {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    Handle {
        stream: match config.sample_format() {
            cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), freq),
            cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), freq),
            cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), freq),
            // not all supported sample formats are included in this example
            _ => panic!("Unsupported sample format!"),
        },
    }
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    base_freq: Arc<Mutex<crate::FreqWrapper>>,
) -> Stream
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    // Produce a sinusoid of maximum amplitude.
    let mut sample_clock = 0f32;
    let mod_freq = 2.0;
    let mod_depth = 50.0;
    let am_freq = 0.5;

    // Envelope parameters
    let attack_time = 0.1; // Attack time in seconds
    let release_time = 0.1; // Release time in seconds
    let sustain_level = 0.8; // Sustain level (0.0 to 1.0)
    let total_duration = 5.0; // Total duration of the sound in seconds

    let attack_samples = (attack_time * sample_rate) as u32;
    let release_samples = (release_time * sample_rate) as u32;
    let total_samples = (total_duration * sample_rate) as u32;

    let mut last_sample = 0.0;
    let mut next_value = move || {
        let mut base_freq = base_freq.clone();
        let freq = base_freq.borrow_mut().lock().unwrap();
        let freq = freq.value;
        sample_clock = (sample_clock + 1.0) % sample_rate;
        // (sample_clock * freq * 2.0 * PI / sample_rate).sin() / 50.0

        let current_sample = sample_clock as u32;

        // Calculate envelope
        let envelope = if current_sample < attack_samples {
            // Attack phase
            current_sample as f32 / attack_samples as f32
        } else if current_sample > total_samples - release_samples {
            // Release phase
            let release_progress = (current_sample - (total_samples - release_samples)) as f32
                / release_samples as f32;
            sustain_level * (1.0 - release_progress)
        } else {
            // Sustain phase
            sustain_level
        };

        let intensity = 1.0;

        // Frequency modulation
        let base_hum = (sample_clock * freq * 2.0 * PI / sample_rate).sin();

        // Frequency modulation (gets stronger over time)
        let mod_freq = 2.0 + intensity * 10.0; // Modulation frequency increases
        let mod_depth = 10.0 * intensity; // Modulation depth increases
        let fm = mod_depth * (sample_clock * mod_freq * 2.0 * PI / sample_rate).sin();

        // Amplitude modulation (gets stronger over time)
        let am_freq = 0.5 + intensity * 2.0; // AM frequency increases
        let am_depth = 0.1 + intensity * 0.4; // AM depth increases
        let am =
            1.0 - am_depth + am_depth * (sample_clock * am_freq * 2.0 * PI / sample_rate).sin();

        // Harmonic content (increases over time)
        let harmonic1 =
            0.5 * intensity * (sample_clock * freq * 2.0 * 2.0 * PI / sample_rate).sin();
        let harmonic2 =
            0.25 * intensity * (sample_clock * freq * 3.0 * 2.0 * PI / sample_rate).sin();

        let harmonic3 =
            0.125 * intensity * (sample_clock * freq * 4.0 * 2.0 * PI / sample_rate).sin();

        // Combine all elements
        let result =
            (base_hum + fm + harmonic1 + harmonic2 + harmonic3) * envelope * am * intensity;

        // Apply simple low-pass smoothing
        let alpha = 0.4;
        last_sample = alpha * result + (1.0 - alpha) * last_sample;

        last_sample / 50.0
    };

    let err_fn = |err| crate::console::console_log!("an error occurred on stream: {}", err);

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _| write_data(data, channels, &mut next_value),
            err_fn,
            None,
        )
        .unwrap();
    stream.play().unwrap();
    stream
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
where
    T: SizedSample + FromSample<f32>,
{
    for frame in output.chunks_mut(channels) {
        let value: T = T::from_sample(next_sample());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
