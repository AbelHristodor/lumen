//! Sound output through rodio. `Sink::open` returns `None` when the box
//! has no output device, the player then runs on the wall clock, muted.
//!
//! The queue holds whole stereo frames, so a pause or a clear can never
//! split a frame and swap left and right. The clock counts only samples of
//! real audio. Silence from a dry queue or a pause does not move it. The
//! device starts pulling at `Sink::open`, before the first feed, and a
//! stall must not push the picture ahead of the sound for good.

use crate::clock::{Clock, SampleClock};
use rodio::Source;
use rodio::stream::{DeviceSinkBuilder, MixerDeviceSink};
use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;

/// Shared between the feeder and the device callback. One entry is one
/// stereo frame, left then right.
#[derive(Default)]
struct Queue {
    frames: VecDeque<[f32; 2]>,
}

/// A rodio source that plays whatever `feed` put in and silence when the
/// queue runs dry, so the stream never ends on its own. A frame is counted
/// on the clock when it is popped. The right sample then waits in
/// `pending` and plays next even if a pause comes in between. Silence is
/// never counted.
struct Pcm {
    queue: Arc<Mutex<Queue>>,
    clock: Arc<SampleClock>,
    pending: Option<f32>,
}

impl Iterator for Pcm {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.pending.take() {
            return Some(right);
        }
        if self.clock.paused() {
            return Some(0.0);
        }
        let frame = self
            .queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .frames
            .pop_front();
        let Some([left, right]) = frame else {
            return Some(0.0);
        };
        self.clock.advance(u64::from(CHANNELS));
        self.pending = Some(right);
        Some(left)
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
            pending: None,
        });
        Some(Sink {
            _stream: stream,
            player,
            queue,
            clock,
        })
    }

    /// Queue interleaved stereo samples. A trailing odd sample is dropped
    /// so frames stay whole.
    pub fn feed(&self, chunk: Vec<f32>) {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .frames
            .extend(chunk.as_chunks::<2>().0.iter().copied());
    }

    /// Interleaved samples waiting to play. The feeder stops pulling from
    /// ffmpeg when this grows past a few hundred milliseconds.
    pub fn queued(&self) -> usize {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .frames
            .len()
            * CHANNELS as usize
    }

    /// Drop every queued frame. A right sample already pending still plays.
    pub fn clear(&self) {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .frames
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

    fn pcm(frames: &[[f32; 2]]) -> (Pcm, Arc<SampleClock>) {
        let queue = Arc::new(Mutex::new(Queue::default()));
        queue.lock().unwrap().frames.extend(frames.iter().copied());
        let clock = Arc::new(SampleClock::new(Duration::ZERO, RATE, CHANNELS));
        let pcm = Pcm {
            queue,
            clock: clock.clone(),
            pending: None,
        };
        (pcm, clock)
    }

    fn frames(n: u32) -> Duration {
        Duration::from_secs_f64(f64::from(n) / f64::from(RATE))
    }

    #[test]
    fn pcm_source_plays_whole_frames_and_counts_only_real_audio() {
        let (mut pcm, clock) = pcm(&[[0.5, -0.5]]);
        assert_eq!(pcm.next(), Some(0.5));
        assert_eq!(clock.now(), frames(1), "counted when popped");
        assert_eq!(pcm.next(), Some(-0.5));
        assert_eq!(pcm.next(), Some(0.0), "silence when dry");
        assert_eq!(pcm.next(), Some(0.0));
        assert_eq!(clock.now(), frames(1), "silence is not counted");
        clock.pause();
        assert_eq!(pcm.next(), Some(0.0));
        assert_eq!(clock.now(), frames(1));
    }

    #[test]
    fn pause_mid_frame_plays_the_pending_right_sample_first() {
        let (mut pcm, clock) = pcm(&[[0.1, 0.2], [0.3, 0.4]]);
        assert_eq!(pcm.next(), Some(0.1));
        clock.pause();
        assert_eq!(pcm.next(), Some(0.2), "the frame finishes");
        assert_eq!(clock.now(), frames(1), "the frame was already counted");
        assert_eq!(pcm.next(), Some(0.0), "then silence");
        assert_eq!(clock.now(), frames(1));
        clock.resume();
        assert_eq!(pcm.next(), Some(0.3), "left stays left after resume");
        assert_eq!(pcm.next(), Some(0.4));
        assert_eq!(clock.now(), frames(2));
    }
}
