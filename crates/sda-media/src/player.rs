//! One file, two ffmpeg children, one clock. `open` probes and starts a
//! paused segment at zero. `seek` kills the segment and starts another at
//! the new time. `tick` moves frames into the `FrameSource` up to the
//! clock and notices the end. All methods take `&self`, the state lives
//! behind one mutex so the app can hold the player in an `Arc`.

use crate::audio::Sink;
use crate::clock::{Clock, WallClock};
use crate::decode::{self, Timed};
use crate::probe::{self, Info};
use sda_render::FrameSource;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Playing,
    Paused,
    Ended,
}

/// The running children and channels for one start offset.
struct Segment {
    start: Duration,
    video: Child,
    frames: Receiver<Timed>,
    /// Next frame pulled from the channel but not yet due.
    pending: Option<Timed>,
    audio: Option<(Child, Receiver<Vec<f32>>)>,
    sink: Option<Sink>,
    clock: Arc<dyn Clock>,
    /// True once the frame channel closed, the picture is over.
    frames_done: bool,
}

struct Inner {
    segment: Option<Segment>,
    state: State,
    volume: f32,
    /// Where a paused or ended player sits. A playing one asks its clock.
    frozen: Duration,
}

pub struct Player {
    ffmpeg: PathBuf,
    file: PathBuf,
    info: Info,
    out: (u32, u32),
    frames: Arc<FrameSource>,
    inner: Mutex<Inner>,
}

/// Samples kept ready in the sink before the feeder waits. 300 ms.
const AHEAD: usize = (crate::audio::RATE as usize * 3 / 10) * crate::audio::CHANNELS as usize;

