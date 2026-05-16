//! Codec registry and codec implementation crate.

#![forbid(unsafe_code)]

pub mod rawvideo;

pub use rawvideo::{PixelFormat, RawVideoDecoder};

pub const COMPONENT_KIND: &str = "avcodec";
