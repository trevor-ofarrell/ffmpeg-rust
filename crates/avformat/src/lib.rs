//! Format, protocol, probing, muxing, and demuxing crate.

#![forbid(unsafe_code)]

pub mod avio;

pub use avio::{AvioReader, AvioWriter};

pub const COMPONENT_KIND: &str = "avformat";