impl Player {
    /// Probe `file`, then start a paused segment at zero so the first frame
    /// shows. `max` is the largest picture the output needs.
    pub fn open(ffmpeg: &Path, file: &Path, max: (u32, u32)) -> anyhow::Result<Player> {
        let info = probe::probe(ffmpeg, file)?;
        let out = decode::fit_size(info.size, max);
        let player = Player {
            ffmpeg: ffmpeg.to_path_buf(),
            file: file.to_path_buf(),
            info,
            out,
            frames: FrameSource::new(out),
            inner: Mutex::new(Inner {
                segment: None,
                state: State::Paused,
                volume: 1.0,
                frozen: Duration::ZERO,
            }),
        };
        player.restart(Duration::ZERO)?;
        Ok(player)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Kill the running segment and start one at `start`. The new clock
    /// is paused when the player is paused.
    fn restart(&self, start: Duration) -> anyhow::Result<()> {
        let mut inner = self.lock();
        if let Some(mut old) = inner.segment.take() {
            let _ = old.video.kill();
            let _ = old.video.wait();
            if let Some((mut a, _)) = old.audio.take() {
                let _ = a.kill();
                let _ = a.wait();
            }
        }
        let (video, frames) = decode::spawn_video(&self.ffmpeg, &self.file, start, self.out)?;
        let mut audio = None;
        let mut sink = None;
        if self.info.has_audio {
            match decode::spawn_audio(&self.ffmpeg, &self.file, start) {
                Ok((child, rx)) => {
                    // A stream that ends before one chunk has no sound to
                    // follow. Same for a box without an output device.
                    match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(first) => {
                            if let Some(s) = Sink::open(start) {
                                s.set_volume(inner.volume);
                                s.feed(first);
                                sink = Some(s);
                                audio = Some((child, rx));
                            } else {
                                let mut child = child;
                                let _ = child.kill();
                                let _ = child.wait();
                            }
                        }
                        Err(_) => {
                            let mut child = child;
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                    }
                }
                Err(e) => log::warn!("audio decode failed, playing muted: {e}"),
            }
        }
        let clock: Arc<dyn Clock> = match &sink {
            Some(s) => s.clock(),
            None => Arc::new(WallClock::new(start)),
        };
        if inner.state != State::Playing {
            clock.pause();
            if let Some(s) = &sink {
                s.pause();
            }
        }
        inner.frozen = start;
        if inner.state == State::Ended {
            inner.state = State::Paused;
        }
        inner.segment = Some(Segment {
            start,
            video,
            frames,
            pending: None,
            audio,
            sink,
            clock,
            frames_done: false,
        });
        self.frames.push_none();
        Ok(())
    }

    pub fn play(&self) {
        let mut inner = self.lock();
        match inner.state {
            State::Playing => {}
            State::Paused => {
                inner.state = State::Playing;
                if let Some(seg) = &inner.segment {
                    seg.clock.resume();
                    if let Some(s) = &seg.sink {
                        s.play();
                    }
                }
            }
            State::Ended => {
                drop(inner);
                let _ = self.restart(Duration::ZERO);
                self.play();
            }
        }
    }

    pub fn pause(&self) {
        let mut guard = self.lock();
        // A plain reference lets the borrow checker split the fields.
        let inner = &mut *guard;
        if inner.state != State::Playing {
            return;
        }
        if let Some(seg) = &inner.segment {
            inner.frozen = seg.clock.now().min(self.info.duration);
            seg.clock.pause();
            if let Some(s) = &seg.sink {
                s.pause();
            }
        }
        inner.state = State::Paused;
    }

    pub fn toggle(&self) {
        if self.state() == State::Playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Restart at `to`, clamped to the file. Paused stays paused and shows
    /// the frame at `to`.
    pub fn seek(&self, to: Duration) {
        let to = to.min(self.info.duration);
        if let Err(e) = self.restart(to) {
            log::warn!("seek failed: {e}");
        }
    }

    /// `by` seconds from the current position, negative goes back.
    pub fn step(&self, by: i64) {
        let now = self.position().as_secs_f64();
        let to = (now + by as f64).clamp(0.0, self.info.duration.as_secs_f64());
        self.seek(Duration::from_secs_f64(to));
    }

    pub fn set_volume(&self, v: f32) {
        let v = v.clamp(0.0, 1.0);
        let inner = self.lock();
        if let Some(s) = inner.segment.as_ref().and_then(|s| s.sink.as_ref()) {
            s.set_volume(v);
        }
        drop(inner);
        self.lock().volume = v;
    }

    pub fn volume(&self) -> f32 {
        self.lock().volume
    }

    /// No sound plays: the file has none, or no device opened.
    pub fn muted(&self) -> bool {
        self.lock()
            .segment
            .as_ref()
            .is_none_or(|s| s.sink.is_none())
    }

    pub fn position(&self) -> Duration {
        let inner = self.lock();
        match (&inner.segment, inner.state) {
            (Some(seg), State::Playing) => seg.clock.now().min(self.info.duration),
            _ => inner.frozen,
        }
    }

    pub fn duration(&self) -> Duration {
        self.info.duration
    }

    pub fn state(&self) -> State {
        self.lock().state
    }

    pub fn frames(&self) -> Arc<FrameSource> {
        self.frames.clone()
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Feed the sink, show the newest frame that is due, and end when the
    /// pictures ran out. Call once per rendered frame.
    pub fn tick(&self) {
        let mut inner = self.lock();
        let state = inner.state;
        let Some(seg) = inner.segment.as_mut() else {
            return;
        };

        // Sound first, so the clock never starves.
        if let (Some((_, rx)), Some(sink)) = (&seg.audio, &seg.sink) {
            while sink.queued() < AHEAD {
                match rx.try_recv() {
                    Ok(chunk) => sink.feed(chunk),
                    Err(_) => break,
                }
            }
        }

        // While paused, show one frame at the segment start and stop.
        let target = if state == State::Playing {
            seg.clock.now()
        } else {
            seg.start
        };
        let mut show: Option<Timed> = None;
        loop {
            let next = match seg.pending.take() {
                Some(t) => Some(t),
                None => match seg.frames.try_recv() {
                    Ok(t) => Some(t),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => {
                        seg.frames_done = true;
                        None
                    }
                },
            };
            match next {
                Some(t) if t.at <= target || show.is_none() && state != State::Playing => {
                    show = Some(t);
                    if state != State::Playing {
                        break;
                    }
                }
                Some(t) => {
                    seg.pending = Some(t);
                    break;
                }
                None => break,
            }
        }
        if let Some(t) = show {
            self.frames.push(t.frame);
        }

        // The end: the frame channel closed and nothing waits to be shown.
        if state == State::Playing && seg.frames_done && seg.pending.is_none() {
            inner.frozen = self.info.duration;
            inner.state = State::Ended;
            if let Some(seg) = inner.segment.as_mut() {
                seg.clock.pause();
                if let Some(s) = &seg.sink {
                    s.pause();
                    s.clear();
                }
            }
            self.frames.push_none();
        }
    }
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Player({})", self.file.display())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let mut inner = self.lock();
        if let Some(mut seg) = inner.segment.take() {
            let _ = seg.video.kill();
            let _ = seg.video.wait();
            if let Some((mut a, _)) = seg.audio.take() {
                let _ = a.kill();
                let _ = a.wait();
            }
        }
    }
}
