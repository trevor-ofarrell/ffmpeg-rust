use crate::frame::{
    FrameStereo3d, FrameStereo3dFlags, FrameStereo3dPrimaryEye, FrameStereo3dType,
    FrameStereo3dView,
};
use crate::{rescale_q, AvError, AvResult, BufferRef, Rational};

pub const AV_NOPTS_VALUE: i64 = i64::MIN;
pub const AV_PACKET_POS_UNKNOWN: i64 = -1;

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

    pub fn new_a53_closed_captions(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::A53ClosedCaptions, data)?;
        PacketA53ClosedCaptions::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_icc_profile(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(PacketSideDataKind::IccProfile, data)?;
        PacketIccProfile::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_skip_samples(value: PacketSkipSamples) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::SkipSamples, value.to_bytes().to_vec())
    }

    pub fn new_param_change(value: PacketParamChange) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::ParamChange, value.to_bytes())
    }

    pub fn new_jp_dualmono(value: PacketJpDualMono) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::JpDualMono, value.to_bytes().to_vec())
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

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
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

    pub fn a53_closed_captions(&self) -> AvResult<Option<PacketA53ClosedCaptions<'_>>> {
        if self.kind != PacketSideDataKind::A53ClosedCaptions {
            return Ok(None);
        }

        PacketA53ClosedCaptions::parse(self.data()).map(Some)
    }

    pub fn icc_profile(&self) -> AvResult<Option<PacketIccProfile<'_>>> {
        if self.kind != PacketSideDataKind::IccProfile {
            return Ok(None);
        }

        PacketIccProfile::parse(self.data()).map(Some)
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

    pub fn jp_dualmono(&self) -> AvResult<Option<PacketJpDualMono>> {
        if self.kind != PacketSideDataKind::JpDualMono {
            return Ok(None);
        }

        PacketJpDualMono::parse(self.data()).map(Some)
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
            return Err(AvError::invalid_argument(
                "packet side data cannot be shrunk to a larger size",
            ));
        }

        self.data.truncate(len);
        Ok(())
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
    opaque_ref: Option<BufferRef>,
    time_base: Rational,
}

impl Packet {
    pub fn new(data: Vec<u8>, stream_index: usize) -> Self {
        Self::with_buffer(BufferRef::from_vec(data), stream_index)
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

    pub fn shrink_side_data(&mut self, kind: &str, len: usize) -> AvResult<bool> {
        let Some(side_data) = self.side_data_mut_by_kind(kind) else {
            return Ok(false);
        };

        side_data.shrink(len)?;
        Ok(true)
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

    pub fn set_opaque_ref(&mut self, opaque_ref: Option<BufferRef>) {
        self.opaque_ref = opaque_ref;
    }

    pub fn take_opaque_ref(&mut self) -> Option<BufferRef> {
        self.opaque_ref.take()
    }

    pub fn clear_opaque_ref(&mut self) {
        self.opaque_ref = None;
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
}

impl Default for Packet {
    fn default() -> Self {
        Self::new(Vec::new(), 0)
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

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    u32::from_le_bytes(bytes)
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

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    u32::from_be_bytes(bytes)
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

        assert_eq!(err.kind(), crate::AvErrorKind::InvalidArgument);
        assert_eq!(
            packet.side_data_by_kind("palette").unwrap().data(),
            &[0, 1, 2]
        );
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
        assert_eq!(dst.opaque_ref().unwrap().as_slice(), &[0xde, 0xad]);
        assert!(dst
            .opaque_ref()
            .unwrap()
            .shares_storage(src.opaque_ref().unwrap()));

        dst.shrink_side_data("palette", 1).unwrap();
        dst.clear_opaque_ref();
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[5]);
        assert_eq!(src.side_data_by_kind("palette").unwrap().data(), &[5, 6]);
        assert!(dst.opaque_ref().is_none());
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

        dst.move_ref_from(&mut src);

        assert_eq!(*released.lock().unwrap(), vec![vec![9]]);
        assert!(src.is_empty());
        assert_eq!(src.stream_index(), 0);
        assert_eq!(src.pts(), None);
        assert!(src.side_data().is_empty());
        assert_eq!(dst.data(), &[1, 2]);
        assert_eq!(dst.stream_index(), 3);
        assert_eq!(dst.pts(), Some(7));
        assert_eq!(dst.duration(), 5);
        assert_eq!(dst.time_base(), Rational::new(1, 48_000).unwrap());
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[4]);

        dst.unref();

        assert_eq!(*released.lock().unwrap(), vec![vec![9], vec![1, 2]]);
        assert!(dst.is_empty());
        assert_eq!(dst.stream_index(), 0);
        assert_eq!(dst.pts(), None);
        assert_eq!(dst.dts(), None);
        assert_eq!(dst.duration(), 0);
        assert_eq!(dst.pos(), None);
        assert_eq!(dst.time_base(), Rational::ZERO);
        assert!(dst.flags().is_empty());
        assert!(dst.side_data().is_empty());
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
