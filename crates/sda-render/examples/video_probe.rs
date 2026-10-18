//! Manual check for the ffmpeg decoder. Decodes a media file with the
//! ffmpeg the app manages, waits half a second, and saves the latest frame.
//!
//! cargo run -p sda-render --example video_probe -- <media> /tmp/frame.png

use std::path::PathBuf;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let (Some(media), Some(out)) = (args.next(), args.next()) else {
        anyhow::bail!("usage: video_probe <media> <out.png>");
    };
    let ffmpeg = sda_core::downloader::find_existing("ffmpeg")
        .ok_or_else(|| anyhow::anyhow!("ffmpeg not found in the tools dir or PATH"))?;
    println!("ffmpeg: {}", ffmpeg.display());
    let source = sda_render::spawn_decoder(&ffmpeg, &PathBuf::from(media), (320, 180))?;
    std::thread::sleep(Duration::from_millis(500));
    let frame = source
        .latest()
        .ok_or_else(|| anyhow::anyhow!("no frame after 500 ms"))?;
    frame.save(&out)?;
    println!("saved {out} {}x{}", frame.width(), frame.height());
    drop(source);
    Ok(())
}
