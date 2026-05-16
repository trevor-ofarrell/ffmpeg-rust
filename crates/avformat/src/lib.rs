//! Format, protocol, probing, muxing, and demuxing crate.

#![forbid(unsafe_code)]

pub mod avio;
pub mod null_muxer;
pub mod probe;

pub use avio::{AvioReader, AvioWriter};
pub use null_muxer::{NullMuxer, NullMuxerReport, NullStreamStats};
pub use probe::{ProbeDescriptor, ProbeMatch, ProbeRegistry, ProbeRequest, ProbeScore};

pub const COMPONENT_KIND: &str = "avformat";
