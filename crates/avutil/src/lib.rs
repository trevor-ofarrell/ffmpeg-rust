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
pub use error::{
    av_error_description, av_make_error_string, av_strerror, AvError, AvErrorCode, AvErrorKind,
    AvResult, AV_ERROR_MAX_STRING_SIZE,
};
pub use frame::{
    AudioFrame, Frame, FrameA53ClosedCaptions, FrameActiveFormatDescription,
    FrameAmbientViewingEnvironment, FrameAudioServiceType, FrameContentLightMetadata, FrameData,
    FrameDetectionBbox, FrameDetectionBboxes, FrameDisplayMatrix, FrameDolbyVisionColorMetadata,
    FrameDolbyVisionDataMapping, FrameDolbyVisionDmData, FrameDolbyVisionMetadata,
    FrameDolbyVisionRpuBuffer, FrameDolbyVisionRpuDataHeader, FrameDownmixInfo, FrameDownmixType,
    FrameDynamicHdrPlus, FrameDynamicHdrVivid, FrameExif, FrameExifBitsPerSample,
    FrameExifColorSpace, FrameExifCommonTags, FrameExifCompositeImage, FrameExifCompression,
    FrameExifContrast, FrameExifCustomRendered, FrameExifEndian, FrameExifEntry,
    FrameExifExposureMode, FrameExifExposureProgram, FrameExifFileSource, FrameExifFillOrder,
    FrameExifFlash, FrameExifGainControl, FrameExifGpsAltitudeRef, FrameExifGpsDifferential,
    FrameExifGpsDirectionRef, FrameExifGpsDistanceRef, FrameExifGpsLatitudeRef,
    FrameExifGpsLongitudeRef, FrameExifGpsMeasureMode, FrameExifGpsSpeedRef, FrameExifGpsStatus,
    FrameExifIfd, FrameExifIfdPointerKind, FrameExifLightSource, FrameExifLinkedIfd,
    FrameExifMeteringMode, FrameExifNewSubfileType, FrameExifOrientation,
    FrameExifPhotometricInterpretation, FrameExifPlanarConfiguration, FrameExifPredictor,
    FrameExifRational, FrameExifResolutionUnit, FrameExifSaturation, FrameExifSceneCaptureType,
    FrameExifSceneType, FrameExifSensingMethod, FrameExifSensitivityType, FrameExifSharpness,
    FrameExifSignedRational, FrameExifSubfileType, FrameExifSubjectArea,
    FrameExifSubjectDistanceRange, FrameExifThresholding, FrameExifTiffType, FrameExifWhiteBalance,
    FrameExifYcbCrPositioning, FrameFilmGrainAomParams, FrameFilmGrainH274Params,
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
    adler32, crc32_ieee, digest_to_hex, md5, sha1, sha224, sha256, sha384, sha512, Adler32, Crc32,
    Md5, Sha1, Sha224, Sha256, Sha384, Sha512,
};
pub use logging::{
    clear_global_log_callback, clear_global_log_records, flush_global_log_repeated,
    global_formatted_log_records, global_formatted_log_records_with_options, global_log,
    global_log_flags, global_log_level, set_global_log_callback, set_global_log_flag,
    set_global_log_flags, set_global_log_level, take_global_log_records, LogColorMode, LogFlags,
    LogFormatOptions, LogLevel, LogRecord, LogTimestamp, Logger, AV_LOG_FORCE_COLOR_ENV,
    AV_LOG_FORCE_NOCOLOR_ENV,
};
pub use options::{
    OptionChild, OptionConstant, OptionDefinition, OptionFlags, OptionKind, OptionMatch,
    OptionQuery, OptionRange, OptionSet, OptionValue,
};
pub use packet::{
    Packet, PacketA53ClosedCaptions, PacketActiveFormatDescription,
    PacketAmbientViewingEnvironment, PacketAudioServiceType, PacketContentLightMetadata,
    PacketCpbProperties, PacketDisplayMatrix, PacketDolbyVisionConf, PacketDoviCompression,
    PacketDynamicHdr10Plus, PacketEncryptionInfo, PacketEncryptionInitInfo,
    PacketEncryptionInitInfoEntry, PacketEncryptionSubsample, PacketExif, PacketFallbackTrack,
    PacketFlags, PacketFrameCropping, PacketH263MbInfo, PacketH263MbInfoEntry,
    PacketHdrPlusColorTransformParams, PacketHdrPlusOverlapProcessOption, PacketHdrPlusPercentile,
    PacketIamfAnimationType, PacketIamfDemixingInfoParam, PacketIamfDemixingInfoSubblock,
    PacketIamfMixGainParam, PacketIamfMixGainSubblock, PacketIamfParamDefinition,
    PacketIamfParamDefinitionType, PacketIamfReconGainInfoParam, PacketIamfReconGainSubblock,
    PacketIccProfile, PacketJpDualMono, PacketJpDualMonoSelection, PacketLcevc,
    PacketMasteringDisplayMetadata, PacketMatroskaBlockAdditional, PacketMpegTsStreamId,
    PacketNewExtradata, PacketOpaque, PacketPalette, PacketParamChange, PacketPictureType,
    PacketProducerReferenceTime, PacketQualityStats, PacketReplayGain, PacketRtcpSenderReport,
    PacketS12mTimecode, PacketSideDataKind, PacketSkipSamples, PacketSkipSamplesReason,
    PacketSphericalMapping, PacketSphericalProjection, PacketStereo3d, PacketStereo3dFlags,
    PacketStereo3dPrimaryEye, PacketStereo3dType, PacketStereo3dView, PacketStringMetadata,
    PacketStringMetadataEntry, PacketSubtitlePosition, PacketThreeDReferenceDisplay,
    PacketThreeDReferenceDisplays, PacketWebVttIdentifier, PacketWebVttSettings, SideData,
    AV_NOPTS_VALUE, AV_PACKET_POS_UNKNOWN,
};
pub use pixel::{
    PixelFormat, PixelFormatClass, PixelFormatDescriptor, AVPALETTE_COUNT, AVPALETTE_SIZE,
};
pub use rational::Rational;
pub use samplefmt::{SampleFormat, SampleFormatFamily, SampleFormatNumericKind};
pub use timebase::{
    add_stable, compare_mod, compare_ts, rescale, rescale_q, rescale_q_rnd,
    rescale_q_rnd_pass_minmax, rescale_rnd, rescale_rnd_pass_minmax, Rounding, AV_TIME_BASE,
    AV_TIME_BASE_Q,
};
