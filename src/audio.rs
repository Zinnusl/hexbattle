use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use cpal::{FromSample, SizedSample};
use std::borrow::BorrowMut;
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
    let default_config = device.default_output_config().unwrap();
    // crate::console::console_log!("{:?}", default_config.buffer_size());
    // let mut config = default_config.config();
    // config.buffer_size = cpal::BufferSize::Fixed(4096 * 2);

    Handle {
        stream: match default_config.sample_format() {
            cpal::SampleFormat::F32 => run::<f32>(&device, &default_config.into(), freq),
            cpal::SampleFormat::I16 => run::<i16>(&device, &default_config.into(), freq),
            cpal::SampleFormat::U16 => run::<u16>(&device, &default_config.into(), freq),
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

    // Envelope parameters
    let attack_time = 0.15; // Attack time in seconds (slightly longer for smoother start)
    let release_time = 0.2; // Release time in seconds (longer for smoother fade-out)
    let sustain_level = 0.7; // Sustain level (0.0 to 1.0) (slightly lower to reduce harshness)
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

        // Gentler frequency modulation
        let mod_freq = 1.5 + intensity * 5.0; // Reduced modulation frequency
        let mod_depth = 5.0 * intensity; // Reduced modulation depth
        let fm = mod_depth * (sample_clock * mod_freq * 2.0 * PI / sample_rate).sin();

        // Gentler amplitude modulation
        let am_freq = 0.3 + intensity * 1.0; // Reduced AM frequency
        let am_depth = 0.05 + intensity * 0.2; // Reduced AM depth
        let am =
            1.0 - am_depth + am_depth * (sample_clock * am_freq * 2.0 * PI / sample_rate).sin();

        // Softer harmonic content with gradual fade-in
        let harmonic_fade = (intensity * 0.7).min(1.0); // Smoother harmonic introduction
        let harmonic1 =
            0.3 * harmonic_fade * (sample_clock * freq * 2.0 * 2.0 * PI / sample_rate).sin();
        let harmonic2 =
            0.15 * harmonic_fade * (sample_clock * freq * 3.0 * 2.0 * PI / sample_rate).sin();
        let harmonic3 =
            0.075 * harmonic_fade * (sample_clock * freq * 4.0 * 2.0 * PI / sample_rate).sin();

        // Combine all elements
        let result =
            (base_hum + fm + harmonic1 + harmonic2 + harmonic3) * envelope * am * intensity;

        // Apply enhanced low-pass smoothing with frequency-dependent smoothing
        let mut alpha = 0.85; // Increased smoothing factor for better transitions
        
        // Make smoothing more aggressive during frequency changes
        let freq_current = freq;
        static mut LAST_FREQ: f32 = 0.0;
        unsafe {
            if (LAST_FREQ - freq_current).abs() > 0.1 {
                alpha = 0.95; // Even more smoothing during frequency transitions
            }
            LAST_FREQ = freq_current;
        }
        
        last_sample = alpha * last_sample + (1.0 - alpha) * result;
        
        // Additional amplitude scaling to prevent clipping
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
