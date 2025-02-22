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
    // Use a larger buffer size to reduce audio artifacts
    let mut config = default_config.config();
    config.buffer_size = cpal::BufferSize::Fixed(2048);

    Handle {
        stream: match default_config.sample_format() {
            cpal::SampleFormat::F32 => run::<f32>(&device, &config, freq),
            cpal::SampleFormat::I16 => run::<i16>(&device, &config, freq),
            cpal::SampleFormat::U16 => run::<u16>(&device, &config, freq),
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
    let attack_time = 0.2; // Attack time in seconds (longer for even smoother start)
    let release_time = 0.35; // Release time in seconds (much longer for smoother fade-out)
    let sustain_level = 0.6; // Sustain level (0.0 to 1.0) (lower to reduce overall intensity)
    let total_duration = 5.0; // Total duration of the sound in seconds
    
    // Anti-pop filter parameters
    let crossfade_time = 0.05; // 50ms crossfade between states
    let dc_block_alpha = 0.995; // DC blocking filter coefficient

    let attack_samples = (attack_time * sample_rate) as u32;
    let release_samples = (release_time * sample_rate) as u32;
    let total_samples = (total_duration * sample_rate) as u32;

    let mut last_sample = 0.0;
    let mut last_output = 0.0;  // For DC blocking filter
    let mut last_freq = 0.0;    // For frequency crossfade
    let mut is_starting = true; // Track if we're just starting
    let mut start_time = 0.0;   // Track time since start
    let mut stop_requested = false; // Track if we're stopping
    let start_fade_duration = 0.1; // 100ms fade in
    let stop_fade_duration = 0.15; // 150ms fade out
    let mut next_value = move || {
        let mut base_freq = base_freq.clone();
        let freq = base_freq.borrow_mut().lock().unwrap();
        let freq = freq.value;
        
        // Crossfade between frequency changes to prevent pops
        let crossfade_samples = (crossfade_time * sample_rate) as u32;
        let freq_diff = freq - last_freq;
        let crossfade_factor = if freq_diff.abs() > 0.1 {
            (sample_clock % crossfade_samples as f32) / crossfade_samples as f32
        } else {
            1.0
        };
        let interpolated_freq = last_freq + freq_diff * crossfade_factor;
        last_freq = freq;

        sample_clock = (sample_clock + 1.0) % sample_rate;
        let current_sample = sample_clock as u32;

        // Calculate envelope with smoothed transitions
        let envelope = if current_sample < attack_samples {
            // Attack phase with smooth curve
            let progress = current_sample as f32 / attack_samples as f32;
            progress * progress * (3.0 - 2.0 * progress) // Smooth cubic interpolation
        } else if current_sample > total_samples - release_samples {
            // Release phase with exponential decay
            let release_progress = (current_sample - (total_samples - release_samples)) as f32
                / release_samples as f32;
            let exp_release = (-4.0 * release_progress).exp(); // Exponential decay
            sustain_level * exp_release
        } else {
            // Sustain phase with slight variation to prevent static sound
            let slight_wobble = 1.0 + 0.02 * (sample_clock * 0.1 * 2.0 * PI / sample_rate).sin();
            sustain_level * slight_wobble
        };

        let intensity = 1.0;

        // Base oscillator with interpolated frequency
        let base_hum = (sample_clock * interpolated_freq * 2.0 * PI / sample_rate).sin();

        // Extra gentle frequency modulation
        let mod_freq = 1.0 + intensity * 3.0; // Further reduced modulation
        let mod_depth = 2.0 * intensity; // Much gentler modulation depth
        let fm = mod_depth * (sample_clock * mod_freq * 2.0 * PI / sample_rate).sin();

        // Minimal amplitude modulation
        let am_freq = 0.2 + intensity * 0.5; // Very slow AM
        let am_depth = 0.03 + intensity * 0.1; // Very subtle AM depth
        let am =
            1.0 - am_depth + am_depth * (sample_clock * am_freq * 2.0 * PI / sample_rate).sin();

        // Gentler harmonics with interpolated frequency
        let harmonic_fade = (intensity * 0.5).min(1.0); // Even smoother harmonic fade
        let harmonic1 =
            0.2 * harmonic_fade * (sample_clock * interpolated_freq * 2.0 * 2.0 * PI / sample_rate).sin();
        let harmonic2 =
            0.1 * harmonic_fade * (sample_clock * interpolated_freq * 3.0 * 2.0 * PI / sample_rate).sin();
        let harmonic3 =
            0.05 * harmonic_fade * (sample_clock * interpolated_freq * 4.0 * 2.0 * PI / sample_rate).sin();

        // Combine elements with envelope shaping
        let raw_result = (base_hum + fm + harmonic1 + harmonic2 + harmonic3) * envelope * am * intensity;

        // Multi-stage smoothing pipeline
        // 1. Initial smoothing
        let smooth_alpha = if freq_diff.abs() > 0.1 { 0.95 } else { 0.9 };
        let smoothed = smooth_alpha * last_sample + (1.0 - smooth_alpha) * raw_result;
        last_sample = smoothed;

        // 2. DC blocking filter
        let dc_blocked = smoothed - last_output + dc_block_alpha * last_output;
        last_output = dc_blocked;

        // Apply volume control with fade-in/fade-out
        let freq = freq.borrow_mut().lock().unwrap();
        let mut volume = freq.volume;

        // Update timing
        if is_starting {
            start_time += 1.0 / sample_rate;
            let fade_factor = (start_time / start_fade_duration).min(1.0);
            volume *= fade_factor;
            if start_time >= start_fade_duration {
                is_starting = false;
            }
        }

        // Check if frequency is zero (indicating stop request)
        if freq.value == 0.0 && !stop_requested {
            stop_requested = true;
            start_time = 0.0;
        }

        // Handle fade-out if stop requested
        if stop_requested {
            start_time += 1.0 / sample_rate;
            let fade_factor = 1.0 - (start_time / stop_fade_duration).min(1.0);
            volume *= fade_factor;
        }

        // Final scaling with volume
        dc_blocked * volume / 65.0
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
