//! The playback clock the picture follows. With sound it counts the
//! samples the device pulled, so picture and sound cannot drift. Without
//! sound it is the wall clock since play, so a silent box still plays.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub trait Clock: Send + Sync {
    /// Position in the file.
    fn now(&self) -> Duration;
    fn pause(&self);
    fn resume(&self);
    fn paused(&self) -> bool;
}

/// Wall clock from `start`, frozen while paused.
pub struct WallClock {
    start: Duration,
    state: Mutex<(Instant, Duration, bool)>,
}

impl WallClock {
    pub fn new(start: Duration) -> Self {
        WallClock {
            start,
            state: Mutex::new((Instant::now(), Duration::ZERO, false)),
        }
    }
}

impl Clock for WallClock {
    fn now(&self) -> Duration {
        let (since, banked, paused) = *self.state.lock().unwrap_or_else(|e| e.into_inner());
        let run = if paused {
            Duration::ZERO
        } else {
            since.elapsed()
        };
        self.start + banked + run
    }

    fn pause(&self) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !st.2 {
            let run = st.0.elapsed();
            st.1 += run;
            st.2 = true;
        }
    }

    fn resume(&self) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.2 {
            st.0 = Instant::now();
            st.2 = false;
        }
    }

    fn paused(&self) -> bool {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).2
    }
}

/// Counts interleaved samples the audio device consumed.
pub struct SampleClock {
    start: Duration,
    rate: u32,
    channels: u16,
    samples: AtomicU64,
    paused: AtomicBool,
}

impl SampleClock {
    pub fn new(start: Duration, rate: u32, channels: u16) -> Self {
        SampleClock {
            start,
            rate,
            channels,
            samples: AtomicU64::new(0),
            paused: AtomicBool::new(false),
        }
    }

    /// The device pulled `n` interleaved samples. Ignored while paused,
    /// rodio keeps pulling silence then.
    pub fn advance(&self, n: u64) {
        if !self.paused.load(Ordering::Relaxed) {
            self.samples.fetch_add(n, Ordering::Relaxed);
        }
    }
}

impl Clock for SampleClock {
    fn now(&self) -> Duration {
        let samples = self.samples.load(Ordering::Relaxed) as f64;
        let per_sec = f64::from(self.rate) * f64::from(self.channels);
        self.start + Duration::from_secs_f64(samples / per_sec)
    }

    fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn wall_clock_runs_and_pauses() {
        let c = WallClock::new(Duration::from_secs(5));
        sleep(Duration::from_millis(30));
        assert!(c.now() >= Duration::from_millis(5_030));
        c.pause();
        let at = c.now();
        sleep(Duration::from_millis(20));
        assert_eq!(c.now(), at);
        c.resume();
        sleep(Duration::from_millis(20));
        assert!(c.now() > at);
    }

    #[test]
    fn sample_clock_counts_samples_from_its_offset() {
        let c = SampleClock::new(Duration::from_secs(1), 48_000, 2);
        c.advance(48_000 * 2);
        assert_eq!(c.now(), Duration::from_secs(2));
        c.pause();
        c.advance(48_000);
        assert_eq!(
            c.now(),
            Duration::from_secs(2),
            "paused clocks ignore samples"
        );
    }
}
