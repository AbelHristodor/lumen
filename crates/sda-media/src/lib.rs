//! Video playback for presenting. Two ffmpeg children decode one file,
//! pictures as raw RGBA and sound as f32 PCM, both started at the same
//! time offset. The player paces pictures against the sound and feeds
//! frames into a `sda_render::FrameSource`. No Iced, no GPU here.

pub mod audio;
pub mod clock;
pub mod decode;
pub mod player;
pub mod probe;

pub use player::{Player, State};
pub use probe::Info;
