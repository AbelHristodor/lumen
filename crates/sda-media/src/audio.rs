//! Sound output through rodio. `Sink::open` returns `None` when the box
//! has no output device, the player then runs on the wall clock, muted.

use crate::clock::{Clock, SampleClock};
use rodio::Source;
use rodio::stream::{DeviceSinkBuilder, MixerDeviceSink};
use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;

/// Shared between the feeder and the device callback.
#[derive(Default)]
struct Queue {
    samples: VecDeque<f32>,
}

/// A rodio source that plays whatever `feed` put in and silence when the
/// queue runs dry, so the stream never ends on its own.
struct Pcm {
    queue: Arc<Mutex<Queue>>,
    clock: Arc<SampleClock>,
}

impl Iterator for Pcm {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.clock.paused() {
            return Some(0.0);
        }
        let sample = self
            .queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .samples
            .pop_front()
            .unwrap_or(0.0);
        self.clock.advance(1);
        Some(sample)
    }
}

impl Source for Pcm {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(CHANNELS).expect("two channels")
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(RATE).expect("48 kHz")
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub struct Sink {
    _stream: MixerDeviceSink,
    player: rodio::Player,
    queue: Arc<Mutex<Queue>>,
    clock: Arc<SampleClock>,
}

impl Sink {
    /// Open the default output device and start a silent stream whose
    /// clock begins at `start`. `None` when there is no usable device.
    pub fn open(start: Duration) -> Option<Sink> {
        // Tests and headless boxes opt out, the player then runs on the
        // wall clock.
        if std::env::var_os("LUMEN_NO_AUDIO").is_some() {
            return None;
        }
        let stream = match DeviceSinkBuilder::open_default_sink() {
            Ok(s) => s,
            Err(e) => {
                log::warn!("no audio output: {e}");
                return None;
            }
        };
        let queue = Arc::new(Mutex::new(Queue::default()));
        let clock = Arc::new(SampleClock::new(start, RATE, CHANNELS));
        let player = rodio::Player::connect_new(stream.mixer());
        player.append(Pcm {
            queue: queue.clone(),
            clock: clock.clone(),
        });
        Some(Sink {
            _stream: stream,
            player,
            queue,
            clock,
        })
    }

    pub fn feed(&self, chunk: Vec<f32>) {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .samples
            .extend(chunk);
    }

    /// Samples waiting to play. The feeder stops pulling from ffmpeg when
    /// this grows past a few hundred milliseconds.
    pub fn queued(&self) -> usize {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .samples
            .len()
    }

    pub fn clear(&self) {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .samples
            .clear();
    }

    pub fn set_volume(&self, v: f32) {
        self.player.set_volume(v.clamp(0.0, 1.0));
    }

    pub fn pause(&self) {
        self.clock.pause();
    }

    pub fn play(&self) {
        self.clock.resume();
    }

    pub fn clock(&self) -> Arc<SampleClock> {
        self.clock.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_source_plays_the_queue_then_silence_and_counts() {
        let queue = Arc::new(Mutex::new(Queue::default()));
        let clock = Arc::new(SampleClock::new(Duration::ZERO, RATE, CHANNELS));
        queue.lock().unwrap().samples.extend([0.5, -0.5]);
        let mut pcm = Pcm {
            queue,
            clock: clock.clone(),
        };
        assert_eq!(pcm.next(), Some(0.5));
        assert_eq!(pcm.next(), Some(-0.5));
        assert_eq!(pcm.next(), Some(0.0), "silence when dry");
        assert_eq!(
            clock.now(),
            Duration::from_secs_f64(3.0 / 2.0 / RATE as f64)
        );
        clock.pause();
        assert_eq!(pcm.next(), Some(0.0));
        assert_eq!(
            clock.now(),
            Duration::from_secs_f64(3.0 / 2.0 / RATE as f64)
        );
    }
}
