use crate::frame::{
    Frame, FrameAmbientViewingEnvironment, FrameDynamicHdrPlus, FrameExif,
    FrameHdrPlusColorTransformParams, FrameHdrPlusOverlapProcessOption, FrameHdrPlusPercentile,
    FrameSideData, FrameSideDataFlags, FrameSideDataKind, FrameStereo3d, FrameStereo3dFlags,
    FrameStereo3dPrimaryEye, FrameStereo3dType, FrameStereo3dView, FrameThreeDReferenceDisplay,
    FrameThreeDReferenceDisplays,
};
use crate::{
    rescale_q, AvError, AvErrorCode, AvErrorKind, AvResult, BufferRef, Dictionary, Rational,
};
use std::collections::VecDeque;
use std::num::NonZeroUsize;

pub const AV_NOPTS_VALUE: i64 = i64::MIN;
pub const AV_PACKET_POS_UNKNOWN: i64 = -1;
pub const AV_INPUT_BUFFER_PADDING_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PacketFlags {
    bits: u32,
}

impl PacketFlags {
    pub const KEY: Self = Self { bits: 0x0001 };
    pub const CORRUPT: Self = Self { bits: 0x0002 };
    pub const DISCARD: Self = Self { bits: 0x0004 };
    pub const TRUSTED: Self = Self { bits: 0x0008 };
    pub const DISPOSABLE: Self = Self { bits: 0x0010 };
    const KNOWN_BITS: u32 = Self::KEY.bits
        | Self::CORRUPT.bits
        | Self::DISCARD.bits
        | Self::TRUSTED.bits
        | Self::DISPOSABLE.bits;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn all() -> Self {
        Self {
            bits: Self::KNOWN_BITS,
        }
    }

    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self {
            bits: bits & Self::KNOWN_BITS,
        }
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }

    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }

    pub fn insert(&mut self, other: Self) {
        self.bits |= other.bits;
    }

    pub fn remove(&mut self, other: Self) {
        self.bits &= !other.bits;
    }

    pub fn set(&mut self, other: Self, enabled: bool) {
        if enabled {
            self.insert(other);
        } else {
            self.remove(other);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketPictureType {
    Unknown = 0,
    I = 1,
    P = 2,
    B = 3,
    S = 4,
    Si = 5,
    Sp = 6,
    Bi = 7,
}

impl PacketPictureType {
    pub fn from_byte(value: u8) -> AvResult<Self> {
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::I),
            2 => Ok(Self::P),
            3 => Ok(Self::B),
            4 => Ok(Self::S),
            5 => Ok(Self::Si),
            6 => Ok(Self::Sp),
            7 => Ok(Self::Bi),
            _ => Err(AvError::invalid_data(format!(
                "invalid packet picture type value {value}"
            ))),
        }
    }

    pub const fn as_byte(self) -> u8 {
        self as u8
    }

    pub const fn ffmpeg_char(self) -> char {
        match self {
            Self::Unknown => '?',
            Self::I => 'I',
            Self::P => 'P',
            Self::B => 'B',
            Self::S => 'S',
            Self::Si => 'i',
            Self::Sp => 'p',
            Self::Bi => 'b',
        }
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::Unknown => "AV_PICTURE_TYPE_NONE",
            Self::I => "AV_PICTURE_TYPE_I",
            Self::P => "AV_PICTURE_TYPE_P",
            Self::B => "AV_PICTURE_TYPE_B",
            Self::S => "AV_PICTURE_TYPE_S",
            Self::Si => "AV_PICTURE_TYPE_SI",
            Self::Sp => "AV_PICTURE_TYPE_SP",
            Self::Bi => "AV_PICTURE_TYPE_BI",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketSideDataKind {
    Palette,
    NewExtradata,
    ParamChange,
    H263MbInfo,
    ReplayGain,
    DisplayMatrix,
    Stereo3d,
    AudioServiceType,
    QualityStats,
    FallbackTrack,
    CpbProperties,
    SkipSamples,
    JpDualMono,
    StringsMetadata,
    SubtitlePosition,
    MatroskaBlockAdditional,
    WebVttIdentifier,
    WebVttSettings,
    MetadataUpdate,
    MpegTsStreamId,
    MasteringDisplayMetadata,
    Spherical,
    ContentLightLevel,
    A53ClosedCaptions,
    EncryptionInitInfo,
    EncryptionInfo,
    ActiveFormatDescription,
    ProducerReferenceTime,
    IccProfile,
    DolbyVisionConf,
    S12mTimecode,
    DynamicHdr10Plus,
    IamfMixGainParam,
    IamfDemixingInfoParam,
    IamfReconGainInfoParam,
    AmbientViewingEnvironment,
    FrameCropping,
    Lcevc,
    ThreeDReferenceDisplays,
    RtcpSenderReport,
    Exif,
    Unknown(String),
}

impl PacketSideDataKind {
    pub const KNOWN: &'static [Self] = &[
        Self::Palette,
        Self::NewExtradata,
        Self::ParamChange,
        Self::H263MbInfo,
        Self::ReplayGain,
        Self::DisplayMatrix,
        Self::Stereo3d,
        Self::AudioServiceType,
        Self::QualityStats,
        Self::FallbackTrack,
        Self::CpbProperties,
        Self::SkipSamples,
        Self::JpDualMono,
        Self::StringsMetadata,
        Self::SubtitlePosition,
        Self::MatroskaBlockAdditional,
        Self::WebVttIdentifier,
        Self::WebVttSettings,
        Self::MetadataUpdate,
        Self::MpegTsStreamId,
        Self::MasteringDisplayMetadata,
        Self::Spherical,
        Self::ContentLightLevel,
        Self::A53ClosedCaptions,
        Self::EncryptionInitInfo,
        Self::EncryptionInfo,
        Self::ActiveFormatDescription,
        Self::ProducerReferenceTime,
        Self::IccProfile,
        Self::DolbyVisionConf,
        Self::S12mTimecode,
        Self::DynamicHdr10Plus,
        Self::IamfMixGainParam,
        Self::IamfDemixingInfoParam,
        Self::IamfReconGainInfoParam,
        Self::AmbientViewingEnvironment,
        Self::FrameCropping,
        Self::Lcevc,
        Self::ThreeDReferenceDisplays,
        Self::RtcpSenderReport,
        Self::Exif,
    ];
    pub const MAX_FFMPEG_PACKET_SIDE_DATA_ELEMS: usize = Self::KNOWN.len();

    pub fn from_name(name: impl Into<String>) -> AvResult<Self> {
        let name = validate_packet_side_data_kind(name.into())?;
        Ok(Self::known_from_name(&name).unwrap_or(Self::Unknown(name)))
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Palette => "palette",
            Self::NewExtradata => "new_extradata",
            Self::ParamChange => "param_change",
            Self::H263MbInfo => "h263_mb_info",
            Self::ReplayGain => "replaygain",
            Self::DisplayMatrix => "displaymatrix",
            Self::Stereo3d => "stereo3d",
            Self::AudioServiceType => "audio_service_type",
            Self::QualityStats => "quality_stats",
            Self::FallbackTrack => "fallback_track",
            Self::CpbProperties => "cpb_properties",
            Self::SkipSamples => "skip_samples",
            Self::JpDualMono => "jp_dualmono",
            Self::StringsMetadata => "strings_metadata",
            Self::SubtitlePosition => "subtitle_position",
            Self::MatroskaBlockAdditional => "matroska_blockadditional",
            Self::WebVttIdentifier => "webvtt_identifier",
            Self::WebVttSettings => "webvtt_settings",
            Self::MetadataUpdate => "metadata_update",
            Self::MpegTsStreamId => "mpegts_stream_id",
            Self::MasteringDisplayMetadata => "mastering_display_metadata",
            Self::Spherical => "spherical",
            Self::ContentLightLevel => "content_light_level",
            Self::A53ClosedCaptions => "a53_cc",
            Self::EncryptionInitInfo => "encryption_init_info",
            Self::EncryptionInfo => "encryption_info",
            Self::ActiveFormatDescription => "afd",
            Self::ProducerReferenceTime => "prft",
            Self::IccProfile => "icc_profile",
            Self::DolbyVisionConf => "dovi_conf",
            Self::S12mTimecode => "s12m_timecode",
            Self::DynamicHdr10Plus => "dynamic_hdr10_plus",
            Self::IamfMixGainParam => "iamf_mix_gain_param",
            Self::IamfDemixingInfoParam => "iamf_demixing_info_param",
            Self::IamfReconGainInfoParam => "iamf_recon_gain_info_param",
            Self::AmbientViewingEnvironment => "ambient_viewing_environment",
            Self::FrameCropping => "frame_cropping",
            Self::Lcevc => "lcevc",
            Self::ThreeDReferenceDisplays => "3d_reference_displays",
            Self::RtcpSenderReport => "rtcp_sr",
            Self::Exif => "exif",
            Self::Unknown(name) => name.as_str(),
        }
    }

    pub fn ffmpeg_constant(&self) -> Option<&'static str> {
        match self {
            Self::Palette => Some("AV_PKT_DATA_PALETTE"),
            Self::NewExtradata => Some("AV_PKT_DATA_NEW_EXTRADATA"),
            Self::ParamChange => Some("AV_PKT_DATA_PARAM_CHANGE"),
            Self::H263MbInfo => Some("AV_PKT_DATA_H263_MB_INFO"),
            Self::ReplayGain => Some("AV_PKT_DATA_REPLAYGAIN"),
            Self::DisplayMatrix => Some("AV_PKT_DATA_DISPLAYMATRIX"),
            Self::Stereo3d => Some("AV_PKT_DATA_STEREO3D"),
            Self::AudioServiceType => Some("AV_PKT_DATA_AUDIO_SERVICE_TYPE"),
            Self::QualityStats => Some("AV_PKT_DATA_QUALITY_STATS"),
            Self::FallbackTrack => Some("AV_PKT_DATA_FALLBACK_TRACK"),
            Self::CpbProperties => Some("AV_PKT_DATA_CPB_PROPERTIES"),
            Self::SkipSamples => Some("AV_PKT_DATA_SKIP_SAMPLES"),
            Self::JpDualMono => Some("AV_PKT_DATA_JP_DUALMONO"),
            Self::StringsMetadata => Some("AV_PKT_DATA_STRINGS_METADATA"),
            Self::SubtitlePosition => Some("AV_PKT_DATA_SUBTITLE_POSITION"),
            Self::MatroskaBlockAdditional => Some("AV_PKT_DATA_MATROSKA_BLOCKADDITIONAL"),
            Self::WebVttIdentifier => Some("AV_PKT_DATA_WEBVTT_IDENTIFIER"),
            Self::WebVttSettings => Some("AV_PKT_DATA_WEBVTT_SETTINGS"),
            Self::MetadataUpdate => Some("AV_PKT_DATA_METADATA_UPDATE"),
            Self::MpegTsStreamId => Some("AV_PKT_DATA_MPEGTS_STREAM_ID"),
            Self::MasteringDisplayMetadata => Some("AV_PKT_DATA_MASTERING_DISPLAY_METADATA"),
            Self::Spherical => Some("AV_PKT_DATA_SPHERICAL"),
            Self::ContentLightLevel => Some("AV_PKT_DATA_CONTENT_LIGHT_LEVEL"),
            Self::A53ClosedCaptions => Some("AV_PKT_DATA_A53_CC"),
            Self::EncryptionInitInfo => Some("AV_PKT_DATA_ENCRYPTION_INIT_INFO"),
            Self::EncryptionInfo => Some("AV_PKT_DATA_ENCRYPTION_INFO"),
            Self::ActiveFormatDescription => Some("AV_PKT_DATA_AFD"),
            Self::ProducerReferenceTime => Some("AV_PKT_DATA_PRFT"),
            Self::IccProfile => Some("AV_PKT_DATA_ICC_PROFILE"),
            Self::DolbyVisionConf => Some("AV_PKT_DATA_DOVI_CONF"),
            Self::S12mTimecode => Some("AV_PKT_DATA_S12M_TIMECODE"),
            Self::DynamicHdr10Plus => Some("AV_PKT_DATA_DYNAMIC_HDR10_PLUS"),
            Self::IamfMixGainParam => Some("AV_PKT_DATA_IAMF_MIX_GAIN_PARAM"),
            Self::IamfDemixingInfoParam => Some("AV_PKT_DATA_IAMF_DEMIXING_INFO_PARAM"),
            Self::IamfReconGainInfoParam => Some("AV_PKT_DATA_IAMF_RECON_GAIN_INFO_PARAM"),
            Self::AmbientViewingEnvironment => Some("AV_PKT_DATA_AMBIENT_VIEWING_ENVIRONMENT"),
            Self::FrameCropping => Some("AV_PKT_DATA_FRAME_CROPPING"),
            Self::Lcevc => Some("AV_PKT_DATA_LCEVC"),
            Self::ThreeDReferenceDisplays => Some("AV_PKT_DATA_3D_REFERENCE_DISPLAYS"),
            Self::RtcpSenderReport => Some("AV_PKT_DATA_RTCP_SR"),
            Self::Exif => Some("AV_PKT_DATA_EXIF"),
            Self::Unknown(_) => None,
        }
    }

    pub fn ffmpeg_value(&self) -> Option<i32> {
        Self::KNOWN
            .iter()
            .position(|kind| kind == self)
            .map(|index| index as i32)
    }

    pub fn from_ffmpeg_value(value: i32) -> Option<Self> {
        let index = usize::try_from(value).ok()?;
        Self::KNOWN.get(index).cloned()
    }

    pub fn ffmpeg_side_data_name_for_value(value: i32) -> Option<&'static str> {
        Self::from_ffmpeg_value(value)?.ffmpeg_side_data_name()
    }

    pub fn ffmpeg_side_data_name(&self) -> Option<&'static str> {
        match self {
            Self::Palette => Some("Palette"),
            Self::NewExtradata => Some("New Extradata"),
            Self::ParamChange => Some("Param Change"),
            Self::H263MbInfo => Some("H263 MB Info"),
            Self::ReplayGain => Some("Replay Gain"),
            Self::DisplayMatrix => Some("Display Matrix"),
            Self::Stereo3d => Some("Stereo 3D"),
            Self::AudioServiceType => Some("Audio Service Type"),
            Self::QualityStats => Some("Quality stats"),
            Self::FallbackTrack => Some("Fallback track"),
            Self::CpbProperties => Some("CPB properties"),
            Self::SkipSamples => Some("Skip Samples"),
            Self::JpDualMono => Some("JP Dual Mono"),
            Self::StringsMetadata => Some("Strings Metadata"),
            Self::SubtitlePosition => Some("Subtitle Position"),
            Self::MatroskaBlockAdditional => Some("Matroska BlockAdditional"),
            Self::WebVttIdentifier => Some("WebVTT ID"),
            Self::WebVttSettings => Some("WebVTT Settings"),
            Self::MetadataUpdate => Some("Metadata Update"),
            Self::MpegTsStreamId => Some("MPEGTS Stream ID"),
            Self::MasteringDisplayMetadata => Some("Mastering display metadata"),
            Self::Spherical => Some("Spherical Mapping"),
            Self::ContentLightLevel => Some("Content light level metadata"),
            Self::A53ClosedCaptions => Some("A53 Closed Captions"),
            Self::EncryptionInitInfo => Some("Encryption initialization data"),
            Self::EncryptionInfo => Some("Encryption info"),
            Self::ActiveFormatDescription => Some("Active Format Description data"),
            Self::ProducerReferenceTime => Some("Producer Reference Time"),
            Self::IccProfile => Some("ICC Profile"),
            Self::DolbyVisionConf => Some("DOVI configuration record"),
            Self::S12mTimecode => Some("SMPTE ST 12-1:2014 timecode"),
            Self::DynamicHdr10Plus => Some("HDR10+ Dynamic Metadata (SMPTE 2094-40)"),
            Self::IamfMixGainParam => Some("IAMF Mix Gain Parameter Data"),
            Self::IamfDemixingInfoParam => Some("IAMF Demixing Info Parameter Data"),
            Self::IamfReconGainInfoParam => Some("IAMF Recon Gain Info Parameter Data"),
            Self::AmbientViewingEnvironment => Some("Ambient viewing environment"),
            Self::FrameCropping => Some("Frame Cropping"),
            Self::Lcevc => Some("LCEVC NAL data"),
            Self::ThreeDReferenceDisplays => Some("3D Reference Displays Info"),
            Self::RtcpSenderReport => Some("RTCP Sender Report"),
            Self::Exif => Some("EXIF metadata"),
            Self::Unknown(_) => None,
        }
    }

    pub fn from_frame_side_data_kind(kind: &FrameSideDataKind) -> Option<Self> {
        match kind {
            FrameSideDataKind::ReplayGain => Some(Self::ReplayGain),
            FrameSideDataKind::DisplayMatrix => Some(Self::DisplayMatrix),
            FrameSideDataKind::Spherical => Some(Self::Spherical),
            FrameSideDataKind::Stereo3d => Some(Self::Stereo3d),
            FrameSideDataKind::AudioServiceType => Some(Self::AudioServiceType),
            FrameSideDataKind::MasteringDisplayMetadata => Some(Self::MasteringDisplayMetadata),
            FrameSideDataKind::ContentLightLevel => Some(Self::ContentLightLevel),
            FrameSideDataKind::IccProfile => Some(Self::IccProfile),
            FrameSideDataKind::AmbientViewingEnvironment => Some(Self::AmbientViewingEnvironment),
            FrameSideDataKind::ThreeDReferenceDisplays => Some(Self::ThreeDReferenceDisplays),
            FrameSideDataKind::Exif => Some(Self::Exif),
            _ => None,
        }
    }

    pub fn frame_side_data_kind(&self) -> Option<FrameSideDataKind> {
        match self {
            Self::ReplayGain => Some(FrameSideDataKind::ReplayGain),
            Self::DisplayMatrix => Some(FrameSideDataKind::DisplayMatrix),
            Self::Spherical => Some(FrameSideDataKind::Spherical),
            Self::Stereo3d => Some(FrameSideDataKind::Stereo3d),
            Self::AudioServiceType => Some(FrameSideDataKind::AudioServiceType),
            Self::MasteringDisplayMetadata => Some(FrameSideDataKind::MasteringDisplayMetadata),
            Self::ContentLightLevel => Some(FrameSideDataKind::ContentLightLevel),
            Self::IccProfile => Some(FrameSideDataKind::IccProfile),
            Self::AmbientViewingEnvironment => Some(FrameSideDataKind::AmbientViewingEnvironment),
            Self::ThreeDReferenceDisplays => Some(FrameSideDataKind::ThreeDReferenceDisplays),
            Self::Exif => Some(FrameSideDataKind::Exif),
            _ => None,
        }
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    fn known_from_name(name: &str) -> Option<Self> {
        let normalized = normalize_packet_side_data_name(name);
        let normalized = normalized
            .strip_prefix("av_pkt_data_")
            .unwrap_or(normalized.as_str());
        match normalized {
            "palette" => Some(Self::Palette),
            "new_extradata" | "newextradata" => Some(Self::NewExtradata),
            "param_change" | "paramchange" => Some(Self::ParamChange),
            "h263_mb_info" | "h263mbinfo" => Some(Self::H263MbInfo),
            "replaygain" | "replay_gain" => Some(Self::ReplayGain),
            "displaymatrix" | "display_matrix" => Some(Self::DisplayMatrix),
            "stereo3d" | "stereo_3d" => Some(Self::Stereo3d),
            "audio_service_type" | "audioservicetype" => Some(Self::AudioServiceType),
            "quality_stats" | "qualitystats" => Some(Self::QualityStats),
            "fallback_track" | "fallbacktrack" => Some(Self::FallbackTrack),
            "cpb_properties" | "cpbproperties" => Some(Self::CpbProperties),
            "skip_samples" | "skipsamples" => Some(Self::SkipSamples),
            "jp_dualmono" | "jp_dual_mono" | "jpdualmono" => Some(Self::JpDualMono),
            "strings_metadata" | "stringsmetadata" => Some(Self::StringsMetadata),
            "subtitle_position" | "subtitleposition" => Some(Self::SubtitlePosition),
            "matroska_blockadditional" | "matroska_block_additional" => {
                Some(Self::MatroskaBlockAdditional)
            }
            "webvtt_identifier" | "webvttidentifier" => Some(Self::WebVttIdentifier),
            "webvtt_settings" | "webvttsettings" => Some(Self::WebVttSettings),
            "metadata_update" | "metadataupdate" => Some(Self::MetadataUpdate),
            "mpegts_stream_id" | "mpegtsstreamid" => Some(Self::MpegTsStreamId),
            "mastering_display_metadata" | "masteringdisplaymetadata" => {
                Some(Self::MasteringDisplayMetadata)
            }
            "spherical" => Some(Self::Spherical),
            "content_light_level" | "contentlightlevel" => Some(Self::ContentLightLevel),
            "a53_cc" | "a53cc" | "a53_closed_captions" => Some(Self::A53ClosedCaptions),
            "encryption_init_info" | "encryptioninitinfo" => Some(Self::EncryptionInitInfo),
            "encryption_info" | "encryptioninfo" => Some(Self::EncryptionInfo),
            "afd" | "active_format_description" => Some(Self::ActiveFormatDescription),
            "prft" | "producer_reference_time" => Some(Self::ProducerReferenceTime),
            "icc_profile" | "iccprofile" => Some(Self::IccProfile),
            "dovi_conf" | "doviconf" | "dolby_vision_conf" => Some(Self::DolbyVisionConf),
            "s12m_timecode" | "s12mtimecode" => Some(Self::S12mTimecode),
            "dynamic_hdr10_plus" | "dynamichdr10plus" | "hdr10_plus" => {
                Some(Self::DynamicHdr10Plus)
            }
            "iamf_mix_gain_param" | "iamfmixgainparam" => Some(Self::IamfMixGainParam),
            "iamf_demixing_info_param" | "iamfdemixinginfoparam" => {
                Some(Self::IamfDemixingInfoParam)
            }
            "iamf_recon_gain_info_param" | "iamfrecongaininfoparam" => {
                Some(Self::IamfReconGainInfoParam)
            }
            "ambient_viewing_environment" | "ambientviewingenvironment" => {
                Some(Self::AmbientViewingEnvironment)
            }
            "frame_cropping" | "framecropping" => Some(Self::FrameCropping),
            "lcevc" => Some(Self::Lcevc),
            "3d_reference_displays" | "3dreferencedisplays" | "three_d_reference_displays" => {
                Some(Self::ThreeDReferenceDisplays)
            }
            "rtcp_sr" | "rtcpsr" | "rtcp_sender_report" => Some(Self::RtcpSenderReport),
            "exif" => Some(Self::Exif),
            _ => None,
        }
    }
}

impl TryFrom<&str> for PacketSideDataKind {
    type Error = AvError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_name(value)
    }
}

impl TryFrom<String> for PacketSideDataKind {
    type Error = AvError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_name(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketPalette<'a> {
    data: &'a [u8],
}

impl<'a> PacketPalette<'a> {
    pub const ENTRY_COUNT: usize = 256;
    pub const ENTRY_LEN: usize = 4;
    pub const DATA_LEN: usize = Self::ENTRY_COUNT * Self::ENTRY_LEN;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "palette packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self { data })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn len(self) -> usize {
        self.data.len()
    }

    pub const fn is_empty(self) -> bool {
        self.data.is_empty()
    }

    pub const fn entry_count(self) -> usize {
        Self::ENTRY_COUNT
    }

    pub fn entry_bytes(self, index: usize) -> Option<[u8; 4]> {
        let start = index.checked_mul(Self::ENTRY_LEN)?;
        let end = start.checked_add(Self::ENTRY_LEN)?;
        let entry = self.data.get(start..end)?;
        let mut bytes = [0; 4];
        bytes.copy_from_slice(entry);
        Some(bytes)
    }

    pub fn entry_native(self, index: usize) -> Option<u32> {
        self.entry_bytes(index).map(u32::from_ne_bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketNewExtradata<'a> {
    data: &'a [u8],
}

impl<'a> PacketNewExtradata<'a> {
    pub const fn parse(data: &'a [u8]) -> AvResult<Self> {
        Ok(Self { data })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn len(self) -> usize {
        self.data.len()
    }

    pub const fn is_empty(self) -> bool {
        self.data.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketH263MbInfoEntry {
    bit_offset: u32,
    quantizer: u8,
    gob_number: u8,
    macroblock_address: u16,
    horizontal_mv_predictor: u8,
    vertical_mv_predictor: u8,
    block3_horizontal_mv_predictor: u8,
    block3_vertical_mv_predictor: u8,
}

impl PacketH263MbInfoEntry {
    pub const DATA_LEN: usize = 12;

    fn parse(data: &[u8]) -> Self {
        debug_assert_eq!(data.len(), Self::DATA_LEN);
        Self {
            bit_offset: read_u32_le(data, 0),
            quantizer: data[4],
            gob_number: data[5],
            macroblock_address: read_u16_le(data, 6),
            horizontal_mv_predictor: data[8],
            vertical_mv_predictor: data[9],
            block3_horizontal_mv_predictor: data[10],
            block3_vertical_mv_predictor: data[11],
        }
    }

    pub const fn bit_offset(self) -> u32 {
        self.bit_offset
    }

    pub const fn quantizer(self) -> u8 {
        self.quantizer
    }

    pub const fn gob_number(self) -> u8 {
        self.gob_number
    }

    pub const fn macroblock_address(self) -> u16 {
        self.macroblock_address
    }

    pub const fn horizontal_mv_predictor(self) -> u8 {
        self.horizontal_mv_predictor
    }

    pub const fn vertical_mv_predictor(self) -> u8 {
        self.vertical_mv_predictor
    }

    pub const fn block3_horizontal_mv_predictor(self) -> u8 {
        self.block3_horizontal_mv_predictor
    }

    pub const fn block3_vertical_mv_predictor(self) -> u8 {
        self.block3_vertical_mv_predictor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketH263MbInfo<'a> {
    data: &'a [u8],
}

impl<'a> PacketH263MbInfo<'a> {
    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if !data
            .chunks_exact(PacketH263MbInfoEntry::DATA_LEN)
            .remainder()
            .is_empty()
        {
            return Err(AvError::invalid_data(format!(
                "H.263 macroblock-info packet side data requires whole {}-byte records, got {} bytes",
                PacketH263MbInfoEntry::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self { data })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn len(self) -> usize {
        self.data.len()
    }

    pub const fn is_empty(self) -> bool {
        self.data.is_empty()
    }

    pub const fn entry_count(self) -> usize {
        self.data.len() / PacketH263MbInfoEntry::DATA_LEN
    }

    pub fn entry(self, index: usize) -> Option<PacketH263MbInfoEntry> {
        let start = index.checked_mul(PacketH263MbInfoEntry::DATA_LEN)?;
        let end = start.checked_add(PacketH263MbInfoEntry::DATA_LEN)?;
        self.data.get(start..end).map(PacketH263MbInfoEntry::parse)
    }

    pub fn entries(self) -> impl ExactSizeIterator<Item = PacketH263MbInfoEntry> + 'a {
        self.data
            .chunks_exact(PacketH263MbInfoEntry::DATA_LEN)
            .map(PacketH263MbInfoEntry::parse)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketQualityStats {
    quality: u32,
    picture_type: PacketPictureType,
    errors: Vec<u64>,
    trailing_data: Vec<u8>,
}

impl PacketQualityStats {
    pub const FF_LAMBDA_MAX: u32 = 256 * 128 - 1;
    pub const HEADER_LEN: usize = 8;
    pub const ERROR_ENTRY_LEN: usize = 8;
    pub const MAX_ERROR_COUNT: usize = u8::MAX as usize;

    pub fn new(quality: u32, picture_type: PacketPictureType, errors: Vec<u64>) -> AvResult<Self> {
        Self::from_parts(quality, picture_type, errors, Vec::new())
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "quality stats packet side data requires at least {} bytes, got {}",
                Self::HEADER_LEN,
                data.len()
            )));
        }

        let quality = read_u32_le(data, 0);
        validate_quality_stats_quality(quality, AvError::invalid_data)?;
        let picture_type = PacketPictureType::from_byte(data[4])?;
        let error_count = usize::from(data[5]);
        if data[6] != 0 || data[7] != 0 {
            return Err(AvError::invalid_data(
                "quality stats packet side data reserved bytes must be zero",
            ));
        }

        let required_len = Self::HEADER_LEN + error_count * Self::ERROR_ENTRY_LEN;
        if data.len() < required_len {
            return Err(AvError::invalid_data(format!(
                "quality stats packet side data declares {error_count} error entries but has {} bytes",
                data.len()
            )));
        }

        let errors = data[Self::HEADER_LEN..required_len]
            .chunks_exact(Self::ERROR_ENTRY_LEN)
            .map(|chunk| read_u64_le(chunk, 0))
            .collect();

        Ok(Self {
            quality,
            picture_type,
            errors,
            trailing_data: data[required_len..].to_vec(),
        })
    }

    fn from_parts(
        quality: u32,
        picture_type: PacketPictureType,
        errors: Vec<u64>,
        trailing_data: Vec<u8>,
    ) -> AvResult<Self> {
        validate_quality_stats_quality(quality, AvError::invalid_argument)?;
        if errors.len() > Self::MAX_ERROR_COUNT {
            return Err(AvError::invalid_argument(format!(
                "quality stats packet side data supports at most {} error entries, got {}",
                Self::MAX_ERROR_COUNT,
                errors.len()
            )));
        }

        Ok(Self {
            quality,
            picture_type,
            errors,
            trailing_data,
        })
    }

    pub const fn quality(&self) -> u32 {
        self.quality
    }

    pub const fn picture_type(&self) -> PacketPictureType {
        self.picture_type
    }

    pub fn errors(&self) -> &[u64] {
        &self.errors
    }

    pub fn trailing_data(&self) -> &[u8] {
        &self.trailing_data
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            Self::HEADER_LEN + self.errors.len() * Self::ERROR_ENTRY_LEN + self.trailing_data.len(),
        );
        bytes.extend_from_slice(&self.quality.to_le_bytes());
        bytes.push(self.picture_type.as_byte());
        bytes.push(self.errors.len() as u8);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        for error in &self.errors {
            bytes.extend_from_slice(&error.to_le_bytes());
        }
        bytes.extend_from_slice(&self.trailing_data);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketFallbackTrack {
    stream_index: i32,
}

impl PacketFallbackTrack {
    pub const DATA_LEN: usize = 4;

    pub fn new(stream_index: i32) -> AvResult<Self> {
        validate_fallback_track_stream_index(stream_index, AvError::invalid_argument)?;
        Ok(Self { stream_index })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "fallback track packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let stream_index = read_i32_ne(data, 0);
        validate_fallback_track_stream_index(stream_index, AvError::invalid_data)?;
        Ok(Self { stream_index })
    }

    pub const fn stream_index(self) -> i32 {
        self.stream_index
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        self.stream_index.to_ne_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketCpbProperties {
    max_bitrate: i64,
    min_bitrate: i64,
    avg_bitrate: i64,
    buffer_size: i64,
    vbv_delay: u64,
}

impl PacketCpbProperties {
    pub const DATA_LEN: usize = 40;
    pub const VBV_DELAY_UNKNOWN: u64 = u64::MAX;

    pub fn new(
        max_bitrate: i64,
        min_bitrate: i64,
        avg_bitrate: i64,
        buffer_size: i64,
        vbv_delay: u64,
    ) -> AvResult<Self> {
        validate_cpb_properties_nonnegative(max_bitrate, "max_bitrate", AvError::invalid_argument)?;
        validate_cpb_properties_nonnegative(min_bitrate, "min_bitrate", AvError::invalid_argument)?;
        validate_cpb_properties_nonnegative(avg_bitrate, "avg_bitrate", AvError::invalid_argument)?;
        validate_cpb_properties_nonnegative(buffer_size, "buffer_size", AvError::invalid_argument)?;

        Ok(Self {
            max_bitrate,
            min_bitrate,
            avg_bitrate,
            buffer_size,
            vbv_delay,
        })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "CPB properties packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let max_bitrate = read_i64_ne(data, 0);
        let min_bitrate = read_i64_ne(data, 8);
        let avg_bitrate = read_i64_ne(data, 16);
        let buffer_size = read_i64_ne(data, 24);
        validate_cpb_properties_nonnegative(max_bitrate, "max_bitrate", AvError::invalid_data)?;
        validate_cpb_properties_nonnegative(min_bitrate, "min_bitrate", AvError::invalid_data)?;
        validate_cpb_properties_nonnegative(avg_bitrate, "avg_bitrate", AvError::invalid_data)?;
        validate_cpb_properties_nonnegative(buffer_size, "buffer_size", AvError::invalid_data)?;

        Ok(Self {
            max_bitrate,
            min_bitrate,
            avg_bitrate,
            buffer_size,
            vbv_delay: read_u64_ne(data, 32),
        })
    }

    pub const fn max_bitrate(self) -> i64 {
        self.max_bitrate
    }

    pub const fn min_bitrate(self) -> i64 {
        self.min_bitrate
    }

    pub const fn avg_bitrate(self) -> i64 {
        self.avg_bitrate
    }

    pub const fn buffer_size(self) -> i64 {
        self.buffer_size
    }

    pub const fn vbv_delay(self) -> u64 {
        self.vbv_delay
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[..8].copy_from_slice(&self.max_bitrate.to_ne_bytes());
        bytes[8..16].copy_from_slice(&self.min_bitrate.to_ne_bytes());
        bytes[16..24].copy_from_slice(&self.avg_bitrate.to_ne_bytes());
        bytes[24..32].copy_from_slice(&self.buffer_size.to_ne_bytes());
        bytes[32..Self::DATA_LEN].copy_from_slice(&self.vbv_delay.to_ne_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketProducerReferenceTime {
    wallclock: i64,
    flags: i32,
    padding: [u8; Self::PADDING_LEN],
}

impl PacketProducerReferenceTime {
    pub const DATA_LEN: usize = 16;
    pub const PADDING_LEN: usize = 4;

    pub const fn new(wallclock: i64, flags: i32) -> Self {
        Self {
            wallclock,
            flags,
            padding: [0; Self::PADDING_LEN],
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "producer reference time packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut padding = [0; Self::PADDING_LEN];
        padding.copy_from_slice(&data[12..Self::DATA_LEN]);
        Ok(Self {
            wallclock: read_i64_ne(data, 0),
            flags: read_i32_ne(data, 8),
            padding,
        })
    }

    pub const fn wallclock(self) -> i64 {
        self.wallclock
    }

    pub const fn flags(self) -> i32 {
        self.flags
    }

    pub const fn padding(self) -> [u8; Self::PADDING_LEN] {
        self.padding
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[..8].copy_from_slice(&self.wallclock.to_ne_bytes());
        bytes[8..12].copy_from_slice(&self.flags.to_ne_bytes());
        bytes[12..Self::DATA_LEN].copy_from_slice(&self.padding);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketRtcpSenderReport {
    ssrc: u32,
    ntp_timestamp: u64,
    rtp_timestamp: u32,
    sender_packet_count: u32,
    sender_octet_count: u32,
    alignment_padding: [u8; Self::ALIGNMENT_PADDING_LEN],
    tail_padding: [u8; Self::TAIL_PADDING_LEN],
}

impl PacketRtcpSenderReport {
    pub const DATA_LEN: usize = 32;
    pub const ALIGNMENT_PADDING_LEN: usize = 4;
    pub const TAIL_PADDING_LEN: usize = 4;

    pub const fn new(
        ssrc: u32,
        ntp_timestamp: u64,
        rtp_timestamp: u32,
        sender_packet_count: u32,
        sender_octet_count: u32,
    ) -> Self {
        Self {
            ssrc,
            ntp_timestamp,
            rtp_timestamp,
            sender_packet_count,
            sender_octet_count,
            alignment_padding: [0; Self::ALIGNMENT_PADDING_LEN],
            tail_padding: [0; Self::TAIL_PADDING_LEN],
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "RTCP sender report packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut alignment_padding = [0; Self::ALIGNMENT_PADDING_LEN];
        alignment_padding.copy_from_slice(&data[4..8]);
        let mut tail_padding = [0; Self::TAIL_PADDING_LEN];
        tail_padding.copy_from_slice(&data[28..Self::DATA_LEN]);

        Ok(Self {
            ssrc: read_u32_ne(data, 0),
            ntp_timestamp: read_u64_ne(data, 8),
            rtp_timestamp: read_u32_ne(data, 16),
            sender_packet_count: read_u32_ne(data, 20),
            sender_octet_count: read_u32_ne(data, 24),
            alignment_padding,
            tail_padding,
        })
    }

    pub const fn ssrc(self) -> u32 {
        self.ssrc
    }

    pub const fn ntp_timestamp(self) -> u64 {
        self.ntp_timestamp
    }

    pub const fn rtp_timestamp(self) -> u32 {
        self.rtp_timestamp
    }

    pub const fn sender_packet_count(self) -> u32 {
        self.sender_packet_count
    }

    pub const fn sender_octet_count(self) -> u32 {
        self.sender_octet_count
    }

    pub const fn alignment_padding(self) -> [u8; Self::ALIGNMENT_PADDING_LEN] {
        self.alignment_padding
    }

    pub const fn tail_padding(self) -> [u8; Self::TAIL_PADDING_LEN] {
        self.tail_padding
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[..4].copy_from_slice(&self.ssrc.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.alignment_padding);
        bytes[8..16].copy_from_slice(&self.ntp_timestamp.to_ne_bytes());
        bytes[16..20].copy_from_slice(&self.rtp_timestamp.to_ne_bytes());
        bytes[20..24].copy_from_slice(&self.sender_packet_count.to_ne_bytes());
        bytes[24..28].copy_from_slice(&self.sender_octet_count.to_ne_bytes());
        bytes[28..Self::DATA_LEN].copy_from_slice(&self.tail_padding);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketSkipSamplesReason {
    PaddingSilence = 0,
    Convergence = 1,
}

impl PacketSkipSamplesReason {
    pub fn from_byte(value: u8) -> AvResult<Self> {
        match value {
            0 => Ok(Self::PaddingSilence),
            1 => Ok(Self::Convergence),
            _ => Err(AvError::invalid_data(format!(
                "invalid packet skip samples reason value {value}"
            ))),
        }
    }

    pub const fn as_byte(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketMasteringDisplayMetadata {
    display_primaries: [[Rational; Self::COORDINATES]; Self::PRIMARIES],
    white_point: [Rational; Self::COORDINATES],
    min_luminance: Rational,
    max_luminance: Rational,
    has_primaries: i32,
    has_luminance: i32,
}

impl PacketMasteringDisplayMetadata {
    pub const PRIMARIES: usize = 3;
    pub const COORDINATES: usize = 2;
    pub const DATA_LEN: usize = 88;

    pub const fn new(
        display_primaries: [[Rational; Self::COORDINATES]; Self::PRIMARIES],
        white_point: [Rational; Self::COORDINATES],
        min_luminance: Rational,
        max_luminance: Rational,
        has_primaries: i32,
        has_luminance: i32,
    ) -> Self {
        Self {
            display_primaries,
            white_point,
            min_luminance,
            max_luminance,
            has_primaries,
            has_luminance,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "mastering display metadata packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut offset = 0;
        let mut display_primaries = [[Rational::ZERO; Self::COORDINATES]; Self::PRIMARIES];
        for primary in &mut display_primaries {
            for coordinate in primary {
                *coordinate = Self::read_rational(data, &mut offset);
            }
        }

        let mut white_point = [Rational::ZERO; Self::COORDINATES];
        for coordinate in &mut white_point {
            *coordinate = Self::read_rational(data, &mut offset);
        }

        let min_luminance = Self::read_rational(data, &mut offset);
        let max_luminance = Self::read_rational(data, &mut offset);
        let has_primaries = Self::read_i32(data, &mut offset);
        let has_luminance = Self::read_i32(data, &mut offset);

        Ok(Self {
            display_primaries,
            white_point,
            min_luminance,
            max_luminance,
            has_primaries,
            has_luminance,
        })
    }

    pub const fn display_primaries(self) -> [[Rational; Self::COORDINATES]; Self::PRIMARIES] {
        self.display_primaries
    }

    pub const fn white_point(self) -> [Rational; Self::COORDINATES] {
        self.white_point
    }

    pub const fn min_luminance(self) -> Rational {
        self.min_luminance
    }

    pub const fn max_luminance(self) -> Rational {
        self.max_luminance
    }

    pub const fn has_primaries(self) -> bool {
        self.has_primaries != 0
    }

    pub const fn has_luminance(self) -> bool {
        self.has_luminance != 0
    }

    pub const fn has_primaries_raw(self) -> i32 {
        self.has_primaries
    }

    pub const fn has_luminance_raw(self) -> i32 {
        self.has_luminance
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        let mut offset = 0;
        for primary in self.display_primaries {
            for coordinate in primary {
                Self::write_rational(&mut bytes, &mut offset, coordinate);
            }
        }
        for coordinate in self.white_point {
            Self::write_rational(&mut bytes, &mut offset, coordinate);
        }
        Self::write_rational(&mut bytes, &mut offset, self.min_luminance);
        Self::write_rational(&mut bytes, &mut offset, self.max_luminance);
        Self::write_i32(&mut bytes, &mut offset, self.has_primaries);
        Self::write_i32(&mut bytes, &mut offset, self.has_luminance);
        bytes
    }

    fn read_rational(data: &[u8], offset: &mut usize) -> Rational {
        let num = Self::read_i32(data, offset);
        let den = Self::read_i32(data, offset);
        Rational::from_raw(num, den)
    }

    fn read_i32(data: &[u8], offset: &mut usize) -> i32 {
        let value = read_i32_ne(data, *offset);
        *offset += 4;
        value
    }

    fn write_rational(bytes: &mut [u8; Self::DATA_LEN], offset: &mut usize, value: Rational) {
        Self::write_i32(bytes, offset, value.num());
        Self::write_i32(bytes, offset, value.den());
    }

    fn write_i32(bytes: &mut [u8; Self::DATA_LEN], offset: &mut usize, value: i32) {
        bytes[*offset..*offset + 4].copy_from_slice(&value.to_ne_bytes());
        *offset += 4;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PacketSphericalProjection {
    Equirectangular = 0,
    Cubemap = 1,
    EquirectangularTile = 2,
    HalfEquirectangular = 3,
    Rectilinear = 4,
    Fisheye = 5,
    ParametricImmersive = 6,
}

impl PacketSphericalProjection {
    pub const KNOWN: [Self; 7] = [
        Self::Equirectangular,
        Self::Cubemap,
        Self::EquirectangularTile,
        Self::HalfEquirectangular,
        Self::Rectilinear,
        Self::Fisheye,
        Self::ParametricImmersive,
    ];

    pub fn from_raw(value: i32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::Equirectangular),
            1 => Ok(Self::Cubemap),
            2 => Ok(Self::EquirectangularTile),
            3 => Ok(Self::HalfEquirectangular),
            4 => Ok(Self::Rectilinear),
            5 => Ok(Self::Fisheye),
            6 => Ok(Self::ParametricImmersive),
            _ => Err(AvError::invalid_data(format!(
                "invalid packet spherical projection value {value}"
            ))),
        }
    }

    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::Equirectangular => "AV_SPHERICAL_EQUIRECTANGULAR",
            Self::Cubemap => "AV_SPHERICAL_CUBEMAP",
            Self::EquirectangularTile => "AV_SPHERICAL_EQUIRECTANGULAR_TILE",
            Self::HalfEquirectangular => "AV_SPHERICAL_HALF_EQUIRECTANGULAR",
            Self::Rectilinear => "AV_SPHERICAL_RECTILINEAR",
            Self::Fisheye => "AV_SPHERICAL_FISHEYE",
            Self::ParametricImmersive => "AV_SPHERICAL_PARAMETRIC_IMMERSIVE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketSphericalMapping {
    projection: PacketSphericalProjection,
    yaw: i32,
    pitch: i32,
    roll: i32,
    bounds: [u32; Self::BOUNDS],
    padding: u32,
}

impl PacketSphericalMapping {
    pub const BOUNDS: usize = 4;
    pub const DATA_LEN: usize = 36;

    pub const fn new(
        projection: PacketSphericalProjection,
        yaw: i32,
        pitch: i32,
        roll: i32,
        bounds: [u32; Self::BOUNDS],
        padding: u32,
    ) -> Self {
        Self {
            projection,
            yaw,
            pitch,
            roll,
            bounds,
            padding,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "spherical mapping packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut offset = 0;
        let projection = PacketSphericalProjection::from_raw(Self::read_i32(data, &mut offset))?;
        let yaw = Self::read_i32(data, &mut offset);
        let pitch = Self::read_i32(data, &mut offset);
        let roll = Self::read_i32(data, &mut offset);
        let mut bounds = [0; Self::BOUNDS];
        for bound in &mut bounds {
            *bound = Self::read_u32(data, &mut offset);
        }
        let padding = Self::read_u32(data, &mut offset);

        Ok(Self {
            projection,
            yaw,
            pitch,
            roll,
            bounds,
            padding,
        })
    }

    pub const fn projection(self) -> PacketSphericalProjection {
        self.projection
    }

    pub const fn yaw(self) -> i32 {
        self.yaw
    }

    pub const fn pitch(self) -> i32 {
        self.pitch
    }

    pub const fn roll(self) -> i32 {
        self.roll
    }

    pub const fn bounds(self) -> [u32; Self::BOUNDS] {
        self.bounds
    }

    pub const fn bound_left(self) -> u32 {
        self.bounds[0]
    }

    pub const fn bound_top(self) -> u32 {
        self.bounds[1]
    }

    pub const fn bound_right(self) -> u32 {
        self.bounds[2]
    }

    pub const fn bound_bottom(self) -> u32 {
        self.bounds[3]
    }

    pub const fn padding(self) -> u32 {
        self.padding
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        let mut offset = 0;
        Self::write_i32(&mut bytes, &mut offset, self.projection.as_raw());
        Self::write_i32(&mut bytes, &mut offset, self.yaw);
        Self::write_i32(&mut bytes, &mut offset, self.pitch);
        Self::write_i32(&mut bytes, &mut offset, self.roll);
        for bound in self.bounds {
            Self::write_u32(&mut bytes, &mut offset, bound);
        }
        Self::write_u32(&mut bytes, &mut offset, self.padding);
        bytes
    }

    fn read_i32(data: &[u8], offset: &mut usize) -> i32 {
        let value = read_i32_ne(data, *offset);
        *offset += 4;
        value
    }

    fn read_u32(data: &[u8], offset: &mut usize) -> u32 {
        let value = read_u32_ne(data, *offset);
        *offset += 4;
        value
    }

    fn write_i32(bytes: &mut [u8; Self::DATA_LEN], offset: &mut usize, value: i32) {
        bytes[*offset..*offset + 4].copy_from_slice(&value.to_ne_bytes());
        *offset += 4;
    }

    fn write_u32(bytes: &mut [u8; Self::DATA_LEN], offset: &mut usize, value: u32) {
        bytes[*offset..*offset + 4].copy_from_slice(&value.to_ne_bytes());
        *offset += 4;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketContentLightMetadata {
    max_content_light_level: u32,
    max_average_light_level: u32,
}

impl PacketContentLightMetadata {
    pub const DATA_LEN: usize = 8;

    pub const fn new(max_content_light_level: u32, max_average_light_level: u32) -> Self {
        Self {
            max_content_light_level,
            max_average_light_level,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "content light metadata packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self {
            max_content_light_level: read_u32_ne(data, 0),
            max_average_light_level: read_u32_ne(data, 4),
        })
    }

    pub const fn max_content_light_level(self) -> u32 {
        self.max_content_light_level
    }

    pub const fn max_average_light_level(self) -> u32 {
        self.max_average_light_level
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[0..4].copy_from_slice(&self.max_content_light_level.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.max_average_light_level.to_ne_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketAmbientViewingEnvironment(FrameAmbientViewingEnvironment);

impl PacketAmbientViewingEnvironment {
    pub const RATIONAL_LEN: usize = FrameAmbientViewingEnvironment::RATIONAL_LEN;
    pub const AMBIENT_ILLUMINANCE_OFFSET: usize =
        FrameAmbientViewingEnvironment::AMBIENT_ILLUMINANCE_OFFSET;
    pub const AMBIENT_LIGHT_X_OFFSET: usize =
        FrameAmbientViewingEnvironment::AMBIENT_LIGHT_X_OFFSET;
    pub const AMBIENT_LIGHT_Y_OFFSET: usize =
        FrameAmbientViewingEnvironment::AMBIENT_LIGHT_Y_OFFSET;
    pub const DATA_LEN: usize = FrameAmbientViewingEnvironment::DATA_LEN;

    pub fn new(
        ambient_illuminance: Rational,
        ambient_light_x: Rational,
        ambient_light_y: Rational,
    ) -> AvResult<Self> {
        FrameAmbientViewingEnvironment::new(ambient_illuminance, ambient_light_x, ambient_light_y)
            .map(Self)
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "ambient viewing environment packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        FrameAmbientViewingEnvironment::parse(data).map(Self)
    }

    pub const fn ambient_illuminance(self) -> Rational {
        self.0.ambient_illuminance()
    }

    pub const fn ambient_light_x(self) -> Rational {
        self.0.ambient_light_x()
    }

    pub const fn ambient_light_y(self) -> Rational {
        self.0.ambient_light_y()
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        self.0.to_bytes()
    }
}

pub type PacketThreeDReferenceDisplay = FrameThreeDReferenceDisplay;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketThreeDReferenceDisplays(FrameThreeDReferenceDisplays);

impl PacketThreeDReferenceDisplays {
    pub const MAX_REF_DISPLAYS: usize = FrameThreeDReferenceDisplays::MAX_REF_DISPLAYS;
    pub const ENTRY_DATA_LEN: usize = FrameThreeDReferenceDisplay::DATA_LEN;
    pub const ENTRIES_OFFSET_OFFSET: usize = FrameThreeDReferenceDisplays::ENTRIES_OFFSET_OFFSET;
    pub const ENTRY_SIZE_OFFSET: usize = FrameThreeDReferenceDisplays::ENTRY_SIZE_OFFSET;
    pub const HEADER_LEN: usize = FrameThreeDReferenceDisplays::HEADER_LEN;
    pub const ENTRIES_OFFSET: usize = FrameThreeDReferenceDisplays::ENTRIES_OFFSET;

    pub fn new(
        prec_ref_display_width: u8,
        ref_viewing_distance_flag: bool,
        prec_ref_viewing_dist: u8,
        displays: Vec<PacketThreeDReferenceDisplay>,
    ) -> AvResult<Self> {
        FrameThreeDReferenceDisplays::new(
            prec_ref_display_width,
            ref_viewing_distance_flag,
            prec_ref_viewing_dist,
            displays,
        )
        .map(Self)
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "3D reference displays packet side data requires at least {} header bytes, got {}",
                Self::HEADER_LEN,
                data.len()
            )));
        }

        FrameThreeDReferenceDisplays::parse(data).map(Self)
    }

    pub const fn prec_ref_display_width(&self) -> u8 {
        self.0.prec_ref_display_width()
    }

    pub const fn ref_viewing_distance_flag(&self) -> bool {
        self.0.ref_viewing_distance_flag()
    }

    pub const fn prec_ref_viewing_dist(&self) -> u8 {
        self.0.prec_ref_viewing_dist()
    }

    pub fn displays(&self) -> &[PacketThreeDReferenceDisplay] {
        self.0.displays()
    }

    pub fn nb_displays(&self) -> usize {
        self.0.nb_displays()
    }

    pub fn display(&self, index: usize) -> Option<PacketThreeDReferenceDisplay> {
        self.0.display(index)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }
}

pub type PacketExif<'a> = FrameExif<'a>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketA53ClosedCaptions<'a> {
    data: &'a [u8],
}

impl<'a> PacketA53ClosedCaptions<'a> {
    pub const BYTES_PER_CC: usize = 3;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() % Self::BYTES_PER_CC != 0 {
            return Err(AvError::invalid_data(format!(
                "A53 closed-caption packet side data requires whole {}-byte CC entries, got {} bytes",
                Self::BYTES_PER_CC,
                data.len()
            )));
        }

        Ok(Self { data })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn is_empty(self) -> bool {
        self.data.is_empty()
    }

    pub const fn entry_count(self) -> usize {
        self.data.len() / Self::BYTES_PER_CC
    }

    pub fn entry(self, index: usize) -> Option<[u8; 3]> {
        let start = index.checked_mul(Self::BYTES_PER_CC)?;
        let end = start.checked_add(Self::BYTES_PER_CC)?;
        let entry = self.data.get(start..end)?;
        let mut bytes = [0; 3];
        bytes.copy_from_slice(entry);
        Some(bytes)
    }

    pub fn entries(self) -> impl ExactSizeIterator<Item = [u8; 3]> + 'a {
        self.data.chunks_exact(Self::BYTES_PER_CC).map(|entry| {
            let mut bytes = [0; 3];
            bytes.copy_from_slice(entry);
            bytes
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketIccProfile<'a> {
    data: &'a [u8],
    declared_size: u32,
    tag_count: u32,
}

impl<'a> PacketIccProfile<'a> {
    pub const HEADER_LEN: usize = 128;
    pub const TAG_COUNT_LEN: usize = 4;
    pub const TAG_RECORD_LEN: usize = 12;
    pub const MIN_DATA_LEN: usize = Self::HEADER_LEN + Self::TAG_COUNT_LEN;
    pub const PROFILE_SIZE_OFFSET: usize = 0;
    pub const SIGNATURE_OFFSET: usize = 36;
    pub const TAG_COUNT_OFFSET: usize = Self::HEADER_LEN;
    pub const ICC_SIGNATURE: [u8; 4] = *b"acsp";

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::MIN_DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "ICC profile packet side data requires at least {} bytes, got {}",
                Self::MIN_DATA_LEN,
                data.len()
            )));
        }

        let declared_size = read_u32_be(data, Self::PROFILE_SIZE_OFFSET);
        if usize::try_from(declared_size).ok() != Some(data.len()) {
            return Err(AvError::invalid_data(format!(
                "ICC profile declared size {} does not match side data length {}",
                declared_size,
                data.len()
            )));
        }

        if data[Self::SIGNATURE_OFFSET..Self::SIGNATURE_OFFSET + Self::ICC_SIGNATURE.len()]
            != Self::ICC_SIGNATURE
        {
            return Err(AvError::invalid_data("ICC profile missing acsp signature"));
        }

        let tag_count = read_u32_be(data, Self::TAG_COUNT_OFFSET);
        let tag_table_len = usize::try_from(tag_count)
            .ok()
            .and_then(|count| count.checked_mul(Self::TAG_RECORD_LEN))
            .and_then(|records_len| Self::MIN_DATA_LEN.checked_add(records_len))
            .ok_or_else(|| AvError::invalid_data("ICC profile tag table length overflow"))?;
        if tag_table_len > data.len() {
            return Err(AvError::invalid_data(format!(
                "ICC profile tag table for {tag_count} records exceeds side data length {}",
                data.len()
            )));
        }

        Ok(Self {
            data,
            declared_size,
            tag_count,
        })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn declared_size(self) -> u32 {
        self.declared_size
    }

    pub const fn tag_count(self) -> u32 {
        self.tag_count
    }

    pub fn profile_version_raw(self) -> u32 {
        read_u32_be(self.data, 8)
    }

    pub fn device_class(self) -> [u8; 4] {
        read_fourcc(self.data, 12)
    }

    pub fn color_space(self) -> [u8; 4] {
        read_fourcc(self.data, 16)
    }

    pub fn profile_connection_space(self) -> [u8; 4] {
        read_fourcc(self.data, 20)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketDoviCompression {
    None = 0,
    Limited = 1,
    Reserved = 2,
    Extended = 3,
}

impl PacketDoviCompression {
    pub fn from_byte(value: u8) -> AvResult<Self> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Limited),
            2 => Ok(Self::Reserved),
            3 => Ok(Self::Extended),
            _ => Err(AvError::invalid_data(format!(
                "invalid Dolby Vision compression value {value}"
            ))),
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::None => "AV_DOVI_COMPRESSION_NONE",
            Self::Limited => "AV_DOVI_COMPRESSION_LIMITED",
            Self::Reserved => "AV_DOVI_COMPRESSION_RESERVED",
            Self::Extended => "AV_DOVI_COMPRESSION_EXTENDED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketDolbyVisionConf {
    dv_version_major: u8,
    dv_version_minor: u8,
    dv_profile: u8,
    dv_level: u8,
    rpu_present_flag: bool,
    el_present_flag: bool,
    bl_present_flag: bool,
    dv_bl_signal_compatibility_id: u8,
    dv_md_compression: PacketDoviCompression,
}

impl PacketDolbyVisionConf {
    pub const DATA_LEN: usize = 9;
    pub const DV_VERSION_MAJOR_OFFSET: usize = 0;
    pub const DV_VERSION_MINOR_OFFSET: usize = 1;
    pub const DV_PROFILE_OFFSET: usize = 2;
    pub const DV_LEVEL_OFFSET: usize = 3;
    pub const RPU_PRESENT_FLAG_OFFSET: usize = 4;
    pub const EL_PRESENT_FLAG_OFFSET: usize = 5;
    pub const BL_PRESENT_FLAG_OFFSET: usize = 6;
    pub const DV_BL_SIGNAL_COMPATIBILITY_ID_OFFSET: usize = 7;
    pub const DV_MD_COMPRESSION_OFFSET: usize = 8;

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        dv_version_major: u8,
        dv_version_minor: u8,
        dv_profile: u8,
        dv_level: u8,
        rpu_present_flag: bool,
        el_present_flag: bool,
        bl_present_flag: bool,
        dv_bl_signal_compatibility_id: u8,
        dv_md_compression: PacketDoviCompression,
    ) -> Self {
        Self {
            dv_version_major,
            dv_version_minor,
            dv_profile,
            dv_level,
            rpu_present_flag,
            el_present_flag,
            bl_present_flag,
            dv_bl_signal_compatibility_id,
            dv_md_compression,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision configuration packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self {
            dv_version_major: data[Self::DV_VERSION_MAJOR_OFFSET],
            dv_version_minor: data[Self::DV_VERSION_MINOR_OFFSET],
            dv_profile: data[Self::DV_PROFILE_OFFSET],
            dv_level: data[Self::DV_LEVEL_OFFSET],
            rpu_present_flag: read_dovi_flag(
                data[Self::RPU_PRESENT_FLAG_OFFSET],
                "rpu_present_flag",
            )?,
            el_present_flag: read_dovi_flag(data[Self::EL_PRESENT_FLAG_OFFSET], "el_present_flag")?,
            bl_present_flag: read_dovi_flag(data[Self::BL_PRESENT_FLAG_OFFSET], "bl_present_flag")?,
            dv_bl_signal_compatibility_id: data[Self::DV_BL_SIGNAL_COMPATIBILITY_ID_OFFSET],
            dv_md_compression: PacketDoviCompression::from_byte(
                data[Self::DV_MD_COMPRESSION_OFFSET],
            )?,
        })
    }

    pub const fn dv_version_major(self) -> u8 {
        self.dv_version_major
    }

    pub const fn dv_version_minor(self) -> u8 {
        self.dv_version_minor
    }

    pub const fn dv_profile(self) -> u8 {
        self.dv_profile
    }

    pub const fn dv_level(self) -> u8 {
        self.dv_level
    }

    pub const fn rpu_present_flag(self) -> bool {
        self.rpu_present_flag
    }

    pub const fn rpu_present_flag_raw(self) -> u8 {
        self.rpu_present_flag as u8
    }

    pub const fn el_present_flag(self) -> bool {
        self.el_present_flag
    }

    pub const fn el_present_flag_raw(self) -> u8 {
        self.el_present_flag as u8
    }

    pub const fn bl_present_flag(self) -> bool {
        self.bl_present_flag
    }

    pub const fn bl_present_flag_raw(self) -> u8 {
        self.bl_present_flag as u8
    }

    pub const fn dv_bl_signal_compatibility_id(self) -> u8 {
        self.dv_bl_signal_compatibility_id
    }

    pub const fn dv_md_compression(self) -> PacketDoviCompression {
        self.dv_md_compression
    }

    pub const fn dv_md_compression_raw(self) -> u8 {
        self.dv_md_compression.raw()
    }

    pub const fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        [
            self.dv_version_major,
            self.dv_version_minor,
            self.dv_profile,
            self.dv_level,
            self.rpu_present_flag_raw(),
            self.el_present_flag_raw(),
            self.bl_present_flag_raw(),
            self.dv_bl_signal_compatibility_id,
            self.dv_md_compression_raw(),
        ]
    }
}

pub type PacketHdrPlusColorTransformParams<'a> = FrameHdrPlusColorTransformParams<'a>;
pub type PacketHdrPlusOverlapProcessOption = FrameHdrPlusOverlapProcessOption;
pub type PacketHdrPlusPercentile = FrameHdrPlusPercentile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketDynamicHdr10Plus<'a>(FrameDynamicHdrPlus<'a>);

impl<'a> PacketDynamicHdr10Plus<'a> {
    pub const ITU_T_T35_COUNTRY_CODE: u8 = FrameDynamicHdrPlus::ITU_T_T35_COUNTRY_CODE;
    pub const APPLICATION_VERSION: u8 = FrameDynamicHdrPlus::APPLICATION_VERSION;
    pub const MAX_WINDOWS: usize = FrameDynamicHdrPlus::MAX_WINDOWS;
    pub const MAX_PEAK_LUMINANCE_ROWS: usize = FrameDynamicHdrPlus::MAX_PEAK_LUMINANCE_ROWS;
    pub const MAX_PEAK_LUMINANCE_COLS: usize = FrameDynamicHdrPlus::MAX_PEAK_LUMINANCE_COLS;
    pub const DATA_LEN: usize = FrameDynamicHdrPlus::DATA_LEN;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "dynamic HDR10+ packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        FrameDynamicHdrPlus::parse(data).map(Self)
    }

    pub const fn as_frame_dynamic_hdr_plus(self) -> FrameDynamicHdrPlus<'a> {
        self.0
    }

    pub const fn data(self) -> &'a [u8] {
        self.0.data()
    }

    pub fn itu_t_t35_country_code(self) -> u8 {
        self.0.itu_t_t35_country_code()
    }

    pub fn application_version(self) -> u8 {
        self.0.application_version()
    }

    pub const fn num_windows(self) -> usize {
        self.0.num_windows()
    }

    pub fn color_transform_params(
        self,
        index: usize,
    ) -> Option<PacketHdrPlusColorTransformParams<'a>> {
        self.0.color_transform_params(index)
    }

    pub fn targeted_system_display_maximum_luminance(self) -> Rational {
        self.0.targeted_system_display_maximum_luminance()
    }

    pub fn targeted_system_display_actual_peak_luminance_flag(self) -> u8 {
        self.0.targeted_system_display_actual_peak_luminance_flag()
    }

    pub fn num_rows_targeted_system_display_actual_peak_luminance(self) -> usize {
        self.0
            .num_rows_targeted_system_display_actual_peak_luminance()
    }

    pub fn num_cols_targeted_system_display_actual_peak_luminance(self) -> usize {
        self.0
            .num_cols_targeted_system_display_actual_peak_luminance()
    }

    pub fn targeted_system_display_actual_peak_luminance(
        self,
        row: usize,
        col: usize,
    ) -> Option<Rational> {
        self.0
            .targeted_system_display_actual_peak_luminance(row, col)
    }

    pub fn mastering_display_actual_peak_luminance_flag(self) -> u8 {
        self.0.mastering_display_actual_peak_luminance_flag()
    }

    pub fn num_rows_mastering_display_actual_peak_luminance(self) -> usize {
        self.0.num_rows_mastering_display_actual_peak_luminance()
    }

    pub fn num_cols_mastering_display_actual_peak_luminance(self) -> usize {
        self.0.num_cols_mastering_display_actual_peak_luminance()
    }

    pub fn mastering_display_actual_peak_luminance(
        self,
        row: usize,
        col: usize,
    ) -> Option<Rational> {
        self.0.mastering_display_actual_peak_luminance(row, col)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketSkipSamples {
    start: u32,
    end: u32,
    start_reason: PacketSkipSamplesReason,
    end_reason: PacketSkipSamplesReason,
}

impl PacketSkipSamples {
    pub const DATA_LEN: usize = 10;

    pub const fn new(
        start: u32,
        end: u32,
        start_reason: PacketSkipSamplesReason,
        end_reason: PacketSkipSamplesReason,
    ) -> Self {
        Self {
            start,
            end,
            start_reason,
            end_reason,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "skip samples packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut start = [0; 4];
        start.copy_from_slice(&data[..4]);
        let mut end = [0; 4];
        end.copy_from_slice(&data[4..8]);

        Ok(Self {
            start: u32::from_le_bytes(start),
            end: u32::from_le_bytes(end),
            start_reason: PacketSkipSamplesReason::from_byte(data[8])?,
            end_reason: PacketSkipSamplesReason::from_byte(data[9])?,
        })
    }

    pub const fn start(self) -> u32 {
        self.start
    }

    pub const fn end(self) -> u32 {
        self.end
    }

    pub const fn start_reason(self) -> PacketSkipSamplesReason {
        self.start_reason
    }

    pub const fn end_reason(self) -> PacketSkipSamplesReason {
        self.end_reason
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let start = self.start.to_le_bytes();
        let end = self.end.to_le_bytes();
        [
            start[0],
            start[1],
            start[2],
            start[3],
            end[0],
            end[1],
            end[2],
            end[3],
            self.start_reason.as_byte(),
            self.end_reason.as_byte(),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketParamChange {
    sample_rate: Option<i32>,
    dimensions: Option<(i32, i32)>,
}

impl PacketParamChange {
    pub const MIN_DATA_LEN: usize = 4;
    pub const MAX_DATA_LEN: usize = 16;
    pub const SAMPLE_RATE_FLAG: u32 = 0x0004;
    pub const DIMENSIONS_FLAG: u32 = 0x0008;
    pub const KNOWN_FLAGS: u32 = Self::SAMPLE_RATE_FLAG | Self::DIMENSIONS_FLAG;

    pub const fn new(sample_rate: Option<i32>, dimensions: Option<(i32, i32)>) -> Self {
        Self {
            sample_rate,
            dimensions,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() < Self::MIN_DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "parameter change packet side data requires at least {} bytes, got {}",
                Self::MIN_DATA_LEN,
                data.len()
            )));
        }

        let flags = read_u32_le(data, 0);
        let unknown_flags = flags & !Self::KNOWN_FLAGS;
        if unknown_flags != 0 {
            return Err(AvError::invalid_data(format!(
                "parameter change packet side data has unknown flags 0x{unknown_flags:08x}"
            )));
        }

        let mut expected_len = Self::MIN_DATA_LEN;
        if flags & Self::SAMPLE_RATE_FLAG != 0 {
            expected_len += 4;
        }
        if flags & Self::DIMENSIONS_FLAG != 0 {
            expected_len += 8;
        }
        if data.len() != expected_len {
            return Err(AvError::invalid_data(format!(
                "parameter change packet side data with flags 0x{flags:08x} requires exactly {expected_len} bytes, got {}",
                data.len()
            )));
        }

        let mut offset = Self::MIN_DATA_LEN;
        let sample_rate = if flags & Self::SAMPLE_RATE_FLAG != 0 {
            let value = read_i32_le(data, offset);
            offset += 4;
            Some(value)
        } else {
            None
        };
        let dimensions = if flags & Self::DIMENSIONS_FLAG != 0 {
            let width = read_i32_le(data, offset);
            let height = read_i32_le(data, offset + 4);
            Some((width, height))
        } else {
            None
        };

        Ok(Self {
            sample_rate,
            dimensions,
        })
    }

    pub const fn sample_rate(self) -> Option<i32> {
        self.sample_rate
    }

    pub const fn dimensions(self) -> Option<(i32, i32)> {
        self.dimensions
    }

    pub const fn flags(self) -> u32 {
        let mut flags = 0;
        if self.sample_rate.is_some() {
            flags |= Self::SAMPLE_RATE_FLAG;
        }
        if self.dimensions.is_some() {
            flags |= Self::DIMENSIONS_FLAG;
        }
        flags
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MAX_DATA_LEN);
        bytes.extend_from_slice(&self.flags().to_le_bytes());
        if let Some(sample_rate) = self.sample_rate {
            bytes.extend_from_slice(&sample_rate.to_le_bytes());
        }
        if let Some((width, height)) = self.dimensions {
            bytes.extend_from_slice(&width.to_le_bytes());
            bytes.extend_from_slice(&height.to_le_bytes());
        }
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketJpDualMonoSelection {
    MainLeft = 0,
    SubRight = 1,
    Both = 2,
}

impl PacketJpDualMonoSelection {
    pub fn from_byte(value: u8) -> AvResult<Self> {
        match value {
            0 => Ok(Self::MainLeft),
            1 => Ok(Self::SubRight),
            2 => Ok(Self::Both),
            _ => Err(AvError::invalid_data(format!(
                "invalid packet JP dual mono channel selection value {value}"
            ))),
        }
    }

    pub const fn as_byte(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketJpDualMono {
    selected_channels: PacketJpDualMonoSelection,
}

impl PacketJpDualMono {
    pub const DATA_LEN: usize = 1;

    pub const fn new(selected_channels: PacketJpDualMonoSelection) -> Self {
        Self { selected_channels }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "JP dual mono packet side data requires exactly {} byte, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self {
            selected_channels: PacketJpDualMonoSelection::from_byte(data[0])?,
        })
    }

    pub const fn selected_channels(self) -> PacketJpDualMonoSelection {
        self.selected_channels
    }

    pub const fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        [self.selected_channels.as_byte()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketMpegTsStreamId {
    stream_id: u8,
}

impl PacketMpegTsStreamId {
    pub const DATA_LEN: usize = 1;

    pub const fn new(stream_id: u8) -> Self {
        Self { stream_id }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "MPEG-TS stream id packet side data requires exactly {} byte, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self { stream_id: data[0] })
    }

    pub const fn stream_id(self) -> u8 {
        self.stream_id
    }

    pub const fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        [self.stream_id]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketStringMetadataEntry<'a> {
    key: &'a [u8],
    value: &'a [u8],
}

impl<'a> PacketStringMetadataEntry<'a> {
    pub const fn key_bytes(self) -> &'a [u8] {
        self.key
    }

    pub const fn value_bytes(self) -> &'a [u8] {
        self.value
    }

    pub fn key_str(self) -> AvResult<&'a str> {
        std::str::from_utf8(self.key)
            .map_err(|_| AvError::invalid_data("packet string metadata key is not valid UTF-8"))
    }

    pub fn value_str(self) -> AvResult<&'a str> {
        std::str::from_utf8(self.value)
            .map_err(|_| AvError::invalid_data("packet string metadata value is not valid UTF-8"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketStringMetadata<'a> {
    data: &'a [u8],
    entries: Vec<PacketStringMetadataEntry<'a>>,
}

impl<'a> PacketStringMetadata<'a> {
    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.is_empty() {
            return Ok(Self {
                data,
                entries: Vec::new(),
            });
        }

        if data.last() != Some(&0) {
            return Err(AvError::invalid_data(
                "packet string metadata must end with a NUL terminator",
            ));
        }

        let mut entries = Vec::new();
        let mut offset = 0;
        while offset < data.len() {
            let key_end = find_nul(data, offset).ok_or_else(|| {
                AvError::invalid_data("packet string metadata key is not NUL terminated")
            })?;
            let key = &data[offset..key_end];
            if key.is_empty() {
                return Err(AvError::invalid_data(
                    "packet string metadata key must not be empty",
                ));
            }

            offset = key_end + 1;
            if offset >= data.len() {
                return Err(AvError::invalid_data(
                    "packet string metadata key is missing a value",
                ));
            }

            let value_end = find_nul(data, offset).ok_or_else(|| {
                AvError::invalid_data("packet string metadata value is not NUL terminated")
            })?;
            let value = &data[offset..value_end];
            entries.push(PacketStringMetadataEntry { key, value });
            offset = value_end + 1;
        }

        Ok(Self { data, entries })
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn len(&self) -> usize {
        self.data.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entries(&self) -> &[PacketStringMetadataEntry<'a>] {
        &self.entries
    }

    pub fn entry(&self, index: usize) -> Option<PacketStringMetadataEntry<'a>> {
        self.entries.get(index).copied()
    }
}

pub fn packet_pack_dictionary(dict: &Dictionary) -> Vec<u8> {
    let mut data = Vec::new();
    for entry in dict.entries() {
        data.extend_from_slice(entry.key().as_bytes());
        data.push(0);
        data.extend_from_slice(entry.value().as_bytes());
        data.push(0);
    }
    data
}

pub fn packet_unpack_dictionary(data: &[u8]) -> AvResult<Dictionary> {
    let metadata = PacketStringMetadata::parse(data)?;
    let mut dict = Dictionary::new();
    for entry in metadata.entries() {
        dict.set(entry.key_str()?, entry.value_str()?)?;
    }
    Ok(dict)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketEncryptionSubsample {
    bytes_of_clear_data: u32,
    bytes_of_protected_data: u32,
}

impl PacketEncryptionSubsample {
    pub const DATA_LEN: usize = 8;

    pub const fn new(bytes_of_clear_data: u32, bytes_of_protected_data: u32) -> Self {
        Self {
            bytes_of_clear_data,
            bytes_of_protected_data,
        }
    }

    pub const fn bytes_of_clear_data(self) -> u32 {
        self.bytes_of_clear_data
    }

    pub const fn bytes_of_protected_data(self) -> u32 {
        self.bytes_of_protected_data
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let clear = self.bytes_of_clear_data.to_be_bytes();
        let protected = self.bytes_of_protected_data.to_be_bytes();
        [
            clear[0],
            clear[1],
            clear[2],
            clear[3],
            protected[0],
            protected[1],
            protected[2],
            protected[3],
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketEncryptionInfo<'a> {
    data: &'a [u8],
    parsed_len: usize,
    scheme: u32,
    crypt_byte_block: u32,
    skip_byte_block: u32,
    key_id: &'a [u8],
    iv: &'a [u8],
    subsamples: Vec<PacketEncryptionSubsample>,
}

impl<'a> PacketEncryptionInfo<'a> {
    pub const HEADER_LEN: usize = 24;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "encryption info packet side data requires at least {} bytes, got {}",
                Self::HEADER_LEN,
                data.len()
            )));
        }

        let key_id_size = usize_from_u32_be(data, 12, "encryption info key ID size")?;
        let iv_size = usize_from_u32_be(data, 16, "encryption info IV size")?;
        let subsample_count = usize_from_u32_be(data, 20, "encryption info subsample count")?;
        let subsamples_len = subsample_count
            .checked_mul(PacketEncryptionSubsample::DATA_LEN)
            .ok_or_else(|| {
                AvError::invalid_data("encryption info subsample byte length overflows usize")
            })?;
        let parsed_len = Self::HEADER_LEN
            .checked_add(key_id_size)
            .and_then(|value| value.checked_add(iv_size))
            .and_then(|value| value.checked_add(subsamples_len))
            .ok_or_else(|| AvError::invalid_data("encryption info byte length overflows usize"))?;

        if data.len() < parsed_len {
            return Err(AvError::invalid_data(format!(
                "encryption info packet side data requires at least {parsed_len} bytes, got {}",
                data.len()
            )));
        }

        let key_id_start = Self::HEADER_LEN;
        let key_id_end = key_id_start + key_id_size;
        let iv_end = key_id_end + iv_size;
        let mut offset = iv_end;
        let mut subsamples = Vec::new();
        for _ in 0..subsample_count {
            subsamples.push(PacketEncryptionSubsample::new(
                read_u32_be(data, offset),
                read_u32_be(data, offset + 4),
            ));
            offset += PacketEncryptionSubsample::DATA_LEN;
        }

        Ok(Self {
            data,
            parsed_len,
            scheme: read_u32_be(data, 0),
            crypt_byte_block: read_u32_be(data, 4),
            skip_byte_block: read_u32_be(data, 8),
            key_id: &data[key_id_start..key_id_end],
            iv: &data[key_id_end..iv_end],
            subsamples,
        })
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn parsed_len(&self) -> usize {
        self.parsed_len
    }

    pub fn trailing_data(&self) -> &'a [u8] {
        &self.data[self.parsed_len..]
    }

    pub const fn scheme(&self) -> u32 {
        self.scheme
    }

    pub const fn scheme_fourcc(&self) -> [u8; 4] {
        self.scheme.to_be_bytes()
    }

    pub const fn crypt_byte_block(&self) -> u32 {
        self.crypt_byte_block
    }

    pub const fn skip_byte_block(&self) -> u32 {
        self.skip_byte_block
    }

    pub const fn key_id(&self) -> &'a [u8] {
        self.key_id
    }

    pub const fn key_id_size(&self) -> usize {
        self.key_id.len()
    }

    pub const fn iv(&self) -> &'a [u8] {
        self.iv
    }

    pub const fn iv_size(&self) -> usize {
        self.iv.len()
    }

    pub fn subsample_count(&self) -> usize {
        self.subsamples.len()
    }

    pub fn subsamples(&self) -> &[PacketEncryptionSubsample] {
        &self.subsamples
    }

    pub fn subsample(&self, index: usize) -> Option<PacketEncryptionSubsample> {
        self.subsamples.get(index).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketEncryptionInitInfoEntry<'a> {
    system_id: &'a [u8],
    key_id_size: usize,
    key_ids: Vec<&'a [u8]>,
    data: &'a [u8],
}

impl<'a> PacketEncryptionInitInfoEntry<'a> {
    pub const fn system_id(&self) -> &'a [u8] {
        self.system_id
    }

    pub const fn system_id_size(&self) -> usize {
        self.system_id.len()
    }

    pub const fn key_id_size(&self) -> usize {
        self.key_id_size
    }

    pub fn key_id_count(&self) -> usize {
        self.key_ids.len()
    }

    pub fn key_ids(&self) -> &[&'a [u8]] {
        &self.key_ids
    }

    pub fn key_id(&self, index: usize) -> Option<&'a [u8]> {
        self.key_ids.get(index).copied()
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn data_size(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketEncryptionInitInfo<'a> {
    data: &'a [u8],
    parsed_len: usize,
    entries: Vec<PacketEncryptionInitInfoEntry<'a>>,
}

impl<'a> PacketEncryptionInitInfo<'a> {
    pub const COUNT_LEN: usize = 4;
    pub const ENTRY_HEADER_LEN: usize = 16;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::COUNT_LEN {
            return Err(AvError::invalid_data(format!(
                "encryption init info packet side data requires at least {} bytes, got {}",
                Self::COUNT_LEN,
                data.len()
            )));
        }

        let entry_count = usize_from_u32_be(data, 0, "encryption init info entry count")?;
        let mut entries = Vec::new();
        let mut offset = Self::COUNT_LEN;
        for _ in 0..entry_count {
            if data.len().saturating_sub(offset) < Self::ENTRY_HEADER_LEN {
                return Err(AvError::invalid_data(
                    "encryption init info entry header is truncated",
                ));
            }

            let system_id_size =
                usize_from_u32_be(data, offset, "encryption init info system ID size")?;
            let key_id_count =
                usize_from_u32_be(data, offset + 4, "encryption init info key ID count")?;
            let key_id_size =
                usize_from_u32_be(data, offset + 8, "encryption init info key ID size")?;
            let data_size = usize_from_u32_be(data, offset + 12, "encryption init info data size")?;
            if key_id_count != 0 && key_id_size == 0 {
                return Err(AvError::invalid_data(
                    "encryption init info key ID count requires nonzero key ID size",
                ));
            }

            let key_ids_len = key_id_count.checked_mul(key_id_size).ok_or_else(|| {
                AvError::invalid_data("encryption init info key ID byte length overflows usize")
            })?;
            let entry_body_len = system_id_size
                .checked_add(key_ids_len)
                .and_then(|value| value.checked_add(data_size))
                .ok_or_else(|| {
                    AvError::invalid_data("encryption init info entry byte length overflows usize")
                })?;

            offset += Self::ENTRY_HEADER_LEN;
            if data.len().saturating_sub(offset) < entry_body_len {
                return Err(AvError::invalid_data(
                    "encryption init info entry payload is truncated",
                ));
            }

            let system_id = &data[offset..offset + system_id_size];
            offset += system_id_size;

            let mut key_ids = Vec::new();
            for _ in 0..key_id_count {
                key_ids.push(&data[offset..offset + key_id_size]);
                offset += key_id_size;
            }

            let entry_data = &data[offset..offset + data_size];
            offset += data_size;

            entries.push(PacketEncryptionInitInfoEntry {
                system_id,
                key_id_size,
                key_ids,
                data: entry_data,
            });
        }

        Ok(Self {
            data,
            parsed_len: offset,
            entries,
        })
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn parsed_len(&self) -> usize {
        self.parsed_len
    }

    pub fn trailing_data(&self) -> &'a [u8] {
        &self.data[self.parsed_len..]
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entries(&self) -> &[PacketEncryptionInitInfoEntry<'a>] {
        &self.entries
    }

    pub fn entry(&self, index: usize) -> Option<&PacketEncryptionInitInfoEntry<'a>> {
        self.entries.get(index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PacketIamfAnimationType {
    Step = 0,
    Linear = 1,
    Bezier = 2,
}

impl PacketIamfAnimationType {
    pub fn from_raw(value: u32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::Step),
            1 => Ok(Self::Linear),
            2 => Ok(Self::Bezier),
            _ => Err(AvError::invalid_data(format!(
                "IAMF mix gain animation type {value} is invalid"
            ))),
        }
    }

    pub const fn as_raw(self) -> u32 {
        self as u32
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::Step => "AV_IAMF_ANIMATION_TYPE_STEP",
            Self::Linear => "AV_IAMF_ANIMATION_TYPE_LINEAR",
            Self::Bezier => "AV_IAMF_ANIMATION_TYPE_BEZIER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PacketIamfParamDefinitionType {
    MixGain = 0,
    Demixing = 1,
    ReconGain = 2,
}

impl PacketIamfParamDefinitionType {
    pub fn from_raw(value: u32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::MixGain),
            1 => Ok(Self::Demixing),
            2 => Ok(Self::ReconGain),
            _ => Err(AvError::invalid_data(format!(
                "IAMF parameter definition type {value} is invalid"
            ))),
        }
    }

    pub const fn as_raw(self) -> u32 {
        self as u32
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::MixGain => "AV_IAMF_PARAMETER_DEFINITION_MIX_GAIN",
            Self::Demixing => "AV_IAMF_PARAMETER_DEFINITION_DEMIXING",
            Self::ReconGain => "AV_IAMF_PARAMETER_DEFINITION_RECON_GAIN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketIamfParamDefinition<'a> {
    data: &'a [u8],
    parsed_len: usize,
    av_class_address: usize,
    subblocks_offset: usize,
    subblock_size: usize,
    subblock_count: usize,
    definition_type: PacketIamfParamDefinitionType,
    parameter_id: u32,
    parameter_rate: u32,
    duration: u32,
    constant_subblock_duration: u32,
}

impl<'a> PacketIamfParamDefinition<'a> {
    const NATIVE_WORD_LEN: usize = core::mem::size_of::<usize>();
    pub const AV_CLASS_OFFSET: usize = 0;
    pub const SUBBLOCKS_OFFSET_OFFSET: usize = Self::NATIVE_WORD_LEN;
    pub const SUBBLOCK_SIZE_OFFSET: usize = Self::NATIVE_WORD_LEN * 2;
    pub const SUBBLOCK_COUNT_OFFSET: usize = Self::NATIVE_WORD_LEN * 3;
    pub const TYPE_OFFSET: usize = Self::SUBBLOCK_COUNT_OFFSET + 4;
    pub const PARAMETER_ID_OFFSET: usize = Self::SUBBLOCK_COUNT_OFFSET + 8;
    pub const PARAMETER_RATE_OFFSET: usize = Self::SUBBLOCK_COUNT_OFFSET + 12;
    pub const DURATION_OFFSET: usize = Self::SUBBLOCK_COUNT_OFFSET + 16;
    pub const CONSTANT_SUBBLOCK_DURATION_OFFSET: usize = Self::SUBBLOCK_COUNT_OFFSET + 20;
    pub const HEADER_LEN: usize = align_native(Self::SUBBLOCK_COUNT_OFFSET + 24);

    fn parse(
        data: &'a [u8],
        expected_type: PacketIamfParamDefinitionType,
        min_subblock_size: usize,
    ) -> AvResult<Self> {
        if data.len() < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "IAMF parameter definition packet side data requires at least {} bytes, got {}",
                Self::HEADER_LEN,
                data.len()
            )));
        }

        let definition_type =
            PacketIamfParamDefinitionType::from_raw(read_u32_ne(data, Self::TYPE_OFFSET))?;
        if definition_type != expected_type {
            return Err(AvError::invalid_data(format!(
                "IAMF parameter definition type {} does not match expected {}",
                definition_type.ffmpeg_constant(),
                expected_type.ffmpeg_constant()
            )));
        }

        let subblocks_offset = read_usize_ne(data, Self::SUBBLOCKS_OFFSET_OFFSET)?;
        if subblocks_offset < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "IAMF parameter definition subblocks offset {subblocks_offset} is before header length {}",
                Self::HEADER_LEN
            )));
        }
        if subblocks_offset > data.len() {
            return Err(AvError::invalid_data(format!(
                "IAMF parameter definition subblocks offset {subblocks_offset} exceeds payload length {}",
                data.len()
            )));
        }

        let subblock_size = read_usize_ne(data, Self::SUBBLOCK_SIZE_OFFSET)?;
        if subblock_size < min_subblock_size {
            return Err(AvError::invalid_data(format!(
                "IAMF parameter definition subblock size {subblock_size} is smaller than required {min_subblock_size}"
            )));
        }

        let subblock_count = usize::try_from(read_u32_ne(data, Self::SUBBLOCK_COUNT_OFFSET))
            .map_err(|_| AvError::invalid_data("IAMF subblock count does not fit in usize"))?;
        let subblocks_len = subblock_count.checked_mul(subblock_size).ok_or_else(|| {
            AvError::invalid_data("IAMF parameter definition subblock byte length overflows usize")
        })?;
        let parsed_len = subblocks_offset.checked_add(subblocks_len).ok_or_else(|| {
            AvError::invalid_data("IAMF parameter definition parsed byte length overflows usize")
        })?;
        if parsed_len > data.len() {
            return Err(AvError::invalid_data(format!(
                "IAMF parameter definition requires at least {parsed_len} bytes, got {}",
                data.len()
            )));
        }

        let parameter_rate = read_u32_ne(data, Self::PARAMETER_RATE_OFFSET);
        if parameter_rate == 0 {
            return Err(AvError::invalid_data(
                "IAMF parameter definition parameter_rate must not be zero",
            ));
        }

        for index in 0..subblock_count {
            let offset = subblocks_offset + index * subblock_size;
            let duration = read_u32_ne(data, offset + Self::NATIVE_WORD_LEN);
            if duration == 0 {
                return Err(AvError::invalid_data(format!(
                    "IAMF parameter definition subblock {index} duration must not be zero"
                )));
            }
        }

        Ok(Self {
            data,
            parsed_len,
            av_class_address: read_usize_ne(data, Self::AV_CLASS_OFFSET)?,
            subblocks_offset,
            subblock_size,
            subblock_count,
            definition_type,
            parameter_id: read_u32_ne(data, Self::PARAMETER_ID_OFFSET),
            parameter_rate,
            duration: read_u32_ne(data, Self::DURATION_OFFSET),
            constant_subblock_duration: read_u32_ne(data, Self::CONSTANT_SUBBLOCK_DURATION_OFFSET),
        })
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn parsed_len(&self) -> usize {
        self.parsed_len
    }

    pub fn trailing_data(&self) -> &'a [u8] {
        &self.data[self.parsed_len..]
    }

    pub const fn av_class_address(&self) -> usize {
        self.av_class_address
    }

    pub const fn subblocks_offset(&self) -> usize {
        self.subblocks_offset
    }

    pub const fn subblock_size(&self) -> usize {
        self.subblock_size
    }

    pub const fn subblock_count(&self) -> usize {
        self.subblock_count
    }

    pub const fn definition_type(&self) -> PacketIamfParamDefinitionType {
        self.definition_type
    }

    pub const fn parameter_id(&self) -> u32 {
        self.parameter_id
    }

    pub const fn parameter_rate(&self) -> u32 {
        self.parameter_rate
    }

    pub const fn duration(&self) -> u32 {
        self.duration
    }

    pub const fn constant_subblock_duration(&self) -> u32 {
        self.constant_subblock_duration
    }

    pub fn subblock_bytes(&self, index: usize) -> Option<&'a [u8]> {
        if index >= self.subblock_count {
            return None;
        }

        let offset = self.subblocks_offset + index * self.subblock_size;
        Some(&self.data[offset..offset + self.subblock_size])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketIamfMixGainSubblock<'a> {
    data: &'a [u8],
    av_class_address: usize,
    subblock_duration: u32,
    animation_type: PacketIamfAnimationType,
    start_point_value: Rational,
    end_point_value: Rational,
    control_point_value: Rational,
    control_point_relative_time: Rational,
}

impl<'a> PacketIamfMixGainSubblock<'a> {
    const NATIVE_WORD_LEN: usize = core::mem::size_of::<usize>();
    pub const AV_CLASS_OFFSET: usize = 0;
    pub const SUBBLOCK_DURATION_OFFSET: usize = Self::NATIVE_WORD_LEN;
    pub const ANIMATION_TYPE_OFFSET: usize = Self::NATIVE_WORD_LEN + 4;
    pub const START_POINT_VALUE_OFFSET: usize = Self::NATIVE_WORD_LEN + 8;
    pub const END_POINT_VALUE_OFFSET: usize = Self::START_POINT_VALUE_OFFSET + 8;
    pub const CONTROL_POINT_VALUE_OFFSET: usize = Self::END_POINT_VALUE_OFFSET + 8;
    pub const CONTROL_POINT_RELATIVE_TIME_OFFSET: usize = Self::CONTROL_POINT_VALUE_OFFSET + 8;
    pub const MIN_DATA_LEN: usize = align_native(Self::CONTROL_POINT_RELATIVE_TIME_OFFSET + 8);

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::MIN_DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "IAMF mix gain subblock requires at least {} bytes, got {}",
                Self::MIN_DATA_LEN,
                data.len()
            )));
        }

        let subblock_duration = read_u32_ne(data, Self::SUBBLOCK_DURATION_OFFSET);
        if subblock_duration == 0 {
            return Err(AvError::invalid_data(
                "IAMF mix gain subblock duration must not be zero",
            ));
        }

        Ok(Self {
            data,
            av_class_address: read_usize_ne(data, Self::AV_CLASS_OFFSET)?,
            subblock_duration,
            animation_type: PacketIamfAnimationType::from_raw(read_u32_ne(
                data,
                Self::ANIMATION_TYPE_OFFSET,
            ))?,
            start_point_value: read_rational_ne(data, Self::START_POINT_VALUE_OFFSET),
            end_point_value: read_rational_ne(data, Self::END_POINT_VALUE_OFFSET),
            control_point_value: read_rational_ne(data, Self::CONTROL_POINT_VALUE_OFFSET),
            control_point_relative_time: read_rational_ne(
                data,
                Self::CONTROL_POINT_RELATIVE_TIME_OFFSET,
            ),
        })
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn av_class_address(self) -> usize {
        self.av_class_address
    }

    pub const fn subblock_duration(self) -> u32 {
        self.subblock_duration
    }

    pub const fn animation_type(self) -> PacketIamfAnimationType {
        self.animation_type
    }

    pub const fn start_point_value(self) -> Rational {
        self.start_point_value
    }

    pub const fn end_point_value(self) -> Rational {
        self.end_point_value
    }

    pub const fn control_point_value(self) -> Rational {
        self.control_point_value
    }

    pub const fn control_point_relative_time(self) -> Rational {
        self.control_point_relative_time
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketIamfMixGainParam<'a> {
    definition: PacketIamfParamDefinition<'a>,
    subblocks: Vec<PacketIamfMixGainSubblock<'a>>,
}

impl<'a> PacketIamfMixGainParam<'a> {
    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        let definition = PacketIamfParamDefinition::parse(
            data,
            PacketIamfParamDefinitionType::MixGain,
            PacketIamfMixGainSubblock::MIN_DATA_LEN,
        )?;
        let mut subblocks = Vec::with_capacity(definition.subblock_count());
        for index in 0..definition.subblock_count() {
            let data = definition.subblock_bytes(index).ok_or_else(|| {
                AvError::invalid_data("IAMF mix gain subblock is missing after validation")
            })?;
            subblocks.push(PacketIamfMixGainSubblock::parse(data)?);
        }

        Ok(Self {
            definition,
            subblocks,
        })
    }

    pub const fn definition(&self) -> &PacketIamfParamDefinition<'a> {
        &self.definition
    }

    pub fn subblocks(&self) -> &[PacketIamfMixGainSubblock<'a>] {
        &self.subblocks
    }

    pub fn subblock_count(&self) -> usize {
        self.subblocks.len()
    }

    pub fn subblock(&self, index: usize) -> Option<PacketIamfMixGainSubblock<'a>> {
        self.subblocks.get(index).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketIamfDemixingInfoSubblock<'a> {
    data: &'a [u8],
    av_class_address: usize,
    subblock_duration: u32,
    dmixp_mode: u32,
}

impl<'a> PacketIamfDemixingInfoSubblock<'a> {
    const NATIVE_WORD_LEN: usize = core::mem::size_of::<usize>();
    pub const AV_CLASS_OFFSET: usize = 0;
    pub const SUBBLOCK_DURATION_OFFSET: usize = Self::NATIVE_WORD_LEN;
    pub const DMIXP_MODE_OFFSET: usize = Self::NATIVE_WORD_LEN + 4;
    pub const MIN_DATA_LEN: usize = align_native(Self::DMIXP_MODE_OFFSET + 4);

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::MIN_DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "IAMF demixing info subblock requires at least {} bytes, got {}",
                Self::MIN_DATA_LEN,
                data.len()
            )));
        }

        let subblock_duration = read_u32_ne(data, Self::SUBBLOCK_DURATION_OFFSET);
        if subblock_duration == 0 {
            return Err(AvError::invalid_data(
                "IAMF demixing info subblock duration must not be zero",
            ));
        }

        Ok(Self {
            data,
            av_class_address: read_usize_ne(data, Self::AV_CLASS_OFFSET)?,
            subblock_duration,
            dmixp_mode: read_u32_ne(data, Self::DMIXP_MODE_OFFSET),
        })
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn av_class_address(self) -> usize {
        self.av_class_address
    }

    pub const fn subblock_duration(self) -> u32 {
        self.subblock_duration
    }

    pub const fn dmixp_mode(self) -> u32 {
        self.dmixp_mode
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketIamfDemixingInfoParam<'a> {
    definition: PacketIamfParamDefinition<'a>,
    subblocks: Vec<PacketIamfDemixingInfoSubblock<'a>>,
}

impl<'a> PacketIamfDemixingInfoParam<'a> {
    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        let definition = PacketIamfParamDefinition::parse(
            data,
            PacketIamfParamDefinitionType::Demixing,
            PacketIamfDemixingInfoSubblock::MIN_DATA_LEN,
        )?;
        let mut subblocks = Vec::with_capacity(definition.subblock_count());
        for index in 0..definition.subblock_count() {
            let data = definition.subblock_bytes(index).ok_or_else(|| {
                AvError::invalid_data("IAMF demixing info subblock is missing after validation")
            })?;
            subblocks.push(PacketIamfDemixingInfoSubblock::parse(data)?);
        }

        Ok(Self {
            definition,
            subblocks,
        })
    }

    pub const fn definition(&self) -> &PacketIamfParamDefinition<'a> {
        &self.definition
    }

    pub fn subblocks(&self) -> &[PacketIamfDemixingInfoSubblock<'a>] {
        &self.subblocks
    }

    pub fn subblock_count(&self) -> usize {
        self.subblocks.len()
    }

    pub fn subblock(&self, index: usize) -> Option<PacketIamfDemixingInfoSubblock<'a>> {
        self.subblocks.get(index).copied()
    }
}

const IAMF_RECON_GAIN_LAYERS: usize = 6;
const IAMF_RECON_GAIN_CHANNELS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketIamfReconGainSubblock<'a> {
    data: &'a [u8],
    av_class_address: usize,
    subblock_duration: u32,
    recon_gain: [[u8; IAMF_RECON_GAIN_CHANNELS]; IAMF_RECON_GAIN_LAYERS],
}

impl<'a> PacketIamfReconGainSubblock<'a> {
    const NATIVE_WORD_LEN: usize = core::mem::size_of::<usize>();
    pub const LAYERS: usize = IAMF_RECON_GAIN_LAYERS;
    pub const CHANNELS: usize = IAMF_RECON_GAIN_CHANNELS;
    pub const AV_CLASS_OFFSET: usize = 0;
    pub const SUBBLOCK_DURATION_OFFSET: usize = Self::NATIVE_WORD_LEN;
    pub const RECON_GAIN_OFFSET: usize = Self::NATIVE_WORD_LEN + 4;
    pub const MIN_DATA_LEN: usize =
        align_native(Self::RECON_GAIN_OFFSET + Self::LAYERS * Self::CHANNELS);

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::MIN_DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "IAMF recon gain subblock requires at least {} bytes, got {}",
                Self::MIN_DATA_LEN,
                data.len()
            )));
        }

        let subblock_duration = read_u32_ne(data, Self::SUBBLOCK_DURATION_OFFSET);
        if subblock_duration == 0 {
            return Err(AvError::invalid_data(
                "IAMF recon gain subblock duration must not be zero",
            ));
        }

        let mut recon_gain = [[0; Self::CHANNELS]; Self::LAYERS];
        for (layer_index, layer) in recon_gain.iter_mut().enumerate() {
            let offset = Self::RECON_GAIN_OFFSET + layer_index * Self::CHANNELS;
            layer.copy_from_slice(&data[offset..offset + Self::CHANNELS]);
        }

        Ok(Self {
            data,
            av_class_address: read_usize_ne(data, Self::AV_CLASS_OFFSET)?,
            subblock_duration,
            recon_gain,
        })
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn av_class_address(self) -> usize {
        self.av_class_address
    }

    pub const fn subblock_duration(self) -> u32 {
        self.subblock_duration
    }

    pub const fn recon_gain(self) -> [[u8; IAMF_RECON_GAIN_CHANNELS]; IAMF_RECON_GAIN_LAYERS] {
        self.recon_gain
    }

    pub fn recon_gain_value(self, layer: usize, channel: usize) -> Option<u8> {
        if layer >= Self::LAYERS || channel >= Self::CHANNELS {
            return None;
        }

        Some(self.recon_gain[layer][channel])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketIamfReconGainInfoParam<'a> {
    definition: PacketIamfParamDefinition<'a>,
    subblocks: Vec<PacketIamfReconGainSubblock<'a>>,
}

impl<'a> PacketIamfReconGainInfoParam<'a> {
    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        let definition = PacketIamfParamDefinition::parse(
            data,
            PacketIamfParamDefinitionType::ReconGain,
            PacketIamfReconGainSubblock::MIN_DATA_LEN,
        )?;
        let mut subblocks = Vec::with_capacity(definition.subblock_count());
        for index in 0..definition.subblock_count() {
            let data = definition.subblock_bytes(index).ok_or_else(|| {
                AvError::invalid_data("IAMF recon gain subblock is missing after validation")
            })?;
            subblocks.push(PacketIamfReconGainSubblock::parse(data)?);
        }

        Ok(Self {
            definition,
            subblocks,
        })
    }

    pub const fn definition(&self) -> &PacketIamfParamDefinition<'a> {
        &self.definition
    }

    pub fn subblocks(&self) -> &[PacketIamfReconGainSubblock<'a>] {
        &self.subblocks
    }

    pub fn subblock_count(&self) -> usize {
        self.subblocks.len()
    }

    pub fn subblock(&self, index: usize) -> Option<PacketIamfReconGainSubblock<'a>> {
        self.subblocks.get(index).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketReplayGain {
    track_gain: i32,
    track_peak: u32,
    album_gain: i32,
    album_peak: u32,
}

impl PacketReplayGain {
    pub const DATA_LEN: usize = 16;
    pub const GAIN_UNKNOWN: i32 = i32::MIN;
    pub const PEAK_UNKNOWN: u32 = 0;

    pub const fn new(track_gain: i32, track_peak: u32, album_gain: i32, album_peak: u32) -> Self {
        Self {
            track_gain,
            track_peak,
            album_gain,
            album_peak,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "replaygain packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self {
            track_gain: read_i32_ne(data, 0),
            track_peak: read_u32_ne(data, 4),
            album_gain: read_i32_ne(data, 8),
            album_peak: read_u32_ne(data, 12),
        })
    }

    pub const fn track_gain(self) -> i32 {
        self.track_gain
    }

    pub const fn track_peak(self) -> u32 {
        self.track_peak
    }

    pub const fn album_gain(self) -> i32 {
        self.album_gain
    }

    pub const fn album_peak(self) -> u32 {
        self.album_peak
    }

    pub const fn track_gain_unknown(self) -> bool {
        self.track_gain == Self::GAIN_UNKNOWN
    }

    pub const fn album_gain_unknown(self) -> bool {
        self.album_gain == Self::GAIN_UNKNOWN
    }

    pub const fn track_peak_unknown(self) -> bool {
        self.track_peak == Self::PEAK_UNKNOWN
    }

    pub const fn album_peak_unknown(self) -> bool {
        self.album_peak == Self::PEAK_UNKNOWN
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[0..4].copy_from_slice(&self.track_gain.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.track_peak.to_ne_bytes());
        bytes[8..12].copy_from_slice(&self.album_gain.to_ne_bytes());
        bytes[12..16].copy_from_slice(&self.album_peak.to_ne_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketSubtitlePosition {
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
}

impl PacketSubtitlePosition {
    pub const DATA_LEN: usize = 16;

    pub const fn new(x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "subtitle position packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self {
            x1: read_u32_le(data, 0),
            y1: read_u32_le(data, 4),
            x2: read_u32_le(data, 8),
            y2: read_u32_le(data, 12),
        })
    }

    pub const fn x1(self) -> u32 {
        self.x1
    }

    pub const fn y1(self) -> u32 {
        self.y1
    }

    pub const fn x2(self) -> u32 {
        self.x2
    }

    pub const fn y2(self) -> u32 {
        self.y2
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let x1 = self.x1.to_le_bytes();
        let y1 = self.y1.to_le_bytes();
        let x2 = self.x2.to_le_bytes();
        let y2 = self.y2.to_le_bytes();
        [
            x1[0], x1[1], x1[2], x1[3], y1[0], y1[1], y1[2], y1[3], x2[0], x2[1], x2[2], x2[3],
            y2[0], y2[1], y2[2], y2[3],
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketMatroskaBlockAdditional {
    block_add_id: u64,
    data: Vec<u8>,
}

impl PacketMatroskaBlockAdditional {
    pub const ID_LEN: usize = 8;
    pub const MIN_DATA_LEN: usize = Self::ID_LEN;

    pub fn new(block_add_id: u64, data: Vec<u8>) -> Self {
        Self { block_add_id, data }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() < Self::MIN_DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "Matroska BlockAdditional packet side data requires at least {} bytes, got {}",
                Self::MIN_DATA_LEN,
                data.len()
            )));
        }

        Ok(Self {
            block_add_id: read_u64_be(data, 0),
            data: data[Self::ID_LEN..].to_vec(),
        })
    }

    pub const fn block_add_id(&self) -> u64 {
        self.block_add_id
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::ID_LEN + self.data.len());
        bytes.extend_from_slice(&self.block_add_id.to_be_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketWebVttIdentifier {
    data: Vec<u8>,
}

impl PacketWebVttIdentifier {
    pub fn new(data: Vec<u8>) -> AvResult<Self> {
        validate_webvtt_line_payload(&data, "WebVTT identifier")?;
        validate_webvtt_identifier_payload(&data)?;
        Ok(Self { data })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        validate_webvtt_line_payload(data, "WebVTT identifier")?;
        validate_webvtt_identifier_payload(data)?;
        Ok(Self {
            data: data.to_vec(),
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn as_str(&self) -> AvResult<&str> {
        std::str::from_utf8(&self.data)
            .map_err(|_| AvError::invalid_data("WebVTT identifier is not valid UTF-8"))
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketWebVttSettings {
    data: Vec<u8>,
}

impl PacketWebVttSettings {
    pub fn new(data: Vec<u8>) -> AvResult<Self> {
        validate_webvtt_line_payload(&data, "WebVTT settings")?;
        Ok(Self { data })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        validate_webvtt_line_payload(data, "WebVTT settings")?;
        Ok(Self {
            data: data.to_vec(),
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn as_str(&self) -> AvResult<&str> {
        std::str::from_utf8(&self.data)
            .map_err(|_| AvError::invalid_data("WebVTT settings are not valid UTF-8"))
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketActiveFormatDescription {
    Same = 8,
    FourThree = 9,
    SixteenNine = 10,
    FourteenNine = 11,
    FourThreeProtectedFourteenNine = 13,
    SixteenNineProtectedFourteenNine = 14,
    ProtectedFourThree = 15,
}

impl PacketActiveFormatDescription {
    pub const DATA_LEN: usize = 1;

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "active format description packet side data requires exactly {} byte, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Self::from_byte(data[0])
    }

    pub fn from_byte(value: u8) -> AvResult<Self> {
        match value {
            8 => Ok(Self::Same),
            9 => Ok(Self::FourThree),
            10 => Ok(Self::SixteenNine),
            11 => Ok(Self::FourteenNine),
            13 => Ok(Self::FourThreeProtectedFourteenNine),
            14 => Ok(Self::SixteenNineProtectedFourteenNine),
            15 => Ok(Self::ProtectedFourThree),
            _ => Err(AvError::invalid_data(format!(
                "invalid active format description value {value}"
            ))),
        }
    }

    pub const fn as_byte(self) -> u8 {
        self as u8
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::Same => "AV_AFD_SAME",
            Self::FourThree => "AV_AFD_4_3",
            Self::SixteenNine => "AV_AFD_16_9",
            Self::FourteenNine => "AV_AFD_14_9",
            Self::FourThreeProtectedFourteenNine => "AV_AFD_4_3_SP_14_9",
            Self::SixteenNineProtectedFourteenNine => "AV_AFD_16_9_SP_14_9",
            Self::ProtectedFourThree => "AV_AFD_SP_4_3",
        }
    }

    pub const fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        [self.as_byte()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketS12mTimecode {
    words: [u32; Self::WORDS],
}

impl PacketS12mTimecode {
    pub const WORDS: usize = 4;
    pub const DATA_LEN: usize = Self::WORDS * 4;
    pub const MIN_TIMECODES: usize = 1;
    pub const MAX_TIMECODES: usize = 3;

    pub fn new(timecodes: &[u32]) -> AvResult<Self> {
        if !(Self::MIN_TIMECODES..=Self::MAX_TIMECODES).contains(&timecodes.len()) {
            return Err(AvError::invalid_argument(format!(
                "S12M timecode packet side data requires {} to {} timecodes, got {}",
                Self::MIN_TIMECODES,
                Self::MAX_TIMECODES,
                timecodes.len()
            )));
        }

        let mut words = [0; Self::WORDS];
        words[0] = timecodes.len() as u32;
        words[1..=timecodes.len()].copy_from_slice(timecodes);
        Ok(Self { words })
    }

    pub fn from_raw_words(words: [u32; Self::WORDS]) -> AvResult<Self> {
        match words[0] {
            1..=3 => Ok(Self { words }),
            count => Err(AvError::invalid_data(format!(
                "invalid S12M timecode count {count}"
            ))),
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "S12M timecode packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut words = [0; Self::WORDS];
        for (word, chunk) in words.iter_mut().zip(data.chunks_exact(4)) {
            let mut bytes = [0; 4];
            bytes.copy_from_slice(chunk);
            *word = u32::from_ne_bytes(bytes);
        }

        Self::from_raw_words(words)
    }

    pub const fn count(self) -> usize {
        self.words[0] as usize
    }

    pub fn timecodes(&self) -> &[u32] {
        &self.words[1..1 + self.count()]
    }

    pub const fn raw_words(self) -> [u32; Self::WORDS] {
        self.words
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        for (word, chunk) in self.words.iter().zip(bytes.chunks_exact_mut(4)) {
            chunk.copy_from_slice(&word.to_ne_bytes());
        }
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketFrameCropping {
    crop_top: u32,
    crop_bottom: u32,
    crop_left: u32,
    crop_right: u32,
}

impl PacketFrameCropping {
    pub const DATA_LEN: usize = 16;

    pub const fn new(crop_top: u32, crop_bottom: u32, crop_left: u32, crop_right: u32) -> Self {
        Self {
            crop_top,
            crop_bottom,
            crop_left,
            crop_right,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "frame cropping packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self {
            crop_top: read_u32_le(data, 0),
            crop_bottom: read_u32_le(data, 4),
            crop_left: read_u32_le(data, 8),
            crop_right: read_u32_le(data, 12),
        })
    }

    pub const fn crop_top(self) -> u32 {
        self.crop_top
    }

    pub const fn crop_bottom(self) -> u32 {
        self.crop_bottom
    }

    pub const fn crop_left(self) -> u32 {
        self.crop_left
    }

    pub const fn crop_right(self) -> u32 {
        self.crop_right
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let top = self.crop_top.to_le_bytes();
        let bottom = self.crop_bottom.to_le_bytes();
        let left = self.crop_left.to_le_bytes();
        let right = self.crop_right.to_le_bytes();
        [
            top[0], top[1], top[2], top[3], bottom[0], bottom[1], bottom[2], bottom[3], left[0],
            left[1], left[2], left[3], right[0], right[1], right[2], right[3],
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketDisplayMatrix {
    elements: [i32; Self::ELEMENTS],
}

impl PacketDisplayMatrix {
    pub const ELEMENTS: usize = 9;
    pub const DATA_LEN: usize = Self::ELEMENTS * 4;

    pub const fn new(elements: [i32; Self::ELEMENTS]) -> Self {
        Self { elements }
    }

    pub const fn identity() -> Self {
        Self {
            elements: [1 << 16, 0, 0, 0, 1 << 16, 0, 0, 0, 1 << 30],
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "display matrix packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut elements = [0; Self::ELEMENTS];
        for (element, chunk) in elements.iter_mut().zip(data.chunks_exact(4)) {
            let mut bytes = [0; 4];
            bytes.copy_from_slice(chunk);
            *element = i32::from_ne_bytes(bytes);
        }

        Ok(Self { elements })
    }

    pub const fn elements(self) -> [i32; Self::ELEMENTS] {
        self.elements
    }

    pub fn as_elements(&self) -> &[i32; Self::ELEMENTS] {
        &self.elements
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        for (element, chunk) in self.elements.iter().zip(bytes.chunks_exact_mut(4)) {
            chunk.copy_from_slice(&element.to_ne_bytes());
        }
        bytes
    }
}

pub type PacketStereo3dType = FrameStereo3dType;
pub type PacketStereo3dFlags = FrameStereo3dFlags;
pub type PacketStereo3dView = FrameStereo3dView;
pub type PacketStereo3dPrimaryEye = FrameStereo3dPrimaryEye;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketStereo3d(FrameStereo3d);

impl PacketStereo3d {
    pub const RATIONAL_LEN: usize = FrameStereo3d::RATIONAL_LEN;
    pub const TYPE_OFFSET: usize = FrameStereo3d::TYPE_OFFSET;
    pub const FLAGS_OFFSET: usize = FrameStereo3d::FLAGS_OFFSET;
    pub const VIEW_OFFSET: usize = FrameStereo3d::VIEW_OFFSET;
    pub const PRIMARY_EYE_OFFSET: usize = FrameStereo3d::PRIMARY_EYE_OFFSET;
    pub const BASELINE_OFFSET: usize = FrameStereo3d::BASELINE_OFFSET;
    pub const HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET: usize =
        FrameStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET;
    pub const HORIZONTAL_FIELD_OF_VIEW_OFFSET: usize =
        FrameStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET;
    pub const DATA_LEN: usize = FrameStereo3d::DATA_LEN;

    pub fn new(
        stereo_type: PacketStereo3dType,
        flags: PacketStereo3dFlags,
        view: PacketStereo3dView,
        primary_eye: PacketStereo3dPrimaryEye,
        baseline: u32,
        horizontal_disparity_adjustment: Rational,
        horizontal_field_of_view: Rational,
    ) -> AvResult<Self> {
        FrameStereo3d::new(
            stereo_type,
            flags,
            view,
            primary_eye,
            baseline,
            horizontal_disparity_adjustment,
            horizontal_field_of_view,
        )
        .map(Self)
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "stereo3d packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        FrameStereo3d::parse(data).map(Self)
    }

    pub const fn stereo_type(self) -> PacketStereo3dType {
        self.0.stereo_type()
    }

    pub const fn flags(self) -> PacketStereo3dFlags {
        self.0.flags()
    }

    pub const fn view(self) -> PacketStereo3dView {
        self.0.view()
    }

    pub const fn primary_eye(self) -> PacketStereo3dPrimaryEye {
        self.0.primary_eye()
    }

    pub const fn baseline(self) -> u32 {
        self.0.baseline()
    }

    pub const fn horizontal_disparity_adjustment(self) -> Rational {
        self.0.horizontal_disparity_adjustment()
    }

    pub const fn horizontal_field_of_view(self) -> Rational {
        self.0.horizontal_field_of_view()
    }

    pub const fn has_inverted_views(self) -> bool {
        self.0.has_inverted_views()
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        self.0.to_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PacketAudioServiceType {
    Main = 0,
    Effects = 1,
    VisuallyImpaired = 2,
    HearingImpaired = 3,
    Dialogue = 4,
    Commentary = 5,
    Emergency = 6,
    VoiceOver = 7,
    Karaoke = 8,
}

impl PacketAudioServiceType {
    pub const DATA_LEN: usize = 4;
    pub const KNOWN: [Self; 9] = [
        Self::Main,
        Self::Effects,
        Self::VisuallyImpaired,
        Self::HearingImpaired,
        Self::Dialogue,
        Self::Commentary,
        Self::Emergency,
        Self::VoiceOver,
        Self::Karaoke,
    ];

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "audio service type packet side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut raw = [0; Self::DATA_LEN];
        raw.copy_from_slice(data);
        Self::from_raw(i32::from_ne_bytes(raw))
    }

    pub fn from_raw(value: i32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::Main),
            1 => Ok(Self::Effects),
            2 => Ok(Self::VisuallyImpaired),
            3 => Ok(Self::HearingImpaired),
            4 => Ok(Self::Dialogue),
            5 => Ok(Self::Commentary),
            6 => Ok(Self::Emergency),
            7 => Ok(Self::VoiceOver),
            8 => Ok(Self::Karaoke),
            _ => Err(AvError::invalid_data(format!(
                "invalid audio service type packet side data value {value}"
            ))),
        }
    }

    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        self.as_raw().to_ne_bytes()
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::Main => "AV_AUDIO_SERVICE_TYPE_MAIN",
            Self::Effects => "AV_AUDIO_SERVICE_TYPE_EFFECTS",
            Self::VisuallyImpaired => "AV_AUDIO_SERVICE_TYPE_VISUALLY_IMPAIRED",
            Self::HearingImpaired => "AV_AUDIO_SERVICE_TYPE_HEARING_IMPAIRED",
            Self::Dialogue => "AV_AUDIO_SERVICE_TYPE_DIALOGUE",
            Self::Commentary => "AV_AUDIO_SERVICE_TYPE_COMMENTARY",
            Self::Emergency => "AV_AUDIO_SERVICE_TYPE_EMERGENCY",
            Self::VoiceOver => "AV_AUDIO_SERVICE_TYPE_VOICE_OVER",
            Self::Karaoke => "AV_AUDIO_SERVICE_TYPE_KARAOKE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketLcevc<'a> {
    data: &'a [u8],
}

impl<'a> PacketLcevc<'a> {
    pub const fn parse(data: &'a [u8]) -> AvResult<Self> {
        Ok(Self { data })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn len(self) -> usize {
        self.data.len()
    }

    pub const fn is_empty(self) -> bool {
        self.data.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideData {
    kind: PacketSideDataKind,
    data: Vec<u8>,
}

impl SideData {
    pub fn new(kind: impl Into<String>, data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::from_name(kind)?, data)
    }

    pub fn new_palette(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::Palette, data)?;
        PacketPalette::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_extradata(data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::NewExtradata, data)
    }

    pub fn new_quality_stats(value: PacketQualityStats) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::QualityStats, value.to_bytes())
    }

    pub fn new_fallback_track(value: PacketFallbackTrack) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::FallbackTrack, value.to_bytes().to_vec())
    }

    pub fn new_replay_gain(value: PacketReplayGain) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::ReplayGain, value.to_bytes().to_vec())
    }

    pub fn new_cpb_properties(value: PacketCpbProperties) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::CpbProperties, value.to_bytes().to_vec())
    }

    pub fn new_producer_reference_time(value: PacketProducerReferenceTime) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::ProducerReferenceTime,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_rtcp_sender_report(value: PacketRtcpSenderReport) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::RtcpSenderReport,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_mastering_display_metadata(value: PacketMasteringDisplayMetadata) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::MasteringDisplayMetadata,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_spherical_mapping(value: PacketSphericalMapping) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::Spherical, value.to_bytes().to_vec())
    }

    pub fn new_content_light_metadata(value: PacketContentLightMetadata) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::ContentLightLevel,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_ambient_viewing_environment(
        value: PacketAmbientViewingEnvironment,
    ) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::AmbientViewingEnvironment,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_three_d_reference_displays(value: PacketThreeDReferenceDisplays) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::ThreeDReferenceDisplays,
            value.to_bytes(),
        )
    }

    pub fn new_exif(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::Exif, data)?;
        PacketExif::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_a53_closed_captions(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::A53ClosedCaptions, data)?;
        PacketA53ClosedCaptions::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_encryption_init_info(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::EncryptionInitInfo, data)?;
        PacketEncryptionInitInfo::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_encryption_info(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::EncryptionInfo, data)?;
        PacketEncryptionInfo::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_icc_profile(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::IccProfile, data)?;
        PacketIccProfile::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_dolby_vision_conf(value: PacketDolbyVisionConf) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::DolbyVisionConf,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_dynamic_hdr10_plus(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::DynamicHdr10Plus, data)?;
        PacketDynamicHdr10Plus::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_iamf_mix_gain_param(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::IamfMixGainParam, data)?;
        PacketIamfMixGainParam::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_iamf_demixing_info_param(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::IamfDemixingInfoParam, data)?;
        PacketIamfDemixingInfoParam::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_iamf_recon_gain_info_param(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::IamfReconGainInfoParam, data)?;
        PacketIamfReconGainInfoParam::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_skip_samples(value: PacketSkipSamples) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::SkipSamples, value.to_bytes().to_vec())
    }

    pub fn new_param_change(value: PacketParamChange) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::ParamChange, value.to_bytes())
    }

    pub fn new_h263_mb_info(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::H263MbInfo, data)?;
        PacketH263MbInfo::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_jp_dualmono(value: PacketJpDualMono) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::JpDualMono, value.to_bytes().to_vec())
    }

    pub fn new_strings_metadata(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::StringsMetadata, data)?;
        PacketStringMetadata::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_metadata_update(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::MetadataUpdate, data)?;
        PacketStringMetadata::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_mpegts_stream_id(value: PacketMpegTsStreamId) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::MpegTsStreamId,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_subtitle_position(value: PacketSubtitlePosition) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::SubtitlePosition,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_matroska_block_additional(value: PacketMatroskaBlockAdditional) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::MatroskaBlockAdditional,
            value.to_bytes(),
        )
    }

    pub fn new_webvtt_identifier(value: PacketWebVttIdentifier) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::WebVttIdentifier, value.to_bytes())
    }

    pub fn new_webvtt_settings(value: PacketWebVttSettings) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::WebVttSettings, value.to_bytes())
    }

    pub fn new_active_format_description(value: PacketActiveFormatDescription) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::ActiveFormatDescription,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_s12m_timecode(value: PacketS12mTimecode) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::S12mTimecode, value.to_bytes().to_vec())
    }

    pub fn new_frame_cropping(value: PacketFrameCropping) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::FrameCropping, value.to_bytes().to_vec())
    }

    pub fn new_display_matrix(value: PacketDisplayMatrix) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::DisplayMatrix, value.to_bytes().to_vec())
    }

    pub fn new_stereo3d(value: PacketStereo3d) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::Stereo3d, value.to_bytes().to_vec())
    }

    pub fn new_audio_service_type(value: PacketAudioServiceType) -> AvResult<Self> {
        Self::new_with_kind(
            PacketSideDataKind::AudioServiceType,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_lcevc(data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::Lcevc, data)
    }

    pub fn new_with_kind(kind: PacketSideDataKind, data: Vec<u8>) -> AvResult<Self> {
        if let PacketSideDataKind::Unknown(name) = &kind {
            validate_packet_side_data_kind(name.clone())?;
        }
        Ok(Self { kind, data })
    }

    pub fn from_frame_side_data(side_data: &FrameSideData) -> AvResult<Self> {
        let kind = PacketSideDataKind::from_frame_side_data_kind(side_data.kind_id())
            .ok_or_else(packet_frame_side_data_map_error)?;
        Self::new_with_kind(kind, side_data.data().to_vec())
    }

    pub fn to_frame_side_data(&self) -> AvResult<FrameSideData> {
        let kind = self
            .kind
            .frame_side_data_kind()
            .ok_or_else(packet_frame_side_data_map_error)?;
        FrameSideData::new_with_kind(kind, self.data.clone())
    }

    pub fn add_to_frame<'a>(
        &self,
        frame: &'a mut Frame,
        flags: FrameSideDataFlags,
    ) -> AvResult<&'a mut FrameSideData> {
        let side_data = self.to_frame_side_data()?;
        frame.add_side_data_with_flags(side_data, flags)
    }

    pub fn kind(&self) -> &str {
        self.kind.name()
    }

    pub fn kind_id(&self) -> &PacketSideDataKind {
        &self.kind
    }

    pub fn is_known_kind(&self) -> bool {
        self.kind.is_known()
    }

    pub fn ffmpeg_constant(&self) -> Option<&'static str> {
        self.kind.ffmpeg_constant()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn palette(&self) -> AvResult<Option<PacketPalette<'_>>> {
        if self.kind != PacketSideDataKind::Palette {
            return Ok(None);
        }

        PacketPalette::parse(self.data()).map(Some)
    }

    pub fn extradata(&self) -> AvResult<Option<PacketNewExtradata<'_>>> {
        if self.kind != PacketSideDataKind::NewExtradata {
            return Ok(None);
        }

        PacketNewExtradata::parse(self.data()).map(Some)
    }

    pub fn quality_stats(&self) -> AvResult<Option<PacketQualityStats>> {
        if self.kind != PacketSideDataKind::QualityStats {
            return Ok(None);
        }

        PacketQualityStats::parse(self.data()).map(Some)
    }

    pub fn fallback_track(&self) -> AvResult<Option<PacketFallbackTrack>> {
        if self.kind != PacketSideDataKind::FallbackTrack {
            return Ok(None);
        }

        PacketFallbackTrack::parse(self.data()).map(Some)
    }

    pub fn replay_gain(&self) -> AvResult<Option<PacketReplayGain>> {
        if self.kind != PacketSideDataKind::ReplayGain {
            return Ok(None);
        }

        PacketReplayGain::parse(self.data()).map(Some)
    }

    pub fn cpb_properties(&self) -> AvResult<Option<PacketCpbProperties>> {
        if self.kind != PacketSideDataKind::CpbProperties {
            return Ok(None);
        }

        PacketCpbProperties::parse(self.data()).map(Some)
    }

    pub fn producer_reference_time(&self) -> AvResult<Option<PacketProducerReferenceTime>> {
        if self.kind != PacketSideDataKind::ProducerReferenceTime {
            return Ok(None);
        }

        PacketProducerReferenceTime::parse(self.data()).map(Some)
    }

    pub fn rtcp_sender_report(&self) -> AvResult<Option<PacketRtcpSenderReport>> {
        if self.kind != PacketSideDataKind::RtcpSenderReport {
            return Ok(None);
        }

        PacketRtcpSenderReport::parse(self.data()).map(Some)
    }

    pub fn mastering_display_metadata(&self) -> AvResult<Option<PacketMasteringDisplayMetadata>> {
        if self.kind != PacketSideDataKind::MasteringDisplayMetadata {
            return Ok(None);
        }

        PacketMasteringDisplayMetadata::parse(self.data()).map(Some)
    }

    pub fn spherical_mapping(&self) -> AvResult<Option<PacketSphericalMapping>> {
        if self.kind != PacketSideDataKind::Spherical {
            return Ok(None);
        }

        PacketSphericalMapping::parse(self.data()).map(Some)
    }

    pub fn content_light_metadata(&self) -> AvResult<Option<PacketContentLightMetadata>> {
        if self.kind != PacketSideDataKind::ContentLightLevel {
            return Ok(None);
        }

        PacketContentLightMetadata::parse(self.data()).map(Some)
    }

    pub fn ambient_viewing_environment(&self) -> AvResult<Option<PacketAmbientViewingEnvironment>> {
        if self.kind != PacketSideDataKind::AmbientViewingEnvironment {
            return Ok(None);
        }

        PacketAmbientViewingEnvironment::parse(self.data()).map(Some)
    }

    pub fn three_d_reference_displays(&self) -> AvResult<Option<PacketThreeDReferenceDisplays>> {
        if self.kind != PacketSideDataKind::ThreeDReferenceDisplays {
            return Ok(None);
        }

        PacketThreeDReferenceDisplays::parse(self.data()).map(Some)
    }

    pub fn exif(&self) -> AvResult<Option<PacketExif<'_>>> {
        if self.kind != PacketSideDataKind::Exif {
            return Ok(None);
        }

        PacketExif::parse(self.data()).map(Some)
    }

    pub fn a53_closed_captions(&self) -> AvResult<Option<PacketA53ClosedCaptions<'_>>> {
        if self.kind != PacketSideDataKind::A53ClosedCaptions {
            return Ok(None);
        }

        PacketA53ClosedCaptions::parse(self.data()).map(Some)
    }

    pub fn encryption_init_info(&self) -> AvResult<Option<PacketEncryptionInitInfo<'_>>> {
        if self.kind != PacketSideDataKind::EncryptionInitInfo {
            return Ok(None);
        }

        PacketEncryptionInitInfo::parse(self.data()).map(Some)
    }

    pub fn encryption_info(&self) -> AvResult<Option<PacketEncryptionInfo<'_>>> {
        if self.kind != PacketSideDataKind::EncryptionInfo {
            return Ok(None);
        }

        PacketEncryptionInfo::parse(self.data()).map(Some)
    }

    pub fn icc_profile(&self) -> AvResult<Option<PacketIccProfile<'_>>> {
        if self.kind != PacketSideDataKind::IccProfile {
            return Ok(None);
        }

        PacketIccProfile::parse(self.data()).map(Some)
    }

    pub fn dolby_vision_conf(&self) -> AvResult<Option<PacketDolbyVisionConf>> {
        if self.kind != PacketSideDataKind::DolbyVisionConf {
            return Ok(None);
        }

        PacketDolbyVisionConf::parse(self.data()).map(Some)
    }

    pub fn dynamic_hdr10_plus(&self) -> AvResult<Option<PacketDynamicHdr10Plus<'_>>> {
        if self.kind != PacketSideDataKind::DynamicHdr10Plus {
            return Ok(None);
        }

        PacketDynamicHdr10Plus::parse(self.data()).map(Some)
    }

    pub fn iamf_mix_gain_param(&self) -> AvResult<Option<PacketIamfMixGainParam<'_>>> {
        if self.kind != PacketSideDataKind::IamfMixGainParam {
            return Ok(None);
        }

        PacketIamfMixGainParam::parse(self.data()).map(Some)
    }

    pub fn iamf_demixing_info_param(&self) -> AvResult<Option<PacketIamfDemixingInfoParam<'_>>> {
        if self.kind != PacketSideDataKind::IamfDemixingInfoParam {
            return Ok(None);
        }

        PacketIamfDemixingInfoParam::parse(self.data()).map(Some)
    }

    pub fn iamf_recon_gain_info_param(&self) -> AvResult<Option<PacketIamfReconGainInfoParam<'_>>> {
        if self.kind != PacketSideDataKind::IamfReconGainInfoParam {
            return Ok(None);
        }

        PacketIamfReconGainInfoParam::parse(self.data()).map(Some)
    }

    pub fn skip_samples(&self) -> AvResult<Option<PacketSkipSamples>> {
        if self.kind != PacketSideDataKind::SkipSamples {
            return Ok(None);
        }

        PacketSkipSamples::parse(self.data()).map(Some)
    }

    pub fn param_change(&self) -> AvResult<Option<PacketParamChange>> {
        if self.kind != PacketSideDataKind::ParamChange {
            return Ok(None);
        }

        PacketParamChange::parse(self.data()).map(Some)
    }

    pub fn h263_mb_info(&self) -> AvResult<Option<PacketH263MbInfo<'_>>> {
        if self.kind != PacketSideDataKind::H263MbInfo {
            return Ok(None);
        }

        PacketH263MbInfo::parse(self.data()).map(Some)
    }

    pub fn jp_dualmono(&self) -> AvResult<Option<PacketJpDualMono>> {
        if self.kind != PacketSideDataKind::JpDualMono {
            return Ok(None);
        }

        PacketJpDualMono::parse(self.data()).map(Some)
    }

    pub fn strings_metadata(&self) -> AvResult<Option<PacketStringMetadata<'_>>> {
        if self.kind != PacketSideDataKind::StringsMetadata {
            return Ok(None);
        }

        PacketStringMetadata::parse(self.data()).map(Some)
    }

    pub fn metadata_update(&self) -> AvResult<Option<PacketStringMetadata<'_>>> {
        if self.kind != PacketSideDataKind::MetadataUpdate {
            return Ok(None);
        }

        PacketStringMetadata::parse(self.data()).map(Some)
    }

    pub fn mpegts_stream_id(&self) -> AvResult<Option<PacketMpegTsStreamId>> {
        if self.kind != PacketSideDataKind::MpegTsStreamId {
            return Ok(None);
        }

        PacketMpegTsStreamId::parse(self.data()).map(Some)
    }

    pub fn subtitle_position(&self) -> AvResult<Option<PacketSubtitlePosition>> {
        if self.kind != PacketSideDataKind::SubtitlePosition {
            return Ok(None);
        }

        PacketSubtitlePosition::parse(self.data()).map(Some)
    }

    pub fn matroska_block_additional(&self) -> AvResult<Option<PacketMatroskaBlockAdditional>> {
        if self.kind != PacketSideDataKind::MatroskaBlockAdditional {
            return Ok(None);
        }

        PacketMatroskaBlockAdditional::parse(self.data()).map(Some)
    }

    pub fn webvtt_identifier(&self) -> AvResult<Option<PacketWebVttIdentifier>> {
        if self.kind != PacketSideDataKind::WebVttIdentifier {
            return Ok(None);
        }

        PacketWebVttIdentifier::parse(self.data()).map(Some)
    }

    pub fn webvtt_settings(&self) -> AvResult<Option<PacketWebVttSettings>> {
        if self.kind != PacketSideDataKind::WebVttSettings {
            return Ok(None);
        }

        PacketWebVttSettings::parse(self.data()).map(Some)
    }

    pub fn active_format_description(&self) -> AvResult<Option<PacketActiveFormatDescription>> {
        if self.kind != PacketSideDataKind::ActiveFormatDescription {
            return Ok(None);
        }

        PacketActiveFormatDescription::parse(self.data()).map(Some)
    }

    pub fn s12m_timecode(&self) -> AvResult<Option<PacketS12mTimecode>> {
        if self.kind != PacketSideDataKind::S12mTimecode {
            return Ok(None);
        }

        PacketS12mTimecode::parse(self.data()).map(Some)
    }

    pub fn frame_cropping(&self) -> AvResult<Option<PacketFrameCropping>> {
        if self.kind != PacketSideDataKind::FrameCropping {
            return Ok(None);
        }

        PacketFrameCropping::parse(self.data()).map(Some)
    }

    pub fn display_matrix(&self) -> AvResult<Option<PacketDisplayMatrix>> {
        if self.kind != PacketSideDataKind::DisplayMatrix {
            return Ok(None);
        }

        PacketDisplayMatrix::parse(self.data()).map(Some)
    }

    pub fn stereo3d(&self) -> AvResult<Option<PacketStereo3d>> {
        if self.kind != PacketSideDataKind::Stereo3d {
            return Ok(None);
        }

        PacketStereo3d::parse(self.data()).map(Some)
    }

    pub fn audio_service_type(&self) -> AvResult<Option<PacketAudioServiceType>> {
        if self.kind != PacketSideDataKind::AudioServiceType {
            return Ok(None);
        }

        PacketAudioServiceType::parse(self.data()).map(Some)
    }

    pub fn lcevc(&self) -> AvResult<Option<PacketLcevc<'_>>> {
        if self.kind != PacketSideDataKind::Lcevc {
            return Ok(None);
        }

        PacketLcevc::parse(self.data()).map(Some)
    }

    pub fn shrink(&mut self, len: usize) -> AvResult<()> {
        if len > self.data.len() {
            return Err(AvError::with_code(
                AvErrorKind::External,
                AvErrorCode::ENOMEM,
                "packet side data cannot be shrunk to a larger size",
            ));
        }

        self.data.truncate(len);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PacketSideDataList {
    entries: Vec<SideData>,
}

impl PacketSideDataList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: Vec<SideData>) -> Self {
        Self { entries }
    }

    pub fn entries(&self) -> &[SideData] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, kind: &PacketSideDataKind) -> Option<&SideData> {
        self.entries
            .iter()
            .find(|side_data| side_data.kind_id() == kind)
    }

    pub fn get_mut(&mut self, kind: &PacketSideDataKind) -> Option<&mut SideData> {
        self.entries
            .iter_mut()
            .find(|side_data| side_data.kind_id() == kind)
    }

    pub fn new_side_data(
        &mut self,
        kind: PacketSideDataKind,
        size: usize,
    ) -> AvResult<&mut SideData> {
        let mut data = Vec::new();
        data.try_reserve_exact(size).map_err(|_| {
            AvError::with_code(
                AvErrorKind::External,
                AvErrorCode::ENOMEM,
                "cannot allocate packet side data",
            )
        })?;
        data.resize(size, 0);
        let side_data = SideData::new_with_kind(kind, data)?;
        Ok(self.add_or_replace(side_data).1)
    }

    pub fn add_side_data(&mut self, side_data: SideData) -> Option<SideData> {
        self.add_or_replace(side_data).0
    }

    pub fn add_from_frame_side_data(
        &mut self,
        side_data: &FrameSideData,
    ) -> AvResult<&mut SideData> {
        let side_data = SideData::from_frame_side_data(side_data)?;
        Ok(self.add_or_replace(side_data).1)
    }

    pub fn remove_kind(&mut self, kind: &PacketSideDataKind) -> Option<SideData> {
        let index = self
            .entries
            .iter()
            .rposition(|side_data| side_data.kind_id() == kind)?;
        Some(self.entries.swap_remove(index))
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn add_or_replace(&mut self, side_data: SideData) -> (Option<SideData>, &mut SideData) {
        if let Some(index) = self
            .entries
            .iter()
            .position(|existing| existing.kind_id() == side_data.kind_id())
        {
            let replaced = std::mem::replace(&mut self.entries[index], side_data);
            return (Some(replaced), &mut self.entries[index]);
        }

        self.entries.push(side_data);
        let index = self.entries.len() - 1;
        (None, &mut self.entries[index])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PacketOpaque {
    address: NonZeroUsize,
}

impl PacketOpaque {
    pub fn new(address: usize) -> AvResult<Self> {
        let Some(address) = NonZeroUsize::new(address) else {
            return Err(AvError::invalid_argument(
                "packet opaque pointer address must not be zero",
            ));
        };

        Ok(Self { address })
    }

    pub const fn from_nonzero(address: NonZeroUsize) -> Self {
        Self { address }
    }

    pub fn from_address(address: usize) -> Option<Self> {
        NonZeroUsize::new(address).map(Self::from_nonzero)
    }

    pub const fn address(self) -> usize {
        self.address.get()
    }

    pub const fn nonzero_address(self) -> NonZeroUsize {
        self.address
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    data: BufferRef,
    pts: i64,
    dts: i64,
    duration: i64,
    pos: i64,
    stream_index: usize,
    flags: PacketFlags,
    side_data: Vec<SideData>,
    opaque: Option<PacketOpaque>,
    opaque_ref: Option<BufferRef>,
    time_base: Rational,
}

impl Packet {
    pub fn new(data: Vec<u8>, stream_index: usize) -> Self {
        Self::with_buffer(BufferRef::from_vec(data), stream_index)
    }

    pub fn from_data(data: Vec<u8>) -> AvResult<Self> {
        let mut buffer = BufferRef::from_vec(data);
        buffer.resize_with_padding(buffer.len(), AV_INPUT_BUFFER_PADDING_SIZE)?;
        Ok(Self::with_buffer(buffer, 0))
    }

    pub fn new_zeroed(size: usize, stream_index: usize) -> AvResult<Self> {
        Ok(Self::with_buffer(
            BufferRef::zeroed_with_padding(size, AV_INPUT_BUFFER_PADDING_SIZE)?,
            stream_index,
        ))
    }

    pub fn with_buffer(data: BufferRef, stream_index: usize) -> Self {
        Self {
            data,
            pts: AV_NOPTS_VALUE,
            dts: AV_NOPTS_VALUE,
            duration: 0,
            pos: AV_PACKET_POS_UNKNOWN,
            stream_index,
            flags: PacketFlags::empty(),
            side_data: Vec::new(),
            opaque: None,
            opaque_ref: None,
            time_base: Rational::ZERO,
        }
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn data_buffer(&self) -> &BufferRef {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_data_writable(&self) -> bool {
        self.data.is_writable()
    }

    pub fn data_mut(&mut self) -> Option<&mut [u8]> {
        self.data.get_mut()
    }

    pub fn make_data_writable(&mut self) -> &mut [u8] {
        self.data.make_mut()
    }

    pub fn make_refcounted(&mut self) -> AvResult<()> {
        if self.has_input_padding() {
            return Ok(());
        }

        self.data
            .resize_with_padding(self.data.len(), AV_INPUT_BUFFER_PADDING_SIZE)
    }

    pub fn make_writable(&mut self) -> AvResult<()> {
        if self.data.is_writable() && self.has_input_padding() {
            return Ok(());
        }

        self.data
            .resize_with_padding(self.data.len(), AV_INPUT_BUFFER_PADDING_SIZE)
    }

    pub fn grow_data(&mut self, grow_by: usize) -> AvResult<()> {
        let len = self.data.len().checked_add(grow_by).ok_or_else(|| {
            AvError::invalid_argument("packet grow size overflows visible payload length")
        })?;
        self.data
            .resize_with_padding(len, AV_INPUT_BUFFER_PADDING_SIZE)
    }

    pub fn shrink_data(&mut self, size: usize) -> AvResult<()> {
        if size >= self.data.len() {
            return Ok(());
        }

        self.data
            .resize_with_padding(size, AV_INPUT_BUFFER_PADDING_SIZE)
    }

    pub fn pts(&self) -> Option<i64> {
        pts_option(self.pts)
    }

    pub fn dts(&self) -> Option<i64> {
        pts_option(self.dts)
    }

    pub fn duration(&self) -> i64 {
        self.duration
    }

    pub fn pos(&self) -> Option<i64> {
        pos_option(self.pos)
    }

    pub fn stream_index(&self) -> usize {
        self.stream_index
    }

    pub fn flags(&self) -> PacketFlags {
        self.flags
    }

    pub fn side_data(&self) -> &[SideData] {
        &self.side_data
    }

    pub fn opaque(&self) -> Option<PacketOpaque> {
        self.opaque
    }

    pub fn opaque_address(&self) -> Option<usize> {
        self.opaque.map(PacketOpaque::address)
    }

    pub fn opaque_ref(&self) -> Option<&BufferRef> {
        self.opaque_ref.as_ref()
    }

    pub fn time_base(&self) -> Rational {
        self.time_base
    }

    pub fn side_data_by_kind(&self, kind: &str) -> Option<&SideData> {
        let Ok(kind) = PacketSideDataKind::from_name(kind) else {
            return None;
        };
        self.side_data_by_kind_id(&kind)
    }

    pub fn side_data_by_kind_id(&self, kind: &PacketSideDataKind) -> Option<&SideData> {
        self.side_data
            .iter()
            .find(|side_data| side_data.kind_id() == kind)
    }

    pub fn side_data_mut_by_kind(&mut self, kind: &str) -> Option<&mut SideData> {
        let Ok(kind) = PacketSideDataKind::from_name(kind) else {
            return None;
        };
        self.side_data_mut_by_kind_id(&kind)
    }

    pub fn side_data_mut_by_kind_id(&mut self, kind: &PacketSideDataKind) -> Option<&mut SideData> {
        self.side_data
            .iter_mut()
            .find(|side_data| side_data.kind_id() == kind)
    }

    pub fn set_pts(&mut self, pts: Option<i64>) {
        self.pts = pts.unwrap_or(AV_NOPTS_VALUE);
    }

    pub fn set_dts(&mut self, dts: Option<i64>) {
        self.dts = dts.unwrap_or(AV_NOPTS_VALUE);
    }

    pub fn set_duration(&mut self, duration: i64) -> AvResult<()> {
        if duration < 0 {
            return Err(AvError::invalid_argument(
                "packet duration must not be negative",
            ));
        }

        self.duration = duration;
        Ok(())
    }

    pub fn set_pos(&mut self, pos: Option<i64>) -> AvResult<()> {
        if let Some(pos) = pos {
            if pos < 0 {
                return Err(AvError::invalid_argument(
                    "packet byte position must not be negative",
                ));
            }
        }

        self.pos = pos.unwrap_or(AV_PACKET_POS_UNKNOWN);
        Ok(())
    }

    pub fn set_flag(&mut self, flag: PacketFlags, enabled: bool) {
        self.flags.set(flag, enabled);
    }

    pub fn set_key(&mut self, is_key: bool) {
        self.set_flag(PacketFlags::KEY, is_key);
    }

    pub fn set_time_base(&mut self, time_base: Rational) -> AvResult<()> {
        self.time_base = Rational::new(time_base.num(), time_base.den())?;
        Ok(())
    }

    pub fn push_side_data(&mut self, side_data: SideData) {
        self.side_data.push(side_data);
    }

    pub fn new_side_data(
        &mut self,
        kind: PacketSideDataKind,
        size: usize,
    ) -> AvResult<&mut SideData> {
        self.ensure_side_data_capacity_for(&kind)?;
        let mut data = Vec::new();
        data.try_reserve_exact(size).map_err(|_| {
            AvError::with_code(
                AvErrorKind::External,
                AvErrorCode::ENOMEM,
                "cannot allocate packet side data",
            )
        })?;
        data.resize(size, 0);
        let side_data = SideData::new_with_kind(kind, data)?;
        Ok(self.add_or_replace_side_data(side_data).1)
    }

    pub fn try_add_side_data(&mut self, side_data: SideData) -> AvResult<Option<SideData>> {
        self.ensure_side_data_capacity_for(side_data.kind_id())?;
        Ok(self.add_or_replace_side_data(side_data).0)
    }

    pub fn add_side_data(&mut self, side_data: SideData) -> Option<SideData> {
        self.add_or_replace_side_data(side_data).0
    }

    fn add_or_replace_side_data(
        &mut self,
        side_data: SideData,
    ) -> (Option<SideData>, &mut SideData) {
        if let Some(index) = self
            .side_data
            .iter()
            .position(|existing| existing.kind_id() == side_data.kind_id())
        {
            let replaced = std::mem::replace(&mut self.side_data[index], side_data);
            return (Some(replaced), &mut self.side_data[index]);
        }

        self.side_data.push(side_data);
        let index = self.side_data.len() - 1;
        (None, &mut self.side_data[index])
    }

    fn ensure_side_data_capacity_for(&self, kind: &PacketSideDataKind) -> AvResult<()> {
        if self
            .side_data
            .iter()
            .any(|existing| existing.kind_id() == kind)
        {
            return Ok(());
        }

        if self.side_data.len() >= PacketSideDataKind::MAX_FFMPEG_PACKET_SIDE_DATA_ELEMS {
            return Err(packet_side_data_capacity_error());
        }

        Ok(())
    }

    pub fn shrink_side_data(&mut self, kind: &str, len: usize) -> AvResult<bool> {
        let Ok(kind) = PacketSideDataKind::from_name(kind) else {
            return Ok(false);
        };

        match self.shrink_side_data_by_kind_id(&kind, len) {
            Ok(()) => Ok(true),
            Err(err) if err.code() == Some(AvErrorCode::ENOENT) => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub fn shrink_side_data_by_kind_id(
        &mut self,
        kind: &PacketSideDataKind,
        len: usize,
    ) -> AvResult<()> {
        let Some(side_data) = self.side_data_mut_by_kind_id(kind) else {
            return Err(AvError::with_code(
                AvErrorKind::NotFound,
                AvErrorCode::ENOENT,
                "packet side data not found",
            ));
        };

        side_data.shrink(len)
    }

    pub fn take_side_data(&mut self, kind: &str) -> Option<SideData> {
        let Ok(kind) = PacketSideDataKind::from_name(kind) else {
            return None;
        };
        self.take_side_data_kind(&kind)
    }

    pub fn take_side_data_kind(&mut self, kind: &PacketSideDataKind) -> Option<SideData> {
        let index = self
            .side_data
            .iter()
            .position(|side_data| side_data.kind_id() == kind)?;
        Some(self.side_data.remove(index))
    }

    pub fn remove_side_data(&mut self, kind: &str) -> bool {
        self.take_side_data(kind).is_some()
    }

    pub fn clear_side_data(&mut self) {
        self.side_data.clear();
    }

    pub fn set_opaque(&mut self, opaque: Option<PacketOpaque>) {
        self.opaque = opaque;
    }

    pub fn set_opaque_address(&mut self, address: usize) {
        self.opaque = PacketOpaque::from_address(address);
    }

    pub fn take_opaque(&mut self) -> Option<PacketOpaque> {
        self.opaque.take()
    }

    pub fn clear_opaque(&mut self) {
        self.opaque = None;
    }

    pub fn set_opaque_ref(&mut self, opaque_ref: Option<BufferRef>) {
        self.opaque_ref = opaque_ref;
    }

    pub fn take_opaque_ref(&mut self) -> Option<BufferRef> {
        self.opaque_ref.take()
    }

    pub fn clear_opaque_ref(&mut self) {
        self.opaque_ref = None;
    }

    pub fn init_legacy(&mut self) {
        self.pts = AV_NOPTS_VALUE;
        self.dts = AV_NOPTS_VALUE;
        self.duration = 0;
        self.pos = AV_PACKET_POS_UNKNOWN;
        self.stream_index = 0;
        self.flags = PacketFlags::empty();
        self.side_data.clear();
        self.opaque = None;
        self.opaque_ref = None;
        self.time_base = Rational::ZERO;
    }

    pub fn unref(&mut self) {
        *self = Self::default();
    }

    pub fn ref_from(&mut self, src: &Self) {
        *self = src.clone();
    }

    pub fn move_ref_from(&mut self, src: &mut Self) {
        *self = std::mem::take(src);
    }

    pub fn copy_props_from(&mut self, src: &Self) {
        self.pts = src.pts;
        self.dts = src.dts;
        self.duration = src.duration;
        self.pos = src.pos;
        self.stream_index = src.stream_index;
        self.flags = src.flags;
        self.side_data = src.side_data.clone();
        self.opaque = src.opaque;
        self.opaque_ref = src.opaque_ref.clone();
        self.time_base = src.time_base;
    }

    pub fn rescale_ts(&mut self, src: Rational, dst: Rational) -> AvResult<()> {
        rescale_q(0, src, dst)?;

        let pts = if self.pts == AV_NOPTS_VALUE {
            AV_NOPTS_VALUE
        } else {
            rescale_q(self.pts, src, dst)?
        };
        let dts = if self.dts == AV_NOPTS_VALUE {
            AV_NOPTS_VALUE
        } else {
            rescale_q(self.dts, src, dst)?
        };
        let duration = if self.duration == 0 {
            0
        } else {
            rescale_q(self.duration, src, dst)?
        };

        self.pts = pts;
        self.dts = dts;
        self.duration = duration;
        Ok(())
    }

    fn has_input_padding(&self) -> bool {
        self.data.padding_len() >= AV_INPUT_BUFFER_PADDING_SIZE
            && self.data.padding_slice()[..AV_INPUT_BUFFER_PADDING_SIZE]
                .iter()
                .all(|byte| *byte == 0)
    }
}

impl Default for Packet {
    fn default() -> Self {
        Self::new(Vec::new(), 0)
    }
}

#[derive(Debug, Default)]
pub struct PacketFifo {
    entries: VecDeque<Packet>,
}

impl PacketFifo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn can_read(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn write_move(&mut self, packet: &mut Packet) -> AvResult<()> {
        let mut stored = Packet::default();
        stored.move_ref_from(packet);
        self.entries.push_back(stored);
        Ok(())
    }

    pub fn write_ref(&mut self, packet: &Packet) -> AvResult<()> {
        let mut stored = Packet::default();
        stored.ref_from(packet);
        self.entries.push_back(stored);
        Ok(())
    }

    pub fn read_move(&mut self, packet: &mut Packet) -> AvResult<()> {
        let mut stored = self.pop_front()?;
        packet.move_ref_from(&mut stored);
        Ok(())
    }

    pub fn read_ref(&mut self, packet: &mut Packet) -> AvResult<()> {
        let stored = self.pop_front()?;
        packet.ref_from(&stored);
        Ok(())
    }

    pub fn peek(&self, offset: usize) -> AvResult<&Packet> {
        self.entries.get(offset).ok_or_else(|| {
            AvError::with_code(
                AvErrorKind::InvalidArgument,
                AvErrorCode::EINVAL,
                "packet FIFO peek offset is out of range",
            )
        })
    }

    pub fn drain(&mut self, nb_elems: usize) -> AvResult<()> {
        if nb_elems > self.entries.len() {
            return Err(AvError::with_code(
                AvErrorKind::InvalidArgument,
                AvErrorCode::EINVAL,
                "packet FIFO drain count is out of range",
            ));
        }

        for _ in 0..nb_elems {
            self.entries.pop_front();
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn pop_front(&mut self) -> AvResult<Packet> {
        self.entries.pop_front().ok_or_else(|| {
            AvError::with_code(
                AvErrorKind::External,
                AvErrorCode::EAGAIN,
                "packet FIFO is empty",
            )
        })
    }
}

fn validate_packet_side_data_kind(kind: String) -> AvResult<String> {
    if kind.trim().is_empty() {
        return Err(AvError::invalid_argument(
            "packet side data kind must not be empty",
        ));
    }
    if kind.contains('\0') {
        return Err(AvError::invalid_argument(
            "packet side data kind must not contain NUL",
        ));
    }

    Ok(kind)
}

fn packet_frame_side_data_map_error() -> AvError {
    AvError::with_code(
        AvErrorKind::InvalidArgument,
        AvErrorCode::EINVAL,
        "packet and frame side data types do not have a matching mapped type",
    )
}

fn packet_side_data_capacity_error() -> AvError {
    AvError::with_code(
        AvErrorKind::InvalidArgument,
        AvErrorCode::from_posix_errno(34),
        "packet side data entry limit exceeded",
    )
}

fn normalize_packet_side_data_name(name: &str) -> String {
    name.trim()
        .chars()
        .filter_map(|ch| match ch {
            'A'..='Z' => Some(ch.to_ascii_lowercase()),
            'a'..='z' | '0'..='9' => Some(ch),
            '_' => Some('_'),
            '-' | ' ' | '\t' | '\r' | '\n' | '/' => Some('_'),
            _ => None,
        })
        .collect()
}

fn pts_option(value: i64) -> Option<i64> {
    if value == AV_NOPTS_VALUE {
        None
    } else {
        Some(value)
    }
}

fn pos_option(value: i64) -> Option<i64> {
    if value == AV_PACKET_POS_UNKNOWN {
        None
    } else {
        Some(value)
    }
}

fn find_nul(data: &[u8], offset: usize) -> Option<usize> {
    data.get(offset..)?
        .iter()
        .position(|byte| *byte == 0)
        .map(|position| offset + position)
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    u32::from_le_bytes(bytes)
}

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    let mut bytes = [0; 2];
    bytes.copy_from_slice(&data[offset..offset + 2]);
    u16::from_le_bytes(bytes)
}

fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

fn read_i32_le(data: &[u8], offset: usize) -> i32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    i32::from_le_bytes(bytes)
}

fn read_u32_ne(data: &[u8], offset: usize) -> u32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    u32::from_ne_bytes(bytes)
}

fn read_usize_ne(data: &[u8], offset: usize) -> AvResult<usize> {
    let end = offset
        .checked_add(core::mem::size_of::<usize>())
        .ok_or_else(|| AvError::invalid_data("native usize offset overflows usize"))?;
    let bytes = data
        .get(offset..end)
        .ok_or_else(|| AvError::invalid_data("native usize payload is truncated"))?;
    let mut raw = [0; core::mem::size_of::<usize>()];
    raw.copy_from_slice(bytes);
    Ok(usize::from_ne_bytes(raw))
}

fn read_u64_ne(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_ne_bytes(bytes)
}

fn read_i64_ne(data: &[u8], offset: usize) -> i64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    i64::from_ne_bytes(bytes)
}

fn read_i32_ne(data: &[u8], offset: usize) -> i32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    i32::from_ne_bytes(bytes)
}

fn read_rational_ne(data: &[u8], offset: usize) -> Rational {
    Rational::from_raw(read_i32_ne(data, offset), read_i32_ne(data, offset + 4))
}

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    u32::from_be_bytes(bytes)
}

fn usize_from_u32_be(data: &[u8], offset: usize, label: &str) -> AvResult<usize> {
    usize::try_from(read_u32_be(data, offset))
        .map_err(|_| AvError::invalid_data(format!("{label} does not fit in usize")))
}

fn read_u64_be(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_be_bytes(bytes)
}

fn read_fourcc(data: &[u8], offset: usize) -> [u8; 4] {
    let mut raw = [0; 4];
    raw.copy_from_slice(&data[offset..offset + 4]);
    raw
}

const fn align_native(size: usize) -> usize {
    let align = core::mem::align_of::<usize>();
    let remainder = size % align;
    if remainder == 0 {
        size
    } else {
        size + align - remainder
    }
}

fn read_dovi_flag(value: u8, field: &str) -> AvResult<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(AvError::invalid_data(format!(
            "Dolby Vision configuration {field} must be 0 or 1, got {value}"
        ))),
    }
}

fn validate_quality_stats_quality(
    quality: u32,
    error: impl FnOnce(String) -> AvError,
) -> AvResult<()> {
    if (1..=PacketQualityStats::FF_LAMBDA_MAX).contains(&quality) {
        Ok(())
    } else {
        Err(error(format!(
            "quality stats packet side data quality {quality} is outside 1..={}",
            PacketQualityStats::FF_LAMBDA_MAX
        )))
    }
}

fn validate_fallback_track_stream_index(
    stream_index: i32,
    error: impl FnOnce(String) -> AvError,
) -> AvResult<()> {
    if stream_index < 0 {
        return Err(error(format!(
            "fallback track packet side data stream index must be nonnegative, got {stream_index}"
        )));
    }

    Ok(())
}

fn validate_cpb_properties_nonnegative(
    value: i64,
    field: &str,
    error: impl FnOnce(String) -> AvError,
) -> AvResult<()> {
    if value < 0 {
        return Err(error(format!(
            "CPB properties packet side data {field} must be nonnegative, got {value}"
        )));
    }

    Ok(())
}

fn validate_webvtt_line_payload(data: &[u8], label: &str) -> AvResult<()> {
    if data.is_empty() {
        return Err(AvError::invalid_data(format!(
            "{label} packet side data must not be empty"
        )));
    }

    if data.iter().any(|byte| matches!(byte, 0 | b'\r' | b'\n')) {
        return Err(AvError::invalid_data(format!(
            "{label} packet side data must be a single NUL-free line"
        )));
    }

    Ok(())
}

fn validate_webvtt_identifier_payload(data: &[u8]) -> AvResult<()> {
    if data.windows(3).any(|window| window == b"-->") {
        return Err(AvError::invalid_data(
            "WebVTT identifier packet side data must not contain a timestamp separator",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{FrameExifEndian, FrameExifTiffType};
    use std::sync::{Arc, Mutex};

    fn minimal_icc_profile() -> Vec<u8> {
        let mut data = vec![0; PacketIccProfile::MIN_DATA_LEN];
        data[0..4].copy_from_slice(&(PacketIccProfile::MIN_DATA_LEN as u32).to_be_bytes());
        data[8..12].copy_from_slice(&0x0430_0000u32.to_be_bytes());
        data[12..16].copy_from_slice(b"mntr");
        data[16..20].copy_from_slice(b"RGB ");
        data[20..24].copy_from_slice(b"XYZ ");
        data[36..40].copy_from_slice(&PacketIccProfile::ICC_SIGNATURE);
        data[PacketIccProfile::TAG_COUNT_OFFSET..PacketIccProfile::TAG_COUNT_OFFSET + 4]
            .copy_from_slice(&0u32.to_be_bytes());
        data
    }

    fn minimal_packet_dynamic_hdr10_plus() -> Vec<u8> {
        let mut data = vec![0; PacketDynamicHdr10Plus::DATA_LEN];
        data[0] = PacketDynamicHdr10Plus::ITU_T_T35_COUNTRY_CODE;
        data[1] = PacketDynamicHdr10Plus::APPLICATION_VERSION;
        data[2] = 1;
        data
    }

    fn minimal_packet_palette() -> Vec<u8> {
        let mut data = vec![0; PacketPalette::DATA_LEN];
        data[0..4].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        let last = (PacketPalette::ENTRY_COUNT - 1) * PacketPalette::ENTRY_LEN;
        data[last..last + PacketPalette::ENTRY_LEN].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
        data
    }

    fn minimal_h263_mb_info() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        data.extend_from_slice(&[31, 2]);
        data.extend_from_slice(&0x0506u16.to_le_bytes());
        data.extend_from_slice(&[0x07, 0x08, 0x09, 0x0a]);
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&[0, u8::MAX]);
        data.extend_from_slice(&u16::MAX.to_le_bytes());
        data.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
        data
    }

    fn minimal_little_exif_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0x010Fu16.to_le_bytes());
        data.extend_from_slice(&FrameExifTiffType::Ascii.raw().to_le_bytes());
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&26u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"Rusty\0");
        data
    }

    fn minimal_big_exif_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x4D, 0x4D, 0x00, 0x2A]);
        data.extend_from_slice(&8u32.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data
    }

    fn write_packet_ambient_rational(
        bytes: &mut [u8; PacketAmbientViewingEnvironment::DATA_LEN],
        offset: usize,
        value: Rational,
    ) {
        bytes[offset..offset + 4].copy_from_slice(&value.num().to_ne_bytes());
        bytes[offset + 4..offset + 8].copy_from_slice(&value.den().to_ne_bytes());
    }

    fn write_packet_three_d_usize(data: &mut [u8], offset: usize, value: usize) {
        data[offset..offset + core::mem::size_of::<usize>()].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_packet_iamf_usize(data: &mut [u8], offset: usize, value: usize) {
        data[offset..offset + core::mem::size_of::<usize>()].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_packet_iamf_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_packet_iamf_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_packet_iamf_rational(data: &mut [u8], offset: usize, value: Rational) {
        write_packet_iamf_i32(data, offset, value.num());
        write_packet_iamf_i32(data, offset + 4, value.den());
    }

    fn packet_iamf_param_definition_payload(
        definition_type: PacketIamfParamDefinitionType,
        subblock_size: usize,
        subblocks: &[Vec<u8>],
    ) -> Vec<u8> {
        let subblocks_offset = PacketIamfParamDefinition::HEADER_LEN;
        let mut data = vec![0; subblocks_offset + subblock_size * subblocks.len()];
        write_packet_iamf_usize(
            &mut data,
            PacketIamfParamDefinition::AV_CLASS_OFFSET,
            0x1111,
        );
        write_packet_iamf_usize(
            &mut data,
            PacketIamfParamDefinition::SUBBLOCKS_OFFSET_OFFSET,
            subblocks_offset,
        );
        write_packet_iamf_usize(
            &mut data,
            PacketIamfParamDefinition::SUBBLOCK_SIZE_OFFSET,
            subblock_size,
        );
        write_packet_iamf_u32(
            &mut data,
            PacketIamfParamDefinition::SUBBLOCK_COUNT_OFFSET,
            subblocks.len() as u32,
        );
        write_packet_iamf_u32(
            &mut data,
            PacketIamfParamDefinition::TYPE_OFFSET,
            definition_type.as_raw(),
        );
        write_packet_iamf_u32(&mut data, PacketIamfParamDefinition::PARAMETER_ID_OFFSET, 7);
        write_packet_iamf_u32(
            &mut data,
            PacketIamfParamDefinition::PARAMETER_RATE_OFFSET,
            48_000,
        );
        write_packet_iamf_u32(&mut data, PacketIamfParamDefinition::DURATION_OFFSET, 960);
        write_packet_iamf_u32(
            &mut data,
            PacketIamfParamDefinition::CONSTANT_SUBBLOCK_DURATION_OFFSET,
            480,
        );

        for (index, subblock) in subblocks.iter().enumerate() {
            assert_eq!(subblock.len(), subblock_size);
            let offset = subblocks_offset + index * subblock_size;
            data[offset..offset + subblock_size].copy_from_slice(subblock);
        }

        data
    }

    fn packet_iamf_mix_gain_subblock(duration: u32, animation_type: u32) -> Vec<u8> {
        let mut data = vec![0; PacketIamfMixGainSubblock::MIN_DATA_LEN];
        write_packet_iamf_usize(
            &mut data,
            PacketIamfMixGainSubblock::AV_CLASS_OFFSET,
            0x2222,
        );
        write_packet_iamf_u32(
            &mut data,
            PacketIamfMixGainSubblock::SUBBLOCK_DURATION_OFFSET,
            duration,
        );
        write_packet_iamf_u32(
            &mut data,
            PacketIamfMixGainSubblock::ANIMATION_TYPE_OFFSET,
            animation_type,
        );
        write_packet_iamf_rational(
            &mut data,
            PacketIamfMixGainSubblock::START_POINT_VALUE_OFFSET,
            Rational::from_raw(-1, 2),
        );
        write_packet_iamf_rational(
            &mut data,
            PacketIamfMixGainSubblock::END_POINT_VALUE_OFFSET,
            Rational::from_raw(3, 4),
        );
        write_packet_iamf_rational(
            &mut data,
            PacketIamfMixGainSubblock::CONTROL_POINT_VALUE_OFFSET,
            Rational::from_raw(1, 3),
        );
        write_packet_iamf_rational(
            &mut data,
            PacketIamfMixGainSubblock::CONTROL_POINT_RELATIVE_TIME_OFFSET,
            Rational::from_raw(1, 2),
        );
        data
    }

    fn minimal_packet_iamf_mix_gain_param() -> Vec<u8> {
        let subblocks = [
            packet_iamf_mix_gain_subblock(480, PacketIamfAnimationType::Linear.as_raw()),
            packet_iamf_mix_gain_subblock(480, PacketIamfAnimationType::Bezier.as_raw()),
        ];
        let mut data = packet_iamf_param_definition_payload(
            PacketIamfParamDefinitionType::MixGain,
            PacketIamfMixGainSubblock::MIN_DATA_LEN,
            &subblocks,
        );
        data.push(0xaa);
        data
    }

    fn packet_iamf_demixing_info_subblock(duration: u32, dmixp_mode: u32) -> Vec<u8> {
        let mut data = vec![0; PacketIamfDemixingInfoSubblock::MIN_DATA_LEN];
        write_packet_iamf_usize(
            &mut data,
            PacketIamfDemixingInfoSubblock::AV_CLASS_OFFSET,
            0x3333,
        );
        write_packet_iamf_u32(
            &mut data,
            PacketIamfDemixingInfoSubblock::SUBBLOCK_DURATION_OFFSET,
            duration,
        );
        write_packet_iamf_u32(
            &mut data,
            PacketIamfDemixingInfoSubblock::DMIXP_MODE_OFFSET,
            dmixp_mode,
        );
        data
    }

    fn minimal_packet_iamf_demixing_info_param() -> Vec<u8> {
        packet_iamf_param_definition_payload(
            PacketIamfParamDefinitionType::Demixing,
            PacketIamfDemixingInfoSubblock::MIN_DATA_LEN,
            &[packet_iamf_demixing_info_subblock(960, 7)],
        )
    }

    fn packet_iamf_recon_gain_subblock(duration: u32) -> Vec<u8> {
        let mut data = vec![0; PacketIamfReconGainSubblock::MIN_DATA_LEN];
        write_packet_iamf_usize(
            &mut data,
            PacketIamfReconGainSubblock::AV_CLASS_OFFSET,
            0x4444,
        );
        write_packet_iamf_u32(
            &mut data,
            PacketIamfReconGainSubblock::SUBBLOCK_DURATION_OFFSET,
            duration,
        );
        for layer in 0..PacketIamfReconGainSubblock::LAYERS {
            for channel in 0..PacketIamfReconGainSubblock::CHANNELS {
                data[PacketIamfReconGainSubblock::RECON_GAIN_OFFSET
                    + layer * PacketIamfReconGainSubblock::CHANNELS
                    + channel] = (layer * 16 + channel) as u8;
            }
        }
        data
    }

    fn minimal_packet_iamf_recon_gain_info_param() -> Vec<u8> {
        packet_iamf_param_definition_payload(
            PacketIamfParamDefinitionType::ReconGain,
            PacketIamfReconGainSubblock::MIN_DATA_LEN,
            &[packet_iamf_recon_gain_subblock(960)],
        )
    }

    #[test]
    fn packet_defaults_to_no_timestamps() {
        let packet = Packet::new(vec![1, 2, 3], 7);

        assert_eq!(packet.data(), &[1, 2, 3]);
        assert_eq!(packet.stream_index(), 7);
        assert_eq!(packet.pts(), None);
        assert_eq!(packet.dts(), None);
        assert_eq!(packet.duration(), 0);
        assert_eq!(packet.pos(), None);
        assert_eq!(packet.len(), 3);
        assert!(!packet.is_empty());
        assert!(packet.opaque().is_none());
        assert_eq!(packet.opaque_address(), None);
        assert!(packet.opaque_ref().is_none());
        assert_eq!(packet.time_base(), Rational::ZERO);
    }

    #[test]
    fn packet_tracks_timestamps_position_flags_and_side_data() {
        let mut packet = Packet::new(vec![0xaa], 0);
        packet.set_pts(Some(12));
        packet.set_dts(Some(10));
        packet.set_duration(2).unwrap();
        packet.set_pos(Some(42)).unwrap();
        packet
            .set_time_base(Rational::new(1, 90_000).unwrap())
            .unwrap();
        packet.set_key(true);
        packet.set_flag(PacketFlags::CORRUPT, true);
        packet.set_flag(PacketFlags::DISCARD, true);
        packet.set_flag(PacketFlags::TRUSTED, true);
        packet.set_flag(PacketFlags::DISPOSABLE, true);
        packet.push_side_data(SideData::new("palette", vec![0, 1, 2]).unwrap());

        assert_eq!(packet.pts(), Some(12));
        assert_eq!(packet.dts(), Some(10));
        assert_eq!(packet.duration(), 2);
        assert_eq!(packet.pos(), Some(42));
        assert_eq!(packet.time_base(), Rational::new(1, 90_000).unwrap());
        assert!(packet.flags().contains(PacketFlags::KEY));
        assert!(packet.flags().contains(PacketFlags::CORRUPT));
        assert!(packet.flags().contains(PacketFlags::DISCARD));
        assert!(packet.flags().contains(PacketFlags::TRUSTED));
        assert!(packet.flags().contains(PacketFlags::DISPOSABLE));
        assert_eq!(packet.side_data()[0].kind(), "palette");
        assert_eq!(packet.side_data()[0].len(), 3);
        assert!(!packet.side_data()[0].is_empty());
    }

    #[test]
    fn packet_flag_values_match_ffmpeg_8_1_1_header() {
        assert_eq!(PacketFlags::KEY.bits(), 0x0001);
        assert_eq!(PacketFlags::CORRUPT.bits(), 0x0002);
        assert_eq!(PacketFlags::DISCARD.bits(), 0x0004);
        assert_eq!(PacketFlags::TRUSTED.bits(), 0x0008);
        assert_eq!(PacketFlags::DISPOSABLE.bits(), 0x0010);
        assert_eq!(PacketFlags::all().bits(), 0x001f);
    }

    #[test]
    fn packet_flags_can_be_cleared() {
        let mut flags = PacketFlags::empty();
        assert!(flags.is_empty());
        flags.insert(PacketFlags::KEY);
        flags.insert(PacketFlags::DISCARD);
        assert!(flags.contains(PacketFlags::KEY));
        assert!(flags.contains(PacketFlags::DISCARD));

        flags.remove(PacketFlags::KEY);
        assert!(!flags.contains(PacketFlags::KEY));
        assert!(flags.contains(PacketFlags::DISCARD));

        let mut packet = Packet::new(Vec::new(), 0);
        packet.set_key(true);
        assert!(packet.flags().contains(PacketFlags::KEY));
        packet.set_key(false);
        assert!(!packet.flags().contains(PacketFlags::KEY));

        packet.set_flag(PacketFlags::CORRUPT, true);
        assert!(packet.flags().contains(PacketFlags::CORRUPT));
        packet.set_flag(PacketFlags::CORRUPT, false);
        assert!(!packet.flags().contains(PacketFlags::CORRUPT));

        let truncated = PacketFlags::from_bits_truncate(0xffff_ffff);
        assert_eq!(truncated.bits(), PacketFlags::all().bits());
    }

    #[test]
    fn packet_rejects_negative_duration_position_and_invalid_side_data_kind() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.set_duration(5).unwrap();
        packet.set_pos(Some(9)).unwrap();
        packet
            .set_time_base(Rational::new(1, 1_000).unwrap())
            .unwrap();

        assert!(packet.set_duration(-1).is_err());
        assert_eq!(packet.duration(), 5);
        assert!(packet.set_pos(Some(-1)).is_err());
        assert_eq!(packet.pos(), Some(9));
        assert_eq!(
            packet
                .set_time_base(Rational::from_raw(1, 0))
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(packet.time_base(), Rational::new(1, 1_000).unwrap());
        packet.set_pos(None).unwrap();
        assert_eq!(packet.pos(), None);
        assert!(SideData::new(" ", Vec::new()).is_err());
        assert!(SideData::new("bad\0kind", Vec::new()).is_err());
    }

    #[test]
    fn packet_side_data_kind_inventory_matches_ffmpeg_8_1_1_header() {
        let constants: Vec<_> = PacketSideDataKind::KNOWN
            .iter()
            .map(|kind| kind.ffmpeg_constant().unwrap())
            .collect();

        assert_eq!(
            constants,
            [
                "AV_PKT_DATA_PALETTE",
                "AV_PKT_DATA_NEW_EXTRADATA",
                "AV_PKT_DATA_PARAM_CHANGE",
                "AV_PKT_DATA_H263_MB_INFO",
                "AV_PKT_DATA_REPLAYGAIN",
                "AV_PKT_DATA_DISPLAYMATRIX",
                "AV_PKT_DATA_STEREO3D",
                "AV_PKT_DATA_AUDIO_SERVICE_TYPE",
                "AV_PKT_DATA_QUALITY_STATS",
                "AV_PKT_DATA_FALLBACK_TRACK",
                "AV_PKT_DATA_CPB_PROPERTIES",
                "AV_PKT_DATA_SKIP_SAMPLES",
                "AV_PKT_DATA_JP_DUALMONO",
                "AV_PKT_DATA_STRINGS_METADATA",
                "AV_PKT_DATA_SUBTITLE_POSITION",
                "AV_PKT_DATA_MATROSKA_BLOCKADDITIONAL",
                "AV_PKT_DATA_WEBVTT_IDENTIFIER",
                "AV_PKT_DATA_WEBVTT_SETTINGS",
                "AV_PKT_DATA_METADATA_UPDATE",
                "AV_PKT_DATA_MPEGTS_STREAM_ID",
                "AV_PKT_DATA_MASTERING_DISPLAY_METADATA",
                "AV_PKT_DATA_SPHERICAL",
                "AV_PKT_DATA_CONTENT_LIGHT_LEVEL",
                "AV_PKT_DATA_A53_CC",
                "AV_PKT_DATA_ENCRYPTION_INIT_INFO",
                "AV_PKT_DATA_ENCRYPTION_INFO",
                "AV_PKT_DATA_AFD",
                "AV_PKT_DATA_PRFT",
                "AV_PKT_DATA_ICC_PROFILE",
                "AV_PKT_DATA_DOVI_CONF",
                "AV_PKT_DATA_S12M_TIMECODE",
                "AV_PKT_DATA_DYNAMIC_HDR10_PLUS",
                "AV_PKT_DATA_IAMF_MIX_GAIN_PARAM",
                "AV_PKT_DATA_IAMF_DEMIXING_INFO_PARAM",
                "AV_PKT_DATA_IAMF_RECON_GAIN_INFO_PARAM",
                "AV_PKT_DATA_AMBIENT_VIEWING_ENVIRONMENT",
                "AV_PKT_DATA_FRAME_CROPPING",
                "AV_PKT_DATA_LCEVC",
                "AV_PKT_DATA_3D_REFERENCE_DISPLAYS",
                "AV_PKT_DATA_RTCP_SR",
                "AV_PKT_DATA_EXIF",
            ]
        );
        assert_eq!(PacketSideDataKind::KNOWN[0].name(), "palette");
        assert_eq!(PacketSideDataKind::KNOWN[40].name(), "exif");
        assert_eq!(PacketSideDataKind::KNOWN.len(), 41);
        for (index, kind) in PacketSideDataKind::KNOWN.iter().enumerate() {
            assert_eq!(kind.ffmpeg_value(), Some(index as i32));
            assert_eq!(
                PacketSideDataKind::from_ffmpeg_value(index as i32),
                Some(kind.clone())
            );
            assert!(kind.ffmpeg_side_data_name().is_some());
            assert_eq!(
                PacketSideDataKind::ffmpeg_side_data_name_for_value(index as i32),
                kind.ffmpeg_side_data_name()
            );
        }
        assert_eq!(PacketSideDataKind::from_ffmpeg_value(-1), None);
        assert_eq!(PacketSideDataKind::from_ffmpeg_value(41), None);
        assert_eq!(
            PacketSideDataKind::ffmpeg_side_data_name_for_value(-1),
            None
        );
        assert_eq!(
            PacketSideDataKind::ffmpeg_side_data_name_for_value(41),
            None
        );
    }

    #[test]
    fn packet_side_data_kind_maps_aliases_and_preserves_unknown_names() {
        assert_eq!(
            PacketSideDataKind::from_name("AV_PKT_DATA_A53_CC").unwrap(),
            PacketSideDataKind::A53ClosedCaptions
        );
        assert_eq!(
            PacketSideDataKind::from_name("Dolby Vision Conf").unwrap(),
            PacketSideDataKind::DolbyVisionConf
        );
        assert_eq!(
            PacketSideDataKind::from_name("rtcp_sender_report").unwrap(),
            PacketSideDataKind::RtcpSenderReport
        );

        let unknown = PacketSideDataKind::from_name("vendor.packet").unwrap();
        assert_eq!(unknown.name(), "vendor.packet");
        assert!(!unknown.is_known());
        assert_eq!(unknown.ffmpeg_constant(), None);
        assert_eq!(unknown.ffmpeg_value(), None);
        assert_eq!(unknown.ffmpeg_side_data_name(), None);

        let side_data = SideData::new("AV_PKT_DATA_PALETTE", vec![1, 2]).unwrap();
        assert_eq!(side_data.kind(), "palette");
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::Palette);
        assert!(side_data.is_known_kind());
        assert_eq!(side_data.ffmpeg_constant(), Some("AV_PKT_DATA_PALETTE"));

        let mut packet = Packet::default();
        packet.push_side_data(side_data);
        packet.push_side_data(SideData::new("vendor_packet", vec![3]).unwrap());

        assert_eq!(
            packet
                .side_data_by_kind("AV_PKT_DATA_PALETTE")
                .unwrap()
                .data(),
            &[1, 2]
        );
        assert_eq!(
            packet
                .side_data_by_kind_id(&PacketSideDataKind::Palette)
                .unwrap()
                .data(),
            &[1, 2]
        );
        assert_eq!(
            packet.take_side_data("vendor_packet").unwrap().kind(),
            "vendor_packet"
        );
        assert!(packet.side_data_by_kind("vendor_packet").is_none());
    }

    #[test]
    fn packet_side_data_parses_palette_payload() {
        let data = minimal_packet_palette();
        let side_data = SideData::new_palette(data.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &PacketSideDataKind::Palette);
        assert_eq!(side_data.data(), data.as_slice());
        assert_eq!(
            PacketSideDataKind::Palette.ffmpeg_constant().unwrap(),
            "AV_PKT_DATA_PALETTE"
        );
        assert_eq!(PacketPalette::ENTRY_COUNT, 256);
        assert_eq!(PacketPalette::ENTRY_LEN, 4);
        assert_eq!(PacketPalette::DATA_LEN, 1024);

        let parsed = side_data.palette().unwrap().unwrap();
        assert_eq!(PacketPalette::parse(&data).unwrap(), parsed);
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.len(), PacketPalette::DATA_LEN);
        assert!(!parsed.is_empty());
        assert_eq!(parsed.entry_count(), PacketPalette::ENTRY_COUNT);
        assert_eq!(parsed.entry_bytes(0), Some([0x11, 0x22, 0x33, 0x44]));
        assert_eq!(
            parsed.entry_native(0),
            Some(u32::from_ne_bytes([0x11, 0x22, 0x33, 0x44]))
        );
        assert_eq!(
            parsed.entry_bytes(PacketPalette::ENTRY_COUNT - 1),
            Some([0xaa, 0xbb, 0xcc, 0xdd])
        );
        assert_eq!(parsed.entry_bytes(PacketPalette::ENTRY_COUNT), None);
        assert_eq!(parsed.entry_native(PacketPalette::ENTRY_COUNT), None);

        let non_palette = SideData::new_with_kind(PacketSideDataKind::QualityStats, data).unwrap();
        assert_eq!(non_palette.palette().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_palette_payload() {
        let valid = minimal_packet_palette();
        for data in [Vec::new(), vec![0; PacketPalette::DATA_LEN - 1], {
            let mut data = valid.clone();
            data.push(0);
            data
        }] {
            assert_eq!(
                PacketPalette::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_palette(data.clone()).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_with_kind(PacketSideDataKind::Palette, data)
                    .unwrap()
                    .palette()
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_preserves_new_extradata_payload() {
        let data = vec![0x01, 0x64, 0x00, 0x1f, 0xff, 0xe1, 0xaa, 0xbb];
        let side_data = SideData::new_extradata(data.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &PacketSideDataKind::NewExtradata);
        assert_eq!(side_data.data(), data.as_slice());
        assert_eq!(
            PacketSideDataKind::NewExtradata.ffmpeg_constant().unwrap(),
            "AV_PKT_DATA_NEW_EXTRADATA"
        );

        let parsed = side_data.extradata().unwrap().unwrap();
        assert_eq!(PacketNewExtradata::parse(&data).unwrap(), parsed);
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.len(), data.len());
        assert!(!parsed.is_empty());

        let empty = SideData::new_extradata(Vec::new()).unwrap();
        let parsed_empty = empty.extradata().unwrap().unwrap();
        assert!(parsed_empty.is_empty());
        assert_eq!(parsed_empty.len(), 0);

        let non_extradata = SideData::new_with_kind(PacketSideDataKind::Palette, data).unwrap();
        assert_eq!(non_extradata.extradata().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_h263_mb_info_payload() {
        let data = minimal_h263_mb_info();
        let side_data = SideData::new_h263_mb_info(data.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &PacketSideDataKind::H263MbInfo);
        assert_eq!(side_data.data(), data.as_slice());
        assert_eq!(
            PacketSideDataKind::H263MbInfo.ffmpeg_constant().unwrap(),
            "AV_PKT_DATA_H263_MB_INFO"
        );
        assert_eq!(PacketH263MbInfoEntry::DATA_LEN, 12);

        let parsed = side_data.h263_mb_info().unwrap().unwrap();
        assert_eq!(PacketH263MbInfo::parse(&data).unwrap(), parsed);
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.len(), data.len());
        assert!(!parsed.is_empty());
        assert_eq!(parsed.entry_count(), 2);
        assert_eq!(parsed.entries().len(), 2);

        let first = parsed.entry(0).unwrap();
        assert_eq!(first.bit_offset(), 0x0102_0304);
        assert_eq!(first.quantizer(), 31);
        assert_eq!(first.gob_number(), 2);
        assert_eq!(first.macroblock_address(), 0x0506);
        assert_eq!(first.horizontal_mv_predictor(), 0x07);
        assert_eq!(first.vertical_mv_predictor(), 0x08);
        assert_eq!(first.block3_horizontal_mv_predictor(), 0x09);
        assert_eq!(first.block3_vertical_mv_predictor(), 0x0a);

        let second = parsed.entry(1).unwrap();
        assert_eq!(second.bit_offset(), u32::MAX);
        assert_eq!(second.quantizer(), 0);
        assert_eq!(second.gob_number(), u8::MAX);
        assert_eq!(second.macroblock_address(), u16::MAX);
        assert_eq!(second.horizontal_mv_predictor(), 0xaa);
        assert_eq!(second.vertical_mv_predictor(), 0xbb);
        assert_eq!(second.block3_horizontal_mv_predictor(), 0xcc);
        assert_eq!(second.block3_vertical_mv_predictor(), 0xdd);
        assert_eq!(parsed.entry(2), None);

        let entries: Vec<_> = parsed.entries().collect();
        assert_eq!(entries, vec![first, second]);

        let empty = SideData::new_h263_mb_info(Vec::new()).unwrap();
        let parsed_empty = empty.h263_mb_info().unwrap().unwrap();
        assert!(parsed_empty.is_empty());
        assert_eq!(parsed_empty.entry_count(), 0);

        let non_h263 = SideData::new_with_kind(PacketSideDataKind::NewExtradata, data).unwrap();
        assert_eq!(non_h263.h263_mb_info().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_h263_mb_info_payload() {
        let valid = minimal_h263_mb_info();
        for data in [vec![0; PacketH263MbInfoEntry::DATA_LEN - 1], {
            let mut data = valid.clone();
            data.push(0);
            data
        }] {
            assert_eq!(
                PacketH263MbInfo::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_h263_mb_info(data.clone()).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_with_kind(PacketSideDataKind::H263MbInfo, data)
                    .unwrap()
                    .h263_mb_info()
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_parses_quality_stats_payload() {
        let expected = PacketQualityStats::new(
            118,
            PacketPictureType::I,
            vec![0x0102_0304_0506_0708, u64::MAX],
        )
        .unwrap();
        let expected_bytes = [
            0x76, 0x00, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
            0x02, 0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ];

        assert_eq!(expected.quality(), 118);
        assert_eq!(expected.picture_type(), PacketPictureType::I);
        assert_eq!(PacketPictureType::I.as_byte(), 1);
        assert_eq!(PacketPictureType::Bi.as_byte(), 7);
        assert_eq!(
            PacketPictureType::Bi.ffmpeg_constant(),
            "AV_PICTURE_TYPE_BI"
        );
        assert_eq!(
            PacketPictureType::from_byte(PacketPictureType::Sp.as_byte()).unwrap(),
            PacketPictureType::Sp
        );
        assert_eq!(
            PacketPictureType::Unknown.ffmpeg_constant(),
            "AV_PICTURE_TYPE_NONE"
        );
        assert_eq!(expected.errors(), &[0x0102_0304_0506_0708, u64::MAX]);
        assert!(expected.trailing_data().is_empty());
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketQualityStats::parse(&expected_bytes).unwrap(),
            expected
        );

        let empty_errors = PacketQualityStats::new(
            PacketQualityStats::FF_LAMBDA_MAX,
            PacketPictureType::Unknown,
            Vec::new(),
        )
        .unwrap();
        assert_eq!(
            empty_errors.to_bytes(),
            [0xff, 0x7f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            PacketQualityStats::parse(&empty_errors.to_bytes()).unwrap(),
            empty_errors
        );

        let mut with_trailing = expected_bytes.to_vec();
        with_trailing.extend_from_slice(&[0xaa, 0xbb, 0xcc]);
        let parsed_with_trailing = PacketQualityStats::parse(&with_trailing).unwrap();
        assert_eq!(parsed_with_trailing.errors(), expected.errors());
        assert_eq!(parsed_with_trailing.trailing_data(), &[0xaa, 0xbb, 0xcc]);
        assert_eq!(parsed_with_trailing.to_bytes(), with_trailing);

        let side_data = SideData::new_quality_stats(expected.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::QualityStats);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.quality_stats().unwrap(), Some(expected));

        let palette =
            SideData::new_with_kind(PacketSideDataKind::Palette, expected_bytes.to_vec()).unwrap();
        assert_eq!(palette.quality_stats().unwrap(), None);
    }

    #[test]
    fn packet_picture_type_values_and_chars_match_ffmpeg_8_1_1_header() {
        let inventory = [
            (PacketPictureType::Unknown, 0, "AV_PICTURE_TYPE_NONE", '?'),
            (PacketPictureType::I, 1, "AV_PICTURE_TYPE_I", 'I'),
            (PacketPictureType::P, 2, "AV_PICTURE_TYPE_P", 'P'),
            (PacketPictureType::B, 3, "AV_PICTURE_TYPE_B", 'B'),
            (PacketPictureType::S, 4, "AV_PICTURE_TYPE_S", 'S'),
            (PacketPictureType::Si, 5, "AV_PICTURE_TYPE_SI", 'i'),
            (PacketPictureType::Sp, 6, "AV_PICTURE_TYPE_SP", 'p'),
            (PacketPictureType::Bi, 7, "AV_PICTURE_TYPE_BI", 'b'),
        ];
        for (picture_type, value, constant, ffmpeg_char) in inventory {
            assert_eq!(picture_type.as_byte(), value);
            assert_eq!(picture_type.ffmpeg_constant(), constant);
            assert_eq!(picture_type.ffmpeg_char(), ffmpeg_char);
            assert_eq!(PacketPictureType::from_byte(value).unwrap(), picture_type);
        }
    }

    #[test]
    fn packet_side_data_rejects_malformed_quality_stats_payload() {
        assert_eq!(
            PacketQualityStats::new(0, PacketPictureType::I, Vec::new())
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PacketQualityStats::new(
                PacketQualityStats::FF_LAMBDA_MAX + 1,
                PacketPictureType::I,
                Vec::new()
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PacketQualityStats::new(1, PacketPictureType::I, vec![0_u64; 256])
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );

        for data in [Vec::new(), vec![0; PacketQualityStats::HEADER_LEN - 1]] {
            assert_eq!(
                PacketQualityStats::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let mut truncated_errors = vec![1, 0, 0, 0, 1, 2, 0, 0];
        truncated_errors.extend_from_slice(&[0; 15]);
        for data in [
            vec![0, 0, 0, 0, 1, 0, 0, 0],
            vec![0, 0x80, 0, 0, 1, 0, 0, 0],
            vec![1, 0, 0, 0, 8, 0, 0, 0],
            vec![1, 0, 0, 0, 1, 0, 1, 0],
            truncated_errors,
        ] {
            assert_eq!(
                PacketQualityStats::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            PacketPictureType::from_byte(8).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let side_data = SideData::new_with_kind(
            PacketSideDataKind::QualityStats,
            vec![1, 0, 0, 0, 1, 1, 0, 0],
        )
        .unwrap();
        assert_eq!(
            side_data.quality_stats().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_fallback_track_payload() {
        for stream_index in [0, 1, i32::MAX] {
            let expected = PacketFallbackTrack::new(stream_index).unwrap();
            assert_eq!(expected.stream_index(), stream_index);
            assert_eq!(expected.to_bytes(), stream_index.to_ne_bytes());
            assert_eq!(
                PacketFallbackTrack::parse(&expected.to_bytes()).unwrap(),
                expected
            );

            let side_data = SideData::new_fallback_track(expected).unwrap();
            assert_eq!(side_data.kind_id(), &PacketSideDataKind::FallbackTrack);
            assert_eq!(side_data.data(), &stream_index.to_ne_bytes());
            assert_eq!(side_data.fallback_track().unwrap(), Some(expected));
        }

        let quality_stats = SideData::new_with_kind(
            PacketSideDataKind::QualityStats,
            PacketFallbackTrack::new(2).unwrap().to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(quality_stats.fallback_track().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_fallback_track_payload() {
        assert_eq!(
            PacketFallbackTrack::new(-1).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );

        for data in [
            Vec::new(),
            vec![0; PacketFallbackTrack::DATA_LEN - 1],
            vec![0; 5],
        ] {
            assert_eq!(
                PacketFallbackTrack::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data = SideData::new_with_kind(
            PacketSideDataKind::FallbackTrack,
            (-1_i32).to_ne_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(
            side_data.fallback_track().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_replay_gain_payload() {
        let expected = PacketReplayGain::new(-250, 0x1020_3040, 125, 0x5060_7080);
        let mut expected_bytes = [0; PacketReplayGain::DATA_LEN];
        expected_bytes[0..4].copy_from_slice(&(-250_i32).to_ne_bytes());
        expected_bytes[4..8].copy_from_slice(&0x1020_3040_u32.to_ne_bytes());
        expected_bytes[8..12].copy_from_slice(&125_i32.to_ne_bytes());
        expected_bytes[12..16].copy_from_slice(&0x5060_7080_u32.to_ne_bytes());

        assert_eq!(PacketReplayGain::DATA_LEN, 16);
        assert_eq!(expected.track_gain(), -250);
        assert_eq!(expected.track_peak(), 0x1020_3040);
        assert_eq!(expected.album_gain(), 125);
        assert_eq!(expected.album_peak(), 0x5060_7080);
        assert!(!expected.track_gain_unknown());
        assert!(!expected.album_gain_unknown());
        assert!(!expected.track_peak_unknown());
        assert!(!expected.album_peak_unknown());
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(PacketReplayGain::parse(&expected_bytes).unwrap(), expected);

        let unknown = PacketReplayGain::new(
            PacketReplayGain::GAIN_UNKNOWN,
            PacketReplayGain::PEAK_UNKNOWN,
            PacketReplayGain::GAIN_UNKNOWN,
            PacketReplayGain::PEAK_UNKNOWN,
        );
        assert!(unknown.track_gain_unknown());
        assert!(unknown.album_gain_unknown());
        assert!(unknown.track_peak_unknown());
        assert!(unknown.album_peak_unknown());
        assert_eq!(
            PacketReplayGain::parse(&unknown.to_bytes()).unwrap(),
            unknown
        );

        let boundaries = PacketReplayGain::new(i32::MIN, u32::MAX, i32::MAX, 1);
        assert_eq!(
            PacketReplayGain::parse(&boundaries.to_bytes()).unwrap(),
            boundaries
        );

        let side_data = SideData::new_replay_gain(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::ReplayGain);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.replay_gain().unwrap(), Some(expected));

        let fallback_track =
            SideData::new_with_kind(PacketSideDataKind::FallbackTrack, expected_bytes.to_vec())
                .unwrap();
        assert_eq!(fallback_track.replay_gain().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_replay_gain_payload() {
        for data in [
            Vec::new(),
            vec![0; PacketReplayGain::DATA_LEN - 1],
            vec![0; PacketReplayGain::DATA_LEN + 1],
        ] {
            assert_eq!(
                PacketReplayGain::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data = SideData::new_with_kind(PacketSideDataKind::ReplayGain, data).unwrap();
            assert_eq!(
                side_data.replay_gain().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let audio_service =
            SideData::new_with_kind(PacketSideDataKind::AudioServiceType, vec![0; 16]).unwrap();
        assert_eq!(audio_service.replay_gain().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_cpb_properties_payload() {
        let expected =
            PacketCpbProperties::new(9_000_000, 1_000_000, 4_000_000, 2_000_000, u64::MAX).unwrap();
        let mut expected_bytes = [0; PacketCpbProperties::DATA_LEN];
        expected_bytes[..8].copy_from_slice(&9_000_000_i64.to_ne_bytes());
        expected_bytes[8..16].copy_from_slice(&1_000_000_i64.to_ne_bytes());
        expected_bytes[16..24].copy_from_slice(&4_000_000_i64.to_ne_bytes());
        expected_bytes[24..32].copy_from_slice(&2_000_000_i64.to_ne_bytes());
        expected_bytes[32..].copy_from_slice(&u64::MAX.to_ne_bytes());

        assert_eq!(expected.max_bitrate(), 9_000_000);
        assert_eq!(expected.min_bitrate(), 1_000_000);
        assert_eq!(expected.avg_bitrate(), 4_000_000);
        assert_eq!(expected.buffer_size(), 2_000_000);
        assert_eq!(expected.vbv_delay(), PacketCpbProperties::VBV_DELAY_UNKNOWN);
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketCpbProperties::parse(&expected_bytes).unwrap(),
            expected
        );

        let boundaries = PacketCpbProperties::new(i64::MAX, 0, i64::MAX - 1, 1, 0).unwrap();
        assert_eq!(boundaries.max_bitrate(), i64::MAX);
        assert_eq!(boundaries.min_bitrate(), 0);
        assert_eq!(boundaries.avg_bitrate(), i64::MAX - 1);
        assert_eq!(boundaries.buffer_size(), 1);
        assert_eq!(boundaries.vbv_delay(), 0);
        assert_eq!(
            PacketCpbProperties::parse(&boundaries.to_bytes()).unwrap(),
            boundaries
        );

        let side_data = SideData::new_cpb_properties(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::CpbProperties);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.cpb_properties().unwrap(), Some(expected));

        let quality_stats =
            SideData::new_with_kind(PacketSideDataKind::QualityStats, expected_bytes.to_vec())
                .unwrap();
        assert_eq!(quality_stats.cpb_properties().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_cpb_properties_payload() {
        for (max_bitrate, min_bitrate, avg_bitrate, buffer_size) in
            [(-1, 0, 0, 0), (0, -1, 0, 0), (0, 0, -1, 0), (0, 0, 0, -1)]
        {
            assert_eq!(
                PacketCpbProperties::new(
                    max_bitrate,
                    min_bitrate,
                    avg_bitrate,
                    buffer_size,
                    PacketCpbProperties::VBV_DELAY_UNKNOWN,
                )
                .unwrap_err()
                .kind(),
                crate::AvErrorKind::InvalidArgument
            );
        }

        for data in [
            Vec::new(),
            vec![0; PacketCpbProperties::DATA_LEN - 1],
            vec![0; PacketCpbProperties::DATA_LEN + 1],
        ] {
            assert_eq!(
                PacketCpbProperties::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        for offset in [0, 8, 16, 24] {
            let mut data =
                PacketCpbProperties::new(1, 2, 3, 4, PacketCpbProperties::VBV_DELAY_UNKNOWN)
                    .unwrap()
                    .to_bytes();
            data[offset..offset + 8].copy_from_slice(&(-1_i64).to_ne_bytes());
            assert_eq!(
                PacketCpbProperties::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data = SideData::new_with_kind(
            PacketSideDataKind::CpbProperties,
            vec![0; PacketCpbProperties::DATA_LEN - 1],
        )
        .unwrap();
        assert_eq!(
            side_data.cpb_properties().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_producer_reference_time_payload() {
        let expected = PacketProducerReferenceTime::new(1_701_234_567_890_123, 24);
        let mut expected_bytes = [0; PacketProducerReferenceTime::DATA_LEN];
        expected_bytes[..8].copy_from_slice(&1_701_234_567_890_123_i64.to_ne_bytes());
        expected_bytes[8..12].copy_from_slice(&24_i32.to_ne_bytes());

        assert_eq!(expected.wallclock(), 1_701_234_567_890_123);
        assert_eq!(expected.flags(), 24);
        assert_eq!(
            expected.padding(),
            [0; PacketProducerReferenceTime::PADDING_LEN]
        );
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketProducerReferenceTime::parse(&expected_bytes).unwrap(),
            expected
        );

        let mut padded_bytes = [0; PacketProducerReferenceTime::DATA_LEN];
        padded_bytes[..8].copy_from_slice(&i64::MIN.to_ne_bytes());
        padded_bytes[8..12].copy_from_slice(&i32::MIN.to_ne_bytes());
        padded_bytes[12..].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
        let padded = PacketProducerReferenceTime::parse(&padded_bytes).unwrap();
        assert_eq!(padded.wallclock(), i64::MIN);
        assert_eq!(padded.flags(), i32::MIN);
        assert_eq!(padded.padding(), [0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(padded.to_bytes(), padded_bytes);

        let side_data = SideData::new_producer_reference_time(expected).unwrap();
        assert_eq!(
            side_data.kind_id(),
            &PacketSideDataKind::ProducerReferenceTime
        );
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.producer_reference_time().unwrap(), Some(expected));

        let quality_stats =
            SideData::new_with_kind(PacketSideDataKind::QualityStats, expected_bytes.to_vec())
                .unwrap();
        assert_eq!(quality_stats.producer_reference_time().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_producer_reference_time_payload() {
        for data in [
            Vec::new(),
            vec![0; PacketProducerReferenceTime::DATA_LEN - 1],
            vec![0; PacketProducerReferenceTime::DATA_LEN + 1],
        ] {
            assert_eq!(
                PacketProducerReferenceTime::parse(&data)
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data = SideData::new_with_kind(
            PacketSideDataKind::ProducerReferenceTime,
            vec![0; PacketProducerReferenceTime::DATA_LEN - 1],
        )
        .unwrap();
        assert_eq!(
            side_data.producer_reference_time().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_rtcp_sender_report_payload() {
        let expected = PacketRtcpSenderReport::new(
            0x0102_0304,
            0x0506_0708_090a_0b0c,
            0x0d0e_0f10,
            0x1112_1314,
            0x1516_1718,
        );
        let mut expected_bytes = [0; PacketRtcpSenderReport::DATA_LEN];
        expected_bytes[..4].copy_from_slice(&0x0102_0304_u32.to_ne_bytes());
        expected_bytes[8..16].copy_from_slice(&0x0506_0708_090a_0b0c_u64.to_ne_bytes());
        expected_bytes[16..20].copy_from_slice(&0x0d0e_0f10_u32.to_ne_bytes());
        expected_bytes[20..24].copy_from_slice(&0x1112_1314_u32.to_ne_bytes());
        expected_bytes[24..28].copy_from_slice(&0x1516_1718_u32.to_ne_bytes());

        assert_eq!(expected.ssrc(), 0x0102_0304);
        assert_eq!(expected.ntp_timestamp(), 0x0506_0708_090a_0b0c);
        assert_eq!(expected.rtp_timestamp(), 0x0d0e_0f10);
        assert_eq!(expected.sender_packet_count(), 0x1112_1314);
        assert_eq!(expected.sender_octet_count(), 0x1516_1718);
        assert_eq!(
            expected.alignment_padding(),
            [0; PacketRtcpSenderReport::ALIGNMENT_PADDING_LEN]
        );
        assert_eq!(
            expected.tail_padding(),
            [0; PacketRtcpSenderReport::TAIL_PADDING_LEN]
        );
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketRtcpSenderReport::parse(&expected_bytes).unwrap(),
            expected
        );

        let mut padded_bytes = [0; PacketRtcpSenderReport::DATA_LEN];
        padded_bytes[..4].copy_from_slice(&u32::MAX.to_ne_bytes());
        padded_bytes[4..8].copy_from_slice(&[0x10, 0x11, 0x12, 0x13]);
        padded_bytes[8..16].copy_from_slice(&u64::MAX.to_ne_bytes());
        padded_bytes[16..20].copy_from_slice(&0x8000_0000_u32.to_ne_bytes());
        padded_bytes[20..24].copy_from_slice(&0x7fff_ffff_u32.to_ne_bytes());
        padded_bytes[24..28].copy_from_slice(&0x1234_5678_u32.to_ne_bytes());
        padded_bytes[28..].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
        let padded = PacketRtcpSenderReport::parse(&padded_bytes).unwrap();
        assert_eq!(padded.ssrc(), u32::MAX);
        assert_eq!(padded.ntp_timestamp(), u64::MAX);
        assert_eq!(padded.rtp_timestamp(), 0x8000_0000);
        assert_eq!(padded.sender_packet_count(), 0x7fff_ffff);
        assert_eq!(padded.sender_octet_count(), 0x1234_5678);
        assert_eq!(padded.alignment_padding(), [0x10, 0x11, 0x12, 0x13]);
        assert_eq!(padded.tail_padding(), [0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(padded.to_bytes(), padded_bytes);

        let side_data = SideData::new_rtcp_sender_report(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::RtcpSenderReport);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.rtcp_sender_report().unwrap(), Some(expected));

        let prft = SideData::new_with_kind(
            PacketSideDataKind::ProducerReferenceTime,
            expected_bytes.to_vec(),
        )
        .unwrap();
        assert_eq!(prft.rtcp_sender_report().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_rtcp_sender_report_payload() {
        for data in [
            Vec::new(),
            vec![0; PacketRtcpSenderReport::DATA_LEN - 1],
            vec![0; PacketRtcpSenderReport::DATA_LEN + 1],
        ] {
            assert_eq!(
                PacketRtcpSenderReport::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data = SideData::new_with_kind(
            PacketSideDataKind::RtcpSenderReport,
            vec![0; PacketRtcpSenderReport::DATA_LEN - 1],
        )
        .unwrap();
        assert_eq!(
            side_data.rtcp_sender_report().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_mastering_display_metadata_payload() {
        fn write_rational(
            bytes: &mut [u8; PacketMasteringDisplayMetadata::DATA_LEN],
            offset: &mut usize,
            value: Rational,
        ) {
            bytes[*offset..*offset + 4].copy_from_slice(&value.num().to_ne_bytes());
            *offset += 4;
            bytes[*offset..*offset + 4].copy_from_slice(&value.den().to_ne_bytes());
            *offset += 4;
        }

        let display_primaries = [
            [
                Rational::from_raw(34_000, 50_000),
                Rational::from_raw(16_000, 50_000),
            ],
            [
                Rational::from_raw(13_250, 50_000),
                Rational::from_raw(34_500, 50_000),
            ],
            [
                Rational::from_raw(7_500, 50_000),
                Rational::from_raw(3_000, 50_000),
            ],
        ];
        let white_point = [
            Rational::from_raw(15_635, 50_000),
            Rational::from_raw(16_450, 50_000),
        ];
        let expected = PacketMasteringDisplayMetadata::new(
            display_primaries,
            white_point,
            Rational::from_raw(50, 10_000),
            Rational::from_raw(1000, 1),
            1,
            2,
        );
        let mut expected_bytes = [0; PacketMasteringDisplayMetadata::DATA_LEN];
        let mut offset = 0;
        for primary in display_primaries {
            for coordinate in primary {
                write_rational(&mut expected_bytes, &mut offset, coordinate);
            }
        }
        for coordinate in white_point {
            write_rational(&mut expected_bytes, &mut offset, coordinate);
        }
        write_rational(&mut expected_bytes, &mut offset, expected.min_luminance());
        write_rational(&mut expected_bytes, &mut offset, expected.max_luminance());
        expected_bytes[offset..offset + 4].copy_from_slice(&1_i32.to_ne_bytes());
        offset += 4;
        expected_bytes[offset..offset + 4].copy_from_slice(&2_i32.to_ne_bytes());

        assert_eq!(PacketMasteringDisplayMetadata::DATA_LEN, 88);
        assert_eq!(PacketMasteringDisplayMetadata::PRIMARIES, 3);
        assert_eq!(PacketMasteringDisplayMetadata::COORDINATES, 2);
        assert_eq!(expected.display_primaries(), display_primaries);
        assert_eq!(expected.white_point(), white_point);
        assert_eq!(expected.min_luminance(), Rational::from_raw(50, 10_000));
        assert_eq!(expected.max_luminance(), Rational::from_raw(1000, 1));
        assert!(expected.has_primaries());
        assert!(expected.has_luminance());
        assert_eq!(expected.has_primaries_raw(), 1);
        assert_eq!(expected.has_luminance_raw(), 2);
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketMasteringDisplayMetadata::parse(&expected_bytes).unwrap(),
            expected
        );

        let raw_values = PacketMasteringDisplayMetadata::new(
            [[Rational::from_raw(2, 4); PacketMasteringDisplayMetadata::COORDINATES];
                PacketMasteringDisplayMetadata::PRIMARIES],
            [Rational::from_raw(0, 0); PacketMasteringDisplayMetadata::COORDINATES],
            Rational::from_raw(0, 0),
            Rational::from_raw(9, 3),
            0,
            -3,
        );
        let roundtrip = PacketMasteringDisplayMetadata::parse(&raw_values.to_bytes()).unwrap();
        assert_eq!(
            roundtrip.display_primaries()[0][0],
            Rational::from_raw(2, 4)
        );
        assert_eq!(roundtrip.white_point()[0], Rational::from_raw(0, 0));
        assert!(!roundtrip.has_primaries());
        assert!(roundtrip.has_luminance());
        assert_eq!(roundtrip.has_luminance_raw(), -3);

        let side_data = SideData::new_mastering_display_metadata(expected).unwrap();
        assert_eq!(
            side_data.kind_id(),
            &PacketSideDataKind::MasteringDisplayMetadata
        );
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(
            side_data.mastering_display_metadata().unwrap(),
            Some(expected)
        );

        let rtcp = SideData::new_with_kind(
            PacketSideDataKind::RtcpSenderReport,
            expected_bytes.to_vec(),
        )
        .unwrap();
        assert_eq!(rtcp.mastering_display_metadata().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_mastering_display_metadata_payload() {
        for data in [
            Vec::new(),
            vec![0; PacketMasteringDisplayMetadata::DATA_LEN - 1],
            vec![0; PacketMasteringDisplayMetadata::DATA_LEN + 1],
        ] {
            assert_eq!(
                PacketMasteringDisplayMetadata::parse(&data)
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::MasteringDisplayMetadata, data)
                    .unwrap();
            assert_eq!(
                side_data.mastering_display_metadata().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let content_light =
            SideData::new_with_kind(PacketSideDataKind::ContentLightLevel, Vec::new()).unwrap();
        assert_eq!(content_light.mastering_display_metadata().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_spherical_mapping_payload() {
        let expected = PacketSphericalMapping::new(
            PacketSphericalProjection::Cubemap,
            90 << 16,
            -15 << 16,
            180 << 16,
            [1, 2, 3, 4],
            12,
        );
        let mut expected_bytes = [0; PacketSphericalMapping::DATA_LEN];
        expected_bytes[0..4].copy_from_slice(&1i32.to_ne_bytes());
        expected_bytes[4..8].copy_from_slice(&(90i32 << 16).to_ne_bytes());
        expected_bytes[8..12].copy_from_slice(&(-15i32 << 16).to_ne_bytes());
        expected_bytes[12..16].copy_from_slice(&(180i32 << 16).to_ne_bytes());
        expected_bytes[16..20].copy_from_slice(&1u32.to_ne_bytes());
        expected_bytes[20..24].copy_from_slice(&2u32.to_ne_bytes());
        expected_bytes[24..28].copy_from_slice(&3u32.to_ne_bytes());
        expected_bytes[28..32].copy_from_slice(&4u32.to_ne_bytes());
        expected_bytes[32..36].copy_from_slice(&12u32.to_ne_bytes());

        assert_eq!(PacketSphericalMapping::DATA_LEN, 36);
        assert_eq!(PacketSphericalMapping::BOUNDS, 4);
        assert_eq!(expected.projection(), PacketSphericalProjection::Cubemap);
        assert_eq!(
            expected.projection().ffmpeg_constant(),
            "AV_SPHERICAL_CUBEMAP"
        );
        assert_eq!(expected.projection().as_raw(), 1);
        assert_eq!(expected.yaw(), 90 << 16);
        assert_eq!(expected.pitch(), -15 << 16);
        assert_eq!(expected.roll(), 180 << 16);
        assert_eq!(expected.bounds(), [1, 2, 3, 4]);
        assert_eq!(expected.bound_left(), 1);
        assert_eq!(expected.bound_top(), 2);
        assert_eq!(expected.bound_right(), 3);
        assert_eq!(expected.bound_bottom(), 4);
        assert_eq!(expected.padding(), 12);
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketSphericalMapping::parse(&expected_bytes).unwrap(),
            expected
        );

        let projections = [
            (
                PacketSphericalProjection::Equirectangular,
                0,
                "AV_SPHERICAL_EQUIRECTANGULAR",
            ),
            (
                PacketSphericalProjection::Cubemap,
                1,
                "AV_SPHERICAL_CUBEMAP",
            ),
            (
                PacketSphericalProjection::EquirectangularTile,
                2,
                "AV_SPHERICAL_EQUIRECTANGULAR_TILE",
            ),
            (
                PacketSphericalProjection::HalfEquirectangular,
                3,
                "AV_SPHERICAL_HALF_EQUIRECTANGULAR",
            ),
            (
                PacketSphericalProjection::Rectilinear,
                4,
                "AV_SPHERICAL_RECTILINEAR",
            ),
            (
                PacketSphericalProjection::Fisheye,
                5,
                "AV_SPHERICAL_FISHEYE",
            ),
            (
                PacketSphericalProjection::ParametricImmersive,
                6,
                "AV_SPHERICAL_PARAMETRIC_IMMERSIVE",
            ),
        ];
        assert_eq!(
            PacketSphericalProjection::KNOWN,
            projections.map(|(projection, _, _)| projection)
        );
        for (projection, raw, ffmpeg_constant) in projections {
            assert_eq!(
                PacketSphericalProjection::from_raw(raw).unwrap(),
                projection
            );
            assert_eq!(projection.as_raw(), raw);
            assert_eq!(projection.ffmpeg_constant(), ffmpeg_constant);
        }

        let raw_bounds = PacketSphericalMapping::new(
            PacketSphericalProjection::EquirectangularTile,
            i32::MIN,
            0,
            i32::MAX,
            [u32::MAX, 0, 0x8000_0000, 42],
            u32::MAX,
        );
        assert_eq!(
            PacketSphericalMapping::parse(&raw_bounds.to_bytes()).unwrap(),
            raw_bounds
        );

        let side_data = SideData::new_spherical_mapping(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::Spherical);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.spherical_mapping().unwrap(), Some(expected));

        let content_light = SideData::new_with_kind(
            PacketSideDataKind::ContentLightLevel,
            expected_bytes.to_vec(),
        )
        .unwrap();
        assert_eq!(content_light.spherical_mapping().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_spherical_mapping_payload() {
        for data in [
            Vec::new(),
            vec![0; PacketSphericalMapping::DATA_LEN - 1],
            vec![0; PacketSphericalMapping::DATA_LEN + 1],
        ] {
            assert_eq!(
                PacketSphericalMapping::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data = SideData::new_with_kind(PacketSideDataKind::Spherical, data).unwrap();
            assert_eq!(
                side_data.spherical_mapping().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        for raw in [-1, 7, i32::MAX] {
            assert_eq!(
                PacketSphericalProjection::from_raw(raw).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let mut data = [0; PacketSphericalMapping::DATA_LEN];
            data[0..4].copy_from_slice(&raw.to_ne_bytes());
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::Spherical, data.to_vec()).unwrap();
            assert_eq!(
                side_data.spherical_mapping().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let content_light =
            SideData::new_with_kind(PacketSideDataKind::ContentLightLevel, vec![0; 36]).unwrap();
        assert_eq!(content_light.spherical_mapping().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_content_light_metadata_payload() {
        let expected = PacketContentLightMetadata::new(1000, 400);
        let mut expected_bytes = [0; PacketContentLightMetadata::DATA_LEN];
        expected_bytes[0..4].copy_from_slice(&1000u32.to_ne_bytes());
        expected_bytes[4..8].copy_from_slice(&400u32.to_ne_bytes());

        assert_eq!(PacketContentLightMetadata::DATA_LEN, 8);
        assert_eq!(expected.max_content_light_level(), 1000);
        assert_eq!(expected.max_average_light_level(), 400);
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketContentLightMetadata::parse(&expected_bytes).unwrap(),
            expected
        );

        let raw_values = PacketContentLightMetadata::new(u32::MAX, 0);
        assert_eq!(
            PacketContentLightMetadata::parse(&raw_values.to_bytes()).unwrap(),
            raw_values
        );

        let side_data = SideData::new_content_light_metadata(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::ContentLightLevel);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.content_light_metadata().unwrap(), Some(expected));

        let rtcp = SideData::new_with_kind(
            PacketSideDataKind::RtcpSenderReport,
            expected_bytes.to_vec(),
        )
        .unwrap();
        assert_eq!(rtcp.content_light_metadata().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_content_light_metadata_payload() {
        for data in [
            Vec::new(),
            vec![0; PacketContentLightMetadata::DATA_LEN - 1],
            vec![0; PacketContentLightMetadata::DATA_LEN + 1],
        ] {
            assert_eq!(
                PacketContentLightMetadata::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data = SideData::new_with_kind(
            PacketSideDataKind::ContentLightLevel,
            vec![0; PacketContentLightMetadata::DATA_LEN - 1],
        )
        .unwrap();
        assert_eq!(
            side_data.content_light_metadata().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_ambient_viewing_environment_payload() {
        let expected = PacketAmbientViewingEnvironment::new(
            Rational::from_raw(203, 10),
            Rational::from_raw(15_635, 50_000),
            Rational::from_raw(16_450, 50_000),
        )
        .unwrap();
        let mut expected_bytes = [0; PacketAmbientViewingEnvironment::DATA_LEN];
        write_packet_ambient_rational(
            &mut expected_bytes,
            PacketAmbientViewingEnvironment::AMBIENT_ILLUMINANCE_OFFSET,
            Rational::from_raw(203, 10),
        );
        write_packet_ambient_rational(
            &mut expected_bytes,
            PacketAmbientViewingEnvironment::AMBIENT_LIGHT_X_OFFSET,
            Rational::from_raw(15_635, 50_000),
        );
        write_packet_ambient_rational(
            &mut expected_bytes,
            PacketAmbientViewingEnvironment::AMBIENT_LIGHT_Y_OFFSET,
            Rational::from_raw(16_450, 50_000),
        );

        assert_eq!(PacketAmbientViewingEnvironment::RATIONAL_LEN, 8);
        assert_eq!(PacketAmbientViewingEnvironment::DATA_LEN, 24);
        assert_eq!(
            PacketAmbientViewingEnvironment::AMBIENT_ILLUMINANCE_OFFSET,
            0
        );
        assert_eq!(PacketAmbientViewingEnvironment::AMBIENT_LIGHT_X_OFFSET, 8);
        assert_eq!(PacketAmbientViewingEnvironment::AMBIENT_LIGHT_Y_OFFSET, 16);
        assert_eq!(
            PacketSideDataKind::AmbientViewingEnvironment
                .ffmpeg_constant()
                .unwrap(),
            "AV_PKT_DATA_AMBIENT_VIEWING_ENVIRONMENT"
        );
        assert_eq!(expected.ambient_illuminance(), Rational::from_raw(203, 10));
        assert_eq!(
            expected.ambient_light_x(),
            Rational::from_raw(15_635, 50_000)
        );
        assert_eq!(
            expected.ambient_light_y(),
            Rational::from_raw(16_450, 50_000)
        );
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketAmbientViewingEnvironment::parse(&expected_bytes).unwrap(),
            expected
        );

        let default_value = PacketAmbientViewingEnvironment::new(
            Rational::from_raw(0, 1),
            Rational::from_raw(0, 1),
            Rational::from_raw(0, 1),
        )
        .unwrap();
        assert_eq!(
            PacketAmbientViewingEnvironment::parse(&default_value.to_bytes()).unwrap(),
            default_value
        );

        let side_data = SideData::new_ambient_viewing_environment(expected).unwrap();
        assert_eq!(
            side_data.kind_id(),
            &PacketSideDataKind::AmbientViewingEnvironment
        );
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(
            side_data.ambient_viewing_environment().unwrap(),
            Some(expected)
        );

        let content_light = SideData::new_with_kind(
            PacketSideDataKind::ContentLightLevel,
            expected_bytes.to_vec(),
        )
        .unwrap();
        assert_eq!(content_light.ambient_viewing_environment().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_ambient_viewing_environment_payload() {
        let valid = PacketAmbientViewingEnvironment::new(
            Rational::from_raw(203, 10),
            Rational::from_raw(15_635, 50_000),
            Rational::from_raw(16_450, 50_000),
        )
        .unwrap()
        .to_bytes();

        let mut invalid_payloads = Vec::new();
        invalid_payloads.push(Vec::new());
        invalid_payloads.push(valid[..PacketAmbientViewingEnvironment::DATA_LEN - 1].to_vec());
        let mut long = valid.to_vec();
        long.push(0);
        invalid_payloads.push(long);

        for (offset, bad_value) in [
            (
                PacketAmbientViewingEnvironment::AMBIENT_ILLUMINANCE_OFFSET,
                Rational::from_raw(-1, 1),
            ),
            (
                PacketAmbientViewingEnvironment::AMBIENT_ILLUMINANCE_OFFSET,
                Rational::from_raw(1, 0),
            ),
            (
                PacketAmbientViewingEnvironment::AMBIENT_LIGHT_X_OFFSET,
                Rational::from_raw(2, 1),
            ),
            (
                PacketAmbientViewingEnvironment::AMBIENT_LIGHT_Y_OFFSET,
                Rational::from_raw(-1, 1),
            ),
            (
                PacketAmbientViewingEnvironment::AMBIENT_LIGHT_Y_OFFSET,
                Rational::from_raw(0, 0),
            ),
        ] {
            let mut invalid = valid;
            write_packet_ambient_rational(&mut invalid, offset, bad_value);
            invalid_payloads.push(invalid.to_vec());
        }

        for data in invalid_payloads {
            assert_eq!(
                PacketAmbientViewingEnvironment::parse(&data)
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::AmbientViewingEnvironment, data)
                    .unwrap();
            assert_eq!(
                side_data.ambient_viewing_environment().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            PacketAmbientViewingEnvironment::new(
                Rational::from_raw(1, 1),
                Rational::from_raw(3, 2),
                Rational::from_raw(0, 1),
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_three_d_reference_displays_payload() {
        let first = PacketThreeDReferenceDisplay::new(0, 1, (12, 34), (5, 67), true, -11);
        let second = PacketThreeDReferenceDisplay::new(2, 3, (10, 20), (4, 40), false, 0);
        let value = PacketThreeDReferenceDisplays::new(31, true, 7, vec![first, second]).unwrap();
        let expected_bytes = value.to_bytes();
        let side_data = SideData::new_three_d_reference_displays(value.clone()).unwrap();

        assert_eq!(PacketThreeDReferenceDisplay::DATA_LEN, 12);
        assert_eq!(PacketThreeDReferenceDisplays::ENTRY_DATA_LEN, 12);
        assert_eq!(
            PacketThreeDReferenceDisplays::HEADER_LEN,
            if core::mem::size_of::<usize>() == 8 {
                24
            } else {
                12
            }
        );
        assert_eq!(
            PacketThreeDReferenceDisplays::ENTRIES_OFFSET,
            PacketThreeDReferenceDisplays::HEADER_LEN
        );
        assert_eq!(
            PacketSideDataKind::ThreeDReferenceDisplays
                .ffmpeg_constant()
                .unwrap(),
            "AV_PKT_DATA_3D_REFERENCE_DISPLAYS"
        );

        assert_eq!(value.prec_ref_display_width(), 31);
        assert!(value.ref_viewing_distance_flag());
        assert_eq!(value.prec_ref_viewing_dist(), 7);
        assert_eq!(value.nb_displays(), 2);
        assert_eq!(value.displays(), &[first, second]);
        assert_eq!(value.display(0), Some(first));
        assert_eq!(value.display(1), Some(second));
        assert_eq!(value.display(2), None);
        assert_eq!(first.left_view_id(), 0);
        assert_eq!(first.right_view_id(), 1);
        assert_eq!(first.exponent_ref_display_width(), 12);
        assert_eq!(first.mantissa_ref_display_width(), 34);
        assert_eq!(first.exponent_ref_viewing_distance(), 5);
        assert_eq!(first.mantissa_ref_viewing_distance(), 67);
        assert!(first.additional_shift_present());
        assert_eq!(first.num_sample_shift(), -11);
        assert_eq!(
            PacketThreeDReferenceDisplay::parse(&first.to_bytes()).unwrap(),
            first
        );
        assert_eq!(
            PacketThreeDReferenceDisplays::parse(&expected_bytes).unwrap(),
            value
        );

        assert_eq!(
            side_data.kind_id(),
            &PacketSideDataKind::ThreeDReferenceDisplays
        );
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.three_d_reference_displays().unwrap(), Some(value));

        let non_tdrdi = SideData::new_with_kind(
            PacketSideDataKind::AmbientViewingEnvironment,
            expected_bytes,
        )
        .unwrap();
        assert_eq!(non_tdrdi.three_d_reference_displays().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_three_d_reference_displays_payload() {
        let display = PacketThreeDReferenceDisplay::new(0, 1, (12, 34), (5, 67), true, -11);
        let value = PacketThreeDReferenceDisplays::new(31, true, 7, vec![display]).unwrap();
        let valid = value.to_bytes();

        assert_eq!(
            PacketThreeDReferenceDisplay::parse(&[0; PacketThreeDReferenceDisplay::DATA_LEN - 1])
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidData
        );
        assert_eq!(
            PacketThreeDReferenceDisplays::parse(
                &[0; PacketThreeDReferenceDisplays::HEADER_LEN - 1]
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidData
        );
        assert_eq!(
            PacketThreeDReferenceDisplays::new(31, true, 7, Vec::new())
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidData
        );
        assert_eq!(
            PacketThreeDReferenceDisplays::new(
                31,
                true,
                7,
                vec![display; PacketThreeDReferenceDisplays::MAX_REF_DISPLAYS + 1],
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidData
        );

        let mut invalid_payloads = Vec::new();
        invalid_payloads.push(valid[..valid.len() - 1].to_vec());
        let mut long = valid.clone();
        long.push(0);
        invalid_payloads.push(long);

        invalid_payloads.push({
            let mut bad = valid.clone();
            bad[0] = 32;
            bad
        });
        invalid_payloads.push({
            let mut bad = valid.clone();
            bad[1] = 2;
            bad
        });
        invalid_payloads.push({
            let mut bad = valid.clone();
            bad[2] = 32;
            bad
        });
        invalid_payloads.push({
            let mut bad = valid.clone();
            bad[3] = 0;
            bad
        });
        invalid_payloads.push({
            let mut bad = valid.clone();
            bad[3] = (PacketThreeDReferenceDisplays::MAX_REF_DISPLAYS + 1) as u8;
            bad
        });
        invalid_payloads.push({
            let mut bad = valid.clone();
            write_packet_three_d_usize(
                &mut bad,
                PacketThreeDReferenceDisplays::ENTRIES_OFFSET_OFFSET,
                PacketThreeDReferenceDisplays::ENTRIES_OFFSET - 2,
            );
            bad
        });
        invalid_payloads.push({
            let mut bad = valid.clone();
            write_packet_three_d_usize(
                &mut bad,
                PacketThreeDReferenceDisplays::ENTRY_SIZE_OFFSET,
                PacketThreeDReferenceDisplay::DATA_LEN + 2,
            );
            bad
        });
        invalid_payloads.push({
            let mut bad = valid.clone();
            bad[PacketThreeDReferenceDisplays::ENTRIES_OFFSET + 8] = 2;
            bad
        });

        for data in invalid_payloads {
            assert_eq!(
                PacketThreeDReferenceDisplays::parse(&data)
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::ThreeDReferenceDisplays, data).unwrap();
            assert_eq!(
                side_data.three_d_reference_displays().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let non_tdrdi =
            SideData::new_with_kind(PacketSideDataKind::AmbientViewingEnvironment, valid).unwrap();
        assert_eq!(non_tdrdi.three_d_reference_displays().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_exif_payload() {
        let exif_bytes = minimal_little_exif_fixture();
        let side_data = SideData::new_exif(exif_bytes.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &PacketSideDataKind::Exif);
        assert_eq!(side_data.data(), exif_bytes.as_slice());
        assert_eq!(
            PacketSideDataKind::Exif.ffmpeg_constant().unwrap(),
            "AV_PKT_DATA_EXIF"
        );
        let parsed = side_data.exif().unwrap().unwrap();
        assert_eq!(parsed.data(), exif_bytes.as_slice());
        assert_eq!(parsed.endian(), FrameExifEndian::Little);
        assert_eq!(parsed.first_ifd_offset(), 8);
        assert_eq!(parsed.ifd_count(), 1);

        let ifd = parsed.ifd(0).unwrap();
        assert_eq!(ifd.offset(), 8);
        assert_eq!(ifd.entry_count(), 1);
        assert_eq!(ifd.next_ifd_offset(), None);
        let entry = ifd.entry(0).unwrap();
        assert_eq!(entry.tag(), 0x010F);
        assert_eq!(entry.tiff_type(), FrameExifTiffType::Ascii);
        assert_eq!(entry.endian(), FrameExifEndian::Little);
        assert_eq!(entry.count(), 6);
        assert_eq!(entry.value_offset(), 26);
        assert_eq!(entry.data_len(), 6);
        assert!(!entry.is_inline());
        assert_eq!(entry.value_or_offset_bytes(), &26u32.to_le_bytes());
        assert_eq!(entry.data_range(), Some((26, 32)));
        assert_eq!(entry.value_data(), b"Rusty\0");
        assert_eq!(entry.ascii_strings().unwrap().unwrap(), ["Rusty"]);
        assert_eq!(PacketExif::parse(parsed.data()).unwrap(), parsed);

        let big = SideData::new_exif(minimal_big_exif_fixture()).unwrap();
        let parsed_big = big.exif().unwrap().unwrap();
        assert_eq!(parsed_big.endian(), FrameExifEndian::Big);
        assert_eq!(parsed_big.first_ifd_offset(), 8);
        assert_eq!(parsed_big.ifd_count(), 1);
        assert!(parsed_big.ifd(0).unwrap().is_empty());
        assert_eq!(parsed_big.ifd(0).unwrap().next_ifd_offset(), None);

        let non_exif =
            SideData::new_with_kind(PacketSideDataKind::AmbientViewingEnvironment, exif_bytes)
                .unwrap();
        assert_eq!(non_exif.exif().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_exif_payload() {
        let exif_bytes = minimal_little_exif_fixture();
        let mut invalid_payloads = vec![
            Vec::new(),
            vec![0; PacketExif::TIFF_HEADER_LEN - 1],
            vec![0x45, 0x78, 0x69, 0x66, 8, 0, 0, 0],
        ];

        invalid_payloads.push({
            let mut bad = exif_bytes.clone();
            bad[4..8].copy_from_slice(&6u32.to_le_bytes());
            bad
        });
        invalid_payloads.push({
            let mut bad = exif_bytes.clone();
            bad[4..8].copy_from_slice(&31u32.to_le_bytes());
            bad
        });
        invalid_payloads.push({
            let mut bad = Vec::new();
            bad.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
            bad.extend_from_slice(&8u32.to_le_bytes());
            bad.extend_from_slice(
                &u16::try_from(PacketExif::MAX_IFD_ENTRIES + 1)
                    .unwrap()
                    .to_le_bytes(),
            );
            bad
        });
        invalid_payloads.push({
            let mut bad = exif_bytes.clone();
            bad.truncate(21);
            bad
        });
        invalid_payloads.push({
            let mut bad = exif_bytes.clone();
            bad[12..14].copy_from_slice(&0u16.to_le_bytes());
            bad
        });
        invalid_payloads.push({
            let mut bad = exif_bytes.clone();
            bad[18..22].copy_from_slice(&250u32.to_le_bytes());
            bad
        });

        for data in invalid_payloads {
            assert_eq!(
                PacketExif::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_with_kind(PacketSideDataKind::Exif, data)
                    .unwrap()
                    .exif()
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            SideData::new_exif(Vec::new()).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let non_exif =
            SideData::new_with_kind(PacketSideDataKind::AmbientViewingEnvironment, exif_bytes)
                .unwrap();
        assert_eq!(non_exif.exif().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_a53_closed_captions_payload() {
        let data = vec![0xfc, 0x80, 0x41, 0xfd, 0x80, 0x42];
        let parsed = PacketA53ClosedCaptions::parse(&data).unwrap();

        assert_eq!(PacketA53ClosedCaptions::BYTES_PER_CC, 3);
        assert_eq!(parsed.data(), data.as_slice());
        assert!(!parsed.is_empty());
        assert_eq!(parsed.entry_count(), 2);
        assert_eq!(parsed.entry(0), Some([0xfc, 0x80, 0x41]));
        assert_eq!(parsed.entry(1), Some([0xfd, 0x80, 0x42]));
        assert_eq!(parsed.entry(2), None);
        assert_eq!(
            parsed.entries().collect::<Vec<_>>(),
            vec![[0xfc, 0x80, 0x41], [0xfd, 0x80, 0x42]]
        );

        let side_data = SideData::new_a53_closed_captions(data.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::A53ClosedCaptions);
        assert_eq!(side_data.data(), data.as_slice());
        assert_eq!(side_data.a53_closed_captions().unwrap(), Some(parsed));

        let empty = SideData::new_a53_closed_captions(Vec::new()).unwrap();
        let parsed_empty = empty.a53_closed_captions().unwrap().unwrap();
        assert!(parsed_empty.is_empty());
        assert_eq!(parsed_empty.entry_count(), 0);
        assert_eq!(parsed_empty.entries().count(), 0);

        let content_light = SideData::new_with_kind(
            PacketSideDataKind::ContentLightLevel,
            PacketContentLightMetadata::new(1000, 400)
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert_eq!(content_light.a53_closed_captions().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_a53_closed_captions_payload() {
        for data in [vec![0], vec![0, 0], vec![0, 0, 0, 0]] {
            assert_eq!(
                PacketA53ClosedCaptions::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_a53_closed_captions(data.clone())
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data =
            SideData::new_with_kind(PacketSideDataKind::A53ClosedCaptions, vec![0, 0]).unwrap();
        assert_eq!(
            side_data.a53_closed_captions().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_icc_profile_payload() {
        let data = minimal_icc_profile();
        let side_data = SideData::new_icc_profile(data.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &PacketSideDataKind::IccProfile);
        assert_eq!(side_data.data(), data.as_slice());
        let parsed = side_data.icc_profile().unwrap().unwrap();
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(
            parsed.declared_size(),
            PacketIccProfile::MIN_DATA_LEN as u32
        );
        assert_eq!(parsed.profile_version_raw(), 0x0430_0000);
        assert_eq!(parsed.device_class(), *b"mntr");
        assert_eq!(parsed.color_space(), *b"RGB ");
        assert_eq!(parsed.profile_connection_space(), *b"XYZ ");
        assert_eq!(parsed.tag_count(), 0);
        assert_eq!(PacketIccProfile::parse(data.as_slice()).unwrap(), parsed);

        let mut with_tag = minimal_icc_profile();
        with_tag.resize(
            PacketIccProfile::MIN_DATA_LEN + PacketIccProfile::TAG_RECORD_LEN,
            0,
        );
        let with_tag_len = with_tag.len() as u32;
        with_tag[0..4].copy_from_slice(&with_tag_len.to_be_bytes());
        with_tag[PacketIccProfile::TAG_COUNT_OFFSET..PacketIccProfile::TAG_COUNT_OFFSET + 4]
            .copy_from_slice(&1u32.to_be_bytes());
        with_tag[132..136].copy_from_slice(b"desc");
        with_tag[136..140].copy_from_slice(&(PacketIccProfile::MIN_DATA_LEN as u32).to_be_bytes());
        with_tag[140..144].copy_from_slice(&0u32.to_be_bytes());
        let side_data = SideData::new_icc_profile(with_tag.clone()).unwrap();
        let parsed = side_data.icc_profile().unwrap().unwrap();
        assert_eq!(parsed.data(), with_tag.as_slice());
        assert_eq!(parsed.declared_size(), with_tag_len);
        assert_eq!(parsed.tag_count(), 1);

        let prft =
            SideData::new_with_kind(PacketSideDataKind::ProducerReferenceTime, data).unwrap();
        assert_eq!(prft.icc_profile().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_icc_profile_payload() {
        for data in [Vec::new(), vec![0; PacketIccProfile::MIN_DATA_LEN - 1]] {
            assert_eq!(
                PacketIccProfile::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data = SideData::new_with_kind(PacketSideDataKind::IccProfile, data).unwrap();
            assert_eq!(
                side_data.icc_profile().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let mut bad_size = minimal_icc_profile();
        bad_size[0..4].copy_from_slice(&999u32.to_be_bytes());
        assert_eq!(
            SideData::new_icc_profile(bad_size).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let mut missing_signature = minimal_icc_profile();
        missing_signature[36..40].copy_from_slice(b"bad!");
        let side_data =
            SideData::new_with_kind(PacketSideDataKind::IccProfile, missing_signature).unwrap();
        assert_eq!(
            side_data.icc_profile().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let mut truncated_tag_table = minimal_icc_profile();
        truncated_tag_table
            [PacketIccProfile::TAG_COUNT_OFFSET..PacketIccProfile::TAG_COUNT_OFFSET + 4]
            .copy_from_slice(&1u32.to_be_bytes());
        let side_data =
            SideData::new_with_kind(PacketSideDataKind::IccProfile, truncated_tag_table).unwrap();
        assert_eq!(
            side_data.icc_profile().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let non_icc =
            SideData::new_with_kind(PacketSideDataKind::RtcpSenderReport, minimal_icc_profile())
                .unwrap();
        assert_eq!(non_icc.icc_profile().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_dolby_vision_conf_payload() {
        let value = PacketDolbyVisionConf::new(
            1,
            0,
            8,
            6,
            true,
            false,
            true,
            4,
            PacketDoviCompression::Limited,
        );
        let expected = [1, 0, 8, 6, 1, 0, 1, 4, 1];

        assert_eq!(PacketDolbyVisionConf::DATA_LEN, 9);
        assert_eq!(PacketDolbyVisionConf::DV_VERSION_MAJOR_OFFSET, 0);
        assert_eq!(PacketDolbyVisionConf::DV_VERSION_MINOR_OFFSET, 1);
        assert_eq!(PacketDolbyVisionConf::DV_PROFILE_OFFSET, 2);
        assert_eq!(PacketDolbyVisionConf::DV_LEVEL_OFFSET, 3);
        assert_eq!(PacketDolbyVisionConf::RPU_PRESENT_FLAG_OFFSET, 4);
        assert_eq!(PacketDolbyVisionConf::EL_PRESENT_FLAG_OFFSET, 5);
        assert_eq!(PacketDolbyVisionConf::BL_PRESENT_FLAG_OFFSET, 6);
        assert_eq!(
            PacketDolbyVisionConf::DV_BL_SIGNAL_COMPATIBILITY_ID_OFFSET,
            7
        );
        assert_eq!(PacketDolbyVisionConf::DV_MD_COMPRESSION_OFFSET, 8);
        assert_eq!(value.to_bytes(), expected);
        assert_eq!(
            PacketSideDataKind::DolbyVisionConf
                .ffmpeg_constant()
                .unwrap(),
            "AV_PKT_DATA_DOVI_CONF"
        );
        assert_eq!(PacketDoviCompression::None.raw(), 0);
        assert_eq!(
            PacketDoviCompression::None.ffmpeg_constant(),
            "AV_DOVI_COMPRESSION_NONE"
        );
        assert_eq!(PacketDoviCompression::Limited.raw(), 1);
        assert_eq!(
            PacketDoviCompression::Limited.ffmpeg_constant(),
            "AV_DOVI_COMPRESSION_LIMITED"
        );
        assert_eq!(PacketDoviCompression::Reserved.raw(), 2);
        assert_eq!(
            PacketDoviCompression::Reserved.ffmpeg_constant(),
            "AV_DOVI_COMPRESSION_RESERVED"
        );
        assert_eq!(PacketDoviCompression::Extended.raw(), 3);
        assert_eq!(
            PacketDoviCompression::Extended.ffmpeg_constant(),
            "AV_DOVI_COMPRESSION_EXTENDED"
        );
        assert_eq!(
            PacketDoviCompression::from_byte(1).unwrap(),
            PacketDoviCompression::Limited
        );

        let parsed = PacketDolbyVisionConf::parse(&expected).unwrap();
        assert_eq!(parsed, value);
        assert_eq!(parsed.dv_version_major(), 1);
        assert_eq!(parsed.dv_version_minor(), 0);
        assert_eq!(parsed.dv_profile(), 8);
        assert_eq!(parsed.dv_level(), 6);
        assert!(parsed.rpu_present_flag());
        assert_eq!(parsed.rpu_present_flag_raw(), 1);
        assert!(!parsed.el_present_flag());
        assert_eq!(parsed.el_present_flag_raw(), 0);
        assert!(parsed.bl_present_flag());
        assert_eq!(parsed.bl_present_flag_raw(), 1);
        assert_eq!(parsed.dv_bl_signal_compatibility_id(), 4);
        assert_eq!(parsed.dv_md_compression(), PacketDoviCompression::Limited);
        assert_eq!(parsed.dv_md_compression_raw(), 1);

        let side_data = SideData::new_dolby_vision_conf(value).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::DolbyVisionConf);
        assert_eq!(side_data.data(), &expected[..]);
        assert_eq!(side_data.dolby_vision_conf().unwrap(), Some(value));

        let non_dovi =
            SideData::new_with_kind(PacketSideDataKind::IccProfile, expected.to_vec()).unwrap();
        assert_eq!(non_dovi.dolby_vision_conf().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_dolby_vision_conf_payload() {
        let valid = PacketDolbyVisionConf::new(
            1,
            0,
            8,
            6,
            true,
            false,
            true,
            4,
            PacketDoviCompression::Limited,
        )
        .to_bytes();

        assert_eq!(
            PacketDoviCompression::from_byte(4).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let mut invalid_payloads =
            vec![Vec::new(), vec![0; PacketDolbyVisionConf::DATA_LEN - 1], {
                let mut data = valid.to_vec();
                data.push(0);
                data
            }];
        for offset in [
            PacketDolbyVisionConf::RPU_PRESENT_FLAG_OFFSET,
            PacketDolbyVisionConf::EL_PRESENT_FLAG_OFFSET,
            PacketDolbyVisionConf::BL_PRESENT_FLAG_OFFSET,
        ] {
            let mut data = valid.to_vec();
            data[offset] = 2;
            invalid_payloads.push(data);
        }
        for raw_compression in [4, 255] {
            let mut data = valid.to_vec();
            data[PacketDolbyVisionConf::DV_MD_COMPRESSION_OFFSET] = raw_compression;
            invalid_payloads.push(data);
        }

        for data in invalid_payloads {
            assert_eq!(
                PacketDolbyVisionConf::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_with_kind(PacketSideDataKind::DolbyVisionConf, data)
                    .unwrap()
                    .dolby_vision_conf()
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let non_dovi =
            SideData::new_with_kind(PacketSideDataKind::IccProfile, valid.to_vec()).unwrap();
        assert_eq!(non_dovi.dolby_vision_conf().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_dynamic_hdr10_plus_payload() {
        let data = minimal_packet_dynamic_hdr10_plus();
        let side_data = SideData::new_dynamic_hdr10_plus(data.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &PacketSideDataKind::DynamicHdr10Plus);
        assert_eq!(side_data.data(), data.as_slice());
        assert_eq!(
            PacketSideDataKind::DynamicHdr10Plus
                .ffmpeg_constant()
                .unwrap(),
            "AV_PKT_DATA_DYNAMIC_HDR10_PLUS"
        );
        assert_eq!(PacketDynamicHdr10Plus::MAX_WINDOWS, 3);
        assert_eq!(PacketDynamicHdr10Plus::MAX_PEAK_LUMINANCE_ROWS, 25);
        assert_eq!(PacketDynamicHdr10Plus::MAX_PEAK_LUMINANCE_COLS, 25);

        let parsed = side_data.dynamic_hdr10_plus().unwrap().unwrap();
        assert_eq!(PacketDynamicHdr10Plus::parse(&data).unwrap(), parsed);
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.as_frame_dynamic_hdr_plus().data(), data.as_slice());
        assert_eq!(
            parsed.itu_t_t35_country_code(),
            PacketDynamicHdr10Plus::ITU_T_T35_COUNTRY_CODE
        );
        assert_eq!(
            parsed.application_version(),
            PacketDynamicHdr10Plus::APPLICATION_VERSION
        );
        assert_eq!(parsed.num_windows(), 1);
        assert!(parsed.color_transform_params(1).is_none());

        let params = parsed.color_transform_params(0).unwrap();
        assert_eq!(
            params.data().len(),
            PacketHdrPlusColorTransformParams::DATA_LEN
        );
        assert_eq!(
            params.overlap_process_option().unwrap(),
            PacketHdrPlusOverlapProcessOption::WeightedAveraging
        );
        assert_eq!(params.num_distribution_maxrgb_percentiles(), 0);
        assert_eq!(params.distribution_maxrgb(0), None);
        assert_eq!(params.tone_mapping_flag(), 0);
        assert_eq!(params.num_bezier_curve_anchors(), 0);
        assert_eq!(params.bezier_curve_anchor(0), None);
        assert_eq!(params.color_saturation_mapping_flag(), 0);

        assert_eq!(
            parsed.targeted_system_display_actual_peak_luminance_flag(),
            0
        );
        assert_eq!(
            parsed.num_rows_targeted_system_display_actual_peak_luminance(),
            0
        );
        assert_eq!(
            parsed.num_cols_targeted_system_display_actual_peak_luminance(),
            0
        );
        assert_eq!(
            parsed.targeted_system_display_actual_peak_luminance(0, 0),
            None
        );
        assert_eq!(parsed.mastering_display_actual_peak_luminance_flag(), 0);
        assert_eq!(parsed.num_rows_mastering_display_actual_peak_luminance(), 0);
        assert_eq!(parsed.num_cols_mastering_display_actual_peak_luminance(), 0);
        assert_eq!(parsed.mastering_display_actual_peak_luminance(0, 0), None);

        let non_hdr10_plus =
            SideData::new_with_kind(PacketSideDataKind::DolbyVisionConf, data).unwrap();
        assert_eq!(non_hdr10_plus.dynamic_hdr10_plus().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_dynamic_hdr10_plus_payload() {
        let valid = minimal_packet_dynamic_hdr10_plus();
        let mut invalid_payloads = vec![
            Vec::new(),
            vec![0; PacketDynamicHdr10Plus::DATA_LEN - 1],
            {
                let mut data = valid.clone();
                data.push(0);
                data
            },
            {
                let mut data = valid.clone();
                data[0] = PacketDynamicHdr10Plus::ITU_T_T35_COUNTRY_CODE - 1;
                data
            },
            {
                let mut data = valid.clone();
                data[1] = PacketDynamicHdr10Plus::APPLICATION_VERSION + 1;
                data
            },
            {
                let mut data = valid.clone();
                data[2] = 0;
                data
            },
            {
                let mut data = valid.clone();
                data[2] = PacketDynamicHdr10Plus::MAX_WINDOWS as u8 + 1;
                data
            },
        ];

        let mut invalid_overlap = valid.clone();
        const PARAMS_OFFSET: usize = 4;
        const OVERLAP_PROCESS_OPTION_OFFSET: usize = 44;
        invalid_overlap[PARAMS_OFFSET + OVERLAP_PROCESS_OPTION_OFFSET
            ..PARAMS_OFFSET + OVERLAP_PROCESS_OPTION_OFFSET + 4]
            .copy_from_slice(&2i32.to_ne_bytes());
        invalid_payloads.push(invalid_overlap);

        for data in invalid_payloads {
            assert_eq!(
                PacketDynamicHdr10Plus::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_dynamic_hdr10_plus(data.clone())
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_with_kind(PacketSideDataKind::DynamicHdr10Plus, data)
                    .unwrap()
                    .dynamic_hdr10_plus()
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_parses_param_change_payload() {
        let sample_rate_only = PacketParamChange::new(Some(48_000), None);
        let sample_rate_bytes = [0x04, 0, 0, 0, 0x80, 0xbb, 0, 0];
        assert_eq!(
            sample_rate_only.flags(),
            PacketParamChange::SAMPLE_RATE_FLAG
        );
        assert_eq!(sample_rate_only.to_bytes(), sample_rate_bytes);
        assert_eq!(
            PacketParamChange::parse(&sample_rate_bytes).unwrap(),
            sample_rate_only
        );
        assert_eq!(sample_rate_only.sample_rate(), Some(48_000));
        assert_eq!(sample_rate_only.dimensions(), None);

        let dimensions_only = PacketParamChange::new(None, Some((1920, 1080)));
        let dimensions_bytes = [0x08, 0, 0, 0, 0x80, 0x07, 0, 0, 0x38, 0x04, 0, 0];
        assert_eq!(dimensions_only.flags(), PacketParamChange::DIMENSIONS_FLAG);
        assert_eq!(dimensions_only.to_bytes(), dimensions_bytes);
        assert_eq!(
            PacketParamChange::parse(&dimensions_bytes).unwrap(),
            dimensions_only
        );
        assert_eq!(dimensions_only.sample_rate(), None);
        assert_eq!(dimensions_only.dimensions(), Some((1920, 1080)));

        let both = PacketParamChange::new(Some(-1), Some((i32::MIN, i32::MAX)));
        let both_bytes = [
            0x0c, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0x80, 0xff, 0xff, 0xff, 0x7f,
        ];
        assert_eq!(
            both.flags(),
            PacketParamChange::SAMPLE_RATE_FLAG | PacketParamChange::DIMENSIONS_FLAG
        );
        assert_eq!(both.to_bytes(), both_bytes);
        assert_eq!(PacketParamChange::parse(&both_bytes).unwrap(), both);

        let no_change = PacketParamChange::new(None, None);
        assert_eq!(no_change.to_bytes(), [0, 0, 0, 0]);
        assert_eq!(PacketParamChange::parse(&[0, 0, 0, 0]).unwrap(), no_change);

        let side_data = SideData::new_param_change(both).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::ParamChange);
        assert_eq!(side_data.data(), &both_bytes[..]);
        assert_eq!(side_data.param_change().unwrap(), Some(both));

        let skip_samples =
            SideData::new_with_kind(PacketSideDataKind::SkipSamples, both_bytes.to_vec()).unwrap();
        assert_eq!(skip_samples.param_change().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_param_change_payload() {
        for data in [
            Vec::new(),
            vec![0; 3],
            vec![0x04, 0, 0, 0],
            vec![0x04, 0, 0, 0, 1],
        ] {
            assert_eq!(
                PacketParamChange::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let trailing = [0, 0, 0, 0, 0];
        assert_eq!(
            PacketParamChange::parse(&trailing).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let unknown_flags = [0x10, 0, 0, 0];
        assert_eq!(
            PacketParamChange::parse(&unknown_flags).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let truncated_dimensions = [0x08, 0, 0, 0, 1, 0, 0, 0];
        assert_eq!(
            PacketParamChange::parse(&truncated_dimensions)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidData
        );

        let side_data =
            SideData::new_with_kind(PacketSideDataKind::ParamChange, vec![0x08, 0, 0, 0]).unwrap();
        assert_eq!(
            side_data.param_change().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_jp_dualmono_payload() {
        for (raw, selection) in [
            (0, PacketJpDualMonoSelection::MainLeft),
            (1, PacketJpDualMonoSelection::SubRight),
            (2, PacketJpDualMonoSelection::Both),
        ] {
            let expected = PacketJpDualMono::new(selection);
            assert_eq!(
                PacketJpDualMonoSelection::from_byte(raw).unwrap(),
                selection
            );
            assert_eq!(selection.as_byte(), raw);
            assert_eq!(expected.to_bytes(), [raw]);
            assert_eq!(PacketJpDualMono::parse(&[raw]).unwrap(), expected);
            assert_eq!(expected.selected_channels(), selection);

            let side_data = SideData::new_jp_dualmono(expected).unwrap();
            assert_eq!(side_data.kind_id(), &PacketSideDataKind::JpDualMono);
            assert_eq!(side_data.data(), &[raw]);
            assert_eq!(side_data.jp_dualmono().unwrap(), Some(expected));
        }

        let skip_samples =
            SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![2]).unwrap();
        assert_eq!(skip_samples.jp_dualmono().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_jp_dualmono_payload() {
        for data in [Vec::new(), vec![0, 1], vec![3]] {
            assert_eq!(
                PacketJpDualMono::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }
        assert_eq!(
            PacketJpDualMonoSelection::from_byte(0xff)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidData
        );

        let side_data = SideData::new_with_kind(PacketSideDataKind::JpDualMono, vec![3]).unwrap();
        assert_eq!(
            side_data.jp_dualmono().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_string_metadata_payloads() {
        let data = b"title\0Clip\0language\0eng\0empty\0\0bin\0\xff\xfe\0".to_vec();
        let parsed = PacketStringMetadata::parse(&data).unwrap();

        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.len(), data.len());
        assert!(!parsed.is_empty());
        assert_eq!(parsed.entry_count(), 4);
        assert_eq!(parsed.entries().len(), 4);
        assert_eq!(parsed.entry(0).unwrap().key_bytes(), b"title");
        assert_eq!(parsed.entry(0).unwrap().value_bytes(), b"Clip");
        assert_eq!(parsed.entry(0).unwrap().key_str().unwrap(), "title");
        assert_eq!(parsed.entry(0).unwrap().value_str().unwrap(), "Clip");
        assert_eq!(parsed.entry(1).unwrap().key_str().unwrap(), "language");
        assert_eq!(parsed.entry(1).unwrap().value_str().unwrap(), "eng");
        assert_eq!(parsed.entry(2).unwrap().key_bytes(), b"empty");
        assert_eq!(parsed.entry(2).unwrap().value_bytes(), b"");
        assert_eq!(parsed.entry(2).unwrap().value_str().unwrap(), "");
        assert_eq!(parsed.entry(3).unwrap().key_bytes(), b"bin");
        assert_eq!(parsed.entry(3).unwrap().value_bytes(), &[0xff, 0xfe]);
        assert_eq!(
            parsed.entry(3).unwrap().value_str().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
        assert_eq!(parsed.entry(4), None);

        let empty = PacketStringMetadata::parse(&[]).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.entry_count(), 0);
        assert_eq!(empty.entries(), &[]);

        let strings_side_data = SideData::new_strings_metadata(data.clone()).unwrap();
        assert_eq!(
            strings_side_data.kind_id(),
            &PacketSideDataKind::StringsMetadata
        );
        assert_eq!(strings_side_data.data(), data.as_slice());
        assert_eq!(
            strings_side_data
                .strings_metadata()
                .unwrap()
                .unwrap()
                .entries(),
            parsed.entries()
        );

        let update_side_data = SideData::new_metadata_update(data.clone()).unwrap();
        assert_eq!(
            update_side_data.kind_id(),
            &PacketSideDataKind::MetadataUpdate
        );
        assert_eq!(update_side_data.data(), data.as_slice());
        assert_eq!(
            update_side_data
                .metadata_update()
                .unwrap()
                .unwrap()
                .entries(),
            parsed.entries()
        );

        let non_strings =
            SideData::new_with_kind(PacketSideDataKind::MpegTsStreamId, data).unwrap();
        assert_eq!(non_strings.strings_metadata().unwrap(), None);
        assert_eq!(non_strings.metadata_update().unwrap(), None);
    }

    #[test]
    fn packet_dictionary_pack_unpack_uses_string_metadata_wire_format() {
        let mut dict = Dictionary::new();
        dict.set("title", "Clip").unwrap();
        dict.set("language", "eng").unwrap();
        dict.set("empty", "").unwrap();

        let packed = packet_pack_dictionary(&dict);

        assert_eq!(packed, b"title\0Clip\0language\0eng\0empty\0\0");

        let unpacked = packet_unpack_dictionary(&packed).unwrap();
        assert_eq!(unpacked.entries(), dict.entries());

        let empty = Dictionary::new();
        assert!(packet_pack_dictionary(&empty).is_empty());
        assert!(packet_unpack_dictionary(&[]).unwrap().is_empty());
    }

    #[test]
    fn packet_dictionary_unpack_rejects_malformed_or_non_utf8_metadata() {
        for data in [
            b"title\0Clip".as_slice(),
            b"\0Clip\0".as_slice(),
            b"title\0".as_slice(),
            b"title\0Clip\0\0".as_slice(),
        ] {
            assert_eq!(
                packet_unpack_dictionary(data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            packet_unpack_dictionary(b"title\0\xff\xfe\0")
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_rejects_malformed_string_metadata_payloads() {
        for data in [
            b"title\0Clip".to_vec(),
            b"\0Clip\0".to_vec(),
            b"title\0".to_vec(),
            b"title\0Clip\0\0".to_vec(),
        ] {
            assert_eq!(
                PacketStringMetadata::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_strings_metadata(data.clone())
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_metadata_update(data.clone())
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );

            let strings_side_data =
                SideData::new_with_kind(PacketSideDataKind::StringsMetadata, data.clone()).unwrap();
            assert_eq!(
                strings_side_data.strings_metadata().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let update_side_data =
                SideData::new_with_kind(PacketSideDataKind::MetadataUpdate, data).unwrap();
            assert_eq!(
                update_side_data.metadata_update().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_parses_encryption_info_payload() {
        let scheme = u32::from_be_bytes(*b"cenc");
        let key_id = [0x10, 0x11, 0x12, 0x13];
        let iv = [0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27];
        let subsamples = [
            PacketEncryptionSubsample::new(3, 100),
            PacketEncryptionSubsample::new(0, 55),
        ];
        let mut data = Vec::new();
        for value in [
            scheme,
            1_u32,
            9_u32,
            key_id.len() as u32,
            iv.len() as u32,
            subsamples.len() as u32,
        ] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data.extend_from_slice(&key_id);
        data.extend_from_slice(&iv);
        for subsample in subsamples {
            data.extend_from_slice(&subsample.to_bytes());
        }
        let parsed_len = data.len();
        data.extend_from_slice(&[0xaa, 0xbb]);

        let parsed = PacketEncryptionInfo::parse(&data).unwrap();
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.parsed_len(), parsed_len);
        assert_eq!(parsed.trailing_data(), &[0xaa, 0xbb]);
        assert_eq!(parsed.scheme(), scheme);
        assert_eq!(parsed.scheme_fourcc(), *b"cenc");
        assert_eq!(parsed.crypt_byte_block(), 1);
        assert_eq!(parsed.skip_byte_block(), 9);
        assert_eq!(parsed.key_id(), key_id.as_slice());
        assert_eq!(parsed.key_id_size(), key_id.len());
        assert_eq!(parsed.iv(), iv.as_slice());
        assert_eq!(parsed.iv_size(), iv.len());
        assert_eq!(parsed.subsample_count(), 2);
        assert_eq!(parsed.subsamples(), &subsamples);
        assert_eq!(parsed.subsample(0), Some(subsamples[0]));
        assert_eq!(parsed.subsample(1), Some(subsamples[1]));
        assert_eq!(parsed.subsample(2), None);

        let side_data = SideData::new_encryption_info(data.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::EncryptionInfo);
        assert_eq!(side_data.data(), data.as_slice());
        assert_eq!(side_data.encryption_info().unwrap(), Some(parsed));

        let non_encryption =
            SideData::new_with_kind(PacketSideDataKind::MpegTsStreamId, data).unwrap();
        assert_eq!(non_encryption.encryption_info().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_encryption_info_payload() {
        let mut truncated_subsamples = Vec::new();
        for value in [u32::from_be_bytes(*b"cenc"), 0, 0, 4, 8, 2] {
            truncated_subsamples.extend_from_slice(&value.to_be_bytes());
        }
        truncated_subsamples.extend_from_slice(&[0x10; 4]);
        truncated_subsamples.extend_from_slice(&[0x20; 8]);
        truncated_subsamples.extend_from_slice(&PacketEncryptionSubsample::new(3, 100).to_bytes());

        let mut impossible_key_size = vec![0; PacketEncryptionInfo::HEADER_LEN];
        impossible_key_size[12..16].copy_from_slice(&u32::MAX.to_be_bytes());

        for data in [
            Vec::new(),
            vec![0; PacketEncryptionInfo::HEADER_LEN - 1],
            truncated_subsamples,
            impossible_key_size,
        ] {
            assert_eq!(
                PacketEncryptionInfo::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_encryption_info(data.clone())
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );

            let side_data =
                SideData::new_with_kind(PacketSideDataKind::EncryptionInfo, data).unwrap();
            assert_eq!(
                side_data.encryption_info().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_parses_encryption_init_info_payload() {
        let mut data = Vec::new();
        data.extend_from_slice(&2_u32.to_be_bytes());

        for value in [4_u32, 2, 3, 5] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data.extend_from_slice(b"sys1");
        data.extend_from_slice(b"abc");
        data.extend_from_slice(b"def");
        data.extend_from_slice(b"hello");

        for value in [0_u32, 0, 16, 3] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data.extend_from_slice(b"pss");

        let parsed_len = data.len();
        data.push(0xff);

        let parsed = PacketEncryptionInitInfo::parse(&data).unwrap();
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.parsed_len(), parsed_len);
        assert_eq!(parsed.trailing_data(), &[0xff]);
        assert_eq!(parsed.entry_count(), 2);
        assert_eq!(parsed.entries().len(), 2);

        let first = parsed.entry(0).unwrap();
        assert_eq!(first.system_id(), b"sys1");
        assert_eq!(first.system_id_size(), 4);
        assert_eq!(first.key_id_size(), 3);
        assert_eq!(first.key_id_count(), 2);
        assert_eq!(first.key_ids(), &[b"abc".as_slice(), b"def".as_slice()]);
        assert_eq!(first.key_id(0), Some(b"abc".as_slice()));
        assert_eq!(first.key_id(1), Some(b"def".as_slice()));
        assert_eq!(first.key_id(2), None);
        assert_eq!(first.data(), b"hello");
        assert_eq!(first.data_size(), 5);

        let second = parsed.entry(1).unwrap();
        assert_eq!(second.system_id(), b"");
        assert_eq!(second.system_id_size(), 0);
        assert_eq!(second.key_id_size(), 16);
        assert_eq!(second.key_id_count(), 0);
        assert!(second.key_ids().is_empty());
        assert_eq!(second.data(), b"pss");
        assert_eq!(second.data_size(), 3);
        assert_eq!(parsed.entry(2), None);

        let side_data = SideData::new_encryption_init_info(data.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::EncryptionInitInfo);
        assert_eq!(side_data.data(), data.as_slice());
        assert_eq!(side_data.encryption_init_info().unwrap(), Some(parsed));

        let empty_bytes = 0_u32.to_be_bytes();
        let empty = PacketEncryptionInitInfo::parse(&empty_bytes).unwrap();
        assert_eq!(empty.entry_count(), 0);
        assert_eq!(empty.parsed_len(), PacketEncryptionInitInfo::COUNT_LEN);

        let non_encryption =
            SideData::new_with_kind(PacketSideDataKind::MpegTsStreamId, data).unwrap();
        assert_eq!(non_encryption.encryption_init_info().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_encryption_init_info_payload() {
        let mut truncated_body = Vec::new();
        truncated_body.extend_from_slice(&1_u32.to_be_bytes());
        for value in [4_u32, 1, 3, 5] {
            truncated_body.extend_from_slice(&value.to_be_bytes());
        }
        truncated_body.extend_from_slice(b"sys");

        let mut zero_key_size_with_keys = Vec::new();
        zero_key_size_with_keys.extend_from_slice(&1_u32.to_be_bytes());
        for value in [0_u32, 1, 0, 0] {
            zero_key_size_with_keys.extend_from_slice(&value.to_be_bytes());
        }

        for data in [
            Vec::new(),
            vec![0; PacketEncryptionInitInfo::COUNT_LEN - 1],
            1_u32.to_be_bytes().to_vec(),
            truncated_body,
            zero_key_size_with_keys,
        ] {
            assert_eq!(
                PacketEncryptionInitInfo::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_encryption_init_info(data.clone())
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );

            let side_data =
                SideData::new_with_kind(PacketSideDataKind::EncryptionInitInfo, data).unwrap();
            assert_eq!(
                side_data.encryption_init_info().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_parses_iamf_mix_gain_param_payload() {
        let data = minimal_packet_iamf_mix_gain_param();
        let parsed = PacketIamfMixGainParam::parse(&data).unwrap();
        let definition = parsed.definition();
        let parsed_len = PacketIamfParamDefinition::HEADER_LEN
            + PacketIamfMixGainSubblock::MIN_DATA_LEN * parsed.subblock_count();

        assert_eq!(definition.data(), data.as_slice());
        assert_eq!(definition.parsed_len(), parsed_len);
        assert_eq!(definition.trailing_data(), &[0xaa]);
        assert_eq!(definition.av_class_address(), 0x1111);
        assert_eq!(
            definition.definition_type(),
            PacketIamfParamDefinitionType::MixGain
        );
        assert_eq!(definition.parameter_id(), 7);
        assert_eq!(definition.parameter_rate(), 48_000);
        assert_eq!(definition.duration(), 960);
        assert_eq!(definition.constant_subblock_duration(), 480);
        assert_eq!(
            definition.subblocks_offset(),
            PacketIamfParamDefinition::HEADER_LEN
        );
        assert_eq!(
            definition.subblock_size(),
            PacketIamfMixGainSubblock::MIN_DATA_LEN
        );
        assert_eq!(definition.subblock_count(), 2);
        assert_eq!(parsed.subblocks().len(), 2);

        let first = parsed.subblock(0).unwrap();
        assert_eq!(first.av_class_address(), 0x2222);
        assert_eq!(first.subblock_duration(), 480);
        assert_eq!(first.animation_type(), PacketIamfAnimationType::Linear);
        assert_eq!(
            first.animation_type().ffmpeg_constant(),
            "AV_IAMF_ANIMATION_TYPE_LINEAR"
        );
        assert_eq!(first.start_point_value(), Rational::from_raw(-1, 2));
        assert_eq!(first.end_point_value(), Rational::from_raw(3, 4));
        assert_eq!(first.control_point_value(), Rational::from_raw(1, 3));
        assert_eq!(
            first.control_point_relative_time(),
            Rational::from_raw(1, 2)
        );
        assert_eq!(
            parsed.subblock(1).unwrap().animation_type(),
            PacketIamfAnimationType::Bezier
        );
        assert_eq!(parsed.subblock(2), None);
        assert_eq!(
            PacketIamfParamDefinitionType::MixGain.ffmpeg_constant(),
            "AV_IAMF_PARAMETER_DEFINITION_MIX_GAIN"
        );

        let side_data = SideData::new_iamf_mix_gain_param(data.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::IamfMixGainParam);
        assert_eq!(side_data.data(), data.as_slice());
        assert_eq!(
            side_data
                .iamf_mix_gain_param()
                .unwrap()
                .unwrap()
                .definition(),
            parsed.definition()
        );

        let non_iamf = SideData::new_with_kind(PacketSideDataKind::DynamicHdr10Plus, data).unwrap();
        assert_eq!(non_iamf.iamf_mix_gain_param().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_iamf_demixing_info_param_payload() {
        let data = minimal_packet_iamf_demixing_info_param();
        let parsed = PacketIamfDemixingInfoParam::parse(&data).unwrap();
        let definition = parsed.definition();

        assert_eq!(
            definition.definition_type(),
            PacketIamfParamDefinitionType::Demixing
        );
        assert_eq!(
            definition.subblock_size(),
            PacketIamfDemixingInfoSubblock::MIN_DATA_LEN
        );
        assert_eq!(definition.subblock_count(), 1);
        assert!(definition.trailing_data().is_empty());

        let subblock = parsed.subblock(0).unwrap();
        assert_eq!(
            subblock.data().len(),
            PacketIamfDemixingInfoSubblock::MIN_DATA_LEN
        );
        assert_eq!(subblock.av_class_address(), 0x3333);
        assert_eq!(subblock.subblock_duration(), 960);
        assert_eq!(subblock.dmixp_mode(), 7);
        assert_eq!(parsed.subblock(1), None);

        let side_data = SideData::new_iamf_demixing_info_param(data.clone()).unwrap();
        assert_eq!(
            side_data.kind_id(),
            &PacketSideDataKind::IamfDemixingInfoParam
        );
        assert_eq!(
            side_data
                .iamf_demixing_info_param()
                .unwrap()
                .unwrap()
                .subblock(0),
            Some(subblock)
        );

        let non_iamf = SideData::new_with_kind(PacketSideDataKind::Lcevc, data).unwrap();
        assert_eq!(non_iamf.iamf_demixing_info_param().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_iamf_recon_gain_info_param_payload() {
        let data = minimal_packet_iamf_recon_gain_info_param();
        let parsed = PacketIamfReconGainInfoParam::parse(&data).unwrap();
        let definition = parsed.definition();

        assert_eq!(
            definition.definition_type(),
            PacketIamfParamDefinitionType::ReconGain
        );
        assert_eq!(
            definition.subblock_size(),
            PacketIamfReconGainSubblock::MIN_DATA_LEN
        );
        assert_eq!(definition.subblock_count(), 1);

        let subblock = parsed.subblock(0).unwrap();
        assert_eq!(
            subblock.data().len(),
            PacketIamfReconGainSubblock::MIN_DATA_LEN
        );
        assert_eq!(subblock.av_class_address(), 0x4444);
        assert_eq!(subblock.subblock_duration(), 960);
        assert_eq!(subblock.recon_gain_value(0, 0), Some(0));
        assert_eq!(subblock.recon_gain_value(5, 11), Some(91));
        assert_eq!(subblock.recon_gain_value(6, 0), None);
        assert_eq!(subblock.recon_gain_value(0, 12), None);
        assert_eq!(subblock.recon_gain()[5][11], 91);

        let side_data = SideData::new_iamf_recon_gain_info_param(data.clone()).unwrap();
        assert_eq!(
            side_data.kind_id(),
            &PacketSideDataKind::IamfReconGainInfoParam
        );
        assert_eq!(
            side_data
                .iamf_recon_gain_info_param()
                .unwrap()
                .unwrap()
                .subblock(0),
            Some(subblock)
        );

        let non_iamf = SideData::new_with_kind(PacketSideDataKind::Lcevc, data).unwrap();
        assert_eq!(non_iamf.iamf_recon_gain_info_param().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_iamf_param_payloads() {
        let valid = minimal_packet_iamf_mix_gain_param();

        let mut wrong_type = valid.clone();
        write_packet_iamf_u32(
            &mut wrong_type,
            PacketIamfParamDefinition::TYPE_OFFSET,
            PacketIamfParamDefinitionType::Demixing.as_raw(),
        );

        let mut zero_parameter_rate = valid.clone();
        write_packet_iamf_u32(
            &mut zero_parameter_rate,
            PacketIamfParamDefinition::PARAMETER_RATE_OFFSET,
            0,
        );

        let mut bad_offset = valid.clone();
        write_packet_iamf_usize(
            &mut bad_offset,
            PacketIamfParamDefinition::SUBBLOCKS_OFFSET_OFFSET,
            PacketIamfParamDefinition::HEADER_LEN - 1,
        );

        let mut bad_size = valid.clone();
        write_packet_iamf_usize(
            &mut bad_size,
            PacketIamfParamDefinition::SUBBLOCK_SIZE_OFFSET,
            PacketIamfMixGainSubblock::MIN_DATA_LEN - 1,
        );

        let mut truncated = valid.clone();
        truncated.truncate(PacketIamfParamDefinition::HEADER_LEN);

        let mut zero_subblock_duration = valid.clone();
        let duration_offset = PacketIamfParamDefinition::HEADER_LEN
            + PacketIamfMixGainSubblock::SUBBLOCK_DURATION_OFFSET;
        write_packet_iamf_u32(&mut zero_subblock_duration, duration_offset, 0);

        let mut bad_animation = valid.clone();
        let animation_offset = PacketIamfParamDefinition::HEADER_LEN
            + PacketIamfMixGainSubblock::ANIMATION_TYPE_OFFSET;
        write_packet_iamf_u32(&mut bad_animation, animation_offset, 99);

        for data in [
            Vec::new(),
            valid[..PacketIamfParamDefinition::HEADER_LEN - 1].to_vec(),
            wrong_type,
            zero_parameter_rate,
            bad_offset,
            bad_size,
            truncated,
            zero_subblock_duration,
            bad_animation,
        ] {
            assert_eq!(
                PacketIamfMixGainParam::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            assert_eq!(
                SideData::new_iamf_mix_gain_param(data.clone())
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );

            let side_data =
                SideData::new_with_kind(PacketSideDataKind::IamfMixGainParam, data).unwrap();
            assert_eq!(
                side_data.iamf_mix_gain_param().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_parses_skip_samples_payload() {
        let expected = PacketSkipSamples::new(
            1024,
            256,
            PacketSkipSamplesReason::PaddingSilence,
            PacketSkipSamplesReason::Convergence,
        );
        let expected_bytes = [0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01];

        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(PacketSkipSamples::parse(&expected_bytes).unwrap(), expected);
        assert_eq!(expected.start(), 1024);
        assert_eq!(expected.end(), 256);
        assert_eq!(
            expected.start_reason(),
            PacketSkipSamplesReason::PaddingSilence
        );
        assert_eq!(expected.end_reason(), PacketSkipSamplesReason::Convergence);
        assert_eq!(
            PacketSkipSamplesReason::from_byte(expected.end_reason().as_byte()).unwrap(),
            PacketSkipSamplesReason::Convergence
        );

        let side_data = SideData::new_skip_samples(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::SkipSamples);
        assert_eq!(side_data.data(), &expected_bytes[..]);
        assert_eq!(side_data.skip_samples().unwrap(), Some(expected));

        let frame_cropping =
            SideData::new_with_kind(PacketSideDataKind::FrameCropping, expected_bytes.to_vec())
                .unwrap();
        assert_eq!(frame_cropping.skip_samples().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_skip_samples_payload() {
        let mut valid = PacketSkipSamples::new(
            1,
            2,
            PacketSkipSamplesReason::PaddingSilence,
            PacketSkipSamplesReason::Convergence,
        )
        .to_bytes();

        assert_eq!(
            PacketSkipSamples::parse(&valid[..PacketSkipSamples::DATA_LEN - 1])
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidData
        );

        valid[8] = 2;
        assert_eq!(
            PacketSkipSamples::parse(&valid).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
        valid[8] = 0;
        valid[9] = 0xff;
        let side_data =
            SideData::new_with_kind(PacketSideDataKind::SkipSamples, valid.to_vec()).unwrap();
        assert_eq!(
            side_data.skip_samples().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_mpegts_stream_id_payload() {
        for raw in [0, 0x47, u8::MAX] {
            let expected = PacketMpegTsStreamId::new(raw);
            assert_eq!(expected.stream_id(), raw);
            assert_eq!(expected.to_bytes(), [raw]);
            assert_eq!(PacketMpegTsStreamId::parse(&[raw]).unwrap(), expected);

            let side_data = SideData::new_mpegts_stream_id(expected).unwrap();
            assert_eq!(side_data.kind_id(), &PacketSideDataKind::MpegTsStreamId);
            assert_eq!(side_data.data(), &[raw]);
            assert_eq!(side_data.mpegts_stream_id().unwrap(), Some(expected));
        }

        let palette = SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x47]).unwrap();
        assert_eq!(palette.mpegts_stream_id().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_mpegts_stream_id_payload() {
        for data in [Vec::new(), vec![0, 1]] {
            assert_eq!(
                PacketMpegTsStreamId::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data =
            SideData::new_with_kind(PacketSideDataKind::MpegTsStreamId, Vec::new()).unwrap();
        assert_eq!(
            side_data.mpegts_stream_id().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_subtitle_position_payload() {
        let expected = PacketSubtitlePosition::new(1, 2, u32::MAX - 1, u32::MAX);
        let expected_bytes = [
            1, 0, 0, 0, 2, 0, 0, 0, 0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ];

        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketSubtitlePosition::parse(&expected_bytes).unwrap(),
            expected
        );
        assert_eq!(expected.x1(), 1);
        assert_eq!(expected.y1(), 2);
        assert_eq!(expected.x2(), u32::MAX - 1);
        assert_eq!(expected.y2(), u32::MAX);

        let side_data = SideData::new_subtitle_position(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::SubtitlePosition);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.subtitle_position().unwrap(), Some(expected));

        let palette =
            SideData::new_with_kind(PacketSideDataKind::Palette, expected_bytes.to_vec()).unwrap();
        assert_eq!(palette.subtitle_position().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_subtitle_position_payload() {
        for data in [vec![0; PacketSubtitlePosition::DATA_LEN - 1], vec![0; 17]] {
            assert_eq!(
                PacketSubtitlePosition::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data =
            SideData::new_with_kind(PacketSideDataKind::SubtitlePosition, vec![0; 4]).unwrap();
        assert_eq!(
            side_data.subtitle_position().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_matroska_block_additional_payload() {
        let expected =
            PacketMatroskaBlockAdditional::new(0x0102_0304_0506_0708, vec![0xaa, 0xbb, 0xcc]);
        let expected_bytes = [1, 2, 3, 4, 5, 6, 7, 8, 0xaa, 0xbb, 0xcc];

        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketMatroskaBlockAdditional::parse(&expected_bytes).unwrap(),
            expected
        );
        assert_eq!(expected.block_add_id(), 0x0102_0304_0506_0708);
        assert_eq!(expected.data(), &[0xaa, 0xbb, 0xcc]);

        let empty = PacketMatroskaBlockAdditional::new(u64::MAX, Vec::new());
        assert_eq!(empty.to_bytes(), [0xff; 8]);
        assert_eq!(
            PacketMatroskaBlockAdditional::parse(&[0xff; 8]).unwrap(),
            empty
        );
        assert!(empty.data().is_empty());

        let side_data = SideData::new_matroska_block_additional(expected.clone()).unwrap();
        assert_eq!(
            side_data.kind_id(),
            &PacketSideDataKind::MatroskaBlockAdditional
        );
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(
            side_data.matroska_block_additional().unwrap(),
            Some(expected)
        );

        let palette =
            SideData::new_with_kind(PacketSideDataKind::Palette, expected_bytes.to_vec()).unwrap();
        assert_eq!(palette.matroska_block_additional().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_matroska_block_additional_payload() {
        for len in 0..PacketMatroskaBlockAdditional::MIN_DATA_LEN {
            assert_eq!(
                PacketMatroskaBlockAdditional::parse(&vec![0; len])
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let side_data =
            SideData::new_with_kind(PacketSideDataKind::MatroskaBlockAdditional, vec![0; 7])
                .unwrap();
        assert_eq!(
            side_data.matroska_block_additional().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_webvtt_identifier_and_settings_payloads() {
        let identifier = PacketWebVttIdentifier::new(b"chapter-01".to_vec()).unwrap();
        assert_eq!(identifier.data(), b"chapter-01");
        assert_eq!(identifier.as_str().unwrap(), "chapter-01");
        assert_eq!(identifier.to_bytes(), b"chapter-01");
        assert_eq!(
            PacketWebVttIdentifier::parse(b"chapter-01").unwrap(),
            identifier
        );

        let raw_identifier = PacketWebVttIdentifier::parse(&[0xff, b'i', b'd']).unwrap();
        assert_eq!(raw_identifier.data(), &[0xff, b'i', b'd']);
        assert_eq!(
            raw_identifier.as_str().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let identifier_side_data = SideData::new_webvtt_identifier(identifier.clone()).unwrap();
        assert_eq!(
            identifier_side_data.kind_id(),
            &PacketSideDataKind::WebVttIdentifier
        );
        assert_eq!(identifier_side_data.data(), b"chapter-01");
        assert_eq!(
            identifier_side_data.webvtt_identifier().unwrap(),
            Some(identifier)
        );

        let settings =
            PacketWebVttSettings::new(b"line:0 position:50% align:start".to_vec()).unwrap();
        assert_eq!(settings.data(), b"line:0 position:50% align:start");
        assert_eq!(
            settings.as_str().unwrap(),
            "line:0 position:50% align:start"
        );
        assert_eq!(
            PacketWebVttSettings::parse(b"line:0 position:50% align:start").unwrap(),
            settings
        );

        let settings_side_data = SideData::new_webvtt_settings(settings.clone()).unwrap();
        assert_eq!(
            settings_side_data.kind_id(),
            &PacketSideDataKind::WebVttSettings
        );
        assert_eq!(
            settings_side_data.data(),
            b"line:0 position:50% align:start"
        );
        assert_eq!(
            settings_side_data.webvtt_settings().unwrap(),
            Some(settings)
        );

        let palette =
            SideData::new_with_kind(PacketSideDataKind::Palette, b"line:0 position:50%".to_vec())
                .unwrap();
        assert_eq!(palette.webvtt_identifier().unwrap(), None);
        assert_eq!(palette.webvtt_settings().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_webvtt_payloads() {
        for data in [
            Vec::new(),
            b"two\nlines".to_vec(),
            b"two\rlines".to_vec(),
            b"nul\0byte".to_vec(),
            b"00:00:00.000 --> 00:00:01.000".to_vec(),
        ] {
            assert_eq!(
                PacketWebVttIdentifier::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        for data in [
            Vec::new(),
            b"two\nlines".to_vec(),
            b"two\rlines".to_vec(),
            b"nul\0byte".to_vec(),
        ] {
            assert_eq!(
                PacketWebVttSettings::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let identifier_side_data = SideData::new_with_kind(
            PacketSideDataKind::WebVttIdentifier,
            b"00:00:00.000 --> 00:00:01.000".to_vec(),
        )
        .unwrap();
        assert_eq!(
            identifier_side_data.webvtt_identifier().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );

        let settings_side_data =
            SideData::new_with_kind(PacketSideDataKind::WebVttSettings, b"line:0\n".to_vec())
                .unwrap();
        assert_eq!(
            settings_side_data.webvtt_settings().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_active_format_description_payload() {
        let expected = [
            (PacketActiveFormatDescription::Same, 8, "AV_AFD_SAME"),
            (PacketActiveFormatDescription::FourThree, 9, "AV_AFD_4_3"),
            (
                PacketActiveFormatDescription::SixteenNine,
                10,
                "AV_AFD_16_9",
            ),
            (
                PacketActiveFormatDescription::FourteenNine,
                11,
                "AV_AFD_14_9",
            ),
            (
                PacketActiveFormatDescription::FourThreeProtectedFourteenNine,
                13,
                "AV_AFD_4_3_SP_14_9",
            ),
            (
                PacketActiveFormatDescription::SixteenNineProtectedFourteenNine,
                14,
                "AV_AFD_16_9_SP_14_9",
            ),
            (
                PacketActiveFormatDescription::ProtectedFourThree,
                15,
                "AV_AFD_SP_4_3",
            ),
        ];

        for (value, byte, ffmpeg_constant) in expected {
            assert_eq!(value.as_byte(), byte);
            assert_eq!(value.ffmpeg_constant(), ffmpeg_constant);
            assert_eq!(
                PacketActiveFormatDescription::from_byte(byte).unwrap(),
                value
            );
            assert_eq!(
                PacketActiveFormatDescription::parse(&[byte]).unwrap(),
                value
            );
            assert_eq!(value.to_bytes(), [byte]);

            let side_data = SideData::new_active_format_description(value).unwrap();
            assert_eq!(
                side_data.kind_id(),
                &PacketSideDataKind::ActiveFormatDescription
            );
            assert_eq!(side_data.data(), &[byte]);
            assert_eq!(side_data.active_format_description().unwrap(), Some(value));
        }

        let replay_gain = SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![8]).unwrap();
        assert_eq!(replay_gain.active_format_description().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_active_format_description_payload() {
        let bad_lengths: [&[u8]; 2] = [&[], &[8, 9]];
        for data in bad_lengths {
            assert_eq!(
                PacketActiveFormatDescription::parse(data)
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::ActiveFormatDescription, data.to_vec())
                    .unwrap();
            assert_eq!(
                side_data.active_format_description().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        for byte in [0, 12, 255] {
            assert_eq!(
                PacketActiveFormatDescription::from_byte(byte)
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::ActiveFormatDescription, vec![byte])
                    .unwrap();
            assert_eq!(
                side_data.active_format_description().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_parses_s12m_timecode_payload() {
        let expected = PacketS12mTimecode::new(&[0x0102_0304, 0xA0B0_C0D0]).unwrap();
        assert_eq!(expected.count(), 2);
        assert_eq!(expected.timecodes(), &[0x0102_0304, 0xA0B0_C0D0]);
        assert_eq!(expected.raw_words(), [2, 0x0102_0304, 0xA0B0_C0D0, 0]);

        let side_data = SideData::new_s12m_timecode(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::S12mTimecode);
        assert_eq!(side_data.data(), &expected.to_bytes()[..]);
        assert_eq!(side_data.s12m_timecode().unwrap(), Some(expected));

        let raw_with_unused =
            PacketS12mTimecode::from_raw_words([1, 0x0A0B_0C0D, 0xFEED_C0DE, 0x1234_5678]).unwrap();
        assert_eq!(raw_with_unused.count(), 1);
        assert_eq!(raw_with_unused.timecodes(), &[0x0A0B_0C0D]);
        assert_eq!(
            raw_with_unused.raw_words(),
            [1, 0x0A0B_0C0D, 0xFEED_C0DE, 0x1234_5678]
        );
        assert_eq!(
            PacketS12mTimecode::parse(&raw_with_unused.to_bytes()).unwrap(),
            raw_with_unused
        );

        let afd = SideData::new_with_kind(
            PacketSideDataKind::ActiveFormatDescription,
            expected.to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(afd.s12m_timecode().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_s12m_timecode_payload() {
        assert_eq!(
            PacketS12mTimecode::new(&[]).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PacketS12mTimecode::new(&[1, 2, 3, 4]).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );

        for data in [Vec::new(), vec![0; 15], vec![0; 17]] {
            assert_eq!(
                PacketS12mTimecode::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::S12mTimecode, data).unwrap();
            assert_eq!(
                side_data.s12m_timecode().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        for count in [0, 4, u32::MAX] {
            let words = [count, 1, 2, 3];
            assert_eq!(
                PacketS12mTimecode::from_raw_words(words)
                    .unwrap_err()
                    .kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data = SideData::new_with_kind(
                PacketSideDataKind::S12mTimecode,
                PacketS12mTimecode { words }.to_bytes().to_vec(),
            )
            .unwrap();
            assert_eq!(
                side_data.s12m_timecode().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn packet_side_data_parses_frame_cropping_payload() {
        let expected = PacketFrameCropping::new(1, 2, 3, 4);
        let expected_bytes = [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0];

        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            PacketFrameCropping::parse(&expected_bytes).unwrap(),
            expected
        );
        assert_eq!(expected.crop_top(), 1);
        assert_eq!(expected.crop_bottom(), 2);
        assert_eq!(expected.crop_left(), 3);
        assert_eq!(expected.crop_right(), 4);

        let side_data = SideData::new_frame_cropping(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::FrameCropping);
        assert_eq!(side_data.data(), &expected_bytes[..]);
        assert_eq!(side_data.frame_cropping().unwrap(), Some(expected));

        let skip_samples =
            SideData::new_with_kind(PacketSideDataKind::SkipSamples, expected_bytes.to_vec())
                .unwrap();
        assert_eq!(skip_samples.frame_cropping().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_frame_cropping_payload() {
        let valid = PacketFrameCropping::new(1, 2, 3, 4).to_bytes();

        assert_eq!(
            PacketFrameCropping::parse(&valid[..PacketFrameCropping::DATA_LEN - 1])
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidData
        );
        assert_eq!(
            PacketFrameCropping::parse(&[0; PacketFrameCropping::DATA_LEN + 1])
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidData
        );

        let side_data =
            SideData::new_with_kind(PacketSideDataKind::FrameCropping, vec![0; 4]).unwrap();
        assert_eq!(
            side_data.frame_cropping().unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_display_matrix_payload() {
        let identity = PacketDisplayMatrix::identity();
        let mut expected_bytes = [0; PacketDisplayMatrix::DATA_LEN];
        for (element, chunk) in identity
            .as_elements()
            .iter()
            .zip(expected_bytes.chunks_exact_mut(4))
        {
            chunk.copy_from_slice(&element.to_ne_bytes());
        }

        assert_eq!(PacketDisplayMatrix::ELEMENTS, 9);
        assert_eq!(PacketDisplayMatrix::DATA_LEN, 36);
        assert_eq!(
            identity.elements(),
            [1 << 16, 0, 0, 0, 1 << 16, 0, 0, 0, 1 << 30]
        );
        assert_eq!(identity.to_bytes(), expected_bytes);
        assert_eq!(
            PacketDisplayMatrix::parse(&expected_bytes).unwrap(),
            identity
        );

        let raw_values = PacketDisplayMatrix::new([
            i32::MIN,
            -1,
            0,
            1,
            1 << 16,
            -(1 << 16),
            1 << 30,
            -(1 << 30),
            i32::MAX,
        ]);
        assert_eq!(
            PacketDisplayMatrix::parse(&raw_values.to_bytes()).unwrap(),
            raw_values
        );

        let side_data = SideData::new_display_matrix(identity).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::DisplayMatrix);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.display_matrix().unwrap(), Some(identity));

        let lcevc = SideData::new_lcevc(expected_bytes.to_vec()).unwrap();
        assert_eq!(lcevc.display_matrix().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_display_matrix_payload() {
        for data in [
            Vec::new(),
            vec![0; PacketDisplayMatrix::DATA_LEN - 1],
            vec![0; PacketDisplayMatrix::DATA_LEN + 1],
        ] {
            assert_eq!(
                PacketDisplayMatrix::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::DisplayMatrix, data).unwrap();
            assert_eq!(
                side_data.display_matrix().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let frame_cropping =
            SideData::new_with_kind(PacketSideDataKind::FrameCropping, vec![0; 36]).unwrap();
        assert_eq!(frame_cropping.display_matrix().unwrap(), None);
    }

    #[test]
    fn packet_side_data_parses_stereo3d_payload() {
        let expected = PacketStereo3d::new(
            PacketStereo3dType::SideBySide,
            PacketStereo3dFlags::INVERT,
            PacketStereo3dView::Right,
            PacketStereo3dPrimaryEye::Left,
            63_500,
            Rational::from_raw(-1, 2),
            Rational::from_raw(90, 1),
        )
        .unwrap();
        let mut expected_bytes = [0; PacketStereo3d::DATA_LEN];
        expected_bytes[PacketStereo3d::TYPE_OFFSET..PacketStereo3d::TYPE_OFFSET + 4]
            .copy_from_slice(&1i32.to_ne_bytes());
        expected_bytes[PacketStereo3d::FLAGS_OFFSET..PacketStereo3d::FLAGS_OFFSET + 4]
            .copy_from_slice(&1i32.to_ne_bytes());
        expected_bytes[PacketStereo3d::VIEW_OFFSET..PacketStereo3d::VIEW_OFFSET + 4]
            .copy_from_slice(&2i32.to_ne_bytes());
        expected_bytes[PacketStereo3d::PRIMARY_EYE_OFFSET..PacketStereo3d::PRIMARY_EYE_OFFSET + 4]
            .copy_from_slice(&1i32.to_ne_bytes());
        expected_bytes[PacketStereo3d::BASELINE_OFFSET..PacketStereo3d::BASELINE_OFFSET + 4]
            .copy_from_slice(&63_500u32.to_ne_bytes());
        expected_bytes[PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET
            ..PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET + 4]
            .copy_from_slice(&(-1i32).to_ne_bytes());
        expected_bytes[PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET + 4
            ..PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET + 8]
            .copy_from_slice(&2i32.to_ne_bytes());
        expected_bytes[PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET
            ..PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET + 4]
            .copy_from_slice(&90i32.to_ne_bytes());
        expected_bytes[PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET + 4
            ..PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET + 8]
            .copy_from_slice(&1i32.to_ne_bytes());

        assert_eq!(PacketStereo3d::RATIONAL_LEN, 8);
        assert_eq!(PacketStereo3d::DATA_LEN, 36);
        assert_eq!(PacketStereo3d::TYPE_OFFSET, 0);
        assert_eq!(PacketStereo3d::FLAGS_OFFSET, 4);
        assert_eq!(PacketStereo3d::VIEW_OFFSET, 8);
        assert_eq!(PacketStereo3d::PRIMARY_EYE_OFFSET, 12);
        assert_eq!(PacketStereo3d::BASELINE_OFFSET, 16);
        assert_eq!(PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET, 20);
        assert_eq!(PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET, 28);
        assert_eq!(expected.stereo_type(), PacketStereo3dType::SideBySide);
        assert_eq!(
            expected.stereo_type().ffmpeg_constant(),
            "AV_STEREO3D_SIDEBYSIDE"
        );
        assert_eq!(expected.flags(), PacketStereo3dFlags::INVERT);
        assert!(expected.has_inverted_views());
        assert_eq!(expected.view(), PacketStereo3dView::Right);
        assert_eq!(expected.view().ffmpeg_constant(), "AV_STEREO3D_VIEW_RIGHT");
        assert_eq!(expected.primary_eye(), PacketStereo3dPrimaryEye::Left);
        assert_eq!(
            expected.primary_eye().ffmpeg_constant(),
            "AV_PRIMARY_EYE_LEFT"
        );
        assert_eq!(expected.baseline(), 63_500);
        assert_eq!(
            expected.horizontal_disparity_adjustment(),
            Rational::from_raw(-1, 2)
        );
        assert_eq!(
            expected.horizontal_field_of_view(),
            Rational::from_raw(90, 1)
        );
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(PacketStereo3d::parse(&expected_bytes).unwrap(), expected);

        let unset_rationals = PacketStereo3d::new(
            PacketStereo3dType::TwoDimensional,
            PacketStereo3dFlags::EMPTY,
            PacketStereo3dView::Packed,
            PacketStereo3dPrimaryEye::None,
            0,
            Rational::from_raw(0, 0),
            Rational::from_raw(0, 0),
        )
        .unwrap();
        assert_eq!(
            PacketStereo3d::parse(&unset_rationals.to_bytes()).unwrap(),
            unset_rationals
        );

        let side_data = SideData::new_stereo3d(expected).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::Stereo3d);
        assert_eq!(side_data.data(), expected_bytes.as_slice());
        assert_eq!(side_data.stereo3d().unwrap(), Some(expected));

        let display_matrix =
            SideData::new_with_kind(PacketSideDataKind::DisplayMatrix, expected_bytes.to_vec())
                .unwrap();
        assert_eq!(display_matrix.stereo3d().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_stereo3d_payload() {
        let valid = PacketStereo3d::new(
            PacketStereo3dType::SideBySide,
            PacketStereo3dFlags::INVERT,
            PacketStereo3dView::Packed,
            PacketStereo3dPrimaryEye::Right,
            1,
            Rational::from_raw(0, 1),
            Rational::from_raw(45, 1),
        )
        .unwrap()
        .to_bytes();

        let mut invalid_payloads = Vec::new();
        invalid_payloads.push(Vec::new());
        invalid_payloads.push(valid[..PacketStereo3d::DATA_LEN - 1].to_vec());
        let mut long = valid.to_vec();
        long.push(0);
        invalid_payloads.push(long);

        for (offset, value) in [
            (PacketStereo3d::TYPE_OFFSET, 9_i32),
            (PacketStereo3d::FLAGS_OFFSET, 2_i32),
            (PacketStereo3d::FLAGS_OFFSET, -1_i32),
            (PacketStereo3d::VIEW_OFFSET, 4_i32),
            (PacketStereo3d::PRIMARY_EYE_OFFSET, 3_i32),
        ] {
            let mut invalid = valid;
            invalid[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
            invalid_payloads.push(invalid.to_vec());
        }

        let mut invalid_disparity = valid;
        invalid_disparity[PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET
            ..PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET + 4]
            .copy_from_slice(&2i32.to_ne_bytes());
        invalid_payloads.push(invalid_disparity.to_vec());

        let mut invalid_disparity_unset = valid;
        invalid_disparity_unset[PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET
            ..PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET + 4]
            .copy_from_slice(&1i32.to_ne_bytes());
        invalid_disparity_unset[PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET + 4
            ..PacketStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET + 8]
            .copy_from_slice(&0i32.to_ne_bytes());
        invalid_payloads.push(invalid_disparity_unset.to_vec());

        let mut invalid_fov_negative = valid;
        invalid_fov_negative[PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET
            ..PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET + 4]
            .copy_from_slice(&(-1i32).to_ne_bytes());
        invalid_payloads.push(invalid_fov_negative.to_vec());

        let mut invalid_fov_unset = valid;
        invalid_fov_unset[PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET
            ..PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET + 4]
            .copy_from_slice(&1i32.to_ne_bytes());
        invalid_fov_unset[PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET + 4
            ..PacketStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET + 8]
            .copy_from_slice(&0i32.to_ne_bytes());
        invalid_payloads.push(invalid_fov_unset.to_vec());

        for data in invalid_payloads {
            assert_eq!(
                PacketStereo3d::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data = SideData::new_with_kind(PacketSideDataKind::Stereo3d, data).unwrap();
            assert_eq!(
                side_data.stereo3d().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            PacketStereo3dType::from_raw(9).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
        assert_eq!(
            PacketStereo3dFlags::from_bits(2).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
        assert_eq!(
            PacketStereo3dFlags::from_raw(-1).unwrap_err().kind(),
            crate::AvErrorKind::InvalidData
        );
        assert_eq!(
            PacketStereo3d::new(
                PacketStereo3dType::SideBySide,
                PacketStereo3dFlags::EMPTY,
                PacketStereo3dView::Packed,
                PacketStereo3dPrimaryEye::None,
                0,
                Rational::from_raw(2, 1),
                Rational::from_raw(0, 1),
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packet_side_data_parses_audio_service_type_payload() {
        let expected = [
            (
                PacketAudioServiceType::Main,
                0,
                "AV_AUDIO_SERVICE_TYPE_MAIN",
            ),
            (
                PacketAudioServiceType::Effects,
                1,
                "AV_AUDIO_SERVICE_TYPE_EFFECTS",
            ),
            (
                PacketAudioServiceType::VisuallyImpaired,
                2,
                "AV_AUDIO_SERVICE_TYPE_VISUALLY_IMPAIRED",
            ),
            (
                PacketAudioServiceType::HearingImpaired,
                3,
                "AV_AUDIO_SERVICE_TYPE_HEARING_IMPAIRED",
            ),
            (
                PacketAudioServiceType::Dialogue,
                4,
                "AV_AUDIO_SERVICE_TYPE_DIALOGUE",
            ),
            (
                PacketAudioServiceType::Commentary,
                5,
                "AV_AUDIO_SERVICE_TYPE_COMMENTARY",
            ),
            (
                PacketAudioServiceType::Emergency,
                6,
                "AV_AUDIO_SERVICE_TYPE_EMERGENCY",
            ),
            (
                PacketAudioServiceType::VoiceOver,
                7,
                "AV_AUDIO_SERVICE_TYPE_VOICE_OVER",
            ),
            (
                PacketAudioServiceType::Karaoke,
                8,
                "AV_AUDIO_SERVICE_TYPE_KARAOKE",
            ),
        ];

        assert_eq!(PacketAudioServiceType::KNOWN.len(), expected.len());
        for ((value, raw, constant), known) in
            expected.into_iter().zip(PacketAudioServiceType::KNOWN)
        {
            assert_eq!(known, value);
            assert_eq!(value.as_raw(), raw);
            assert_eq!(value.ffmpeg_constant(), constant);
            assert_eq!(value.to_bytes(), raw.to_ne_bytes());
            assert_eq!(PacketAudioServiceType::from_raw(raw).unwrap(), value);
            assert_eq!(
                PacketAudioServiceType::parse(&value.to_bytes()).unwrap(),
                value
            );

            let side_data = SideData::new_audio_service_type(value).unwrap();
            assert_eq!(side_data.kind_id(), &PacketSideDataKind::AudioServiceType);
            assert_eq!(side_data.data(), value.to_bytes().as_slice());
            assert_eq!(side_data.audio_service_type().unwrap(), Some(value));
        }

        let display_matrix =
            SideData::new_with_kind(PacketSideDataKind::DisplayMatrix, vec![0; 4]).unwrap();
        assert_eq!(display_matrix.audio_service_type().unwrap(), None);
    }

    #[test]
    fn packet_side_data_rejects_malformed_audio_service_type_payload() {
        for data in [
            Vec::new(),
            vec![0; PacketAudioServiceType::DATA_LEN - 1],
            vec![0; PacketAudioServiceType::DATA_LEN + 1],
            (-1_i32).to_ne_bytes().to_vec(),
            9_i32.to_ne_bytes().to_vec(),
            i32::MAX.to_ne_bytes().to_vec(),
        ] {
            assert_eq!(
                PacketAudioServiceType::parse(&data).unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
            let side_data =
                SideData::new_with_kind(PacketSideDataKind::AudioServiceType, data).unwrap();
            assert_eq!(
                side_data.audio_service_type().unwrap_err().kind(),
                crate::AvErrorKind::InvalidData
            );
        }

        let lcevc = SideData::new_lcevc(vec![0; PacketAudioServiceType::DATA_LEN]).unwrap();
        assert_eq!(lcevc.audio_service_type().unwrap(), None);
    }

    #[test]
    fn packet_side_data_preserves_lcevc_payload() {
        let payload = vec![0x00, 0x00, 0x03, 0x7e, 0xaa, 0x00, 0x00, 0x03, 0xbb];
        let parsed = PacketLcevc::parse(&payload).unwrap();
        assert_eq!(parsed.data(), payload.as_slice());
        assert_eq!(parsed.len(), payload.len());
        assert!(!parsed.is_empty());

        let side_data = SideData::new_lcevc(payload.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &PacketSideDataKind::Lcevc);
        assert_eq!(side_data.data(), payload.as_slice());
        assert_eq!(side_data.lcevc().unwrap(), Some(parsed));

        let empty = SideData::new_lcevc(Vec::new()).unwrap();
        let empty_lcevc = empty.lcevc().unwrap().unwrap();
        assert_eq!(empty_lcevc.data(), &[]);
        assert_eq!(empty_lcevc.len(), 0);
        assert!(empty_lcevc.is_empty());

        let frame_cropping =
            SideData::new_with_kind(PacketSideDataKind::FrameCropping, payload).unwrap();
        assert_eq!(frame_cropping.lcevc().unwrap(), None);
    }

    #[test]
    fn packet_side_data_lookup_and_shrink_preserve_order() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.push_side_data(SideData::new("palette", vec![0, 1, 2, 3]).unwrap());
        packet.push_side_data(SideData::new("skip_samples", vec![4, 5, 6]).unwrap());
        packet.push_side_data(SideData::new("palette", vec![7, 8]).unwrap());

        assert_eq!(
            packet.side_data_by_kind("palette").unwrap().data(),
            &[0, 1, 2, 3]
        );
        assert_eq!(
            packet.side_data_by_kind("skip_samples").unwrap().data(),
            &[4, 5, 6]
        );
        assert!(packet.side_data_by_kind("missing").is_none());

        assert!(packet.shrink_side_data("palette", 2).unwrap());
        assert_eq!(packet.side_data_by_kind("palette").unwrap().data(), &[0, 1]);
        assert_eq!(packet.side_data()[1].kind(), "skip_samples");
        assert_eq!(packet.side_data()[2].data(), &[7, 8]);
        assert!(!packet.shrink_side_data("missing", 0).unwrap());
    }

    #[test]
    fn packet_side_data_shrink_errors_do_not_mutate_payload() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.push_side_data(SideData::new("palette", vec![0, 1, 2]).unwrap());

        let err = packet.shrink_side_data("palette", 4).unwrap_err();

        assert_eq!(err.kind(), crate::AvErrorKind::External);
        assert_eq!(err.code(), Some(crate::AvErrorCode::ENOMEM));
        assert_eq!(
            packet.side_data_by_kind("palette").unwrap().data(),
            &[0, 1, 2]
        );
    }

    #[test]
    fn packet_shrink_side_data_by_kind_id_reports_ffmpeg_errors() {
        let mut packet = Packet::new(Vec::new(), 0);

        let missing = packet
            .shrink_side_data_by_kind_id(&PacketSideDataKind::Palette, 0)
            .unwrap_err();
        assert_eq!(missing.kind(), crate::AvErrorKind::NotFound);
        assert_eq!(missing.code(), Some(crate::AvErrorCode::ENOENT));

        packet.push_side_data(SideData::new("palette", vec![0, 1, 2]).unwrap());

        let too_large = packet
            .shrink_side_data_by_kind_id(&PacketSideDataKind::Palette, 4)
            .unwrap_err();
        assert_eq!(too_large.kind(), crate::AvErrorKind::External);
        assert_eq!(too_large.code(), Some(crate::AvErrorCode::ENOMEM));
        assert_eq!(
            packet.side_data_by_kind("palette").unwrap().data(),
            &[0, 1, 2]
        );

        packet
            .shrink_side_data_by_kind_id(&PacketSideDataKind::Palette, 1)
            .unwrap();
        assert_eq!(packet.side_data_by_kind("palette").unwrap().data(), &[0]);
    }

    #[test]
    fn packet_side_data_take_remove_and_clear_are_scoped() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.push_side_data(SideData::new("palette", vec![0]).unwrap());
        packet.push_side_data(SideData::new("skip_samples", vec![1]).unwrap());
        packet.push_side_data(SideData::new("palette", vec![2]).unwrap());

        let taken = packet.take_side_data("palette").unwrap();
        assert_eq!(taken.data(), &[0]);
        assert_eq!(packet.side_data().len(), 2);
        assert_eq!(packet.side_data_by_kind("palette").unwrap().data(), &[2]);

        assert!(packet.remove_side_data("skip_samples"));
        assert!(!packet.remove_side_data("missing"));
        assert_eq!(packet.side_data().len(), 1);

        packet.clear_side_data();
        assert!(packet.side_data().is_empty());
    }

    #[test]
    fn packet_new_side_data_zeroes_and_replaces_first_matching_kind() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.push_side_data(SideData::new("palette", vec![0xaa]).unwrap());
        packet.push_side_data(SideData::new("skip_samples", vec![0xbb]).unwrap());
        packet.push_side_data(SideData::new("palette", vec![0xcc]).unwrap());

        {
            let entry = packet
                .new_side_data(PacketSideDataKind::NewExtradata, 3)
                .unwrap();
            assert_eq!(entry.kind_id(), &PacketSideDataKind::NewExtradata);
            assert_eq!(entry.data(), &[0, 0, 0]);
            entry.data_mut().copy_from_slice(&[1, 2, 3]);
        }

        assert_eq!(packet.side_data().len(), 4);
        assert_eq!(
            packet.side_data_by_kind("new_extradata").unwrap().data(),
            &[1, 2, 3]
        );

        {
            let entry = packet
                .new_side_data(PacketSideDataKind::Palette, 2)
                .unwrap();
            assert_eq!(entry.kind_id(), &PacketSideDataKind::Palette);
            assert_eq!(entry.data(), &[0, 0]);
            entry.data_mut().copy_from_slice(&[9, 8]);
        }

        assert_eq!(packet.side_data().len(), 4);
        assert_eq!(packet.side_data()[0].kind(), "palette");
        assert_eq!(packet.side_data()[0].data(), &[9, 8]);
        assert_eq!(packet.side_data()[1].kind(), "skip_samples");
        assert_eq!(packet.side_data()[2].kind(), "palette");
        assert_eq!(packet.side_data()[2].data(), &[0xcc]);
    }

    #[test]
    fn packet_new_side_data_accepts_zero_size() {
        let mut packet = Packet::new(Vec::new(), 0);
        let entry = packet
            .new_side_data(PacketSideDataKind::NewExtradata, 0)
            .unwrap();

        assert_eq!(entry.kind_id(), &PacketSideDataKind::NewExtradata);
        assert_eq!(entry.len(), 0);
        assert!(entry.data().is_empty());
        assert_eq!(packet.side_data().len(), 1);
        assert!(packet
            .side_data_by_kind_id(&PacketSideDataKind::NewExtradata)
            .unwrap()
            .data()
            .is_empty());
    }

    #[test]
    fn packet_add_side_data_replaces_first_matching_kind() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.push_side_data(SideData::new("palette", vec![0]).unwrap());
        packet.push_side_data(SideData::new("skip_samples", vec![1]).unwrap());
        packet.push_side_data(SideData::new("palette", vec![2]).unwrap());

        let replaced = packet
            .add_side_data(SideData::new("palette", vec![9, 8]).unwrap())
            .unwrap();

        assert_eq!(replaced.data(), &[0]);
        assert_eq!(packet.side_data().len(), 3);
        assert_eq!(packet.side_data()[0].kind(), "palette");
        assert_eq!(packet.side_data()[0].data(), &[9, 8]);
        assert_eq!(packet.side_data()[1].kind(), "skip_samples");
        assert_eq!(packet.side_data()[2].kind(), "palette");
        assert_eq!(packet.side_data()[2].data(), &[2]);

        assert!(packet
            .add_side_data(SideData::new("new_extradata", vec![7]).unwrap())
            .is_none());
        assert_eq!(packet.side_data().len(), 4);
        assert_eq!(
            packet.side_data_by_kind("new_extradata").unwrap().data(),
            &[7]
        );
    }

    #[test]
    fn packet_try_add_side_data_reports_ffmpeg_entry_limit() {
        let mut packet = Packet::new(Vec::new(), 0);
        for (index, kind) in PacketSideDataKind::KNOWN.iter().enumerate() {
            assert!(packet
                .try_add_side_data(
                    SideData::new_with_kind(kind.clone(), vec![index as u8]).unwrap()
                )
                .unwrap()
                .is_none());
        }

        assert_eq!(
            packet.side_data().len(),
            PacketSideDataKind::MAX_FFMPEG_PACKET_SIDE_DATA_ELEMS
        );

        let replaced = packet
            .try_add_side_data(
                SideData::new_with_kind(PacketSideDataKind::Palette, vec![0xaa]).unwrap(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(replaced.data(), &[0]);
        assert_eq!(
            packet.side_data().len(),
            PacketSideDataKind::MAX_FFMPEG_PACKET_SIDE_DATA_ELEMS
        );
        assert_eq!(
            packet
                .side_data_by_kind_id(&PacketSideDataKind::Palette)
                .unwrap()
                .data(),
            &[0xaa]
        );

        let err = packet
            .try_add_side_data(
                SideData::new("vendor.private.extra_packet_data", vec![0xee]).unwrap(),
            )
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(err.code(), Some(AvErrorCode::from_posix_errno(34)));
        assert_eq!(
            packet.side_data().len(),
            PacketSideDataKind::MAX_FFMPEG_PACKET_SIDE_DATA_ELEMS
        );
        assert!(packet
            .side_data_by_kind("vendor.private.extra_packet_data")
            .is_none());

        let err = packet
            .new_side_data(
                PacketSideDataKind::Unknown("vendor.private.new_packet_data".to_string()),
                1,
            )
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(err.code(), Some(AvErrorCode::from_posix_errno(34)));
        assert_eq!(
            packet.side_data().len(),
            PacketSideDataKind::MAX_FFMPEG_PACKET_SIDE_DATA_ELEMS
        );
    }

    #[test]
    fn packet_side_data_list_matches_standalone_array_lifecycle() {
        let mut list = PacketSideDataList::new();
        assert!(list.is_empty());

        let entry = list
            .new_side_data(PacketSideDataKind::NewExtradata, 0)
            .unwrap();
        assert_eq!(entry.kind_id(), &PacketSideDataKind::NewExtradata);
        assert!(entry.data().is_empty());
        assert_eq!(list.len(), 1);
        list.clear();
        assert!(list.is_empty());

        let entry = list
            .new_side_data(PacketSideDataKind::NewExtradata, 4)
            .unwrap();
        entry.data_mut().copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(list.len(), 1);
        assert_eq!(
            list.get(&PacketSideDataKind::NewExtradata).unwrap().data(),
            &[0x11, 0x22, 0x33, 0x44]
        );

        let entry = list
            .new_side_data(PacketSideDataKind::NewExtradata, 2)
            .unwrap();
        entry.data_mut().copy_from_slice(&[0xaa, 0xbb]);
        assert_eq!(list.entries().len(), 1);
        assert_eq!(
            list.get(&PacketSideDataKind::NewExtradata).unwrap().data(),
            &[0xaa, 0xbb]
        );

        let replaced = list
            .add_side_data(SideData::new_extradata(vec![0x55, 0x66, 0x77]).unwrap())
            .unwrap();
        assert_eq!(replaced.data(), &[0xaa, 0xbb]);
        assert_eq!(list.entries().len(), 1);
        assert_eq!(
            list.get(&PacketSideDataKind::NewExtradata).unwrap().data(),
            &[0x55, 0x66, 0x77]
        );

        assert!(list
            .add_side_data(
                SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x99]).unwrap()
            )
            .is_none());
        assert_eq!(list.entries().len(), 2);
        assert_eq!(
            list.entries()[0].kind_id(),
            &PacketSideDataKind::NewExtradata
        );
        assert_eq!(list.entries()[1].kind_id(), &PacketSideDataKind::Palette);

        let removed = list.remove_kind(&PacketSideDataKind::NewExtradata).unwrap();
        assert_eq!(removed.data(), &[0x55, 0x66, 0x77]);
        assert_eq!(list.entries().len(), 1);
        assert_eq!(list.entries()[0].kind_id(), &PacketSideDataKind::Palette);
        assert_eq!(list.entries()[0].data(), &[0x99]);
        assert!(list
            .remove_kind(&PacketSideDataKind::NewExtradata)
            .is_none());

        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn packet_side_data_list_remove_uses_last_match_swap_semantics() {
        let mut list = PacketSideDataList::from_entries(vec![
            SideData::new_extradata(vec![0x00]).unwrap(),
            SideData::new_with_kind(PacketSideDataKind::Palette, vec![0x11]).unwrap(),
            SideData::new_extradata(vec![0x22]).unwrap(),
            SideData::new_with_kind(PacketSideDataKind::SkipSamples, vec![0x33]).unwrap(),
        ]);

        let removed = list.remove_kind(&PacketSideDataKind::NewExtradata).unwrap();

        assert_eq!(removed.data(), &[0x22]);
        assert_eq!(list.entries().len(), 3);
        assert_eq!(
            list.entries()[0].kind_id(),
            &PacketSideDataKind::NewExtradata
        );
        assert_eq!(list.entries()[0].data(), &[0x00]);
        assert_eq!(list.entries()[1].kind_id(), &PacketSideDataKind::Palette);
        assert_eq!(
            list.entries()[2].kind_id(),
            &PacketSideDataKind::SkipSamples
        );
        assert_eq!(list.entries()[2].data(), &[0x33]);
    }

    #[test]
    fn packet_side_data_maps_global_frame_side_data() {
        let expected = [
            (
                PacketSideDataKind::ReplayGain,
                FrameSideDataKind::ReplayGain,
            ),
            (
                PacketSideDataKind::DisplayMatrix,
                FrameSideDataKind::DisplayMatrix,
            ),
            (PacketSideDataKind::Spherical, FrameSideDataKind::Spherical),
            (PacketSideDataKind::Stereo3d, FrameSideDataKind::Stereo3d),
            (
                PacketSideDataKind::AudioServiceType,
                FrameSideDataKind::AudioServiceType,
            ),
            (
                PacketSideDataKind::MasteringDisplayMetadata,
                FrameSideDataKind::MasteringDisplayMetadata,
            ),
            (
                PacketSideDataKind::ContentLightLevel,
                FrameSideDataKind::ContentLightLevel,
            ),
            (
                PacketSideDataKind::IccProfile,
                FrameSideDataKind::IccProfile,
            ),
            (
                PacketSideDataKind::AmbientViewingEnvironment,
                FrameSideDataKind::AmbientViewingEnvironment,
            ),
            (
                PacketSideDataKind::ThreeDReferenceDisplays,
                FrameSideDataKind::ThreeDReferenceDisplays,
            ),
            (PacketSideDataKind::Exif, FrameSideDataKind::Exif),
        ];

        for (packet_kind, frame_kind) in expected {
            assert_eq!(packet_kind.frame_side_data_kind(), Some(frame_kind.clone()));
            assert_eq!(
                PacketSideDataKind::from_frame_side_data_kind(&frame_kind),
                Some(packet_kind)
            );
        }

        assert_eq!(
            PacketSideDataKind::NewExtradata.frame_side_data_kind(),
            None
        );
        assert_eq!(
            PacketSideDataKind::from_frame_side_data_kind(&FrameSideDataKind::A53ClosedCaptions),
            None
        );
    }

    #[test]
    fn packet_side_data_list_adds_from_frame_side_data_with_replacement() {
        let mut list = PacketSideDataList::new();
        let first =
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![1, 2, 3]).unwrap();
        list.add_from_frame_side_data(&first).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list.entries()[0].kind_id(), &PacketSideDataKind::ReplayGain);
        assert_eq!(list.entries()[0].data(), &[1, 2, 3]);

        let replacement =
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![9, 8]).unwrap();
        let entry = list.add_from_frame_side_data(&replacement).unwrap();
        assert_eq!(entry.kind_id(), &PacketSideDataKind::ReplayGain);
        assert_eq!(entry.data(), &[9, 8]);
        assert_eq!(list.len(), 1);

        let unmapped =
            FrameSideData::new_with_kind(FrameSideDataKind::A53ClosedCaptions, vec![0]).unwrap();
        let err = list.add_from_frame_side_data(&unmapped).unwrap_err();
        assert_eq!(err.kind(), crate::AvErrorKind::InvalidArgument);
        assert_eq!(err.code(), Some(AvErrorCode::EINVAL));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn packet_side_data_adds_to_frame_with_flags() {
        let source = SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![1]).unwrap();
        let mut frame = Frame::empty();

        source
            .add_to_frame(&mut frame, FrameSideDataFlags::EMPTY)
            .unwrap();
        assert_eq!(frame.side_data().len(), 1);
        assert_eq!(
            frame.side_data()[0].kind_id(),
            &FrameSideDataKind::ReplayGain
        );
        assert_eq!(frame.side_data()[0].data(), &[1]);

        let duplicate_err = source
            .add_to_frame(&mut frame, FrameSideDataFlags::EMPTY)
            .unwrap_err();
        assert_eq!(duplicate_err.kind(), crate::AvErrorKind::External);
        assert_eq!(duplicate_err.code(), Some(AvErrorCode::ENOMEM));
        assert_eq!(frame.side_data().len(), 1);
        assert_eq!(frame.side_data()[0].data(), &[1]);

        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![2, 3])
            .unwrap()
            .add_to_frame(&mut frame, FrameSideDataFlags::REPLACE)
            .unwrap();
        assert_eq!(frame.side_data().len(), 1);
        assert_eq!(frame.side_data()[0].data(), &[2, 3]);

        frame
            .add_side_data_with_flags(
                FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![4]).unwrap(),
                FrameSideDataFlags::EMPTY,
            )
            .unwrap();
        SideData::new_with_kind(PacketSideDataKind::ReplayGain, vec![5])
            .unwrap()
            .add_to_frame(&mut frame, FrameSideDataFlags::UNIQUE)
            .unwrap();
        assert_eq!(frame.side_data().len(), 2);
        assert_eq!(
            frame.side_data()[0].kind_id(),
            &FrameSideDataKind::DisplayMatrix
        );
        assert_eq!(
            frame.side_data()[1].kind_id(),
            &FrameSideDataKind::ReplayGain
        );
        assert_eq!(frame.side_data()[1].data(), &[5]);

        let unmapped = SideData::new_extradata(vec![0xaa]).unwrap();
        let err = unmapped
            .add_to_frame(&mut frame, FrameSideDataFlags::EMPTY)
            .unwrap_err();
        assert_eq!(err.kind(), crate::AvErrorKind::InvalidArgument);
        assert_eq!(err.code(), Some(AvErrorCode::EINVAL));
        assert_eq!(frame.side_data().len(), 2);
    }

    #[test]
    fn packet_from_data_and_zeroed_constructors_add_input_padding() {
        let packet = Packet::from_data(vec![0xaa, 0xbb]).unwrap();
        assert_eq!(packet.stream_index(), 0);
        assert_eq!(packet.data(), &[0xaa, 0xbb]);
        assert_eq!(
            packet.data_buffer().padding_len(),
            AV_INPUT_BUFFER_PADDING_SIZE
        );
        assert!(packet
            .data_buffer()
            .padding_slice()
            .iter()
            .all(|byte| *byte == 0));
        assert!(packet.is_data_writable());

        let zeroed = Packet::new_zeroed(3, 7).unwrap();
        assert_eq!(zeroed.stream_index(), 7);
        assert_eq!(zeroed.data(), &[0, 0, 0]);
        assert_eq!(
            zeroed.data_buffer().padding_len(),
            AV_INPUT_BUFFER_PADDING_SIZE
        );
        assert!(zeroed
            .data_buffer()
            .padding_slice()
            .iter()
            .all(|byte| *byte == 0));
    }

    #[test]
    fn packet_grow_and_shrink_data_preserve_payload_and_padding() {
        let mut packet = Packet::from_data(vec![0xaa, 0xbb]).unwrap();
        packet.grow_data(3).unwrap();
        assert_eq!(packet.data(), &[0xaa, 0xbb, 0, 0, 0]);
        assert_eq!(
            packet.data_buffer().padding_len(),
            AV_INPUT_BUFFER_PADDING_SIZE
        );
        assert!(packet
            .data_buffer()
            .padding_slice()
            .iter()
            .all(|byte| *byte == 0));

        packet.shrink_data(2).unwrap();
        assert_eq!(packet.data(), &[0xaa, 0xbb]);
        assert_eq!(
            packet.data_buffer().padding_len(),
            AV_INPUT_BUFFER_PADDING_SIZE
        );
        assert!(packet
            .data_buffer()
            .padding_slice()
            .iter()
            .all(|byte| *byte == 0));

        packet.shrink_data(99).unwrap();
        assert_eq!(packet.data(), &[0xaa, 0xbb]);
    }

    #[test]
    fn packet_unpadded_payload_helpers_add_padding_and_preserve_bytes() {
        let mut grown = Packet::new(vec![0xaa, 0xbb], 0);
        grown.grow_data(2).unwrap();
        assert_eq!(grown.data(), &[0xaa, 0xbb, 0, 0]);
        assert_eq!(
            grown.data_buffer().padding_len(),
            AV_INPUT_BUFFER_PADDING_SIZE
        );
        assert!(grown
            .data_buffer()
            .padding_slice()
            .iter()
            .all(|byte| *byte == 0));
        assert!(grown.is_data_writable());

        let mut writable = Packet::new(vec![0xaa, 0xbb], 0);
        writable.make_writable().unwrap();
        writable.make_data_writable()[0] = 0xcc;
        assert_eq!(writable.data(), &[0xcc, 0xbb]);
        assert_eq!(
            writable.data_buffer().padding_len(),
            AV_INPUT_BUFFER_PADDING_SIZE
        );
        assert!(writable
            .data_buffer()
            .padding_slice()
            .iter()
            .all(|byte| *byte == 0));
        assert!(writable.is_data_writable());
    }

    #[test]
    fn packet_make_writable_detaches_shared_payload_with_padding() {
        let src = Packet::from_data(vec![0xaa, 0xbb]).unwrap();
        let mut dst = Packet::default();
        dst.ref_from(&src);
        assert!(!dst.is_data_writable());
        assert!(dst.data_buffer().shares_storage(src.data_buffer()));

        dst.make_writable().unwrap();
        assert!(dst.is_data_writable());
        assert!(!dst.data_buffer().shares_storage(src.data_buffer()));
        assert_eq!(
            dst.data_buffer().padding_len(),
            AV_INPUT_BUFFER_PADDING_SIZE
        );

        dst.make_data_writable()[0] = 0xcc;
        assert_eq!(src.data(), &[0xaa, 0xbb]);
        assert_eq!(dst.data(), &[0xcc, 0xbb]);
    }

    #[test]
    fn packet_make_refcounted_adds_padding_without_detaching_padded_refs() {
        let mut unpadded = Packet::new(vec![0xaa, 0xbb], 0);
        unpadded.make_refcounted().unwrap();
        assert_eq!(unpadded.data(), &[0xaa, 0xbb]);
        assert_eq!(
            unpadded.data_buffer().padding_len(),
            AV_INPUT_BUFFER_PADDING_SIZE
        );
        assert!(unpadded.is_data_writable());

        let src = Packet::from_data(vec![0x11, 0x22]).unwrap();
        let mut dst = Packet::default();
        dst.ref_from(&src);
        assert!(!dst.is_data_writable());
        assert!(dst.data_buffer().shares_storage(src.data_buffer()));
        dst.make_refcounted().unwrap();
        assert!(!dst.is_data_writable());
        assert!(dst.data_buffer().shares_storage(src.data_buffer()));
    }

    #[test]
    fn packet_ref_from_shares_payload_and_copies_side_data() {
        let mut src = Packet::new(vec![1, 2, 3], 4);
        src.set_pts(Some(12));
        src.set_dts(Some(10));
        src.set_duration(2).unwrap();
        src.set_pos(Some(42)).unwrap();
        src.set_time_base(Rational::new(1, 90_000).unwrap())
            .unwrap();
        src.set_key(true);
        src.push_side_data(SideData::new("palette", vec![5, 6]).unwrap());
        src.set_opaque_address(0xfeed_cafe);
        src.set_opaque_ref(Some(BufferRef::from_vec(vec![0xde, 0xad])));

        let mut dst = Packet::new(vec![9], 99);
        dst.push_side_data(SideData::new("old", vec![8]).unwrap());
        dst.ref_from(&src);

        assert_eq!(dst.data(), &[1, 2, 3]);
        assert!(dst.data_buffer().shares_storage(src.data_buffer()));
        assert_eq!(dst.stream_index(), 4);
        assert_eq!(dst.pts(), Some(12));
        assert_eq!(dst.dts(), Some(10));
        assert_eq!(dst.duration(), 2);
        assert_eq!(dst.pos(), Some(42));
        assert_eq!(dst.time_base(), Rational::new(1, 90_000).unwrap());
        assert!(dst.flags().contains(PacketFlags::KEY));
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[5, 6]);
        assert_eq!(dst.opaque_address(), Some(0xfeed_cafe));
        assert_eq!(dst.opaque_ref().unwrap().as_slice(), &[0xde, 0xad]);
        assert!(dst
            .opaque_ref()
            .unwrap()
            .shares_storage(src.opaque_ref().unwrap()));

        dst.shrink_side_data("palette", 1).unwrap();
        dst.clear_opaque();
        dst.clear_opaque_ref();
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[5]);
        assert_eq!(src.side_data_by_kind("palette").unwrap().data(), &[5, 6]);
        assert!(dst.opaque().is_none());
        assert_eq!(src.opaque_address(), Some(0xfeed_cafe));
        assert!(dst.opaque_ref().is_none());
        assert!(src.opaque_ref().is_some());
    }

    #[test]
    fn packet_clone_matches_ref_from_shape() {
        let mut src = Packet::new(vec![1, 2, 3], 4);
        src.set_pts(Some(12));
        src.set_dts(Some(10));
        src.set_duration(2).unwrap();
        src.set_pos(Some(42)).unwrap();
        src.set_time_base(Rational::new(1, 90_000).unwrap())
            .unwrap();
        src.set_flag(PacketFlags::KEY, true);
        src.push_side_data(SideData::new("palette", vec![5, 6]).unwrap());
        src.set_opaque_address(0xfeed_cafe);
        src.set_opaque_ref(Some(BufferRef::from_vec(vec![0xde, 0xad])));

        let mut cloned = src.clone();

        assert_eq!(cloned.data(), &[1, 2, 3]);
        assert!(cloned.data_buffer().shares_storage(src.data_buffer()));
        assert_eq!(cloned.stream_index(), 4);
        assert_eq!(cloned.pts(), Some(12));
        assert_eq!(cloned.dts(), Some(10));
        assert_eq!(cloned.duration(), 2);
        assert_eq!(cloned.pos(), Some(42));
        assert_eq!(cloned.time_base(), Rational::new(1, 90_000).unwrap());
        assert!(cloned.flags().contains(PacketFlags::KEY));
        assert_eq!(cloned.side_data_by_kind("palette").unwrap().data(), &[5, 6]);
        assert_eq!(cloned.opaque_address(), Some(0xfeed_cafe));
        assert_eq!(cloned.opaque_ref().unwrap().as_slice(), &[0xde, 0xad]);
        assert!(cloned
            .opaque_ref()
            .unwrap()
            .shares_storage(src.opaque_ref().unwrap()));

        cloned.shrink_side_data("palette", 1).unwrap();
        cloned.make_data_writable()[0] = 9;
        cloned.clear_opaque_ref();

        assert_eq!(cloned.data(), &[9, 2, 3]);
        assert_eq!(src.data(), &[1, 2, 3]);
        assert!(!cloned.data_buffer().shares_storage(src.data_buffer()));
        assert_eq!(cloned.side_data_by_kind("palette").unwrap().data(), &[5]);
        assert_eq!(src.side_data_by_kind("palette").unwrap().data(), &[5, 6]);
        assert!(cloned.opaque_ref().is_none());
        assert!(src.opaque_ref().is_some());
    }

    #[test]
    fn packet_make_data_writable_detaches_shared_payload() {
        let src = Packet::new(vec![1, 2, 3], 0);
        let mut dst = Packet::default();
        dst.ref_from(&src);

        assert!(dst.data_buffer().shares_storage(src.data_buffer()));
        assert!(!dst.is_data_writable());
        assert!(dst.data_mut().is_none());

        dst.make_data_writable()[0] = 9;

        assert_eq!(dst.data(), &[9, 2, 3]);
        assert_eq!(src.data(), &[1, 2, 3]);
        assert!(!dst.data_buffer().shares_storage(src.data_buffer()));
        assert!(dst.is_data_writable());
        assert!(src.is_data_writable());
    }

    #[test]
    fn packet_zero_size_payload_helpers_keep_ffmpeg_padding() {
        fn assert_empty_padded_writable(packet: &Packet) {
            assert!(packet.is_empty());
            assert_eq!(
                packet.data_buffer().padding_len(),
                AV_INPUT_BUFFER_PADDING_SIZE
            );
            assert!(packet
                .data_buffer()
                .padding_slice()
                .iter()
                .all(|byte| *byte == 0));
            assert!(packet.is_data_writable());
        }

        let new_zero = Packet::new_zeroed(0, 0).unwrap();
        assert_empty_padded_writable(&new_zero);

        let from_zero_data = Packet::from_data(Vec::new()).unwrap();
        assert_empty_padded_writable(&from_zero_data);

        let mut refcounted = Packet::default();
        refcounted.make_refcounted().unwrap();
        assert_empty_padded_writable(&refcounted);

        let mut writable = Packet::default();
        writable.make_writable().unwrap();
        assert_empty_padded_writable(&writable);
    }

    #[test]
    fn packet_move_ref_and_unref_reset_packets_and_release_payloads() {
        let released = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let capture_old = Arc::clone(&released);
        let mut dst = Packet::with_buffer(
            BufferRef::from_vec_with_release_callback(vec![9], move |data| {
                capture_old.lock().unwrap().push(data);
            }),
            8,
        );
        dst.push_side_data(SideData::new("old", vec![0]).unwrap());
        dst.set_opaque_address(0x1111);

        let capture_src = Arc::clone(&released);
        let mut src = Packet::with_buffer(
            BufferRef::from_vec_with_release_callback(vec![1, 2], move |data| {
                capture_src.lock().unwrap().push(data);
            }),
            3,
        );
        src.set_pts(Some(7));
        src.set_duration(5).unwrap();
        src.set_time_base(Rational::new(1, 48_000).unwrap())
            .unwrap();
        src.push_side_data(SideData::new("palette", vec![4]).unwrap());
        src.set_opaque_address(0x2222);

        dst.move_ref_from(&mut src);

        assert_eq!(*released.lock().unwrap(), vec![vec![9]]);
        assert!(src.is_empty());
        assert_eq!(src.stream_index(), 0);
        assert_eq!(src.pts(), None);
        assert!(src.opaque().is_none());
        assert!(src.side_data().is_empty());
        assert_eq!(dst.data(), &[1, 2]);
        assert_eq!(dst.stream_index(), 3);
        assert_eq!(dst.pts(), Some(7));
        assert_eq!(dst.duration(), 5);
        assert_eq!(dst.time_base(), Rational::new(1, 48_000).unwrap());
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[4]);
        assert_eq!(dst.opaque_address(), Some(0x2222));

        dst.unref();

        assert_eq!(*released.lock().unwrap(), vec![vec![9], vec![1, 2]]);
        assert!(dst.is_empty());
        assert_eq!(dst.stream_index(), 0);
        assert_eq!(dst.pts(), None);
        assert_eq!(dst.dts(), None);
        assert_eq!(dst.duration(), 0);
        assert_eq!(dst.pos(), None);
        assert_eq!(dst.time_base(), Rational::ZERO);
        assert!(dst.opaque().is_none());
        assert!(dst.flags().is_empty());
        assert!(dst.side_data().is_empty());
    }

    #[test]
    fn packet_legacy_init_resets_fields_but_preserves_payload() {
        let mut packet = Packet::from_data(vec![1, 2, 3]).unwrap();
        packet.set_pts(Some(12));
        packet.set_dts(Some(10));
        packet.set_duration(5).unwrap();
        packet.set_pos(Some(42)).unwrap();
        packet
            .set_time_base(Rational::new(1, 90_000).unwrap())
            .unwrap();
        packet.set_key(true);
        packet.set_flag(PacketFlags::CORRUPT, true);
        packet.push_side_data(SideData::new("palette", vec![4, 5]).unwrap());
        packet.set_opaque_address(0xfeed);
        packet.set_opaque_ref(Some(BufferRef::from_vec(vec![0xde, 0xad])));

        packet.init_legacy();

        assert_eq!(packet.data(), &[1, 2, 3]);
        assert_eq!(packet.stream_index(), 0);
        assert_eq!(packet.pts(), None);
        assert_eq!(packet.dts(), None);
        assert_eq!(packet.duration(), 0);
        assert_eq!(packet.pos(), None);
        assert_eq!(packet.time_base(), Rational::ZERO);
        assert!(packet.flags().is_empty());
        assert!(packet.side_data().is_empty());
        assert!(packet.opaque().is_none());
        assert!(packet.opaque_ref().is_none());
    }

    #[test]
    fn packet_fifo_moves_refs_peeks_reads_and_drains_packets() {
        let mut fifo = PacketFifo::new();
        assert!(fifo.is_empty());
        assert_eq!(fifo.can_read(), 0);

        let mut moved = Packet::from_data(vec![0xaa, 0xbb, 0xcc]).unwrap();
        moved.set_pts(Some(90_000));
        moved.set_dts(Some(45_000));
        moved.set_duration(180_000).unwrap();
        moved.set_pos(Some(1_234)).unwrap();
        moved.set_flag(PacketFlags::KEY, true);
        moved.set_flag(PacketFlags::CORRUPT, true);
        moved
            .set_time_base(Rational::new(1, 90_000).unwrap())
            .unwrap();
        moved.push_side_data(SideData::new_extradata(vec![0x11, 0x22, 0x33]).unwrap());
        moved.set_opaque_address(0x1234);
        moved.set_opaque_ref(Some(BufferRef::from_vec(vec![0xde, 0xad, 0xbe])));

        let moved_payload = moved.data_buffer().clone();
        fifo.write_move(&mut moved).unwrap();
        assert!(moved.is_empty());
        assert_eq!(moved.stream_index(), 0);
        assert_eq!(moved.pts(), None);
        assert!(moved.side_data().is_empty());
        assert_eq!(fifo.can_read(), 1);

        let peeked = fifo.peek(0).unwrap();
        assert_eq!(peeked.data(), &[0xaa, 0xbb, 0xcc]);
        assert!(peeked.data_buffer().shares_storage(&moved_payload));
        assert_eq!(peeked.pts(), Some(90_000));
        assert_eq!(peeked.side_data()[0].data(), &[0x11, 0x22, 0x33]);
        let peek_err = fifo.peek(1).unwrap_err();
        assert_eq!(peek_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(peek_err.code(), Some(AvErrorCode::EINVAL));

        let mut move_dst = Packet::new(vec![0x44], 9);
        fifo.read_move(&mut move_dst).unwrap();
        assert_eq!(fifo.can_read(), 0);
        assert_eq!(move_dst.data(), &[0xaa, 0xbb, 0xcc]);
        assert!(move_dst.data_buffer().shares_storage(&moved_payload));
        assert_eq!(move_dst.pts(), Some(90_000));

        let mut source = Packet::from_data(vec![0x10, 0x20]).unwrap();
        source.set_pts(Some(33));
        source.set_duration(44).unwrap();
        source.set_opaque_ref(Some(BufferRef::from_vec(vec![0xee])));
        let source_payload = source.data_buffer().clone();
        fifo.write_ref(&source).unwrap();
        assert_eq!(source.data(), &[0x10, 0x20]);
        assert_eq!(source.pts(), Some(33));
        assert_eq!(fifo.can_read(), 1);
        assert!(fifo
            .peek(0)
            .unwrap()
            .data_buffer()
            .shares_storage(&source_payload));

        let mut ref_dst = Packet::default();
        fifo.read_ref(&mut ref_dst).unwrap();
        assert_eq!(fifo.can_read(), 0);
        assert_eq!(source.data(), &[0x10, 0x20]);
        assert_eq!(ref_dst.data(), &[0x10, 0x20]);
        assert!(ref_dst.data_buffer().shares_storage(source.data_buffer()));
        assert!(ref_dst
            .opaque_ref()
            .unwrap()
            .shares_storage(source.opaque_ref().unwrap()));

        let mut first = Packet::new(vec![1], 1);
        let mut second = Packet::new(vec![2], 2);
        fifo.write_move(&mut first).unwrap();
        fifo.write_move(&mut second).unwrap();
        assert_eq!(fifo.can_read(), 2);
        fifo.drain(1).unwrap();
        assert_eq!(fifo.can_read(), 1);
        assert_eq!(fifo.peek(0).unwrap().data(), &[2]);
        fifo.drain(1).unwrap();
        assert!(fifo.is_empty());

        let mut empty_dst = Packet::default();
        let read_err = fifo.read_move(&mut empty_dst).unwrap_err();
        assert_eq!(read_err.code(), Some(AvErrorCode::EAGAIN));
        let drain_err = fifo.drain(1).unwrap_err();
        assert_eq!(drain_err.code(), Some(AvErrorCode::EINVAL));
    }

    #[test]
    fn packet_copy_props_preserves_destination_payload() {
        let mut src = Packet::new(vec![1, 2, 3], 4);
        src.set_pts(Some(12));
        src.set_dts(Some(10));
        src.set_duration(2).unwrap();
        src.set_pos(Some(42)).unwrap();
        src.set_time_base(Rational::new(1, 90_000).unwrap())
            .unwrap();
        src.set_key(true);
        src.set_flag(PacketFlags::CORRUPT, true);
        src.push_side_data(SideData::new("palette", vec![5, 6]).unwrap());
        src.set_opaque(Some(PacketOpaque::new(0x3333).unwrap()));

        let mut dst = Packet::new(vec![9, 8], 99);
        dst.set_pts(Some(99));
        dst.set_duration(9).unwrap();
        dst.set_time_base(Rational::new(1, 1_000).unwrap()).unwrap();
        dst.push_side_data(SideData::new("old", vec![7]).unwrap());

        dst.copy_props_from(&src);

        assert_eq!(dst.data(), &[9, 8]);
        assert!(!dst.data_buffer().shares_storage(src.data_buffer()));
        assert_eq!(dst.stream_index(), 4);
        assert_eq!(dst.pts(), Some(12));
        assert_eq!(dst.dts(), Some(10));
        assert_eq!(dst.duration(), 2);
        assert_eq!(dst.pos(), Some(42));
        assert_eq!(dst.time_base(), Rational::new(1, 90_000).unwrap());
        assert!(dst.flags().contains(PacketFlags::KEY));
        assert!(dst.flags().contains(PacketFlags::CORRUPT));
        assert!(dst.side_data_by_kind("old").is_none());
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[5, 6]);
        assert_eq!(dst.opaque_address(), Some(0x3333));
    }

    #[test]
    fn packet_copy_props_copies_side_data_without_aliasing() {
        let mut src = Packet::new(vec![1], 0);
        src.push_side_data(SideData::new("palette", vec![5, 6]).unwrap());

        let mut dst = Packet::new(vec![9], 1);
        dst.copy_props_from(&src);
        dst.shrink_side_data("palette", 1).unwrap();

        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[5]);
        assert_eq!(src.side_data_by_kind("palette").unwrap().data(), &[5, 6]);
    }

    #[test]
    fn packet_opaque_address_tracks_nullable_raw_pointer_metadata() {
        assert_eq!(
            PacketOpaque::new(0).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(PacketOpaque::from_address(0), None);

        let opaque = PacketOpaque::new(0x1234).unwrap();
        assert_eq!(opaque.address(), 0x1234);
        assert_eq!(opaque.nonzero_address().get(), 0x1234);
        assert_eq!(PacketOpaque::from_address(0x1234), Some(opaque));
        assert_eq!(PacketOpaque::from_nonzero(opaque.nonzero_address()), opaque);

        let mut src = Packet::new(vec![1], 0);
        src.set_opaque(Some(opaque));
        assert_eq!(src.opaque(), Some(opaque));
        assert_eq!(src.opaque_address(), Some(0x1234));

        let mut dst = Packet::new(vec![9], 1);
        dst.copy_props_from(&src);
        assert_eq!(dst.data(), &[9]);
        assert_eq!(dst.opaque(), Some(opaque));

        dst.set_opaque_address(0);
        assert!(dst.opaque().is_none());
        dst.set_opaque_address(0xabcd);
        assert_eq!(dst.opaque_address(), Some(0xabcd));

        let taken = dst.take_opaque().unwrap();
        assert_eq!(taken.address(), 0xabcd);
        assert!(dst.opaque().is_none());
        assert_eq!(src.opaque(), Some(opaque));
    }

    #[test]
    fn packet_opaque_ref_copy_props_shares_and_clear_releases_last_reference() {
        let released = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = Arc::clone(&released);
        let mut src = Packet::new(vec![1], 0);
        src.set_opaque_ref(Some(BufferRef::from_vec_with_release_callback(
            vec![0xaa, 0xbb],
            move |data| {
                capture.lock().unwrap().push(data);
            },
        )));

        let mut dst = Packet::new(vec![9], 1);
        dst.copy_props_from(&src);

        assert_eq!(dst.data(), &[9]);
        assert_eq!(dst.opaque_ref().unwrap().as_slice(), &[0xaa, 0xbb]);
        assert!(dst
            .opaque_ref()
            .unwrap()
            .shares_storage(src.opaque_ref().unwrap()));

        dst.clear_opaque_ref();

        assert!(dst.opaque_ref().is_none());
        assert!(src.opaque_ref().is_some());
        assert!(released.lock().unwrap().is_empty());

        src.clear_opaque_ref();

        assert_eq!(*released.lock().unwrap(), vec![vec![0xaa, 0xbb]]);
    }

    #[test]
    fn packet_opaque_ref_move_take_and_unref_release_lifecycle() {
        let released = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let capture_old = Arc::clone(&released);
        let mut dst = Packet::new(vec![9], 9);
        dst.set_opaque_ref(Some(BufferRef::from_vec_with_release_callback(
            vec![0x10],
            move |data| {
                capture_old.lock().unwrap().push(data);
            },
        )));

        let capture_src = Arc::clone(&released);
        let mut src = Packet::new(vec![1], 1);
        src.set_opaque_ref(Some(BufferRef::from_vec_with_release_callback(
            vec![0x20],
            move |data| {
                capture_src.lock().unwrap().push(data);
            },
        )));

        dst.move_ref_from(&mut src);

        assert_eq!(*released.lock().unwrap(), vec![vec![0x10]]);
        assert!(src.opaque_ref().is_none());
        assert_eq!(dst.opaque_ref().unwrap().as_slice(), &[0x20]);

        let taken = dst.take_opaque_ref().unwrap();
        assert!(dst.opaque_ref().is_none());
        assert_eq!(taken.as_slice(), &[0x20]);
        assert_eq!(*released.lock().unwrap(), vec![vec![0x10]]);
        drop(taken);
        assert_eq!(*released.lock().unwrap(), vec![vec![0x10], vec![0x20]]);

        let capture_unref = Arc::clone(&released);
        dst.set_opaque_ref(Some(BufferRef::from_vec_with_release_callback(
            vec![0x30],
            move |data| {
                capture_unref.lock().unwrap().push(data);
            },
        )));

        dst.unref();

        assert!(dst.opaque_ref().is_none());
        assert_eq!(
            *released.lock().unwrap(),
            vec![vec![0x10], vec![0x20], vec![0x30]]
        );
    }

    #[test]
    fn packet_time_base_defaults_copies_resets_and_rescale_preserves() {
        let mut src = Packet::new(vec![1, 2], 0);
        assert_eq!(src.time_base(), Rational::ZERO);

        src.set_time_base(Rational::from_raw(2, 4)).unwrap();
        assert_eq!(src.time_base(), Rational::new(1, 2).unwrap());
        assert_eq!(
            src.set_time_base(Rational::from_raw(1, 0))
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(src.time_base(), Rational::new(1, 2).unwrap());

        src.set_pts(Some(2));
        src.set_duration(2).unwrap();
        src.rescale_ts(Rational::new(1, 2).unwrap(), Rational::new(1, 4).unwrap())
            .unwrap();
        assert_eq!(src.time_base(), Rational::new(1, 2).unwrap());

        let mut props = Packet::new(vec![9], 1);
        props.copy_props_from(&src);
        assert_eq!(props.data(), &[9]);
        assert_eq!(props.time_base(), src.time_base());

        let mut packet_ref = Packet::default();
        packet_ref.ref_from(&src);
        assert_eq!(packet_ref.time_base(), src.time_base());

        let mut moved = Packet::default();
        moved.move_ref_from(&mut packet_ref);
        assert_eq!(moved.time_base(), src.time_base());
        assert_eq!(packet_ref.time_base(), Rational::ZERO);

        moved.unref();
        assert_eq!(moved.time_base(), Rational::ZERO);
    }

    #[test]
    fn packet_rescales_valid_timestamps_and_duration() {
        let src = Rational::new(1, 90_000).unwrap();
        let dst = Rational::new(1, 1_000).unwrap();
        let mut packet = Packet::new(vec![0], 3);
        packet.set_pts(Some(90_000));
        packet.set_dts(Some(45_000));
        packet.set_duration(3_003).unwrap();
        packet.set_pos(Some(77)).unwrap();
        packet.set_time_base(src).unwrap();
        packet.set_key(true);
        packet.push_side_data(SideData::new("palette", vec![1, 2, 3]).unwrap());

        packet.rescale_ts(src, dst).unwrap();

        assert_eq!(packet.pts(), Some(1_000));
        assert_eq!(packet.dts(), Some(500));
        assert_eq!(packet.duration(), 33);
        assert_eq!(packet.pos(), Some(77));
        assert_eq!(packet.time_base(), src);
        assert_eq!(packet.stream_index(), 3);
        assert!(packet.flags().contains(PacketFlags::KEY));
        assert_eq!(packet.side_data()[0].data(), &[1, 2, 3]);
    }

    #[test]
    fn packet_rescale_ignores_unknown_timestamps() {
        let src = Rational::new(1, 48_000).unwrap();
        let dst = Rational::new(1, 1_000).unwrap();
        let mut packet = Packet::new(Vec::new(), 0);
        packet.set_duration(48_000).unwrap();

        packet.rescale_ts(src, dst).unwrap();

        assert_eq!(packet.pts(), None);
        assert_eq!(packet.dts(), None);
        assert_eq!(packet.duration(), 1_000);
    }

    #[test]
    fn packet_rescale_errors_do_not_mutate_timing_fields() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.set_pts(Some(10));
        packet.set_dts(Some(9));
        packet.set_duration(8).unwrap();

        let invalid_err = packet
            .rescale_ts(Rational::from_raw(1, 0), Rational::ONE)
            .unwrap_err();

        assert_eq!(invalid_err.kind(), crate::AvErrorKind::InvalidArgument);
        assert_eq!(packet.pts(), Some(10));
        assert_eq!(packet.dts(), Some(9));
        assert_eq!(packet.duration(), 8);

        packet.set_pts(Some(i64::MAX));
        packet.set_dts(Some(9));
        packet.set_duration(8).unwrap();
        let overflow_err = packet
            .rescale_ts(Rational::ONE, Rational::new(1, 2).unwrap())
            .unwrap_err();

        assert_eq!(overflow_err.kind(), crate::AvErrorKind::InvalidArgument);
        assert_eq!(packet.pts(), Some(i64::MAX));
        assert_eq!(packet.dts(), Some(9));
        assert_eq!(packet.duration(), 8);
    }
}
