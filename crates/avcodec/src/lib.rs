//! Codec registry and codec implementation crate.

#![forbid(unsafe_code)]

pub mod pcm;
pub mod rawvideo;

pub use pcm::PcmS16leDecoder;
pub use rawvideo::{PixelFormat, RawVideoDecoder};

pub const COMPONENT_KIND: &str = "avcodec";
