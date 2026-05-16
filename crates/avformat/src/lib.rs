//! Format, protocol, probing, muxing, and demuxing crate.

#![forbid(unsafe_code)]

pub mod avio;
pub mod framecrc_muxer;
pub mod hash_muxer;
pub mod image2;
pub mod null_muxer;
pub mod pcm;
pub mod probe;
pub mod rawvideo;
pub mod wav;
pub mod yuv4mpegpipe;

pub use avio::{AvioReader, AvioWriter};
pub use framecrc_muxer::{FrameCrcMuxer, FrameCrcRecord};
pub use hash_muxer::{HashAlgorithm, HashMuxer, HashMuxerReport};
pub use image2::{Image2Demuxer, Image2Entry, Image2Frame, Image2Info, Image2Pattern};
pub use null_muxer::{NullMuxer, NullMuxerReport, NullStreamStats};
pub use pcm::{PcmS16leDemuxer, PcmS16leInfo, PcmS16leMuxer, PcmS16leMuxerInfo};
pub use probe::{ProbeDescriptor, ProbeMatch, ProbeRegistry, ProbeRequest, ProbeScore};
pub use rawvideo::{RawVideoDemuxer, RawVideoInfo, RawVideoMuxer, RawVideoPixelFormat};
pub use wav::{WavDemuxer, WavInfo, WavMuxer};
pub use yuv4mpegpipe::{Yuv4MpegChroma, Yuv4MpegDemuxer, Yuv4MpegInfo, Yuv4MpegInterlace};

pub const COMPONENT_KIND: &str = "avformat";
