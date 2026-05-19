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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideData {
    kind: PacketSideDataKind,
    data: Vec<u8>,
}

impl SideData {
    pub fn new(kind: impl Into<String>, data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::from_name(kind)?, data)
    }

    pub fn new_skip_samples(value: PacketSkipSamples) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::SkipSamples, value.to_bytes().to_vec())
    }

    pub fn new_param_change(value: PacketParamChange) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::ParamChange, value.to_bytes())
    }

    pub fn new_frame_cropping(value: PacketFrameCropping) -> AvResult<Self> {
        Self::new_with_kind(PacketSideDataKind::FrameCropping, value.to_bytes().to_vec())
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

    pub fn frame_cropping(&self) -> AvResult<Option<PacketFrameCropping>> {
        if self.kind != PacketSideDataKind::FrameCropping {
            return Ok(None);
        }

        PacketFrameCropping::parse(self.data()).map(Some)
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

fn read_i32_le(data: &[u8], offset: usize) -> i32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    i32::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

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
