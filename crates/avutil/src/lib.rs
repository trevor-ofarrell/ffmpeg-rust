//! Shared primitives for the FFmpeg-compatible Rust implementation.

pub mod bitreader;
pub mod bitwriter;
pub mod buffer;
pub mod byteio;
pub mod channel_layout;
pub mod dict;
pub mod error;
pub mod frame;
pub mod hash;
pub mod logging;
pub mod options;
pub mod packet;
pub mod pixel;
pub mod rational;
pub mod samplefmt;
pub mod timebase;

pub use bitreader::BitReader;
pub use bitwriter::BitWriter;
pub use buffer::{BufferPool, BufferPoolCallbacks, BufferRef, BufferSlice};
pub use byteio::{ByteReader, ByteWriter};
pub use channel_layout::{Channel, ChannelLayout};
pub use dict::{Dictionary, DictionaryEntry, DictionarySet, MatchMode, SetMode};
pub use error::{AvError, AvErrorKind, AvResult};
pub use frame::{
    AudioFrame, Frame, FrameA53ClosedCaptions, FrameActiveFormatDescription,
    FrameAmbientViewingEnvironment, FrameAudioServiceType, FrameContentLightMetadata, FrameData,
    FrameDetectionBbox, FrameDetectionBboxes, FrameDisplayMatrix, FrameDolbyVisionColorMetadata,
    FrameDolbyVisionDataMapping, FrameDolbyVisionDmData, FrameDolbyVisionMetadata,
    FrameDolbyVisionRpuBuffer, FrameDolbyVisionRpuDataHeader, FrameDownmixInfo, FrameDownmixType,
    FrameDynamicHdrPlus, FrameDynamicHdrVivid, FrameFilmGrainAomParams, FrameFilmGrainH274Params,
    FrameFilmGrainParams, FrameFilmGrainParamsType, FrameGopTimecode,
    FrameHdrPlusColorTransformParams, FrameHdrPlusOverlapProcessOption, FrameHdrPlusPercentile,
    FrameHdrVivid3SplineParams, FrameHdrVividColorToneMappingParams,
    FrameHdrVividColorTransformParams, FrameIccProfile, FrameLcevc, FrameMasteringDisplayMetadata,
    FrameMatrixEncoding, FrameMotionVector, FrameMotionVectors, FramePanScan,
    FrameRegionOfInterest, FrameRegionsOfInterest, FrameReplayGain, FrameS12mTimecode,
    FrameSeiUnregistered, FrameSideData, FrameSideDataDescriptor, FrameSideDataKind,
    FrameSideDataProperties, FrameSkipSamples, FrameSkipSamplesReason, FrameSphericalMapping,
    FrameSphericalProjection, FrameStereo3d, FrameStereo3dFlags, FrameStereo3dPrimaryEye,
    FrameStereo3dType, FrameStereo3dView, FrameThreeDReferenceDisplay,
    FrameThreeDReferenceDisplays, FrameVideoBlockParams, FrameVideoEncParams,
    FrameVideoEncParamsType, FrameVideoHint, FrameVideoHintType, FrameVideoRect, FrameViewId,
    VideoFrame,
};
pub use hash::{
    adler32, crc32_ieee, digest_to_hex, md5, sha224, sha256, sha384, sha512, Adler32, Crc32, Md5,
    Sha224, Sha256, Sha384, Sha512,
};
pub use logging::{LogFlags, LogLevel, LogRecord, Logger};
pub use options::{
    OptionChild, OptionConstant, OptionDefinition, OptionFlags, OptionKind, OptionMatch,
    OptionQuery, OptionRange, OptionSet, OptionValue,
};
pub use packet::{Packet, PacketFlags, SideData, AV_NOPTS_VALUE, AV_PACKET_POS_UNKNOWN};
pub use pixel::PixelFormat;
pub use rational::Rational;
pub use samplefmt::{SampleFormat, SampleFormatFamily, SampleFormatNumericKind};
pub use timebase::{rescale_q, rescale_q_rnd, rescale_q_rnd_pass_minmax, Rounding};
