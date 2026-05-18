use crate::probe::{ProbeDescriptor, ProbeRegistry};
use avutil::{AvError, AvErrorKind, AvResult, BitReader, ByteReader, Dictionary, Packet, SideData};

const FTYP_ID: &[u8; 4] = b"ftyp";
const MOOV_ID: &[u8; 4] = b"moov";
const MVHD_ID: &[u8; 4] = b"mvhd";
const MVEX_ID: &[u8; 4] = b"mvex";
const MOOF_ID: &[u8; 4] = b"moof";
const TRAK_ID: &[u8; 4] = b"trak";
const TKHD_ID: &[u8; 4] = b"tkhd";
const EDTS_ID: &[u8; 4] = b"edts";
const UDTA_ID: &[u8; 4] = b"udta";
const META_ID: &[u8; 4] = b"meta";
const ILST_ID: &[u8; 4] = b"ilst";
const DATA_ID: &[u8; 4] = b"data";
const COVR_ID: &[u8; 4] = b"covr";
const FREEFORM_ID: &[u8; 4] = b"----";
const MEAN_ID: &[u8; 4] = b"mean";
const NAME_ID: &[u8; 4] = b"name";
const MDIA_ID: &[u8; 4] = b"mdia";
const MDHD_ID: &[u8; 4] = b"mdhd";
const HDLR_ID: &[u8; 4] = b"hdlr";
const MINF_ID: &[u8; 4] = b"minf";
const STBL_ID: &[u8; 4] = b"stbl";
const STSD_ID: &[u8; 4] = b"stsd";
const STTS_ID: &[u8; 4] = b"stts";
const CTTS_ID: &[u8; 4] = b"ctts";
const STSC_ID: &[u8; 4] = b"stsc";
const STSZ_ID: &[u8; 4] = b"stsz";
const STSS_ID: &[u8; 4] = b"stss";
const STCO_ID: &[u8; 4] = b"stco";
const CO64_ID: &[u8; 4] = b"co64";
const MDAT_ID: &[u8; 4] = b"mdat";
const AVCC_ID: &[u8; 4] = b"avcC";
const HVCC_ID: &[u8; 4] = b"hvcC";
const ESDS_ID: &[u8; 4] = b"esds";
const WAVE_ID: &[u8; 4] = b"wave";
const CHAN_ID: &[u8; 4] = b"chan";
const BTRT_ID: &[u8; 4] = b"btrt";
const DAMR_ID: &[u8; 4] = b"damr";
const DAC3_ID: &[u8; 4] = b"dac3";
const DEC3_ID: &[u8; 4] = b"dec3";
const DOPS_ID: &[u8; 4] = b"dOps";
const DFLA_ID: &[u8; 4] = b"dfLa";
const ALAC_ID: &[u8; 4] = b"alac";
const FRMA_ID: &[u8; 4] = b"frma";
const TERMINATOR_ID: &[u8; 4] = b"\0\0\0\0";
const PASP_ID: &[u8; 4] = b"pasp";
const COLR_ID: &[u8; 4] = b"colr";
const NCLC_ID: &[u8; 4] = b"nclc";
const NCLX_ID: &[u8; 4] = b"nclx";
const PROF_ID: &[u8; 4] = b"prof";
const RICC_ID: &[u8; 4] = b"rICC";
const MAX_MOV_SAMPLE_COUNT: usize = 1_000_000;
const METADATA_DATA_TYPE_RESERVED: u32 = 0;
const METADATA_DATA_TYPE_UTF8: u32 = 1;
const METADATA_DATA_TYPE_UTF16: u32 = 2;
const METADATA_DATA_TYPE_JPEG: u32 = 0x0d;
const METADATA_DATA_TYPE_PNG: u32 = 0x0e;
const METADATA_DATA_TYPE_BMP: u32 = 0x1b;
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const MOV_PROBE_NAME: &str = "mov,mp4,m4a,3gp,3g2,mj2";
const MOV_PROBE_EXTENSIONS: &[&str] = &["mov", "mp4", "m4a", "3gp", "3g2", "mj2"];
const MOV_PROBE_MIME_TYPES: &[&str] = &[
    "video/quicktime",
    "video/mp4",
    "audio/mp4",
    "application/mp4",
];

pub fn mov_probe_descriptor() -> AvResult<ProbeDescriptor> {
    ProbeDescriptor::new_with_offset_signatures(
        MOV_PROBE_NAME,
        MOV_PROBE_EXTENSIONS,
        MOV_PROBE_MIME_TYPES,
        &[],
        &[(4, FTYP_ID.as_slice())],
    )
}

pub fn register_mov_probe(registry: &mut ProbeRegistry) -> AvResult<()> {
    registry.register(mov_probe_descriptor()?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovInfo {
    major_brand: String,
    minor_version: u32,
    compatible_brands: Vec<String>,
    timescale: u32,
    duration: Option<u64>,
    metadata: Dictionary,
    cover_art: Vec<MovCoverArt>,
    tracks: Vec<MovTrackInfo>,
    has_media_data: bool,
}

impl MovInfo {
    pub fn major_brand(&self) -> &str {
        &self.major_brand
    }

    pub fn minor_version(&self) -> u32 {
        self.minor_version
    }

    pub fn compatible_brands(&self) -> &[String] {
        &self.compatible_brands
    }

    pub fn timescale(&self) -> u32 {
        self.timescale
    }

    pub fn duration(&self) -> Option<u64> {
        self.duration
    }

    pub fn metadata(&self) -> &Dictionary {
        &self.metadata
    }

    pub fn cover_art(&self) -> &[MovCoverArt] {
        &self.cover_art
    }

    pub fn tracks(&self) -> &[MovTrackInfo] {
        &self.tracks
    }

    pub fn has_media_data(&self) -> bool {
        self.has_media_data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovCoverArt {
    codec: String,
    mime_type: String,
    data_type: u32,
    data: Vec<u8>,
}

impl MovCoverArt {
    pub fn codec(&self) -> &str {
        &self.codec
    }

    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    pub fn data_type(&self) -> u32 {
        self.data_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovCodecParameters {
    codec_tag: String,
    data_reference_index: u16,
    extra_data: Vec<u8>,
    details: MovSampleEntryDetails,
}

impl MovCodecParameters {
    pub fn codec_tag(&self) -> &str {
        &self.codec_tag
    }

    pub fn data_reference_index(&self) -> u16 {
        self.data_reference_index
    }

    pub fn extra_data(&self) -> &[u8] {
        &self.extra_data
    }

    pub fn details(&self) -> &MovSampleEntryDetails {
        &self.details
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovSampleEntryDetails {
    Generic,
    Audio(Box<MovAudioSampleEntry>),
    Video(Box<MovVideoSampleEntry>),
    Subtitle(MovSubtitleSampleEntry),
    Data(MovDataSampleEntry),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovSubtitleSampleEntry {
    TimedText(MovTimedTextSampleEntry),
}

impl MovSubtitleSampleEntry {
    pub fn timed_text(&self) -> Option<&MovTimedTextSampleEntry> {
        match self {
            Self::TimedText(entry) => Some(entry),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovTimedTextSampleEntry {
    display_flags: u32,
    horizontal_justification: i8,
    vertical_justification: i8,
    background_color_rgba: [u8; 4],
    default_text_box: MovTextBoxRecord,
    default_style: MovTextStyleRecord,
    child_boxes: Vec<MovSampleEntryChildBox>,
}

impl MovTimedTextSampleEntry {
    pub fn display_flags(&self) -> u32 {
        self.display_flags
    }

    pub fn horizontal_justification(&self) -> i8 {
        self.horizontal_justification
    }

    pub fn vertical_justification(&self) -> i8 {
        self.vertical_justification
    }

    pub fn background_color_rgba(&self) -> [u8; 4] {
        self.background_color_rgba
    }

    pub fn default_text_box(&self) -> &MovTextBoxRecord {
        &self.default_text_box
    }

    pub fn default_style(&self) -> &MovTextStyleRecord {
        &self.default_style
    }

    pub fn child_boxes(&self) -> &[MovSampleEntryChildBox] {
        &self.child_boxes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovTextBoxRecord {
    top: i16,
    left: i16,
    bottom: i16,
    right: i16,
}

impl MovTextBoxRecord {
    pub fn top(&self) -> i16 {
        self.top
    }

    pub fn left(&self) -> i16 {
        self.left
    }

    pub fn bottom(&self) -> i16 {
        self.bottom
    }

    pub fn right(&self) -> i16 {
        self.right
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovTextStyleRecord {
    start_char: u16,
    end_char: u16,
    font_id: u16,
    face_style_flags: u8,
    font_size: u8,
    text_color_rgba: [u8; 4],
}

impl MovTextStyleRecord {
    pub fn start_char(&self) -> u16 {
        self.start_char
    }

    pub fn end_char(&self) -> u16 {
        self.end_char
    }

    pub fn font_id(&self) -> u16 {
        self.font_id
    }

    pub fn face_style_flags(&self) -> u8 {
        self.face_style_flags
    }

    pub fn font_size(&self) -> u8 {
        self.font_size
    }

    pub fn text_color_rgba(&self) -> [u8; 4] {
        self.text_color_rgba
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovDataSampleEntry {
    XmlMetadata(MovXmlMetadataSampleEntry),
    TextMetadata(MovTextMetadataSampleEntry),
}

impl MovDataSampleEntry {
    pub fn xml_metadata(&self) -> Option<&MovXmlMetadataSampleEntry> {
        match self {
            Self::XmlMetadata(entry) => Some(entry),
            Self::TextMetadata(_) => None,
        }
    }

    pub fn text_metadata(&self) -> Option<&MovTextMetadataSampleEntry> {
        match self {
            Self::TextMetadata(entry) => Some(entry),
            Self::XmlMetadata(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovXmlMetadataSampleEntry {
    content_encoding: String,
    namespace: String,
    schema_location: String,
}

impl MovXmlMetadataSampleEntry {
    pub fn content_encoding(&self) -> &str {
        &self.content_encoding
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn schema_location(&self) -> &str {
        &self.schema_location
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovTextMetadataSampleEntry {
    content_encoding: String,
    mime_format: String,
}

impl MovTextMetadataSampleEntry {
    pub fn content_encoding(&self) -> &str {
        &self.content_encoding
    }

    pub fn mime_format(&self) -> &str {
        &self.mime_format
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAudioSampleEntry {
    version: u16,
    revision_level: u16,
    vendor: u32,
    channel_count: u16,
    sample_size: u16,
    compression_id: i16,
    packet_size: u16,
    sample_rate: u32,
    sample_rate_fixed_16_16: u32,
    version_fields: MovAudioSampleEntryVersionFields,
    elementary_stream_descriptor: Option<MovElementaryStreamDescriptor>,
    bit_rate: Option<MovBitRateBox>,
    amr_specific: Option<MovAmrSpecificBox>,
    ac3_specific: Option<MovAc3SpecificBox>,
    ec3_specific: Option<MovEc3SpecificBox>,
    opus_specific: Option<MovOpusSpecificBox>,
    flac_specific: Option<MovFlacSpecificBox>,
    alac_specific: Option<MovAlacSpecificBox>,
    wave_extension: Option<MovAudioWaveExtension>,
    channel_layout: Option<MovAudioChannelLayout>,
    child_boxes: Vec<MovSampleEntryChildBox>,
}

impl MovAudioSampleEntry {
    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn revision_level(&self) -> u16 {
        self.revision_level
    }

    pub fn vendor(&self) -> u32 {
        self.vendor
    }

    pub fn channel_count(&self) -> u16 {
        self.channel_count
    }

    pub fn effective_channel_count(&self) -> u32 {
        if let Some(alac_specific) = self.alac_specific() {
            return u32::from(alac_specific.config().num_channels());
        }
        if let Some(flac_specific) = &self.flac_specific {
            return u32::from(flac_specific.streaminfo().channels());
        }
        match &self.version_fields {
            MovAudioSampleEntryVersionFields::Version2(fields) => fields.num_audio_channels(),
            MovAudioSampleEntryVersionFields::Version0
            | MovAudioSampleEntryVersionFields::Version1(_) => u32::from(self.channel_count),
        }
    }

    pub fn sample_size(&self) -> u16 {
        self.sample_size
    }

    pub fn effective_bits_per_sample(&self) -> u32 {
        if let Some(alac_specific) = self.alac_specific() {
            return u32::from(alac_specific.config().bit_depth());
        }
        if let Some(flac_specific) = &self.flac_specific {
            return u32::from(flac_specific.streaminfo().bits_per_sample());
        }
        match &self.version_fields {
            MovAudioSampleEntryVersionFields::Version2(fields) => {
                let const_bits_per_channel = fields.const_bits_per_channel();
                if const_bits_per_channel == 0 {
                    u32::from(self.sample_size)
                } else {
                    const_bits_per_channel
                }
            }
            MovAudioSampleEntryVersionFields::Version0
            | MovAudioSampleEntryVersionFields::Version1(_) => u32::from(self.sample_size),
        }
    }

    pub fn compression_id(&self) -> i16 {
        self.compression_id
    }

    pub fn packet_size(&self) -> u16 {
        self.packet_size
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn sample_rate_fixed_16_16(&self) -> u32 {
        self.sample_rate_fixed_16_16
    }

    pub fn version_fields(&self) -> &MovAudioSampleEntryVersionFields {
        &self.version_fields
    }

    pub fn version1_fields(&self) -> Option<&MovAudioSampleEntryVersion1Fields> {
        match &self.version_fields {
            MovAudioSampleEntryVersionFields::Version1(fields) => Some(fields),
            MovAudioSampleEntryVersionFields::Version0
            | MovAudioSampleEntryVersionFields::Version2(_) => None,
        }
    }

    pub fn version2_fields(&self) -> Option<&MovAudioSampleEntryVersion2Fields> {
        match &self.version_fields {
            MovAudioSampleEntryVersionFields::Version2(fields) => Some(fields),
            MovAudioSampleEntryVersionFields::Version0
            | MovAudioSampleEntryVersionFields::Version1(_) => None,
        }
    }

    pub fn elementary_stream_descriptor(&self) -> Option<&MovElementaryStreamDescriptor> {
        self.elementary_stream_descriptor.as_ref()
    }

    pub fn bit_rate(&self) -> Option<&MovBitRateBox> {
        self.bit_rate.as_ref()
    }

    pub fn amr_specific(&self) -> Option<&MovAmrSpecificBox> {
        self.amr_specific.as_ref()
    }

    pub fn ac3_specific(&self) -> Option<&MovAc3SpecificBox> {
        self.ac3_specific.as_ref()
    }

    pub fn ec3_specific(&self) -> Option<&MovEc3SpecificBox> {
        self.ec3_specific.as_ref()
    }

    pub fn opus_specific(&self) -> Option<&MovOpusSpecificBox> {
        self.opus_specific.as_ref()
    }

    pub fn flac_specific(&self) -> Option<&MovFlacSpecificBox> {
        self.flac_specific.as_ref()
    }

    pub fn alac_specific(&self) -> Option<&MovAlacSpecificBox> {
        self.alac_specific.as_ref().or_else(|| {
            self.wave_extension
                .as_ref()
                .and_then(MovAudioWaveExtension::alac_specific)
        })
    }

    pub fn wave_extension(&self) -> Option<&MovAudioWaveExtension> {
        self.wave_extension.as_ref()
    }

    pub fn channel_layout(&self) -> Option<&MovAudioChannelLayout> {
        self.channel_layout.as_ref()
    }

    pub fn child_boxes(&self) -> &[MovSampleEntryChildBox] {
        &self.child_boxes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovAudioSampleEntryVersionFields {
    Version0,
    Version1(MovAudioSampleEntryVersion1Fields),
    Version2(MovAudioSampleEntryVersion2Fields),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAudioSampleEntryVersion1Fields {
    samples_per_packet: u32,
    bytes_per_packet: u32,
    bytes_per_frame: u32,
    bytes_per_sample: u32,
}

impl MovAudioSampleEntryVersion1Fields {
    pub fn samples_per_packet(&self) -> u32 {
        self.samples_per_packet
    }

    pub fn bytes_per_packet(&self) -> u32 {
        self.bytes_per_packet
    }

    pub fn bytes_per_frame(&self) -> u32 {
        self.bytes_per_frame
    }

    pub fn bytes_per_sample(&self) -> u32 {
        self.bytes_per_sample
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAudioSampleEntryVersion2Fields {
    size_of_struct_only: u32,
    audio_sample_rate_bits: u64,
    num_audio_channels: u32,
    always_7f000000: u32,
    const_bits_per_channel: u32,
    format_specific_flags: u32,
    const_bytes_per_audio_packet: u32,
    const_lpcm_frames_per_audio_packet: u32,
}

impl MovAudioSampleEntryVersion2Fields {
    pub fn size_of_struct_only(&self) -> u32 {
        self.size_of_struct_only
    }

    pub fn audio_sample_rate(&self) -> f64 {
        f64::from_bits(self.audio_sample_rate_bits)
    }

    pub fn audio_sample_rate_bits(&self) -> u64 {
        self.audio_sample_rate_bits
    }

    pub fn num_audio_channels(&self) -> u32 {
        self.num_audio_channels
    }

    pub fn always_7f000000(&self) -> u32 {
        self.always_7f000000
    }

    pub fn const_bits_per_channel(&self) -> u32 {
        self.const_bits_per_channel
    }

    pub fn format_specific_flags(&self) -> u32 {
        self.format_specific_flags
    }

    pub fn const_bytes_per_audio_packet(&self) -> u32 {
        self.const_bytes_per_audio_packet
    }

    pub fn const_lpcm_frames_per_audio_packet(&self) -> u32 {
        self.const_lpcm_frames_per_audio_packet
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAudioWaveExtension {
    original_format: Option<String>,
    elementary_stream_descriptor: Option<MovElementaryStreamDescriptor>,
    alac_specific: Option<MovAlacSpecificBox>,
    has_terminator: bool,
    child_boxes: Vec<MovSampleEntryChildBox>,
}

impl MovAudioWaveExtension {
    pub fn original_format(&self) -> Option<&str> {
        self.original_format.as_deref()
    }

    pub fn elementary_stream_descriptor(&self) -> Option<&MovElementaryStreamDescriptor> {
        self.elementary_stream_descriptor.as_ref()
    }

    pub fn alac_specific(&self) -> Option<&MovAlacSpecificBox> {
        self.alac_specific.as_ref()
    }

    pub fn has_terminator(&self) -> bool {
        self.has_terminator
    }

    pub fn child_boxes(&self) -> &[MovSampleEntryChildBox] {
        &self.child_boxes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovElementaryStreamDescriptor {
    version: u8,
    flags: [u8; 3],
    descriptor: Vec<u8>,
}

impl MovElementaryStreamDescriptor {
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn flags(&self) -> [u8; 3] {
        self.flags
    }

    pub fn descriptor(&self) -> &[u8] {
        &self.descriptor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovBitRateBox {
    buffer_size_db: u32,
    max_bitrate: u32,
    avg_bitrate: u32,
}

impl MovBitRateBox {
    pub fn buffer_size_db(&self) -> u32 {
        self.buffer_size_db
    }

    pub fn max_bitrate(&self) -> u32 {
        self.max_bitrate
    }

    pub fn avg_bitrate(&self) -> u32 {
        self.avg_bitrate
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAmrSpecificBox {
    vendor: String,
    decoder_version: u8,
    mode_set: u16,
    mode_change_period: u8,
    frames_per_sample: u8,
}

impl MovAmrSpecificBox {
    pub fn vendor(&self) -> &str {
        &self.vendor
    }

    pub fn decoder_version(&self) -> u8 {
        self.decoder_version
    }

    pub fn mode_set(&self) -> u16 {
        self.mode_set
    }

    pub fn mode_change_period(&self) -> u8 {
        self.mode_change_period
    }

    pub fn frames_per_sample(&self) -> u8 {
        self.frames_per_sample
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAc3SpecificBox {
    fscod: u8,
    bsid: u8,
    bsmod: u8,
    acmod: u8,
    lfeon: bool,
    bit_rate_code: u8,
}

impl MovAc3SpecificBox {
    pub fn fscod(&self) -> u8 {
        self.fscod
    }

    pub fn bsid(&self) -> u8 {
        self.bsid
    }

    pub fn bsmod(&self) -> u8 {
        self.bsmod
    }

    pub fn acmod(&self) -> u8 {
        self.acmod
    }

    pub fn lfeon(&self) -> bool {
        self.lfeon
    }

    pub fn bit_rate_code(&self) -> u8 {
        self.bit_rate_code
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovEc3SpecificBox {
    data_rate: u16,
    num_ind_sub: u8,
    substreams: Vec<MovEc3IndependentSubstream>,
    trailing_reserved_bytes: Vec<u8>,
}

impl MovEc3SpecificBox {
    pub fn data_rate(&self) -> u16 {
        self.data_rate
    }

    pub fn num_ind_sub(&self) -> u8 {
        self.num_ind_sub
    }

    pub fn substreams(&self) -> &[MovEc3IndependentSubstream] {
        &self.substreams
    }

    pub fn trailing_reserved_bytes(&self) -> &[u8] {
        &self.trailing_reserved_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovEc3IndependentSubstream {
    fscod: u8,
    bsid: u8,
    asvc: bool,
    bsmod: u8,
    acmod: u8,
    lfeon: bool,
    num_dep_sub: u8,
    chan_loc: Option<u16>,
}

impl MovEc3IndependentSubstream {
    pub fn fscod(&self) -> u8 {
        self.fscod
    }

    pub fn bsid(&self) -> u8 {
        self.bsid
    }

    pub fn asvc(&self) -> bool {
        self.asvc
    }

    pub fn bsmod(&self) -> u8 {
        self.bsmod
    }

    pub fn acmod(&self) -> u8 {
        self.acmod
    }

    pub fn lfeon(&self) -> bool {
        self.lfeon
    }

    pub fn num_dep_sub(&self) -> u8 {
        self.num_dep_sub
    }

    pub fn chan_loc(&self) -> Option<u16> {
        self.chan_loc
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovOpusSpecificBox {
    version: u8,
    output_channel_count: u8,
    pre_skip: u16,
    input_sample_rate: u32,
    output_gain: i16,
    channel_mapping_family: u8,
    stream_count: Option<u8>,
    coupled_count: Option<u8>,
    channel_mapping: Vec<u8>,
}

impl MovOpusSpecificBox {
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn output_channel_count(&self) -> u8 {
        self.output_channel_count
    }

    pub fn pre_skip(&self) -> u16 {
        self.pre_skip
    }

    pub fn input_sample_rate(&self) -> u32 {
        self.input_sample_rate
    }

    pub fn output_gain(&self) -> i16 {
        self.output_gain
    }

    pub fn channel_mapping_family(&self) -> u8 {
        self.channel_mapping_family
    }

    pub fn stream_count(&self) -> Option<u8> {
        self.stream_count
    }

    pub fn coupled_count(&self) -> Option<u8> {
        self.coupled_count
    }

    pub fn channel_mapping(&self) -> &[u8] {
        &self.channel_mapping
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovFlacSpecificBox {
    version: u8,
    flags: [u8; 3],
    metadata_blocks: Vec<MovFlacMetadataBlock>,
    streaminfo: MovFlacStreamInfo,
}

impl MovFlacSpecificBox {
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn flags(&self) -> [u8; 3] {
        self.flags
    }

    pub fn metadata_blocks(&self) -> &[MovFlacMetadataBlock] {
        &self.metadata_blocks
    }

    pub fn streaminfo(&self) -> &MovFlacStreamInfo {
        &self.streaminfo
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovFlacMetadataBlock {
    last: bool,
    block_type: u8,
    data: Vec<u8>,
}

impl MovFlacMetadataBlock {
    pub fn last(&self) -> bool {
        self.last
    }

    pub fn block_type(&self) -> u8 {
        self.block_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovFlacStreamInfo {
    min_block_size: u16,
    max_block_size: u16,
    min_frame_size: u32,
    max_frame_size: u32,
    sample_rate: u32,
    channels: u8,
    bits_per_sample: u8,
    total_samples: u64,
    md5: [u8; 16],
}

impl MovFlacStreamInfo {
    pub fn min_block_size(&self) -> u16 {
        self.min_block_size
    }

    pub fn max_block_size(&self) -> u16 {
        self.max_block_size
    }

    pub fn min_frame_size(&self) -> u32 {
        self.min_frame_size
    }

    pub fn max_frame_size(&self) -> u32 {
        self.max_frame_size
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u8 {
        self.channels
    }

    pub fn bits_per_sample(&self) -> u8 {
        self.bits_per_sample
    }

    pub fn total_samples(&self) -> u64 {
        self.total_samples
    }

    pub fn md5(&self) -> &[u8; 16] {
        &self.md5
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAlacSpecificBox {
    version: u8,
    flags: [u8; 3],
    config: MovAlacSpecificConfig,
    channel_layout: Option<MovAlacChannelLayoutInfo>,
}

impl MovAlacSpecificBox {
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn flags(&self) -> [u8; 3] {
        self.flags
    }

    pub fn config(&self) -> &MovAlacSpecificConfig {
        &self.config
    }

    pub fn channel_layout(&self) -> Option<&MovAlacChannelLayoutInfo> {
        self.channel_layout.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAlacSpecificConfig {
    frame_length: u32,
    compatible_version: u8,
    bit_depth: u8,
    pb: u8,
    mb: u8,
    kb: u8,
    num_channels: u8,
    max_run: u16,
    max_frame_bytes: u32,
    avg_bit_rate: u32,
    sample_rate: u32,
}

impl MovAlacSpecificConfig {
    pub fn frame_length(&self) -> u32 {
        self.frame_length
    }

    pub fn compatible_version(&self) -> u8 {
        self.compatible_version
    }

    pub fn bit_depth(&self) -> u8 {
        self.bit_depth
    }

    pub fn pb(&self) -> u8 {
        self.pb
    }

    pub fn mb(&self) -> u8 {
        self.mb
    }

    pub fn kb(&self) -> u8 {
        self.kb
    }

    pub fn num_channels(&self) -> u8 {
        self.num_channels
    }

    pub fn max_run(&self) -> u16 {
        self.max_run
    }

    pub fn max_frame_bytes(&self) -> u32 {
        self.max_frame_bytes
    }

    pub fn avg_bit_rate(&self) -> u32 {
        self.avg_bit_rate
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAlacChannelLayoutInfo {
    channel_layout_tag: u32,
    reserved: [u32; 2],
}

impl MovAlacChannelLayoutInfo {
    pub fn channel_layout_tag(&self) -> u32 {
        self.channel_layout_tag
    }

    pub fn reserved(&self) -> [u32; 2] {
        self.reserved
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAudioChannelLayout {
    version: u8,
    flags: [u8; 3],
    channel_layout_tag: u32,
    channel_bitmap: u32,
    channel_descriptions: Vec<MovAudioChannelDescription>,
}

impl MovAudioChannelLayout {
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn flags(&self) -> [u8; 3] {
        self.flags
    }

    pub fn channel_layout_tag(&self) -> u32 {
        self.channel_layout_tag
    }

    pub fn channel_bitmap(&self) -> u32 {
        self.channel_bitmap
    }

    pub fn channel_descriptions(&self) -> &[MovAudioChannelDescription] {
        &self.channel_descriptions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAudioChannelDescription {
    channel_label: u32,
    channel_flags: u32,
    coordinate_bits: [u32; 3],
}

impl MovAudioChannelDescription {
    pub fn channel_label(&self) -> u32 {
        self.channel_label
    }

    pub fn channel_flags(&self) -> u32 {
        self.channel_flags
    }

    pub fn coordinate_bits(&self) -> [u32; 3] {
        self.coordinate_bits
    }

    pub fn coordinates(&self) -> [f32; 3] {
        self.coordinate_bits.map(f32::from_bits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovVideoSampleEntry {
    width: u16,
    height: u16,
    frame_count: u16,
    compressor_name: String,
    depth: u16,
    avc_decoder_configuration: Option<MovAvcDecoderConfiguration>,
    hevc_decoder_configuration: Option<MovHevcDecoderConfiguration>,
    pixel_aspect_ratio: Option<MovPixelAspectRatio>,
    color_information: Option<MovColorInformation>,
    child_boxes: Vec<MovSampleEntryChildBox>,
}

impl MovVideoSampleEntry {
    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn frame_count(&self) -> u16 {
        self.frame_count
    }

    pub fn compressor_name(&self) -> &str {
        &self.compressor_name
    }

    pub fn depth(&self) -> u16 {
        self.depth
    }

    pub fn avc_decoder_configuration(&self) -> Option<&MovAvcDecoderConfiguration> {
        self.avc_decoder_configuration.as_ref()
    }

    pub fn hevc_decoder_configuration(&self) -> Option<&MovHevcDecoderConfiguration> {
        self.hevc_decoder_configuration.as_ref()
    }

    pub fn pixel_aspect_ratio(&self) -> Option<&MovPixelAspectRatio> {
        self.pixel_aspect_ratio.as_ref()
    }

    pub fn color_information(&self) -> Option<&MovColorInformation> {
        self.color_information.as_ref()
    }

    pub fn child_boxes(&self) -> &[MovSampleEntryChildBox] {
        &self.child_boxes
    }

    pub fn avc_decoder_configuration_record(&self) -> Option<&[u8]> {
        self.child_payload_for_fourcc(AVCC_ID)
    }

    pub fn hevc_decoder_configuration_record(&self) -> Option<&[u8]> {
        self.child_payload_for_fourcc(HVCC_ID)
    }

    fn child_payload_for_fourcc(&self, box_type: &[u8; 4]) -> Option<&[u8]> {
        self.child_boxes
            .iter()
            .find(|child| child.box_type.as_bytes() == box_type)
            .map(MovSampleEntryChildBox::payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovAvcDecoderConfiguration {
    configuration_version: u8,
    profile_indication: u8,
    profile_compatibility: u8,
    level_indication: u8,
    nal_length_size: u8,
    sequence_parameter_sets: Vec<Vec<u8>>,
    picture_parameter_sets: Vec<Vec<u8>>,
    extension_data: Vec<u8>,
}

impl MovAvcDecoderConfiguration {
    pub fn configuration_version(&self) -> u8 {
        self.configuration_version
    }

    pub fn profile_indication(&self) -> u8 {
        self.profile_indication
    }

    pub fn profile_compatibility(&self) -> u8 {
        self.profile_compatibility
    }

    pub fn level_indication(&self) -> u8 {
        self.level_indication
    }

    pub fn nal_length_size(&self) -> u8 {
        self.nal_length_size
    }

    pub fn sequence_parameter_sets(&self) -> &[Vec<u8>] {
        &self.sequence_parameter_sets
    }

    pub fn picture_parameter_sets(&self) -> &[Vec<u8>] {
        &self.picture_parameter_sets
    }

    pub fn extension_data(&self) -> &[u8] {
        &self.extension_data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovHevcDecoderConfiguration {
    configuration_version: u8,
    general_profile_space: u8,
    general_tier_flag: bool,
    general_profile_idc: u8,
    general_profile_compatibility_flags: u32,
    general_constraint_indicator_flags: u64,
    general_level_idc: u8,
    min_spatial_segmentation_idc: u16,
    parallelism_type: u8,
    chroma_format: u8,
    bit_depth_luma: u8,
    bit_depth_chroma: u8,
    average_frame_rate: u16,
    constant_frame_rate: u8,
    num_temporal_layers: u8,
    temporal_id_nested: bool,
    nal_length_size: u8,
    arrays: Vec<MovHevcDecoderConfigurationArray>,
}

impl MovHevcDecoderConfiguration {
    pub fn configuration_version(&self) -> u8 {
        self.configuration_version
    }

    pub fn general_profile_space(&self) -> u8 {
        self.general_profile_space
    }

    pub fn general_tier_flag(&self) -> bool {
        self.general_tier_flag
    }

    pub fn general_profile_idc(&self) -> u8 {
        self.general_profile_idc
    }

    pub fn general_profile_compatibility_flags(&self) -> u32 {
        self.general_profile_compatibility_flags
    }

    pub fn general_constraint_indicator_flags(&self) -> u64 {
        self.general_constraint_indicator_flags
    }

    pub fn general_level_idc(&self) -> u8 {
        self.general_level_idc
    }

    pub fn min_spatial_segmentation_idc(&self) -> u16 {
        self.min_spatial_segmentation_idc
    }

    pub fn parallelism_type(&self) -> u8 {
        self.parallelism_type
    }

    pub fn chroma_format(&self) -> u8 {
        self.chroma_format
    }

    pub fn bit_depth_luma(&self) -> u8 {
        self.bit_depth_luma
    }

    pub fn bit_depth_chroma(&self) -> u8 {
        self.bit_depth_chroma
    }

    pub fn average_frame_rate(&self) -> u16 {
        self.average_frame_rate
    }

    pub fn constant_frame_rate(&self) -> u8 {
        self.constant_frame_rate
    }

    pub fn num_temporal_layers(&self) -> u8 {
        self.num_temporal_layers
    }

    pub fn temporal_id_nested(&self) -> bool {
        self.temporal_id_nested
    }

    pub fn nal_length_size(&self) -> u8 {
        self.nal_length_size
    }

    pub fn arrays(&self) -> &[MovHevcDecoderConfigurationArray] {
        &self.arrays
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovHevcDecoderConfigurationArray {
    array_completeness: bool,
    nal_unit_type: u8,
    nal_units: Vec<Vec<u8>>,
}

impl MovHevcDecoderConfigurationArray {
    pub fn array_completeness(&self) -> bool {
        self.array_completeness
    }

    pub fn nal_unit_type(&self) -> u8 {
        self.nal_unit_type
    }

    pub fn nal_units(&self) -> &[Vec<u8>] {
        &self.nal_units
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovPixelAspectRatio {
    horizontal_spacing: u32,
    vertical_spacing: u32,
}

impl MovPixelAspectRatio {
    pub fn horizontal_spacing(&self) -> u32 {
        self.horizontal_spacing
    }

    pub fn vertical_spacing(&self) -> u32 {
        self.vertical_spacing
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovColorInformation {
    color_type: String,
    color_parameters: Option<MovColorParameters>,
    icc_profile: Option<Vec<u8>>,
}

impl MovColorInformation {
    pub fn color_type(&self) -> &str {
        &self.color_type
    }

    pub fn color_parameters(&self) -> Option<&MovColorParameters> {
        self.color_parameters.as_ref()
    }

    pub fn color_primaries(&self) -> Option<u16> {
        self.color_parameters
            .as_ref()
            .map(MovColorParameters::color_primaries)
    }

    pub fn transfer_characteristics(&self) -> Option<u16> {
        self.color_parameters
            .as_ref()
            .map(MovColorParameters::transfer_characteristics)
    }

    pub fn matrix_coefficients(&self) -> Option<u16> {
        self.color_parameters
            .as_ref()
            .map(MovColorParameters::matrix_coefficients)
    }

    pub fn full_range(&self) -> Option<bool> {
        self.color_parameters
            .as_ref()
            .map(MovColorParameters::full_range)
    }

    pub fn icc_profile(&self) -> Option<&[u8]> {
        self.icc_profile.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovColorParameters {
    color_primaries: u16,
    transfer_characteristics: u16,
    matrix_coefficients: u16,
    full_range: bool,
}

impl MovColorParameters {
    pub fn color_primaries(&self) -> u16 {
        self.color_primaries
    }

    pub fn transfer_characteristics(&self) -> u16 {
        self.transfer_characteristics
    }

    pub fn matrix_coefficients(&self) -> u16 {
        self.matrix_coefficients
    }

    pub fn full_range(&self) -> bool {
        self.full_range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovSampleEntryChildBox {
    box_type: String,
    payload: Vec<u8>,
}

impl MovSampleEntryChildBox {
    pub fn box_type(&self) -> &str {
        &self.box_type
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovTrackInfo {
    id: u32,
    duration: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
    handler_type: Option<String>,
    media_timescale: u32,
    media_duration: Option<u64>,
    metadata: Dictionary,
    cover_art: Vec<MovCoverArt>,
    codec_parameters: Option<MovCodecParameters>,
    sample_count: usize,
}

impl MovTrackInfo {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn duration(&self) -> Option<u64> {
        self.duration
    }

    pub fn width(&self) -> Option<u32> {
        self.width
    }

    pub fn height(&self) -> Option<u32> {
        self.height
    }

    pub fn handler_type(&self) -> Option<&str> {
        self.handler_type.as_deref()
    }

    pub fn media_timescale(&self) -> u32 {
        self.media_timescale
    }

    pub fn media_duration(&self) -> Option<u64> {
        self.media_duration
    }

    pub fn metadata(&self) -> &Dictionary {
        &self.metadata
    }

    pub fn cover_art(&self) -> &[MovCoverArt] {
        &self.cover_art
    }

    pub fn codec_tag(&self) -> Option<&str> {
        self.codec_parameters
            .as_ref()
            .map(MovCodecParameters::codec_tag)
    }

    pub fn codec_parameters(&self) -> Option<&MovCodecParameters> {
        self.codec_parameters.as_ref()
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovDemuxer<'a> {
    info: MovInfo,
    packets: Option<Vec<Packet>>,
    next_packet: usize,
    _input: &'a [u8],
}

impl<'a> MovDemuxer<'a> {
    pub fn open(input: &'a [u8]) -> AvResult<Self> {
        let top_level = read_box_headers(input, 0, input.len(), "MOV/MP4 file")?;
        let mut ftyp = None;
        let mut movie_header = None;
        let mut movie_metadata = Dictionary::new();
        let mut movie_cover_art = Vec::new();
        let mut tracks = Vec::new();
        let mut mdat_ranges = Vec::new();
        let mut has_movie_extends = false;
        let mut has_movie_fragment = false;

        for header in top_level {
            let payload = &input[header.payload_start..header.payload_end];
            match &header.box_type {
                FTYP_ID => ftyp = Some(parse_ftyp(payload)?),
                MOOV_ID => {
                    let movie = parse_moov(input, &header)?;
                    movie_header = Some(movie.header);
                    movie_metadata = movie.metadata;
                    movie_cover_art = movie.cover_art;
                    tracks = movie.tracks;
                    has_movie_extends = movie.has_movie_extends;
                }
                MDAT_ID => mdat_ranges.push((header.payload_start, header.payload_end)),
                MOOF_ID => has_movie_fragment = true,
                _ => {}
            }
        }

        let ftyp = ftyp.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing ftyp box"))?;
        let movie_header =
            movie_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing moov box"))?;
        if has_movie_extends || has_movie_fragment {
            return Err(AvError::unsupported(
                "fragmented MOV/MP4 files with mvex/moof boxes are not implemented",
            ));
        }
        if tracks.is_empty() {
            return Err(AvError::invalid_data("MOV/MP4 missing trak boxes"));
        }
        let packets = build_packets(input, &tracks, &mdat_ranges)?;

        let track_info = tracks.into_iter().map(|track| track.info).collect();
        Ok(Self {
            info: MovInfo {
                major_brand: ftyp.major_brand,
                minor_version: ftyp.minor_version,
                compatible_brands: ftyp.compatible_brands,
                timescale: movie_header.timescale,
                duration: movie_header.duration,
                metadata: movie_metadata,
                cover_art: movie_cover_art,
                tracks: track_info,
                has_media_data: !mdat_ranges.is_empty(),
            },
            packets,
            next_packet: 0,
            _input: input,
        })
    }

    pub fn info(&self) -> &MovInfo {
        &self.info
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        let Some(packets) = &self.packets else {
            return Err(AvError::unsupported(
                "MOV/MP4 sample table packet extraction is not implemented for this file",
            ));
        };
        let Some(packet) = packets.get(self.next_packet).cloned() else {
            return Ok(None);
        };
        self.next_packet += 1;
        Ok(Some(packet))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FtypInfo {
    major_brand: String,
    minor_version: u32,
    compatible_brands: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MovieHeader {
    timescale: u32,
    duration: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MovieData {
    header: MovieHeader,
    metadata: Dictionary,
    cover_art: Vec<MovCoverArt>,
    tracks: Vec<TrackData>,
    has_movie_extends: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackData {
    info: MovTrackInfo,
    sample_table: Option<SampleTable>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrackHeader {
    id: u32,
    duration: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MediaHeader {
    timescale: u32,
    duration: Option<u64>,
    language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MediaData {
    header: MediaHeader,
    handler_type: Option<String>,
    metadata: Dictionary,
    sample_table: Option<SampleTable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampleTable {
    codec_parameters: MovCodecParameters,
    sample_sizes: Vec<usize>,
    sample_durations: Vec<u32>,
    composition_offsets: Option<Vec<i64>>,
    sample_to_chunks: Vec<SampleToChunkEntry>,
    chunk_offsets: Vec<u64>,
    sync_samples: Option<Vec<bool>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SampleToChunkEntry {
    first_chunk: u32,
    samples_per_chunk: u32,
    sample_description_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoxHeader {
    box_type: [u8; 4],
    payload_start: usize,
    payload_end: usize,
}

impl BoxHeader {
    fn payload<'a>(&self, input: &'a [u8]) -> &'a [u8] {
        &input[self.payload_start..self.payload_end]
    }
}

fn parse_ftyp(payload: &[u8]) -> AvResult<FtypInfo> {
    if payload.len() < 8 {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "MOV/MP4 ftyp box is shorter than 8 bytes",
        ));
    }
    if (payload.len() - 8) % 4 != 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 ftyp compatible brand list is not FourCC-aligned",
        ));
    }

    let mut reader = ByteReader::new(payload);
    let major_brand = fourcc_to_string(read_fourcc(&mut reader)?);
    let minor_version = reader.read_u32_be()?;
    let mut compatible_brands = Vec::new();
    while !reader.is_eof() {
        compatible_brands.push(fourcc_to_string(read_fourcc(&mut reader)?));
    }

    Ok(FtypInfo {
        major_brand,
        minor_version,
        compatible_brands,
    })
}

fn parse_moov(input: &[u8], moov: &BoxHeader) -> AvResult<MovieData> {
    let mut header = None;
    let mut metadata = ParsedMetadata::new();
    let mut tracks = Vec::new();
    let mut has_movie_extends = false;

    for child in read_box_headers(input, moov.payload_start, moov.payload_end, "MOV/MP4 moov")? {
        match &child.box_type {
            MVHD_ID => header = Some(parse_mvhd(child.payload(input))?),
            MVEX_ID => has_movie_extends = true,
            TRAK_ID => tracks.push(parse_trak(input, &child)?),
            UDTA_ID => merge_metadata(&mut metadata, parse_udta(input, &child)?)?,
            _ => {}
        }
    }

    let header = header.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing mvhd box"))?;
    Ok(MovieData {
        header,
        metadata: metadata.tags,
        cover_art: metadata.cover_art,
        tracks,
        has_movie_extends,
    })
}

fn parse_trak(input: &[u8], trak: &BoxHeader) -> AvResult<TrackData> {
    let mut track_header = None;
    let mut media_data = None;
    let mut metadata = ParsedMetadata::new();

    for child in read_box_headers(input, trak.payload_start, trak.payload_end, "MOV/MP4 trak")? {
        match &child.box_type {
            TKHD_ID => track_header = Some(parse_tkhd(child.payload(input))?),
            EDTS_ID => {
                return Err(AvError::unsupported(
                    "MOV/MP4 edit lists with edts/elst boxes are not implemented",
                ));
            }
            MDIA_ID => media_data = Some(parse_mdia(input, &child)?),
            UDTA_ID => merge_metadata(&mut metadata, parse_udta(input, &child)?)?,
            _ => {}
        }
    }

    let track_header =
        track_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 track missing tkhd box"))?;
    let media_data =
        media_data.ok_or_else(|| AvError::invalid_data("MOV/MP4 track missing mdhd box"))?;
    let codec_parameters = media_data
        .sample_table
        .as_ref()
        .map(|table| table.codec_parameters.clone());
    let sample_count = media_data
        .sample_table
        .as_ref()
        .map_or(0, |table| table.sample_sizes.len());
    for entry in media_data.metadata.entries() {
        set_metadata_value(&mut metadata.tags, entry.key(), entry.value().to_owned())?;
    }

    Ok(TrackData {
        info: MovTrackInfo {
            id: track_header.id,
            duration: track_header.duration,
            width: track_header.width,
            height: track_header.height,
            handler_type: media_data.handler_type,
            media_timescale: media_data.header.timescale,
            media_duration: media_data.header.duration,
            metadata: metadata.tags,
            cover_art: metadata.cover_art,
            codec_parameters,
            sample_count,
        },
        sample_table: media_data.sample_table,
    })
}

fn parse_mdia(input: &[u8], mdia: &BoxHeader) -> AvResult<MediaData> {
    let mut media_header = None;
    let mut handler_type = None;
    let mut metadata = Dictionary::new();
    let mut sample_table = None;
    for child in read_box_headers(input, mdia.payload_start, mdia.payload_end, "MOV/MP4 mdia")? {
        match &child.box_type {
            MDHD_ID => {
                let header = parse_mdhd(child.payload(input))?;
                if let Some(language) = &header.language {
                    set_metadata_value(&mut metadata, "language", language.clone())?;
                }
                media_header = Some(header);
            }
            HDLR_ID => {
                let handler = parse_hdlr(child.payload(input))?;
                handler_type = Some(handler.handler_type);
                if let Some(handler_name) = handler.name {
                    set_metadata_value(&mut metadata, "handler_name", handler_name)?;
                }
            }
            MINF_ID => sample_table = parse_minf(input, &child)?,
            _ => {}
        }
    }
    let header =
        media_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 mdia missing mdhd box"))?;
    Ok(MediaData {
        header,
        handler_type,
        metadata,
        sample_table,
    })
}

fn parse_minf(input: &[u8], minf: &BoxHeader) -> AvResult<Option<SampleTable>> {
    let mut sample_table = None;
    for child in read_box_headers(input, minf.payload_start, minf.payload_end, "MOV/MP4 minf")? {
        if child.box_type == *STBL_ID {
            sample_table = Some(parse_stbl(input, &child)?);
        }
    }
    Ok(sample_table)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedMetadata {
    tags: Dictionary,
    cover_art: Vec<MovCoverArt>,
}

impl ParsedMetadata {
    fn new() -> Self {
        Self::default()
    }
}

fn parse_udta(input: &[u8], udta: &BoxHeader) -> AvResult<ParsedMetadata> {
    let mut metadata = ParsedMetadata::new();
    for child in read_box_headers(input, udta.payload_start, udta.payload_end, "MOV/MP4 udta")? {
        if child.box_type == *META_ID {
            merge_metadata(&mut metadata, parse_meta(input, &child)?)?;
        }
    }
    Ok(metadata)
}

fn parse_meta(input: &[u8], meta: &BoxHeader) -> AvResult<ParsedMetadata> {
    let mut metadata = ParsedMetadata::new();
    let payload = meta.payload(input);
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 meta")?;
    for child in read_box_headers(payload, reader.position(), payload.len(), "MOV/MP4 meta")? {
        if child.box_type == *ILST_ID {
            merge_metadata(&mut metadata, parse_ilst(payload, &child)?)?;
        }
    }
    Ok(metadata)
}

fn parse_ilst(input: &[u8], ilst: &BoxHeader) -> AvResult<ParsedMetadata> {
    let mut metadata = ParsedMetadata::new();
    for item in read_box_headers(input, ilst.payload_start, ilst.payload_end, "MOV/MP4 ilst")? {
        let children = read_box_headers(
            input,
            item.payload_start,
            item.payload_end,
            "MOV/MP4 ilst metadata item",
        )?;
        if item.box_type == *COVR_ID {
            metadata
                .cover_art
                .extend(parse_cover_art_metadata_item(input, &children)?);
            continue;
        }
        if item.box_type == *FREEFORM_ID {
            if let Some((key, value)) = parse_freeform_metadata_item(input, &children)? {
                set_metadata_value(&mut metadata.tags, &key, value)?;
            }
            continue;
        }
        let Some((key, value_kind)) = metadata_item_mapping(item.box_type) else {
            continue;
        };
        for child in children {
            if child.box_type == *DATA_ID {
                if let Some(value) = parse_metadata_data(child.payload(input), value_kind)? {
                    set_metadata_value(&mut metadata.tags, key, value)?;
                }
            }
        }
    }
    Ok(metadata)
}

fn set_metadata_value(metadata: &mut Dictionary, key: &str, value: String) -> AvResult<()> {
    metadata.set(key, value).map_err(|err| {
        AvError::invalid_data(format!(
            "MOV/MP4 metadata value for {key} is invalid: {}",
            err.message()
        ))
    })?;
    Ok(())
}

fn parse_cover_art_metadata_item(
    input: &[u8],
    children: &[BoxHeader],
) -> AvResult<Vec<MovCoverArt>> {
    let mut cover_art = Vec::new();
    for child in children {
        if child.box_type == *DATA_ID {
            if let Some(art) = parse_cover_art_data_child(child.payload(input))? {
                cover_art.push(art);
            }
        }
    }
    Ok(cover_art)
}

fn parse_cover_art_data_child(payload: &[u8]) -> AvResult<Option<MovCoverArt>> {
    let (data_type, value) = parse_metadata_data_payload(payload, "MOV/MP4 cover art data")?;
    let Some((codec, mime_type)) = cover_art_codec(data_type, value) else {
        return Ok(None);
    };
    Ok(Some(MovCoverArt {
        codec: codec.to_owned(),
        mime_type: mime_type.to_owned(),
        data_type,
        data: value.to_vec(),
    }))
}

fn cover_art_codec(data_type: u32, value: &[u8]) -> Option<(&'static str, &'static str)> {
    let declared = match data_type {
        METADATA_DATA_TYPE_JPEG => Some(("mjpeg", "image/jpeg")),
        METADATA_DATA_TYPE_PNG => Some(("png", "image/png")),
        METADATA_DATA_TYPE_BMP => Some(("bmp", "image/bmp")),
        _ => None,
    }?;
    if data_type != METADATA_DATA_TYPE_BMP && value.len() >= PNG_SIGNATURE.len() {
        if value.starts_with(PNG_SIGNATURE) {
            Some(("png", "image/png"))
        } else {
            Some(("mjpeg", "image/jpeg"))
        }
    } else {
        Some(declared)
    }
}

fn merge_metadata(target: &mut ParsedMetadata, source: ParsedMetadata) -> AvResult<()> {
    let ParsedMetadata { tags, cover_art } = source;
    for entry in tags.entries() {
        target.tags.set(entry.key(), entry.value()).map_err(|err| {
            AvError::invalid_data(format!(
                "MOV/MP4 metadata value for {} is invalid: {}",
                entry.key(),
                err.message()
            ))
        })?;
    }
    target.cover_art.extend(cover_art);
    Ok(())
}

fn parse_freeform_metadata_item(
    input: &[u8],
    children: &[BoxHeader],
) -> AvResult<Option<(String, String)>> {
    let mut mean = None;
    let mut name = None;
    let mut value = None;

    for child in children {
        match &child.box_type {
            MEAN_ID => mean = Some(parse_freeform_text_child(child.payload(input), "mean")?),
            NAME_ID => name = Some(parse_freeform_text_child(child.payload(input), "name")?),
            DATA_ID => value = parse_freeform_data_child(child.payload(input))?,
            _ => {}
        }
    }

    let Some(mean) = mean else {
        return Ok(None);
    };
    let Some(name) = name else {
        return Ok(None);
    };
    let Some(value) = value else {
        return Ok(None);
    };
    if mean.is_empty() || name.is_empty() || name == "cdec" {
        return Ok(None);
    }
    Ok(Some((name, value)))
}

fn parse_freeform_text_child(payload: &[u8], child_name: &str) -> AvResult<String> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 freeform metadata child")?;
    let value = reader.read_exact(reader.remaining())?;
    std::str::from_utf8(value)
        .map(|value| value.to_owned())
        .map_err(|_| {
            AvError::invalid_data(format!(
                "MOV/MP4 freeform metadata {child_name} is not valid UTF-8"
            ))
        })
}

fn parse_freeform_data_child(payload: &[u8]) -> AvResult<Option<String>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 freeform metadata data")?;
    ensure_remaining(&reader, 4, "MOV/MP4 freeform metadata data")?;
    reader.skip(4)?;
    let value = reader.read_exact(reader.remaining())?;
    std::str::from_utf8(value)
        .map(|value| Some(value.to_owned()))
        .map_err(|_| AvError::invalid_data("MOV/MP4 freeform metadata data is not valid UTF-8"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetadataValueKind {
    Text,
    NumberPair,
    GenreIndex,
    Int8NoPadding,
    Int8WithPadding,
}

fn metadata_item_mapping(box_type: [u8; 4]) -> Option<(&'static str, MetadataValueKind)> {
    match box_type {
        [0xa9, b'n', b'a', b'm'] => Some(("title", MetadataValueKind::Text)),
        [0xa9, b'A', b'R', b'T'] => Some(("artist", MetadataValueKind::Text)),
        [b'a', b'A', b'R', b'T'] => Some(("album_artist", MetadataValueKind::Text)),
        [0xa9, b'a', b'l', b'b'] => Some(("album", MetadataValueKind::Text)),
        [0xa9, b'd', b'a', b'y'] => Some(("date", MetadataValueKind::Text)),
        [0xa9, b'g', b'e', b'n'] => Some(("genre", MetadataValueKind::Text)),
        [0xa9, b'c', b'm', b't'] => Some(("comment", MetadataValueKind::Text)),
        [b'd', b'e', b's', b'c'] => Some(("description", MetadataValueKind::Text)),
        [b'l', b'd', b'e', b's'] => Some(("long_description", MetadataValueKind::Text)),
        [0xa9, b't', b'o', b'o'] => Some(("encoder", MetadataValueKind::Text)),
        [b'a', b'k', b'I', b'D'] => Some(("account_type", MetadataValueKind::Int8NoPadding)),
        [b'c', b'p', b'i', b'l'] => Some(("compilation", MetadataValueKind::Int8NoPadding)),
        [b'e', b'g', b'i', b'd'] => Some(("episode_uid", MetadataValueKind::Int8NoPadding)),
        [b'g', b'n', b'r', b'e'] => Some(("genre", MetadataValueKind::GenreIndex)),
        [b'h', b'd', b'v', b'd'] => Some(("hd_video", MetadataValueKind::Int8NoPadding)),
        [b'p', b'c', b's', b't'] => Some(("podcast", MetadataValueKind::Int8NoPadding)),
        [b'p', b'g', b'a', b'p'] => Some(("gapless_playback", MetadataValueKind::Int8NoPadding)),
        [b'r', b't', b'n', b'g'] => Some(("rating", MetadataValueKind::Int8NoPadding)),
        [b's', b't', b'i', b'k'] => Some(("media_type", MetadataValueKind::Int8NoPadding)),
        [b't', b'r', b'k', b'n'] => Some(("track", MetadataValueKind::NumberPair)),
        [b't', b'v', b'e', b's'] => Some(("episode_sort", MetadataValueKind::Int8WithPadding)),
        [b't', b'v', b's', b'n'] => Some(("season_number", MetadataValueKind::Int8WithPadding)),
        [b'd', b'i', b's', b'k'] => Some(("disc", MetadataValueKind::NumberPair)),
        _ => None,
    }
}

fn parse_metadata_data(payload: &[u8], value_kind: MetadataValueKind) -> AvResult<Option<String>> {
    let (data_type, value) = parse_metadata_data_payload(payload, "MOV/MP4 metadata data")?;

    match value_kind {
        MetadataValueKind::Text => parse_text_metadata_value(data_type, value),
        MetadataValueKind::NumberPair => parse_number_pair_metadata_value(data_type, value),
        MetadataValueKind::GenreIndex => parse_genre_index_metadata_value(data_type, value),
        MetadataValueKind::Int8NoPadding => parse_int8_metadata_value(value, 0),
        MetadataValueKind::Int8WithPadding => parse_int8_metadata_value(value, 3),
    }
}

fn parse_metadata_data_payload<'a>(
    payload: &'a [u8],
    context: &'static str,
) -> AvResult<(u32, &'a [u8])> {
    let mut reader = ByteReader::new(payload);
    let (_version, flags) = read_full_box_header(&mut reader, context)?;
    ensure_remaining(&reader, 4, context)?;
    reader.skip(4)?;
    let value = reader.read_exact(reader.remaining())?;
    let data_type = u32::from(flags[0]) << 16 | u32::from(flags[1]) << 8 | u32::from(flags[2]);
    Ok((data_type, value))
}

fn parse_text_metadata_value(data_type: u32, value: &[u8]) -> AvResult<Option<String>> {
    match data_type {
        METADATA_DATA_TYPE_UTF8 => std::str::from_utf8(value)
            .map(|value| Some(value.to_owned()))
            .map_err(|_| AvError::invalid_data("MOV/MP4 text metadata is not valid UTF-8")),
        METADATA_DATA_TYPE_UTF16 => parse_utf16_metadata_value(value).map(Some),
        _ => Ok(None),
    }
}

fn parse_utf16_metadata_value(value: &[u8]) -> AvResult<String> {
    if value.len() % 2 != 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 text metadata is not valid UTF-16",
        ));
    }
    let (little_endian, payload) = match value {
        [0xfe, 0xff, rest @ ..] => (false, rest),
        [0xff, 0xfe, rest @ ..] => (true, rest),
        _ => (false, value),
    };
    let units = payload.chunks_exact(2).map(|bytes| {
        if little_endian {
            u16::from_le_bytes([bytes[0], bytes[1]])
        } else {
            u16::from_be_bytes([bytes[0], bytes[1]])
        }
    });
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map_err(|_| AvError::invalid_data("MOV/MP4 text metadata is not valid UTF-16"))
}

fn parse_number_pair_metadata_value(data_type: u32, value: &[u8]) -> AvResult<Option<String>> {
    if data_type != METADATA_DATA_TYPE_RESERVED {
        return Ok(None);
    }
    let mut reader = ByteReader::new(value);
    ensure_remaining(&reader, 6, "MOV/MP4 numeric metadata pair")?;
    reader.skip(2)?;
    let current = reader.read_u16_be()?;
    let total = reader.read_u16_be()?;
    if current == 0 && total == 0 {
        return Ok(None);
    }
    if total == 0 {
        Ok(Some(current.to_string()))
    } else {
        Ok(Some(format!("{current}/{total}")))
    }
}

fn parse_genre_index_metadata_value(data_type: u32, value: &[u8]) -> AvResult<Option<String>> {
    if data_type != METADATA_DATA_TYPE_RESERVED {
        return Ok(None);
    }
    let mut reader = ByteReader::new(value);
    ensure_remaining(&reader, 2, "MOV/MP4 genre metadata index")?;
    let one_based_index = reader.read_u16_be()?;
    ensure_box_consumed(&reader, "MOV/MP4 genre metadata index")?;
    if one_based_index == 0 {
        return Ok(None);
    }
    let index = usize::from(one_based_index - 1);
    let genre = ID3V1_GENRES.get(index).ok_or_else(|| {
        AvError::invalid_data(format!(
            "MOV/MP4 genre metadata index {one_based_index} is not recognized"
        ))
    })?;
    Ok(Some((*genre).to_owned()))
}

fn parse_int8_metadata_value(value: &[u8], padding_bytes: usize) -> AvResult<Option<String>> {
    let mut reader = ByteReader::new(value);
    ensure_remaining(&reader, padding_bytes + 1, "MOV/MP4 int8 metadata")?;
    if padding_bytes > 0 {
        reader.skip(padding_bytes)?;
    }
    Ok(Some(reader.read_u8()?.to_string()))
}

const ID3V1_GENRES: &[&str] = &[
    "Blues",
    "Classic Rock",
    "Country",
    "Dance",
    "Disco",
    "Funk",
    "Grunge",
    "Hip-Hop",
    "Jazz",
    "Metal",
    "New Age",
    "Oldies",
    "Other",
    "Pop",
    "R&B",
    "Rap",
    "Reggae",
    "Rock",
    "Techno",
    "Industrial",
    "Alternative",
    "Ska",
    "Death Metal",
    "Pranks",
    "Soundtrack",
    "Euro-Techno",
    "Ambient",
    "Trip-Hop",
    "Vocal",
    "Jazz+Funk",
    "Fusion",
    "Trance",
    "Classical",
    "Instrumental",
    "Acid",
    "House",
    "Game",
    "Sound Clip",
    "Gospel",
    "Noise",
    "Alternative Rock",
    "Bass",
    "Soul",
    "Punk",
    "Space",
    "Meditative",
    "Instrumental Pop",
    "Instrumental Rock",
    "Ethnic",
    "Gothic",
    "Darkwave",
    "Techno-Industrial",
    "Electronic",
    "Pop-Folk",
    "Eurodance",
    "Dream",
    "Southern Rock",
    "Comedy",
    "Cult",
    "Gangsta",
    "Top 40",
    "Christian Rap",
    "Pop/Funk",
    "Jungle",
    "Native US",
    "Cabaret",
    "New Wave",
    "Psychedelic",
    "Rave",
    "Showtunes",
    "Trailer",
    "Lo-Fi",
    "Tribal",
    "Acid Punk",
    "Acid Jazz",
    "Polka",
    "Retro",
    "Musical",
    "Rock & Roll",
    "Hard Rock",
];

fn parse_stbl(input: &[u8], stbl: &BoxHeader) -> AvResult<SampleTable> {
    let mut codec_parameters = None;
    let mut sample_durations = None;
    let mut composition_offsets = None;
    let mut sample_to_chunks = None;
    let mut sample_sizes = None;
    let mut chunk_offsets = None;
    let mut sync_sample_numbers = None;

    for child in read_box_headers(input, stbl.payload_start, stbl.payload_end, "MOV/MP4 stbl")? {
        match &child.box_type {
            STSD_ID => codec_parameters = Some(parse_stsd(child.payload(input))?),
            STTS_ID => sample_durations = Some(parse_stts(child.payload(input))?),
            CTTS_ID => composition_offsets = Some(parse_ctts(child.payload(input))?),
            STSC_ID => sample_to_chunks = Some(parse_stsc(child.payload(input))?),
            STSZ_ID => sample_sizes = Some(parse_stsz(child.payload(input))?),
            STSS_ID => sync_sample_numbers = Some(parse_stss(child.payload(input))?),
            STCO_ID => chunk_offsets = Some(parse_stco(child.payload(input))?),
            CO64_ID => chunk_offsets = Some(parse_co64(child.payload(input))?),
            _ => {}
        }
    }

    let codec_parameters =
        codec_parameters.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stsd"))?;
    let sample_durations =
        sample_durations.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stts"))?;
    let sample_to_chunks =
        sample_to_chunks.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stsc"))?;
    let sample_sizes =
        sample_sizes.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stsz"))?;
    let chunk_offsets =
        chunk_offsets.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stco/co64"))?;
    if sample_durations.len() != sample_sizes.len() {
        return Err(AvError::invalid_data(format!(
            "MOV/MP4 stts describes {} samples but stsz describes {}",
            sample_durations.len(),
            sample_sizes.len()
        )));
    }
    if let Some(offsets) = &composition_offsets {
        if offsets.len() != sample_sizes.len() {
            return Err(AvError::invalid_data(format!(
                "MOV/MP4 ctts describes {} samples but stsz describes {}",
                offsets.len(),
                sample_sizes.len()
            )));
        }
    }
    let sync_samples = sync_sample_numbers
        .map(|sample_numbers| sync_sample_flags(sample_numbers, sample_sizes.len()))
        .transpose()?;

    Ok(SampleTable {
        codec_parameters,
        sample_sizes,
        sample_durations,
        composition_offsets,
        sample_to_chunks,
        chunk_offsets,
        sync_samples,
    })
}

fn parse_stsd(payload: &[u8]) -> AvResult<MovCodecParameters> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stsd")?;
    let entry_count = reader.read_u32_be()?;
    if entry_count == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 stsd must contain at least one sample entry",
        ));
    }
    if entry_count != 1 {
        return Err(AvError::unsupported(
            "MOV/MP4 stsd with multiple sample entries is not implemented",
        ));
    }
    ensure_remaining(&reader, 16, "MOV/MP4 stsd sample entry")?;
    let entry_size = usize::try_from(reader.read_u32_be()?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 stsd sample entry size is out of range"))?;
    if entry_size < 16 {
        return Err(AvError::invalid_data(
            "MOV/MP4 stsd sample entry is shorter than its base header",
        ));
    }
    let entry_type = read_fourcc(&mut reader)?;
    let remaining_entry = entry_size
        .checked_sub(8)
        .ok_or_else(|| AvError::invalid_data("MOV/MP4 stsd sample entry size underflow"))?;
    if remaining_entry > reader.remaining() {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "MOV/MP4 stsd sample entry exceeds box payload",
        ));
    }
    reader.skip(6)?;
    let data_reference_index = reader.read_u16_be()?;
    let extra_data = reader.read_exact(entry_size - 16)?.to_vec();
    if !reader.is_eof() {
        return Err(AvError::invalid_data(
            "MOV/MP4 stsd contains trailing sample entry data",
        ));
    }
    let codec_tag = fourcc_to_string(entry_type);
    let details = parse_sample_entry_details(&codec_tag, &extra_data)?;
    Ok(MovCodecParameters {
        codec_tag,
        data_reference_index,
        extra_data,
        details,
    })
}

fn parse_sample_entry_details(
    codec_tag: &str,
    extra_data: &[u8],
) -> AvResult<MovSampleEntryDetails> {
    match codec_tag.as_bytes() {
        b"avc1" | b"hvc1" | b"hev1" | b"mp4v" => parse_visual_sample_entry(extra_data)
            .map(Box::new)
            .map(MovSampleEntryDetails::Video),
        b"raw " if extra_data.len() >= 70 => parse_visual_sample_entry(extra_data)
            .map(Box::new)
            .map(MovSampleEntryDetails::Video),
        tag if is_audio_sample_entry(tag) => parse_audio_sample_entry(tag, extra_data)
            .map(Box::new)
            .map(MovSampleEntryDetails::Audio),
        b"tx3g" => parse_timed_text_sample_entry(extra_data)
            .map(MovSubtitleSampleEntry::TimedText)
            .map(MovSampleEntryDetails::Subtitle),
        b"metx" => parse_xml_metadata_sample_entry(extra_data)
            .map(MovDataSampleEntry::XmlMetadata)
            .map(MovSampleEntryDetails::Data),
        b"mett" => parse_text_metadata_sample_entry(extra_data)
            .map(MovDataSampleEntry::TextMetadata)
            .map(MovSampleEntryDetails::Data),
        _ => Ok(MovSampleEntryDetails::Generic),
    }
}

fn is_audio_sample_entry(codec_tag: &[u8]) -> bool {
    matches!(
        codec_tag,
        b"mp4a"
            | b"alac"
            | b"Opus"
            | b"fLaC"
            | b"enca"
            | b"ac-3"
            | b"ec-3"
            | b"lpcm"
            | b"sowt"
            | b"twos"
            | b"in24"
            | b"in32"
            | b"fl32"
            | b"fl64"
            | b"ulaw"
            | b"alaw"
            | b"samr"
            | b"sawb"
    )
}

fn parse_timed_text_sample_entry(extra_data: &[u8]) -> AvResult<MovTimedTextSampleEntry> {
    let mut reader = ByteReader::new(extra_data);
    ensure_remaining(&reader, 30, "MOV/MP4 tx3g sample entry")?;
    let display_flags = reader.read_u32_be()?;
    let horizontal_justification = reader.read_i8()?;
    let vertical_justification = reader.read_i8()?;
    let background_color_rgba = read_fixed_array_4(&mut reader)?;
    let default_text_box = MovTextBoxRecord {
        top: reader.read_i16_be()?,
        left: reader.read_i16_be()?,
        bottom: reader.read_i16_be()?,
        right: reader.read_i16_be()?,
    };
    let default_style = MovTextStyleRecord {
        start_char: reader.read_u16_be()?,
        end_char: reader.read_u16_be()?,
        font_id: reader.read_u16_be()?,
        face_style_flags: reader.read_u8()?,
        font_size: reader.read_u8()?,
        text_color_rgba: read_fixed_array_4(&mut reader)?,
    };
    let child_boxes = parse_sample_entry_child_boxes(
        extra_data,
        reader.position(),
        extra_data.len(),
        "MOV/MP4 tx3g sample entry children",
    )?;
    Ok(MovTimedTextSampleEntry {
        display_flags,
        horizontal_justification,
        vertical_justification,
        background_color_rgba,
        default_text_box,
        default_style,
        child_boxes,
    })
}

fn parse_xml_metadata_sample_entry(extra_data: &[u8]) -> AvResult<MovXmlMetadataSampleEntry> {
    let mut reader = ByteReader::new(extra_data);
    let content_encoding = read_null_terminated_utf8(&mut reader, "MOV/MP4 metx content encoding")?;
    let namespace = read_null_terminated_utf8(&mut reader, "MOV/MP4 metx namespace")?;
    let schema_location = read_null_terminated_utf8(&mut reader, "MOV/MP4 metx schema location")?;
    ensure_box_consumed(&reader, "MOV/MP4 metx sample entry")?;
    Ok(MovXmlMetadataSampleEntry {
        content_encoding,
        namespace,
        schema_location,
    })
}

fn parse_text_metadata_sample_entry(extra_data: &[u8]) -> AvResult<MovTextMetadataSampleEntry> {
    let mut reader = ByteReader::new(extra_data);
    let content_encoding = read_null_terminated_utf8(&mut reader, "MOV/MP4 mett content encoding")?;
    let mime_format = read_null_terminated_utf8(&mut reader, "MOV/MP4 mett MIME format")?;
    ensure_box_consumed(&reader, "MOV/MP4 mett sample entry")?;
    Ok(MovTextMetadataSampleEntry {
        content_encoding,
        mime_format,
    })
}

fn read_fixed_array_4(reader: &mut ByteReader<'_>) -> AvResult<[u8; 4]> {
    let bytes = reader.read_exact(4)?;
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_null_terminated_utf8(reader: &mut ByteReader<'_>, context: &str) -> AvResult<String> {
    let mut bytes = Vec::new();
    while !reader.is_eof() {
        let byte = reader.read_u8()?;
        if byte == 0 {
            return std::str::from_utf8(&bytes)
                .map(str::to_owned)
                .map_err(|_| AvError::invalid_data(format!("{context} is not valid UTF-8")));
        }
        bytes.push(byte);
    }
    Err(AvError::invalid_data(format!(
        "{context} is missing a null terminator"
    )))
}

fn parse_audio_sample_entry(codec_tag: &[u8], extra_data: &[u8]) -> AvResult<MovAudioSampleEntry> {
    const SAMPLE_ENTRY_BASE_HEADER_SIZE: usize = 16;
    const AUDIO_SAMPLE_ENTRY_V2_STRUCT_SIZE: usize = 72;

    let mut reader = ByteReader::new(extra_data);
    ensure_remaining(&reader, 20, "MOV/MP4 AudioSampleEntry")?;
    let version = reader.read_u16_be()?;
    let revision_level = reader.read_u16_be()?;
    let vendor = reader.read_u32_be()?;
    let channel_count = reader.read_u16_be()?;
    let sample_size = reader.read_u16_be()?;
    let compression_id = reader.read_i16_be()?;
    let packet_size = reader.read_u16_be()?;
    let sample_rate_fixed_16_16 = reader.read_u32_be()?;
    let version_fields = match version {
        0 => MovAudioSampleEntryVersionFields::Version0,
        1 => {
            ensure_remaining(&reader, 16, "MOV/MP4 AudioSampleEntry version 1")?;
            MovAudioSampleEntryVersionFields::Version1(MovAudioSampleEntryVersion1Fields {
                samples_per_packet: reader.read_u32_be()?,
                bytes_per_packet: reader.read_u32_be()?,
                bytes_per_frame: reader.read_u32_be()?,
                bytes_per_sample: reader.read_u32_be()?,
            })
        }
        2 => {
            ensure_remaining(&reader, 36, "MOV/MP4 AudioSampleEntry version 2")?;
            MovAudioSampleEntryVersionFields::Version2(MovAudioSampleEntryVersion2Fields {
                size_of_struct_only: reader.read_u32_be()?,
                audio_sample_rate_bits: reader.read_u64_be()?,
                num_audio_channels: reader.read_u32_be()?,
                always_7f000000: reader.read_u32_be()?,
                const_bits_per_channel: reader.read_u32_be()?,
                format_specific_flags: reader.read_u32_be()?,
                const_bytes_per_audio_packet: reader.read_u32_be()?,
                const_lpcm_frames_per_audio_packet: reader.read_u32_be()?,
            })
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "MOV/MP4 AudioSampleEntry version {version} is not implemented"
            )));
        }
    };
    let sample_rate = audio_sample_entry_sample_rate(sample_rate_fixed_16_16, &version_fields)?;
    let child_boxes_start = match &version_fields {
        MovAudioSampleEntryVersionFields::Version0
        | MovAudioSampleEntryVersionFields::Version1(_) => reader.position(),
        MovAudioSampleEntryVersionFields::Version2(fields) => {
            let size_of_struct_only =
                usize::try_from(fields.size_of_struct_only()).map_err(|_| {
                    AvError::invalid_data(
                        "MOV/MP4 AudioSampleEntry version 2 struct size is out of range",
                    )
                })?;
            if size_of_struct_only < AUDIO_SAMPLE_ENTRY_V2_STRUCT_SIZE {
                return Err(AvError::invalid_data(
                    "MOV/MP4 AudioSampleEntry version 2 struct size is too small",
                ));
            }
            let child_boxes_start = size_of_struct_only
                .checked_sub(SAMPLE_ENTRY_BASE_HEADER_SIZE)
                .ok_or_else(|| {
                    AvError::invalid_data(
                        "MOV/MP4 AudioSampleEntry version 2 struct size underflow",
                    )
                })?;
            if child_boxes_start < reader.position() {
                return Err(AvError::invalid_data(
                    "MOV/MP4 AudioSampleEntry version 2 extension offset overlaps fields",
                ));
            }
            if child_boxes_start > extra_data.len() {
                return Err(AvError::new(
                    AvErrorKind::EndOfFile,
                    "MOV/MP4 AudioSampleEntry version 2 extension offset exceeds sample entry",
                ));
            }
            reader.skip(child_boxes_start - reader.position())?;
            reader.position()
        }
    };
    let child_boxes = parse_sample_entry_child_boxes(
        extra_data,
        child_boxes_start,
        extra_data.len(),
        "MOV/MP4 AudioSampleEntry children",
    )?;
    let elementary_stream_descriptor =
        parse_audio_sample_entry_elementary_stream_descriptor(&child_boxes)?;
    let bit_rate = parse_audio_sample_entry_bit_rate(&child_boxes)?;
    let amr_specific = parse_audio_sample_entry_amr_specific(&child_boxes)?;
    let ac3_specific = parse_audio_sample_entry_ac3_specific(&child_boxes, codec_tag == b"ac-3")?;
    let ec3_specific = parse_audio_sample_entry_ec3_specific(&child_boxes, codec_tag == b"ec-3")?;
    let opus_specific = parse_audio_sample_entry_opus_specific(&child_boxes, codec_tag == b"Opus")?;
    let flac_specific = parse_audio_sample_entry_flac_specific(&child_boxes, codec_tag == b"fLaC")?;
    let alac_specific = parse_audio_sample_entry_alac_specific(&child_boxes)?;
    let wave_extension = parse_audio_sample_entry_wave_extension(&child_boxes)?;
    validate_audio_sample_entry_alac_boxes(
        codec_tag == ALAC_ID,
        &alac_specific,
        wave_extension
            .as_ref()
            .and_then(MovAudioWaveExtension::alac_specific),
    )?;
    validate_audio_sample_entry_alac_fields(
        channel_count,
        sample_size,
        sample_rate,
        alac_specific.as_ref().or_else(|| {
            wave_extension
                .as_ref()
                .and_then(MovAudioWaveExtension::alac_specific)
        }),
    )?;
    let sample_rate = flac_specific
        .as_ref()
        .map_or(sample_rate, |specific| specific.streaminfo().sample_rate());
    let sample_rate = alac_specific
        .as_ref()
        .or_else(|| {
            wave_extension
                .as_ref()
                .and_then(MovAudioWaveExtension::alac_specific)
        })
        .map_or(sample_rate, |specific| specific.config().sample_rate());
    validate_audio_sample_entry_flac_fields(channel_count, sample_size, &flac_specific)?;
    let channel_layout = parse_audio_sample_entry_channel_layout(&child_boxes)?;
    Ok(MovAudioSampleEntry {
        version,
        revision_level,
        vendor,
        channel_count,
        sample_size,
        compression_id,
        packet_size,
        sample_rate,
        sample_rate_fixed_16_16,
        version_fields,
        elementary_stream_descriptor,
        bit_rate,
        amr_specific,
        ac3_specific,
        ec3_specific,
        opus_specific,
        flac_specific,
        alac_specific,
        wave_extension,
        channel_layout,
        child_boxes,
    })
}

fn audio_sample_entry_sample_rate(
    sample_rate_fixed_16_16: u32,
    version_fields: &MovAudioSampleEntryVersionFields,
) -> AvResult<u32> {
    match version_fields {
        MovAudioSampleEntryVersionFields::Version2(fields) => {
            let sample_rate = fields.audio_sample_rate();
            if !sample_rate.is_finite() || sample_rate < 0.0 || sample_rate > f64::from(u32::MAX) {
                return Err(AvError::invalid_data(
                    "MOV/MP4 AudioSampleEntry version 2 sample rate is invalid",
                ));
            }
            Ok(sample_rate as u32)
        }
        MovAudioSampleEntryVersionFields::Version0
        | MovAudioSampleEntryVersionFields::Version1(_) => Ok(sample_rate_fixed_16_16 >> 16),
    }
}

fn parse_audio_sample_entry_wave_extension(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovAudioWaveExtension>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == WAVE_ID)
        .map(|child| parse_wave_extension(child.payload()))
        .transpose()
}

fn parse_audio_sample_entry_elementary_stream_descriptor(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovElementaryStreamDescriptor>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == ESDS_ID)
        .map(|child| parse_esds(child.payload()))
        .transpose()
}

fn parse_audio_sample_entry_bit_rate(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovBitRateBox>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == BTRT_ID)
        .map(|child| parse_btrt(child.payload()))
        .transpose()
}

fn parse_audio_sample_entry_amr_specific(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovAmrSpecificBox>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == DAMR_ID)
        .map(|child| parse_damr(child.payload()))
        .transpose()
}

fn parse_audio_sample_entry_ac3_specific(
    child_boxes: &[MovSampleEntryChildBox],
    required: bool,
) -> AvResult<Option<MovAc3SpecificBox>> {
    let mut matches = child_boxes
        .iter()
        .filter(|child| child.box_type.as_bytes() == DAC3_ID);
    let Some(child) = matches.next() else {
        if required {
            return Err(AvError::invalid_data(
                "MOV/MP4 AC-3 sample entry is missing required dac3 box",
            ));
        }
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(AvError::invalid_data(
            "MOV/MP4 AC-3 sample entry must not contain multiple dac3 boxes",
        ));
    }
    parse_dac3(child.payload()).map(Some)
}

fn parse_audio_sample_entry_ec3_specific(
    child_boxes: &[MovSampleEntryChildBox],
    required: bool,
) -> AvResult<Option<MovEc3SpecificBox>> {
    let mut matches = child_boxes
        .iter()
        .filter(|child| child.box_type.as_bytes() == DEC3_ID);
    let Some(child) = matches.next() else {
        if required {
            return Err(AvError::invalid_data(
                "MOV/MP4 E-AC-3 sample entry is missing required dec3 box",
            ));
        }
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(AvError::invalid_data(
            "MOV/MP4 E-AC-3 sample entry must not contain multiple dec3 boxes",
        ));
    }
    parse_dec3(child.payload()).map(Some)
}

fn parse_audio_sample_entry_opus_specific(
    child_boxes: &[MovSampleEntryChildBox],
    required: bool,
) -> AvResult<Option<MovOpusSpecificBox>> {
    let mut matches = child_boxes
        .iter()
        .filter(|child| child.box_type.as_bytes() == DOPS_ID);
    let Some(child) = matches.next() else {
        if required {
            return Err(AvError::invalid_data(
                "MOV/MP4 Opus sample entry is missing required dOps box",
            ));
        }
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(AvError::invalid_data(
            "MOV/MP4 Opus sample entry must not contain multiple dOps boxes",
        ));
    }
    parse_dops(child.payload()).map(Some)
}

fn parse_audio_sample_entry_flac_specific(
    child_boxes: &[MovSampleEntryChildBox],
    required: bool,
) -> AvResult<Option<MovFlacSpecificBox>> {
    let mut matches = child_boxes
        .iter()
        .filter(|child| child.box_type.as_bytes() == DFLA_ID);
    let Some(child) = matches.next() else {
        if required {
            return Err(AvError::invalid_data(
                "MOV/MP4 FLAC sample entry is missing required dfLa box",
            ));
        }
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(AvError::invalid_data(
            "MOV/MP4 FLAC sample entry must not contain multiple dfLa boxes",
        ));
    }
    parse_dfla(child.payload()).map(Some)
}

fn parse_audio_sample_entry_alac_specific(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovAlacSpecificBox>> {
    let mut matches = child_boxes
        .iter()
        .filter(|child| child.box_type.as_bytes() == ALAC_ID);
    let Some(child) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(AvError::invalid_data(
            "MOV/MP4 ALAC sample entry must not contain multiple alac boxes",
        ));
    }
    parse_alac(child.payload()).map(Some)
}

fn validate_audio_sample_entry_alac_boxes(
    required: bool,
    direct_alac: &Option<MovAlacSpecificBox>,
    wave_alac: Option<&MovAlacSpecificBox>,
) -> AvResult<()> {
    if direct_alac.is_some() && wave_alac.is_some() {
        return Err(AvError::invalid_data(
            "MOV/MP4 ALAC sample entry must not contain both direct and wave alac boxes",
        ));
    }
    if required && direct_alac.is_none() && wave_alac.is_none() {
        return Err(AvError::invalid_data(
            "MOV/MP4 ALAC sample entry is missing required alac box",
        ));
    }
    Ok(())
}

fn validate_audio_sample_entry_alac_fields(
    channel_count: u16,
    sample_size: u16,
    sample_rate: u32,
    alac_specific: Option<&MovAlacSpecificBox>,
) -> AvResult<()> {
    let Some(alac_specific) = alac_specific else {
        return Ok(());
    };
    let config = alac_specific.config();
    if u32::from(channel_count) != u32::from(config.num_channels()) {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac channelcount must match ALAC specific config",
        ));
    }
    if u32::from(sample_size) != u32::from(config.bit_depth()) {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac samplesize must match ALAC specific config",
        ));
    }
    if sample_rate != config.sample_rate() {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac sample rate must match ALAC specific config",
        ));
    }
    Ok(())
}

fn validate_audio_sample_entry_flac_fields(
    channel_count: u16,
    sample_size: u16,
    flac_specific: &Option<MovFlacSpecificBox>,
) -> AvResult<()> {
    let Some(flac_specific) = flac_specific else {
        return Ok(());
    };
    let streaminfo = flac_specific.streaminfo();
    if u32::from(channel_count) != u32::from(streaminfo.channels()) {
        return Err(AvError::invalid_data(
            "MOV/MP4 fLaC channelcount must match dfLa STREAMINFO",
        ));
    }
    if u32::from(sample_size) != u32::from(streaminfo.bits_per_sample()) {
        return Err(AvError::invalid_data(
            "MOV/MP4 fLaC samplesize must match dfLa STREAMINFO",
        ));
    }
    Ok(())
}

fn parse_audio_sample_entry_channel_layout(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovAudioChannelLayout>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == CHAN_ID)
        .map(|child| parse_audio_channel_layout(child.payload()))
        .transpose()
}

fn parse_wave_extension(payload: &[u8]) -> AvResult<MovAudioWaveExtension> {
    let child_boxes = parse_sample_entry_child_boxes(payload, 0, payload.len(), "MOV/MP4 wave")?;
    let original_format = parse_wave_original_format(&child_boxes)?;
    let elementary_stream_descriptor = parse_wave_esds(&child_boxes)?;
    let alac_specific = parse_wave_alac_specific(&child_boxes)?;
    let has_terminator = child_boxes
        .iter()
        .any(|child| child.box_type.as_bytes() == TERMINATOR_ID);
    validate_wave_terminator(&child_boxes)?;
    Ok(MovAudioWaveExtension {
        original_format,
        elementary_stream_descriptor,
        alac_specific,
        has_terminator,
        child_boxes,
    })
}

fn parse_wave_original_format(child_boxes: &[MovSampleEntryChildBox]) -> AvResult<Option<String>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == FRMA_ID)
        .map(|child| {
            if child.payload().len() != 4 {
                return Err(AvError::invalid_data(
                    "MOV/MP4 wave frma atom must contain exactly one FourCC",
                ));
            }
            let mut format = [0_u8; 4];
            format.copy_from_slice(child.payload());
            Ok(fourcc_to_string(format))
        })
        .transpose()
}

fn parse_wave_esds(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovElementaryStreamDescriptor>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == ESDS_ID)
        .map(|child| parse_esds(child.payload()))
        .transpose()
}

fn parse_wave_alac_specific(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovAlacSpecificBox>> {
    let mut matches = child_boxes
        .iter()
        .filter(|child| child.box_type.as_bytes() == ALAC_ID);
    let Some(child) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(AvError::invalid_data(
            "MOV/MP4 wave atom must not contain multiple alac boxes",
        ));
    }
    parse_alac(child.payload()).map(Some)
}

fn parse_esds(payload: &[u8]) -> AvResult<MovElementaryStreamDescriptor> {
    let mut reader = ByteReader::new(payload);
    let (version, flags) = read_full_box_header(&mut reader, "MOV/MP4 esds")?;
    if version != 0 {
        return Err(AvError::unsupported(format!(
            "MOV/MP4 esds version {version} is not implemented"
        )));
    }
    if flags != [0, 0, 0] {
        return Err(AvError::invalid_data("MOV/MP4 esds flags must be zero"));
    }
    let descriptor = reader.read_exact(reader.remaining())?.to_vec();
    if descriptor.is_empty() {
        return Err(AvError::invalid_data(
            "MOV/MP4 esds descriptor payload must not be empty",
        ));
    }
    Ok(MovElementaryStreamDescriptor {
        version,
        flags,
        descriptor,
    })
}

fn parse_btrt(payload: &[u8]) -> AvResult<MovBitRateBox> {
    let mut reader = ByteReader::new(payload);
    ensure_remaining(&reader, 12, "MOV/MP4 btrt")?;
    let buffer_size_db = reader.read_u32_be()?;
    let max_bitrate = reader.read_u32_be()?;
    let avg_bitrate = reader.read_u32_be()?;
    ensure_box_consumed(&reader, "MOV/MP4 btrt")?;
    Ok(MovBitRateBox {
        buffer_size_db,
        max_bitrate,
        avg_bitrate,
    })
}

fn parse_damr(payload: &[u8]) -> AvResult<MovAmrSpecificBox> {
    let mut reader = ByteReader::new(payload);
    ensure_remaining(&reader, 9, "MOV/MP4 damr")?;
    let vendor = fourcc_to_string(read_fourcc(&mut reader)?);
    let decoder_version = reader.read_u8()?;
    let mode_set = reader.read_u16_be()?;
    let mode_change_period = reader.read_u8()?;
    let frames_per_sample = reader.read_u8()?;
    ensure_box_consumed(&reader, "MOV/MP4 damr")?;
    if frames_per_sample == 0 || frames_per_sample >= 16 {
        return Err(AvError::invalid_data(
            "MOV/MP4 damr frames_per_sample must be in 1..16",
        ));
    }
    Ok(MovAmrSpecificBox {
        vendor,
        decoder_version,
        mode_set,
        mode_change_period,
        frames_per_sample,
    })
}

fn parse_dac3(payload: &[u8]) -> AvResult<MovAc3SpecificBox> {
    if payload.len() != 3 {
        return Err(if payload.len() < 3 {
            AvError::new(
                AvErrorKind::EndOfFile,
                "MOV/MP4 dac3 payload is shorter than 24 bits",
            )
        } else {
            AvError::invalid_data("MOV/MP4 dac3 payload must contain exactly 24 bits")
        });
    }

    let mut reader = BitReader::new(payload);
    let fscod = u8::try_from(reader.read_bits(2)?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 dac3 fscod is out of range"))?;
    let bsid = u8::try_from(reader.read_bits(5)?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 dac3 bsid is out of range"))?;
    let bsmod = u8::try_from(reader.read_bits(3)?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 dac3 bsmod is out of range"))?;
    let acmod = u8::try_from(reader.read_bits(3)?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 dac3 acmod is out of range"))?;
    let lfeon = reader.read_bit()?;
    let bit_rate_code = u8::try_from(reader.read_bits(5)?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 dac3 bit_rate_code is out of range"))?;
    let reserved = reader.read_bits(5)?;

    if fscod == 3 {
        return Err(AvError::invalid_data(
            "MOV/MP4 dac3 fscod value 3 is reserved",
        ));
    }
    if bit_rate_code > 18 {
        return Err(AvError::invalid_data(
            "MOV/MP4 dac3 bit_rate_code must be in 0..=18",
        ));
    }
    if reserved != 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 dac3 reserved bits must be zero",
        ));
    }
    if !reader.is_eof() {
        return Err(AvError::invalid_data("MOV/MP4 dac3 has trailing bits"));
    }

    Ok(MovAc3SpecificBox {
        fscod,
        bsid,
        bsmod,
        acmod,
        lfeon,
        bit_rate_code,
    })
}

fn parse_dec3(payload: &[u8]) -> AvResult<MovEc3SpecificBox> {
    let mut reader = BitReader::new(payload);
    let data_rate = u16::try_from(reader.read_bits(13)?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 dec3 data_rate is out of range"))?;
    let num_ind_sub = u8::try_from(reader.read_bits(3)?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 dec3 num_ind_sub is out of range"))?;
    let substream_count = usize::from(num_ind_sub) + 1;
    let mut substreams = Vec::with_capacity(substream_count);

    for _ in 0..substream_count {
        let fscod = u8::try_from(reader.read_bits(2)?)
            .map_err(|_| AvError::invalid_data("MOV/MP4 dec3 fscod is out of range"))?;
        let bsid = u8::try_from(reader.read_bits(5)?)
            .map_err(|_| AvError::invalid_data("MOV/MP4 dec3 bsid is out of range"))?;
        let reserved = reader.read_bit()?;
        let asvc = reader.read_bit()?;
        let bsmod = u8::try_from(reader.read_bits(3)?)
            .map_err(|_| AvError::invalid_data("MOV/MP4 dec3 bsmod is out of range"))?;
        let acmod = u8::try_from(reader.read_bits(3)?)
            .map_err(|_| AvError::invalid_data("MOV/MP4 dec3 acmod is out of range"))?;
        let lfeon = reader.read_bit()?;
        let reserved3 = reader.read_bits(3)?;
        let num_dep_sub = u8::try_from(reader.read_bits(4)?)
            .map_err(|_| AvError::invalid_data("MOV/MP4 dec3 num_dep_sub is out of range"))?;

        if reserved {
            return Err(AvError::invalid_data(
                "MOV/MP4 dec3 reserved independent-substream bit must be zero",
            ));
        }
        if reserved3 != 0 {
            return Err(AvError::invalid_data(
                "MOV/MP4 dec3 reserved independent-substream bits must be zero",
            ));
        }

        let chan_loc = if num_dep_sub > 0 {
            Some(
                u16::try_from(reader.read_bits(9)?)
                    .map_err(|_| AvError::invalid_data("MOV/MP4 dec3 chan_loc is out of range"))?,
            )
        } else {
            let reserved = reader.read_bit()?;
            if reserved {
                return Err(AvError::invalid_data(
                    "MOV/MP4 dec3 no-dependent-substream reserved bit must be zero",
                ));
            }
            None
        };

        substreams.push(MovEc3IndependentSubstream {
            fscod,
            bsid,
            asvc,
            bsmod,
            acmod,
            lfeon,
            num_dep_sub,
            chan_loc,
        });
    }

    let trailing_reserved_bytes = if reader.is_aligned() {
        payload[reader.bit_position() / 8..].to_vec()
    } else {
        return Err(AvError::invalid_data(
            "MOV/MP4 dec3 parser stopped at a non-byte-aligned reserved field",
        ));
    };

    Ok(MovEc3SpecificBox {
        data_rate,
        num_ind_sub,
        substreams,
        trailing_reserved_bytes,
    })
}

fn parse_dops(payload: &[u8]) -> AvResult<MovOpusSpecificBox> {
    let mut reader = ByteReader::new(payload);
    ensure_remaining(&reader, 1, "MOV/MP4 dOps version")?;
    let version = reader.read_u8()?;
    if version != 0 {
        return Err(AvError::unsupported(format!(
            "MOV/MP4 dOps version {version} is not implemented"
        )));
    }

    ensure_remaining(&reader, 10, "MOV/MP4 dOps version 0 fields")?;
    let output_channel_count = reader.read_u8()?;
    if output_channel_count == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 dOps output channel count must be nonzero",
        ));
    }
    let pre_skip = reader.read_u16_be()?;
    let input_sample_rate = reader.read_u32_be()?;
    let output_gain = reader.read_i16_be()?;
    let channel_mapping_family = reader.read_u8()?;
    let (stream_count, coupled_count, channel_mapping) = if channel_mapping_family == 0 {
        ensure_box_consumed(&reader, "MOV/MP4 dOps")?;
        (None, None, Vec::new())
    } else {
        ensure_remaining(&reader, 2, "MOV/MP4 dOps channel mapping table")?;
        let stream_count = reader.read_u8()?;
        let coupled_count = reader.read_u8()?;
        if stream_count == 0 {
            return Err(AvError::invalid_data(
                "MOV/MP4 dOps stream count must be nonzero",
            ));
        }
        if coupled_count > stream_count {
            return Err(AvError::invalid_data(
                "MOV/MP4 dOps coupled count must not exceed stream count",
            ));
        }
        let channel_mapping = reader
            .read_exact(usize::from(output_channel_count))?
            .to_vec();
        ensure_box_consumed(&reader, "MOV/MP4 dOps")?;
        (Some(stream_count), Some(coupled_count), channel_mapping)
    };

    Ok(MovOpusSpecificBox {
        version,
        output_channel_count,
        pre_skip,
        input_sample_rate,
        output_gain,
        channel_mapping_family,
        stream_count,
        coupled_count,
        channel_mapping,
    })
}

fn parse_alac(payload: &[u8]) -> AvResult<MovAlacSpecificBox> {
    let mut reader = ByteReader::new(payload);
    let (version, flags) = read_full_box_header(&mut reader, "MOV/MP4 alac")?;
    if version != 0 {
        return Err(AvError::unsupported(format!(
            "MOV/MP4 alac version {version} is not implemented"
        )));
    }
    if flags != [0, 0, 0] {
        return Err(AvError::invalid_data("MOV/MP4 alac flags must be zero"));
    }

    let config = parse_alac_specific_config(&mut reader)?;
    let channel_layout = match reader.remaining() {
        0 => None,
        24 => Some(parse_alac_channel_layout_info(&mut reader)?),
        remaining if remaining < 24 => {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                "MOV/MP4 alac channel layout info is truncated",
            ));
        }
        _ => {
            return Err(AvError::invalid_data(
                "MOV/MP4 alac box contains unsupported trailing bytes",
            ));
        }
    };
    ensure_box_consumed(&reader, "MOV/MP4 alac")?;

    Ok(MovAlacSpecificBox {
        version,
        flags,
        config,
        channel_layout,
    })
}

fn parse_alac_specific_config(reader: &mut ByteReader<'_>) -> AvResult<MovAlacSpecificConfig> {
    ensure_remaining(reader, 24, "MOV/MP4 alac specific config")?;
    let frame_length = reader.read_u32_be()?;
    let compatible_version = reader.read_u8()?;
    let bit_depth = reader.read_u8()?;
    let pb = reader.read_u8()?;
    let mb = reader.read_u8()?;
    let kb = reader.read_u8()?;
    let num_channels = reader.read_u8()?;
    let max_run = reader.read_u16_be()?;
    let max_frame_bytes = reader.read_u32_be()?;
    let avg_bit_rate = reader.read_u32_be()?;
    let sample_rate = reader.read_u32_be()?;

    if frame_length == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac frame length must be nonzero",
        ));
    }
    if compatible_version != 0 {
        return Err(AvError::unsupported(format!(
            "MOV/MP4 alac compatible version {compatible_version} is not implemented"
        )));
    }
    if bit_depth == 0 || bit_depth > 32 {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac bit depth must be in 1..=32",
        ));
    }
    if num_channels == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac channel count must be nonzero",
        ));
    }
    if sample_rate == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac sample rate must be nonzero",
        ));
    }

    Ok(MovAlacSpecificConfig {
        frame_length,
        compatible_version,
        bit_depth,
        pb,
        mb,
        kb,
        num_channels,
        max_run,
        max_frame_bytes,
        avg_bit_rate,
        sample_rate,
    })
}

fn parse_alac_channel_layout_info(
    reader: &mut ByteReader<'_>,
) -> AvResult<MovAlacChannelLayoutInfo> {
    ensure_remaining(reader, 24, "MOV/MP4 alac channel layout info")?;
    let size = reader.read_u32_be()?;
    let atom_type = read_fourcc(reader)?;
    let (version, flags) = read_full_box_header(reader, "MOV/MP4 alac channel layout info")?;
    let channel_layout_tag = reader.read_u32_be()?;
    let reserved1 = reader.read_u32_be()?;
    let reserved2 = reader.read_u32_be()?;

    if size != 24 {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac channel layout info size must be 24",
        ));
    }
    if &atom_type != CHAN_ID {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac channel layout info atom type must be chan",
        ));
    }
    if version != 0 {
        return Err(AvError::unsupported(format!(
            "MOV/MP4 alac channel layout info version {version} is not implemented"
        )));
    }
    if flags != [0, 0, 0] {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac channel layout info flags must be zero",
        ));
    }
    if reserved1 != 0 || reserved2 != 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 alac channel layout info reserved fields must be zero",
        ));
    }

    Ok(MovAlacChannelLayoutInfo {
        channel_layout_tag,
        reserved: [reserved1, reserved2],
    })
}

fn parse_dfla(payload: &[u8]) -> AvResult<MovFlacSpecificBox> {
    const FLAC_METADATA_BLOCK_FORBIDDEN: u8 = 127;
    const FLAC_METADATA_BLOCK_STREAMINFO: u8 = 0;

    let mut reader = ByteReader::new(payload);
    let (version, flags) = read_full_box_header(&mut reader, "MOV/MP4 dfLa")?;
    if version != 0 {
        return Err(AvError::unsupported(format!(
            "MOV/MP4 dfLa version {version} is not implemented"
        )));
    }
    if flags != [0, 0, 0] {
        return Err(AvError::invalid_data("MOV/MP4 dfLa flags must be zero"));
    }

    let mut metadata_blocks = Vec::new();
    while !reader.is_eof() {
        ensure_remaining(&reader, 4, "MOV/MP4 dfLa metadata block header")?;
        let header = reader.read_u8()?;
        let last = (header & 0x80) != 0;
        let block_type = header & 0x7f;
        if block_type == FLAC_METADATA_BLOCK_FORBIDDEN {
            return Err(AvError::invalid_data(
                "MOV/MP4 dfLa metadata block type 127 is forbidden",
            ));
        }
        let length = usize::try_from(reader.read_u24_be()?).map_err(|_| {
            AvError::invalid_data("MOV/MP4 dfLa metadata block length is out of range")
        })?;
        let data = reader.read_exact(length)?.to_vec();
        if last && !reader.is_eof() {
            return Err(AvError::invalid_data(
                "MOV/MP4 dfLa last metadata block flag appears before the end of the box",
            ));
        }
        metadata_blocks.push(MovFlacMetadataBlock {
            last,
            block_type,
            data,
        });
    }

    if metadata_blocks.is_empty() {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa must contain at least one FLAC metadata block",
        ));
    }
    if !metadata_blocks.last().is_some_and(|block| block.last()) {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa final metadata block must set the last flag",
        ));
    }
    if metadata_blocks[0].block_type() != FLAC_METADATA_BLOCK_STREAMINFO {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa first metadata block must be STREAMINFO",
        ));
    }
    if metadata_blocks[1..]
        .iter()
        .any(|block| block.block_type() == FLAC_METADATA_BLOCK_STREAMINFO)
    {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa must not contain more than one STREAMINFO metadata block",
        ));
    }

    let streaminfo = parse_flac_streaminfo(metadata_blocks[0].data())?;
    Ok(MovFlacSpecificBox {
        version,
        flags,
        metadata_blocks,
        streaminfo,
    })
}

fn parse_flac_streaminfo(data: &[u8]) -> AvResult<MovFlacStreamInfo> {
    if data.len() != 34 {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa STREAMINFO metadata block must contain exactly 34 bytes",
        ));
    }

    let mut reader = ByteReader::new(data);
    let min_block_size = reader.read_u16_be()?;
    let max_block_size = reader.read_u16_be()?;
    let min_frame_size = reader.read_u24_be()?;
    let max_frame_size = reader.read_u24_be()?;
    let stream_fields = reader.read_exact(8)?;
    let mut bits = BitReader::new(stream_fields);
    let sample_rate = u32::try_from(bits.read_bits(20)?).map_err(|_| {
        AvError::invalid_data("MOV/MP4 dfLa STREAMINFO sample rate is out of range")
    })?;
    let channels = u8::try_from(bits.read_bits(3)? + 1).map_err(|_| {
        AvError::invalid_data("MOV/MP4 dfLa STREAMINFO channel count is out of range")
    })?;
    let bits_per_sample = u8::try_from(bits.read_bits(5)? + 1)
        .map_err(|_| AvError::invalid_data("MOV/MP4 dfLa STREAMINFO bit depth is out of range"))?;
    let total_samples = bits.read_bits(36)?;
    let mut md5 = [0_u8; 16];
    md5.copy_from_slice(reader.read_exact(16)?);
    ensure_box_consumed(&reader, "MOV/MP4 dfLa STREAMINFO")?;

    if min_block_size < 16 {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa STREAMINFO minimum block size is too small",
        ));
    }
    if max_block_size < min_block_size {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa STREAMINFO maximum block size is smaller than minimum block size",
        ));
    }
    if sample_rate == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa STREAMINFO sample rate must be nonzero for audio",
        ));
    }
    if bits_per_sample < 4 {
        return Err(AvError::invalid_data(
            "MOV/MP4 dfLa STREAMINFO bit depth must be at least 4",
        ));
    }

    Ok(MovFlacStreamInfo {
        min_block_size,
        max_block_size,
        min_frame_size,
        max_frame_size,
        sample_rate,
        channels,
        bits_per_sample,
        total_samples,
        md5,
    })
}

fn validate_wave_terminator(child_boxes: &[MovSampleEntryChildBox]) -> AvResult<()> {
    for (index, child) in child_boxes.iter().enumerate() {
        if child.box_type.as_bytes() == TERMINATOR_ID {
            if !child.payload().is_empty() {
                return Err(AvError::invalid_data(
                    "MOV/MP4 wave terminator atom must not contain payload",
                ));
            }
            if index + 1 != child_boxes.len() {
                return Err(AvError::invalid_data(
                    "MOV/MP4 wave terminator atom must be the final child atom",
                ));
            }
        }
    }
    Ok(())
}

fn parse_audio_channel_layout(payload: &[u8]) -> AvResult<MovAudioChannelLayout> {
    const CHANNEL_DESCRIPTION_SIZE: usize = 20;

    let mut reader = ByteReader::new(payload);
    let (version, flags) = read_full_box_header(&mut reader, "MOV/MP4 chan")?;
    if version != 0 {
        return Err(AvError::unsupported(format!(
            "MOV/MP4 chan version {version} is not implemented"
        )));
    }
    if flags != [0, 0, 0] {
        return Err(AvError::invalid_data("MOV/MP4 chan flags must be zero"));
    }
    ensure_remaining(&reader, 12, "MOV/MP4 chan channel layout")?;
    let channel_layout_tag = reader.read_u32_be()?;
    let channel_bitmap = reader.read_u32_be()?;
    let description_count = usize::try_from(reader.read_u32_be()?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 chan description count is out of range"))?;
    let description_bytes = description_count
        .checked_mul(CHANNEL_DESCRIPTION_SIZE)
        .ok_or_else(|| AvError::invalid_data("MOV/MP4 chan description byte count overflow"))?;
    ensure_remaining(
        &reader,
        description_bytes,
        "MOV/MP4 chan channel descriptions",
    )?;
    let mut channel_descriptions = Vec::with_capacity(description_count);
    for _ in 0..description_count {
        channel_descriptions.push(MovAudioChannelDescription {
            channel_label: reader.read_u32_be()?,
            channel_flags: reader.read_u32_be()?,
            coordinate_bits: [
                reader.read_u32_be()?,
                reader.read_u32_be()?,
                reader.read_u32_be()?,
            ],
        });
    }
    ensure_box_consumed(&reader, "MOV/MP4 chan")?;
    Ok(MovAudioChannelLayout {
        version,
        flags,
        channel_layout_tag,
        channel_bitmap,
        channel_descriptions,
    })
}

fn parse_visual_sample_entry(extra_data: &[u8]) -> AvResult<MovVideoSampleEntry> {
    let mut reader = ByteReader::new(extra_data);
    ensure_remaining(&reader, 70, "MOV/MP4 VisualSampleEntry")?;
    reader.skip(16)?;
    let width = reader.read_u16_be()?;
    let height = reader.read_u16_be()?;
    reader.skip(12)?;
    let frame_count = reader.read_u16_be()?;
    let compressor_name = parse_pascal_string_31(reader.read_exact(32)?);
    let depth = reader.read_u16_be()?;
    reader.skip(2)?;
    let child_boxes = parse_sample_entry_child_boxes(
        extra_data,
        reader.position(),
        extra_data.len(),
        "MOV/MP4 VisualSampleEntry children",
    )?;
    let avc_decoder_configuration = parse_visual_sample_entry_avc_configuration(&child_boxes)?;
    let hevc_decoder_configuration = parse_visual_sample_entry_hevc_configuration(&child_boxes)?;
    let pixel_aspect_ratio = parse_visual_sample_entry_pixel_aspect_ratio(&child_boxes)?;
    let color_information = parse_visual_sample_entry_color_information(&child_boxes)?;
    Ok(MovVideoSampleEntry {
        width,
        height,
        frame_count,
        compressor_name,
        depth,
        avc_decoder_configuration,
        hevc_decoder_configuration,
        pixel_aspect_ratio,
        color_information,
        child_boxes,
    })
}

fn parse_sample_entry_child_boxes(
    input: &[u8],
    start: usize,
    end: usize,
    context: &str,
) -> AvResult<Vec<MovSampleEntryChildBox>> {
    read_box_headers(input, start, end, context)?
        .into_iter()
        .map(|header| {
            Ok(MovSampleEntryChildBox {
                box_type: fourcc_to_string(header.box_type),
                payload: header.payload(input).to_vec(),
            })
        })
        .collect()
}

fn parse_visual_sample_entry_avc_configuration(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovAvcDecoderConfiguration>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == AVCC_ID)
        .map(|child| parse_avcc(child.payload()))
        .transpose()
}

fn parse_visual_sample_entry_hevc_configuration(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovHevcDecoderConfiguration>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == HVCC_ID)
        .map(|child| parse_hvcc(child.payload()))
        .transpose()
}

fn parse_visual_sample_entry_pixel_aspect_ratio(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovPixelAspectRatio>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == PASP_ID)
        .map(|child| parse_pasp(child.payload()))
        .transpose()
}

fn parse_visual_sample_entry_color_information(
    child_boxes: &[MovSampleEntryChildBox],
) -> AvResult<Option<MovColorInformation>> {
    child_boxes
        .iter()
        .find(|child| child.box_type.as_bytes() == COLR_ID)
        .map(|child| parse_colr(child.payload()))
        .transpose()
        .map(Option::flatten)
}

fn parse_avcc(payload: &[u8]) -> AvResult<MovAvcDecoderConfiguration> {
    let mut reader = ByteReader::new(payload);
    ensure_remaining(&reader, 6, "MOV/MP4 avcC")?;
    let configuration_version = reader.read_u8()?;
    if configuration_version != 1 {
        return Err(AvError::unsupported(
            "MOV/MP4 avcC configurationVersion values other than 1 are not implemented",
        ));
    }
    let profile_indication = reader.read_u8()?;
    let profile_compatibility = reader.read_u8()?;
    let level_indication = reader.read_u8()?;
    let nal_length_size = (reader.read_u8()? & 0x03) + 1;
    let sequence_parameter_set_count = reader.read_u8()? & 0x1f;
    let sequence_parameter_sets = parse_avcc_parameter_sets(
        &mut reader,
        usize::from(sequence_parameter_set_count),
        "SPS",
    )?;

    ensure_remaining(&reader, 1, "MOV/MP4 avcC picture parameter set count")?;
    let picture_parameter_set_count = reader.read_u8()?;
    let picture_parameter_sets =
        parse_avcc_parameter_sets(&mut reader, usize::from(picture_parameter_set_count), "PPS")?;
    let extension_data = reader.read_exact(reader.remaining())?.to_vec();

    Ok(MovAvcDecoderConfiguration {
        configuration_version,
        profile_indication,
        profile_compatibility,
        level_indication,
        nal_length_size,
        sequence_parameter_sets,
        picture_parameter_sets,
        extension_data,
    })
}

fn parse_avcc_parameter_sets(
    reader: &mut ByteReader<'_>,
    count: usize,
    label: &str,
) -> AvResult<Vec<Vec<u8>>> {
    let mut parameter_sets = Vec::with_capacity(count);
    for _ in 0..count {
        ensure_remaining(reader, 2, format!("MOV/MP4 avcC {label} length"))?;
        let len = usize::from(reader.read_u16_be()?);
        if len == 0 {
            return Err(AvError::invalid_data(format!(
                "MOV/MP4 avcC {label} parameter set must not be empty"
            )));
        }
        ensure_remaining(reader, len, format!("MOV/MP4 avcC {label} data"))?;
        parameter_sets.push(reader.read_exact(len)?.to_vec());
    }
    Ok(parameter_sets)
}

fn parse_hvcc(payload: &[u8]) -> AvResult<MovHevcDecoderConfiguration> {
    let mut reader = ByteReader::new(payload);
    ensure_remaining(&reader, 23, "MOV/MP4 hvcC")?;
    let configuration_version = reader.read_u8()?;
    if configuration_version != 1 {
        return Err(AvError::unsupported(
            "MOV/MP4 hvcC configurationVersion values other than 1 are not implemented",
        ));
    }

    let profile = reader.read_u8()?;
    let general_profile_space = profile >> 6;
    let general_tier_flag = profile & 0x20 != 0;
    let general_profile_idc = profile & 0x1f;
    let general_profile_compatibility_flags = reader.read_u32_be()?;
    let general_constraint_indicator_flags = reader.read_u48_be()?;
    let general_level_idc = reader.read_u8()?;
    let min_spatial_segmentation_idc = reader.read_u16_be()? & 0x0fff;
    let parallelism_type = reader.read_u8()? & 0x03;
    let chroma_format = reader.read_u8()? & 0x03;
    let bit_depth_luma = (reader.read_u8()? & 0x07) + 8;
    let bit_depth_chroma = (reader.read_u8()? & 0x07) + 8;
    let average_frame_rate = reader.read_u16_be()?;
    let frame_packing = reader.read_u8()?;
    let constant_frame_rate = frame_packing >> 6;
    let num_temporal_layers = (frame_packing >> 3) & 0x07;
    let temporal_id_nested = frame_packing & 0x04 != 0;
    let nal_length_size = (frame_packing & 0x03) + 1;
    let array_count = usize::from(reader.read_u8()?);
    let arrays = parse_hvcc_arrays(&mut reader, array_count)?;
    ensure_box_consumed(&reader, "MOV/MP4 hvcC")?;

    Ok(MovHevcDecoderConfiguration {
        configuration_version,
        general_profile_space,
        general_tier_flag,
        general_profile_idc,
        general_profile_compatibility_flags,
        general_constraint_indicator_flags,
        general_level_idc,
        min_spatial_segmentation_idc,
        parallelism_type,
        chroma_format,
        bit_depth_luma,
        bit_depth_chroma,
        average_frame_rate,
        constant_frame_rate,
        num_temporal_layers,
        temporal_id_nested,
        nal_length_size,
        arrays,
    })
}

fn parse_hvcc_arrays(
    reader: &mut ByteReader<'_>,
    count: usize,
) -> AvResult<Vec<MovHevcDecoderConfigurationArray>> {
    let mut arrays = Vec::with_capacity(count);
    for _ in 0..count {
        ensure_remaining(reader, 3, "MOV/MP4 hvcC NAL unit array header")?;
        let header = reader.read_u8()?;
        let array_completeness = header & 0x80 != 0;
        let nal_unit_type = header & 0x3f;
        let nal_unit_count = usize::from(reader.read_u16_be()?);
        let mut nal_units = Vec::with_capacity(nal_unit_count);
        for _ in 0..nal_unit_count {
            ensure_remaining(reader, 2, "MOV/MP4 hvcC NAL unit length")?;
            let len = usize::from(reader.read_u16_be()?);
            if len == 0 {
                return Err(AvError::invalid_data(
                    "MOV/MP4 hvcC NAL units must not be empty",
                ));
            }
            ensure_remaining(reader, len, "MOV/MP4 hvcC NAL unit data")?;
            nal_units.push(reader.read_exact(len)?.to_vec());
        }
        arrays.push(MovHevcDecoderConfigurationArray {
            array_completeness,
            nal_unit_type,
            nal_units,
        });
    }
    Ok(arrays)
}

fn parse_pasp(payload: &[u8]) -> AvResult<MovPixelAspectRatio> {
    let mut reader = ByteReader::new(payload);
    ensure_remaining(&reader, 8, "MOV/MP4 pasp")?;
    let horizontal_spacing = reader.read_u32_be()?;
    let vertical_spacing = reader.read_u32_be()?;
    ensure_box_consumed(&reader, "MOV/MP4 pasp")?;
    if horizontal_spacing == 0 || vertical_spacing == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 pasp spacing values must be non-zero",
        ));
    }
    Ok(MovPixelAspectRatio {
        horizontal_spacing,
        vertical_spacing,
    })
}

fn parse_colr(payload: &[u8]) -> AvResult<Option<MovColorInformation>> {
    let mut reader = ByteReader::new(payload);
    ensure_remaining(&reader, 4, "MOV/MP4 colr")?;
    let color_type = read_fourcc(&mut reader)?;
    match &color_type {
        NCLX_ID => parse_colr_color_parameters(&mut reader, color_type, true).map(Some),
        NCLC_ID => parse_colr_color_parameters(&mut reader, color_type, false).map(Some),
        RICC_ID | PROF_ID => parse_colr_icc_profile(&mut reader, color_type).map(Some),
        _ => Ok(None),
    }
}

fn parse_colr_color_parameters(
    reader: &mut ByteReader<'_>,
    color_type: [u8; 4],
    has_full_range_flag: bool,
) -> AvResult<MovColorInformation> {
    let context = format!("MOV/MP4 colr {}", fourcc_to_string(color_type));
    ensure_remaining(reader, 6, &context)?;
    let color_primaries = reader.read_u16_be()?;
    let transfer_characteristics = reader.read_u16_be()?;
    let matrix_coefficients = reader.read_u16_be()?;
    let full_range = if has_full_range_flag {
        ensure_remaining(reader, 1, &context)?;
        reader.read_u8()? & 0x80 != 0
    } else {
        false
    };
    ensure_box_consumed(reader, &context)?;

    Ok(MovColorInformation {
        color_type: fourcc_to_string(color_type),
        color_parameters: Some(MovColorParameters {
            color_primaries,
            transfer_characteristics,
            matrix_coefficients,
            full_range,
        }),
        icc_profile: None,
    })
}

fn parse_colr_icc_profile(
    reader: &mut ByteReader<'_>,
    color_type: [u8; 4],
) -> AvResult<MovColorInformation> {
    let context = format!("MOV/MP4 colr {}", fourcc_to_string(color_type));
    if reader.remaining() == 0 {
        return Err(AvError::invalid_data(format!(
            "{context} ICC profile must not be empty"
        )));
    }
    let icc_profile = reader.read_exact(reader.remaining())?.to_vec();
    Ok(MovColorInformation {
        color_type: fourcc_to_string(color_type),
        color_parameters: None,
        icc_profile: Some(icc_profile),
    })
}

fn parse_pascal_string_31(input: &[u8]) -> String {
    let declared_len = input.first().copied().unwrap_or(0) as usize;
    let len = declared_len.min(31).min(input.len().saturating_sub(1));
    String::from_utf8_lossy(&input[1..1 + len]).into_owned()
}

fn parse_stts(payload: &[u8]) -> AvResult<Vec<u32>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stts")?;
    let entry_count = reader.read_u32_be()?;
    let mut durations = Vec::new();
    for _ in 0..entry_count {
        ensure_remaining(&reader, 8, "MOV/MP4 stts entry")?;
        let sample_count = checked_sample_count(reader.read_u32_be()?, "MOV/MP4 stts")?;
        let sample_delta = reader.read_u32_be()?;
        let new_len = durations
            .len()
            .checked_add(sample_count)
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 stts sample count overflow"))?;
        if new_len > MAX_MOV_SAMPLE_COUNT {
            return Err(AvError::unsupported(
                "MOV/MP4 stts sample count exceeds current implementation limit",
            ));
        }
        durations.resize(new_len, sample_delta);
    }
    if durations.is_empty() {
        return Err(AvError::invalid_data("MOV/MP4 stts describes zero samples"));
    }
    ensure_box_consumed(&reader, "MOV/MP4 stts")?;
    Ok(durations)
}

fn parse_ctts(payload: &[u8]) -> AvResult<Vec<i64>> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 ctts")?;
    if version > 1 {
        return Err(AvError::unsupported(format!(
            "unsupported MOV/MP4 ctts version {version}"
        )));
    }

    let entry_count = reader.read_u32_be()?;
    let mut offsets = Vec::new();
    for _ in 0..entry_count {
        ensure_remaining(&reader, 8, "MOV/MP4 ctts entry")?;
        let sample_count = checked_sample_count(reader.read_u32_be()?, "MOV/MP4 ctts")?;
        let sample_offset = if version == 0 {
            i64::from(reader.read_u32_be()?)
        } else {
            i64::from(reader.read_i32_be()?)
        };
        let new_len = offsets
            .len()
            .checked_add(sample_count)
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 ctts sample count overflow"))?;
        if new_len > MAX_MOV_SAMPLE_COUNT {
            return Err(AvError::unsupported(
                "MOV/MP4 ctts sample count exceeds current implementation limit",
            ));
        }
        offsets.resize(new_len, sample_offset);
    }
    if offsets.is_empty() {
        return Err(AvError::invalid_data("MOV/MP4 ctts describes zero samples"));
    }
    ensure_box_consumed(&reader, "MOV/MP4 ctts")?;
    Ok(offsets)
}

fn parse_stsc(payload: &[u8]) -> AvResult<Vec<SampleToChunkEntry>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stsc")?;
    let entry_count = reader.read_u32_be()?;
    if entry_count == 0 {
        return Err(AvError::invalid_data("MOV/MP4 stsc has no entries"));
    }

    let mut entries = Vec::new();
    let mut previous_first_chunk = 0;
    for _ in 0..entry_count {
        ensure_remaining(&reader, 12, "MOV/MP4 stsc entry")?;
        let first_chunk = reader.read_u32_be()?;
        let samples_per_chunk = reader.read_u32_be()?;
        let sample_description_index = reader.read_u32_be()?;
        if first_chunk == 0 || first_chunk <= previous_first_chunk {
            return Err(AvError::invalid_data(
                "MOV/MP4 stsc first_chunk values must be positive and increasing",
            ));
        }
        if samples_per_chunk == 0 {
            return Err(AvError::invalid_data(
                "MOV/MP4 stsc samples_per_chunk must be non-zero",
            ));
        }
        if sample_description_index != 1 {
            return Err(AvError::unsupported(
                "MOV/MP4 stsc sample_description_index values other than 1 are not implemented",
            ));
        }
        previous_first_chunk = first_chunk;
        entries.push(SampleToChunkEntry {
            first_chunk,
            samples_per_chunk,
            sample_description_index,
        });
    }
    ensure_box_consumed(&reader, "MOV/MP4 stsc")?;
    Ok(entries)
}

fn parse_stsz(payload: &[u8]) -> AvResult<Vec<usize>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stsz")?;
    let default_sample_size = reader.read_u32_be()?;
    let sample_count = checked_sample_count(reader.read_u32_be()?, "MOV/MP4 stsz")?;
    if sample_count == 0 {
        return Err(AvError::invalid_data("MOV/MP4 stsz describes zero samples"));
    }

    let sample_sizes = if default_sample_size != 0 {
        let size = checked_sample_size(default_sample_size, "MOV/MP4 stsz default sample size")?;
        vec![size; sample_count]
    } else {
        let mut sizes = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            ensure_remaining(&reader, 4, "MOV/MP4 stsz sample size")?;
            sizes.push(checked_sample_size(
                reader.read_u32_be()?,
                "MOV/MP4 stsz sample size",
            )?);
        }
        sizes
    };
    ensure_box_consumed(&reader, "MOV/MP4 stsz")?;
    Ok(sample_sizes)
}

fn parse_stss(payload: &[u8]) -> AvResult<Vec<u32>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stss")?;
    let entry_count = usize::try_from(reader.read_u32_be()?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 stss entry count is out of range"))?;
    if entry_count > MAX_MOV_SAMPLE_COUNT {
        return Err(AvError::unsupported(
            "MOV/MP4 stss entry count exceeds current implementation limit",
        ));
    }
    let mut sample_numbers = Vec::with_capacity(entry_count);
    let mut previous_sample_number = 0;
    for _ in 0..entry_count {
        ensure_remaining(&reader, 4, "MOV/MP4 stss entry")?;
        let sample_number = reader.read_u32_be()?;
        if sample_number == 0 || sample_number <= previous_sample_number {
            return Err(AvError::invalid_data(
                "MOV/MP4 stss sample numbers must be positive and increasing",
            ));
        }
        previous_sample_number = sample_number;
        sample_numbers.push(sample_number);
    }
    ensure_box_consumed(&reader, "MOV/MP4 stss")?;
    Ok(sample_numbers)
}

fn parse_stco(payload: &[u8]) -> AvResult<Vec<u64>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stco")?;
    let entry_count = checked_table_entry_count(reader.read_u32_be()?, "MOV/MP4 stco")?;
    let mut offsets = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        ensure_remaining(&reader, 4, "MOV/MP4 stco entry")?;
        offsets.push(u64::from(reader.read_u32_be()?));
    }
    ensure_box_consumed(&reader, "MOV/MP4 stco")?;
    Ok(offsets)
}

fn parse_co64(payload: &[u8]) -> AvResult<Vec<u64>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 co64")?;
    let entry_count = checked_table_entry_count(reader.read_u32_be()?, "MOV/MP4 co64")?;
    let mut offsets = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        ensure_remaining(&reader, 8, "MOV/MP4 co64 entry")?;
        offsets.push(reader.read_u64_be()?);
    }
    ensure_box_consumed(&reader, "MOV/MP4 co64")?;
    Ok(offsets)
}

fn parse_mvhd(payload: &[u8]) -> AvResult<MovieHeader> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 mvhd")?;
    let (timescale, duration) = match version {
        0 => {
            ensure_remaining(&reader, 16, "MOV/MP4 mvhd version 0")?;
            reader.skip(8)?;
            let timescale = reader.read_u32_be()?;
            let duration = unknown_u32_duration(reader.read_u32_be()?);
            (timescale, duration)
        }
        1 => {
            ensure_remaining(&reader, 28, "MOV/MP4 mvhd version 1")?;
            reader.skip(16)?;
            let timescale = reader.read_u32_be()?;
            let duration = unknown_u64_duration(reader.read_u64_be()?);
            (timescale, duration)
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "unsupported MOV/MP4 mvhd version {version}"
            )));
        }
    };
    validate_timescale(timescale, "MOV/MP4 movie timescale")?;
    Ok(MovieHeader {
        timescale,
        duration,
    })
}

fn parse_tkhd(payload: &[u8]) -> AvResult<TrackHeader> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 tkhd")?;
    let (id, duration) = match version {
        0 => {
            ensure_remaining(&reader, 80, "MOV/MP4 tkhd version 0")?;
            reader.skip(8)?;
            let id = reader.read_u32_be()?;
            reader.skip(4)?;
            let duration = unknown_u32_duration(reader.read_u32_be()?);
            reader.skip(52)?;
            (id, duration)
        }
        1 => {
            ensure_remaining(&reader, 92, "MOV/MP4 tkhd version 1")?;
            reader.skip(16)?;
            let id = reader.read_u32_be()?;
            reader.skip(4)?;
            let duration = unknown_u64_duration(reader.read_u64_be()?);
            reader.skip(52)?;
            (id, duration)
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "unsupported MOV/MP4 tkhd version {version}"
            )));
        }
    };
    if id == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 tkhd track ID must be non-zero",
        ));
    }

    let width = fixed_16_16_to_dimension(reader.read_u32_be()?);
    let height = fixed_16_16_to_dimension(reader.read_u32_be()?);
    Ok(TrackHeader {
        id,
        duration,
        width,
        height,
    })
}

fn parse_mdhd(payload: &[u8]) -> AvResult<MediaHeader> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 mdhd")?;
    let (timescale, duration) = match version {
        0 => {
            ensure_remaining(&reader, 20, "MOV/MP4 mdhd version 0")?;
            reader.skip(8)?;
            let timescale = reader.read_u32_be()?;
            let duration = unknown_u32_duration(reader.read_u32_be()?);
            (timescale, duration)
        }
        1 => {
            ensure_remaining(&reader, 32, "MOV/MP4 mdhd version 1")?;
            reader.skip(16)?;
            let timescale = reader.read_u32_be()?;
            let duration = unknown_u64_duration(reader.read_u64_be()?);
            (timescale, duration)
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "unsupported MOV/MP4 mdhd version {version}"
            )));
        }
    };
    let language = decode_mdhd_language(reader.read_u16_be()?);
    reader.skip(2)?;
    validate_timescale(timescale, "MOV/MP4 media timescale")?;
    Ok(MediaHeader {
        timescale,
        duration,
        language,
    })
}

fn decode_mdhd_language(language: u16) -> Option<String> {
    if language == 0 {
        return None;
    }

    let mut out = String::with_capacity(3);
    for shift in [10, 5, 0] {
        let value = u8::try_from((language >> shift) & 0x1f).ok()?;
        if !(1..=26).contains(&value) {
            return None;
        }
        out.push(char::from(b'a' + value - 1));
    }
    Some(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HandlerInfo {
    handler_type: String,
    name: Option<String>,
}

fn parse_hdlr(payload: &[u8]) -> AvResult<HandlerInfo> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 hdlr")?;
    if version != 0 {
        return Err(AvError::unsupported(format!(
            "unsupported MOV/MP4 hdlr version {version}"
        )));
    }
    ensure_remaining(&reader, 20, "MOV/MP4 hdlr")?;
    reader.skip(4)?;
    let handler_type = fourcc_to_string(read_fourcc(&mut reader)?);
    reader.skip(12)?;

    let name = reader.read_exact(reader.remaining())?;
    let name = name
        .iter()
        .position(|byte| *byte == 0)
        .map_or(name, |nul| &name[..nul]);
    if name.is_empty() {
        return Ok(HandlerInfo {
            handler_type,
            name: None,
        });
    }
    let name = std::str::from_utf8(name)
        .map_err(|_| AvError::invalid_data("MOV/MP4 hdlr name is not valid UTF-8"))?
        .to_owned();
    Ok(HandlerInfo {
        handler_type,
        name: Some(name),
    })
}

fn build_packets(
    input: &[u8],
    tracks: &[TrackData],
    mdat_ranges: &[(usize, usize)],
) -> AvResult<Option<Vec<Packet>>> {
    let tracks_with_tables = tracks
        .iter()
        .enumerate()
        .filter_map(|(stream_index, track)| {
            track
                .sample_table
                .as_ref()
                .map(|table| (stream_index, track, table))
        })
        .collect::<Vec<_>>();
    if tracks_with_tables.is_empty() {
        return Ok(None);
    }
    if tracks_with_tables.len() != 1 {
        return Err(AvError::unsupported(
            "MOV/MP4 packet extraction for multiple populated tracks is not implemented",
        ));
    }
    if mdat_ranges.is_empty() {
        return Err(AvError::invalid_data(
            "MOV/MP4 sample table references media data but no mdat box is present",
        ));
    }

    let (stream_index, track, table) = tracks_with_tables[0];
    let sample_spans = build_sample_spans(table)?;
    let mut packets = Vec::with_capacity(sample_spans.len());
    let mut dts = 0_i64;

    for (sample_index, span) in sample_spans.into_iter().enumerate() {
        let start = usize::try_from(span.offset)
            .map_err(|_| AvError::invalid_data("MOV/MP4 sample offset is out of range"))?;
        let end = start
            .checked_add(span.size)
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 sample range overflow"))?;
        if end > input.len() || !range_within_mdat(start, end, mdat_ranges) {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                "MOV/MP4 sample payload exceeds mdat bounds",
            ));
        }

        let mut packet = Packet::new(input[start..end].to_vec(), stream_index);
        let pts = dts
            .checked_add(composition_offset(table, sample_index))
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 packet PTS overflow"))?;
        packet.set_pts(Some(pts));
        packet.set_dts(Some(dts));
        packet.set_duration(i64::from(span.duration))?;
        packet.set_key(is_sync_sample(table, sample_index));
        packet.push_side_data(SideData::new(
            "mov_track_id",
            track.info.id.to_be_bytes().to_vec(),
        )?);
        packet.push_side_data(SideData::new(
            "mov_codec_tag",
            table.codec_parameters.codec_tag.as_bytes().to_vec(),
        )?);
        dts = dts
            .checked_add(i64::from(span.duration))
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 packet DTS overflow"))?;
        packets.push(packet);
    }

    Ok(Some(packets))
}

fn is_sync_sample(table: &SampleTable, sample_index: usize) -> bool {
    table
        .sync_samples
        .as_ref()
        .map_or(true, |sync_samples| sync_samples[sample_index])
}

fn composition_offset(table: &SampleTable, sample_index: usize) -> i64 {
    table
        .composition_offsets
        .as_ref()
        .map_or(0, |offsets| offsets[sample_index])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SampleSpan {
    offset: u64,
    size: usize,
    duration: u32,
}

fn build_sample_spans(table: &SampleTable) -> AvResult<Vec<SampleSpan>> {
    if table.sample_to_chunks[0].first_chunk != 1 {
        return Err(AvError::invalid_data(
            "MOV/MP4 first stsc entry must start at chunk 1",
        ));
    }

    let mut spans = Vec::with_capacity(table.sample_sizes.len());
    let mut sample_index = 0;
    for (chunk_index_zero, chunk_offset) in table.chunk_offsets.iter().copied().enumerate() {
        if sample_index == table.sample_sizes.len() {
            break;
        }

        let chunk_index = u32::try_from(chunk_index_zero + 1)
            .map_err(|_| AvError::unsupported("MOV/MP4 chunk index exceeds u32 range"))?;
        let entry = sample_to_chunk_entry(&table.sample_to_chunks, chunk_index)?;
        if entry.sample_description_index != 1 {
            return Err(AvError::unsupported(
                "MOV/MP4 sample descriptions other than index 1 are not implemented",
            ));
        }

        let samples_in_chunk = usize::try_from(entry.samples_per_chunk).map_err(|_| {
            AvError::unsupported("MOV/MP4 samples_per_chunk exceeds addressable memory")
        })?;
        let mut relative_offset = 0_u64;
        for _ in 0..samples_in_chunk {
            if sample_index == table.sample_sizes.len() {
                break;
            }
            let size = table.sample_sizes[sample_index];
            let offset = chunk_offset
                .checked_add(relative_offset)
                .ok_or_else(|| AvError::invalid_data("MOV/MP4 sample offset overflow"))?;
            spans.push(SampleSpan {
                offset,
                size,
                duration: table.sample_durations[sample_index],
            });
            relative_offset = relative_offset
                .checked_add(
                    u64::try_from(size).map_err(|_| {
                        AvError::invalid_data("MOV/MP4 sample size does not fit u64")
                    })?,
                )
                .ok_or_else(|| AvError::invalid_data("MOV/MP4 chunk payload offset overflow"))?;
            sample_index += 1;
        }
    }

    if sample_index != table.sample_sizes.len() {
        return Err(AvError::invalid_data(
            "MOV/MP4 stsc/stco tables do not map every sample",
        ));
    }
    Ok(spans)
}

fn sample_to_chunk_entry(
    entries: &[SampleToChunkEntry],
    chunk_index: u32,
) -> AvResult<SampleToChunkEntry> {
    let mut selected = None;
    for entry in entries {
        if entry.first_chunk > chunk_index {
            break;
        }
        selected = Some(*entry);
    }
    selected.ok_or_else(|| AvError::invalid_data("MOV/MP4 chunk has no stsc mapping"))
}

fn range_within_mdat(start: usize, end: usize, mdat_ranges: &[(usize, usize)]) -> bool {
    mdat_ranges
        .iter()
        .any(|(mdat_start, mdat_end)| start >= *mdat_start && end <= *mdat_end)
}

fn sync_sample_flags(sample_numbers: Vec<u32>, sample_count: usize) -> AvResult<Vec<bool>> {
    let mut flags = vec![false; sample_count];
    for sample_number in sample_numbers {
        let sample_index = usize::try_from(sample_number - 1)
            .map_err(|_| AvError::invalid_data("MOV/MP4 stss sample number is out of range"))?;
        let Some(flag) = flags.get_mut(sample_index) else {
            return Err(AvError::invalid_data(
                "MOV/MP4 stss sample number exceeds stsz sample count",
            ));
        };
        *flag = true;
    }
    Ok(flags)
}

fn read_box_headers(
    input: &[u8],
    start: usize,
    end: usize,
    context: &str,
) -> AvResult<Vec<BoxHeader>> {
    if start > end || end > input.len() {
        return Err(AvError::invalid_argument(format!(
            "{context} box range is out of bounds"
        )));
    }

    let mut headers = Vec::new();
    let mut position = start;
    while position < end {
        if end - position < 8 {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!("{context} box header is truncated"),
            ));
        }

        let mut reader = ByteReader::new(&input[position..end]);
        let size32 = reader.read_u32_be()?;
        let box_type = read_fourcc(&mut reader)?;
        let (header_size, box_size) = match size32 {
            0 => (8_usize, end - position),
            1 => {
                let extended = reader.read_u64_be()?;
                let size = usize::try_from(extended).map_err(|_| {
                    AvError::invalid_data(format!("{context} extended box size is out of range"))
                })?;
                (16, size)
            }
            size => {
                let size = usize::try_from(size).map_err(|_| {
                    AvError::invalid_data(format!("{context} box size is out of range"))
                })?;
                (8, size)
            }
        };
        if box_size < header_size {
            return Err(AvError::invalid_data(format!(
                "{context} `{}` box size is smaller than its header",
                fourcc_to_string(box_type)
            )));
        }

        let box_end = position
            .checked_add(box_size)
            .ok_or_else(|| AvError::invalid_data(format!("{context} box size overflow")))?;
        if box_end > end {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!(
                    "{context} `{}` box exceeds parent bounds",
                    fourcc_to_string(box_type)
                ),
            ));
        }

        headers.push(BoxHeader {
            box_type,
            payload_start: position + header_size,
            payload_end: box_end,
        });
        position = box_end;
    }
    Ok(headers)
}

fn read_full_box_header(reader: &mut ByteReader<'_>, context: &str) -> AvResult<(u8, [u8; 3])> {
    ensure_remaining(reader, 4, context)?;
    let version = reader.read_u8()?;
    let flags = reader.read_exact(3)?;
    Ok((version, [flags[0], flags[1], flags[2]]))
}

fn ensure_remaining(
    reader: &ByteReader<'_>,
    count: usize,
    context: impl AsRef<str>,
) -> AvResult<()> {
    if reader.remaining() < count {
        let context = context.as_ref();
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            format!("{context} box payload is truncated"),
        ));
    }
    Ok(())
}

fn ensure_box_consumed(reader: &ByteReader<'_>, context: &str) -> AvResult<()> {
    if !reader.is_eof() {
        return Err(AvError::invalid_data(format!(
            "{context} box contains trailing bytes"
        )));
    }
    Ok(())
}

fn checked_sample_count(value: u32, context: &str) -> AvResult<usize> {
    if value == 0 {
        return Err(AvError::invalid_data(format!(
            "{context} sample count must be non-zero"
        )));
    }
    let value = usize::try_from(value)
        .map_err(|_| AvError::invalid_data(format!("{context} sample count is out of range")))?;
    if value > MAX_MOV_SAMPLE_COUNT {
        return Err(AvError::unsupported(format!(
            "{context} sample count exceeds current implementation limit"
        )));
    }
    Ok(value)
}

fn checked_table_entry_count(value: u32, context: &str) -> AvResult<usize> {
    if value == 0 {
        return Err(AvError::invalid_data(format!(
            "{context} entry count must be non-zero"
        )));
    }
    usize::try_from(value)
        .map_err(|_| AvError::invalid_data(format!("{context} entry count is out of range")))
}

fn checked_sample_size(value: u32, context: &str) -> AvResult<usize> {
    if value == 0 {
        return Err(AvError::invalid_data(format!("{context} must be non-zero")));
    }
    usize::try_from(value).map_err(|_| AvError::invalid_data(format!("{context} is out of range")))
}

fn validate_timescale(timescale: u32, name: &str) -> AvResult<()> {
    if timescale == 0 {
        return Err(AvError::invalid_data(format!("{name} must be non-zero")));
    }
    Ok(())
}

fn unknown_u32_duration(duration: u32) -> Option<u64> {
    if duration == u32::MAX {
        None
    } else {
        Some(u64::from(duration))
    }
}

fn unknown_u64_duration(duration: u64) -> Option<u64> {
    if duration == u64::MAX {
        None
    } else {
        Some(duration)
    }
}

fn fixed_16_16_to_dimension(value: u32) -> Option<u32> {
    let whole = value >> 16;
    if whole == 0 {
        None
    } else {
        Some(whole)
    }
}

fn read_fourcc(reader: &mut ByteReader<'_>) -> AvResult<[u8; 4]> {
    let bytes = reader.read_exact(4)?;
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn fourcc_to_string(fourcc: [u8; 4]) -> String {
    String::from_utf8_lossy(&fourcc).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::ByteWriter;

    const METADATA_DATA_TYPE_SIGNED_INTEGER: u32 = 21;

    #[test]
    fn parses_ftyp_movie_and_track_metadata() {
        let bytes = mp4_v0_fixture();
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().major_brand(), "isom");
        assert_eq!(demuxer.info().minor_version(), 512);
        assert_eq!(
            demuxer.info().compatible_brands(),
            &["isom".to_string(), "iso2".to_string(), "avc1".to_string()]
        );
        assert_eq!(demuxer.info().timescale(), 1_000);
        assert_eq!(demuxer.info().duration(), Some(5_000));
        assert!(demuxer.info().has_media_data());
        assert_eq!(demuxer.info().tracks().len(), 1);

        let track = &demuxer.info().tracks()[0];
        assert_eq!(track.id(), 1);
        assert_eq!(track.duration(), Some(5_000));
        assert_eq!(track.width(), Some(1_920));
        assert_eq!(track.height(), Some(1_080));
        assert_eq!(track.media_timescale(), 90_000);
        assert_eq!(track.media_duration(), Some(450_000));
        assert_eq!(track.codec_tag(), None);
        assert_eq!(track.sample_count(), 0);

        assert_eq!(
            demuxer.read_packet().unwrap_err().kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn supports_extended_size_boxes_and_version_one_headers() {
        let bytes = mp4_v1_fixture_with_extended_free_box();
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let track = &demuxer.info().tracks()[0];

        assert_eq!(demuxer.info().duration(), Some(7_000_000_000));
        assert_eq!(track.id(), 7);
        assert_eq!(track.duration(), None);
        assert_eq!(track.width(), Some(3840));
        assert_eq!(track.height(), Some(2160));
        assert_eq!(track.media_timescale(), 48_000);
        assert_eq!(track.media_duration(), Some(96_000));
    }

    #[test]
    fn extracts_media_handler_name_as_track_metadata() {
        let bytes = mp4_with_media_handler_name(b"Rust Video Handler\0");
        let demuxer = MovDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().tracks()[0].handler_type(), Some("vide"));
        assert_eq!(
            demuxer.info().tracks()[0].metadata().get("handler_name"),
            Some("Rust Video Handler")
        );
    }

    #[test]
    fn extracts_media_handler_type_without_handler_name() {
        let bytes = mp4_with_media_handler(*b"soun", b"");
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let track = &demuxer.info().tracks()[0];

        assert_eq!(track.handler_type(), Some("soun"));
        assert_eq!(track.metadata().get("handler_name"), None);
    }

    #[test]
    fn extracts_media_language_as_track_metadata() {
        let bytes = mp4_with_media_language("eng");
        let demuxer = MovDemuxer::open(&bytes).unwrap();

        assert_eq!(
            demuxer.info().tracks()[0].metadata().get("language"),
            Some("eng")
        );
    }

    #[test]
    fn rejects_invalid_media_handler_name() {
        let bytes = mp4_with_media_handler_name(b"\xff\0");

        assert_eq!(
            MovDemuxer::open(&bytes).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn accepts_size_zero_top_level_box_as_remaining_payload() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&ftyp_box());
        bytes.extend_from_slice(&box_size_zero(*MOOV_ID, &moov_v0_box_payload()));

        let demuxer = MovDemuxer::open(&bytes).unwrap();

        assert!(!demuxer.info().has_media_data());
        assert_eq!(demuxer.info().tracks()[0].id(), 1);
    }

    #[test]
    fn rejects_bad_box_bounds_and_required_metadata() {
        assert_eq!(
            MovDemuxer::open(b"not mp4").unwrap_err().kind(),
            AvErrorKind::EndOfFile
        );

        let mut oversized = box_(*FTYP_ID, &ftyp_payload());
        oversized[0..4].copy_from_slice(&999_u32.to_be_bytes());
        assert_eq!(
            MovDemuxer::open(&oversized).unwrap_err().kind(),
            AvErrorKind::EndOfFile
        );

        let mut undersized = Vec::new();
        undersized.extend_from_slice(&4_u32.to_be_bytes());
        undersized.extend_from_slice(FTYP_ID);
        assert_eq!(
            MovDemuxer::open(&undersized).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        assert_eq!(
            MovDemuxer::open(&box_(*MOOV_ID, &moov_v0_box_payload()))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&ftyp_box()).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_without_track_header())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_invalid_full_boxes_and_ftyp_payloads() {
        let mut bad_ftyp_payload = ftyp_payload();
        bad_ftyp_payload.push(b'x');
        let bad_ftyp = box_(*FTYP_ID, &bad_ftyp_payload);
        assert_eq!(
            MovDemuxer::open(&bad_ftyp).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        assert_eq!(
            MovDemuxer::open(&mp4_with_mvhd_timescale(0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_track_id(0)).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_mvhd_version(2))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_truncated_mdhd())
                .unwrap_err()
                .kind(),
            AvErrorKind::EndOfFile
        );
    }

    #[test]
    fn validates_movie_metadata_atom_boundaries() {
        let data_box = box_(*b"data", b"\0\0\0\x01\0\0\0\0Rust");
        let item = box_(*b"name", &data_box);
        let ilst = box_(*ILST_ID, &item);
        let meta = box_(*META_ID, &full_box(0, &ilst));
        let bytes = mp4_with_moov_extra_box(box_(*UDTA_ID, &meta));

        let demuxer = MovDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().tracks()[0].id(), 1);
        assert!(!demuxer.info().has_media_data());
        assert!(demuxer.info().metadata().is_empty());
        assert!(demuxer.info().tracks()[0].metadata().is_empty());
    }

    #[test]
    fn extracts_movie_metadata_from_common_ilst_data_atoms() {
        let ilst = ilst_box(&[
            ilst_utf8_item([0xa9, b'n', b'a', b'm'], "Rust Movie"),
            ilst_utf8_item([0xa9, b'A', b'R', b'T'], "Ferris"),
            ilst_utf8_item(*b"aART", "Rustaceans"),
            ilst_utf8_item([0xa9, b'a', b'l', b'b'], "Rewrite Sessions"),
            ilst_utf8_item(*b"desc", "Parser coverage"),
            ilst_number_pair_item(*b"trkn", 3, 12),
            ilst_number_pair_item(*b"disk", 1, 2),
        ]);
        let bytes = mp4_with_moov_extra_box(box_(*UDTA_ID, &meta_box(ilst)));

        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let metadata = demuxer.info().metadata();

        assert_eq!(metadata.get("title"), Some("Rust Movie"));
        assert_eq!(metadata.get("artist"), Some("Ferris"));
        assert_eq!(metadata.get("album_artist"), Some("Rustaceans"));
        assert_eq!(metadata.get("album"), Some("Rewrite Sessions"));
        assert_eq!(metadata.get("description"), Some("Parser coverage"));
        assert_eq!(metadata.get("track"), Some("3/12"));
        assert_eq!(metadata.get("disc"), Some("1/2"));
    }

    #[test]
    fn extracts_utf16_movie_metadata_from_ilst_data_atoms() {
        let ilst = ilst_box(&[
            ilst_utf16_be_item([0xa9, b'n', b'a', b'm'], "Rust UTF-16"),
            ilst_utf16_le_bom_item([0xa9, b't', b'o', b'o'], "Encoder \u{1f680}"),
        ]);
        let bytes = mp4_with_moov_extra_box(box_(*UDTA_ID, &meta_box(ilst)));

        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let metadata = demuxer.info().metadata();

        assert_eq!(metadata.get("title"), Some("Rust UTF-16"));
        assert_eq!(metadata.get("encoder"), Some("Encoder \u{1f680}"));
    }

    #[test]
    fn extracts_genre_metadata_from_ilst_index_atoms() {
        let ilst = ilst_box(&[ilst_genre_index_item(18)]);
        let bytes = mp4_with_moov_extra_box(box_(*UDTA_ID, &meta_box(ilst)));

        let demuxer = MovDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().metadata().get("genre"), Some("Rock"));
    }

    #[test]
    fn extracts_int8_metadata_from_ilst_data_atoms() {
        let ilst = ilst_box(&[
            ilst_int8_item(*b"akID", 2),
            ilst_int8_item(*b"cpil", 1),
            ilst_int8_item(*b"egid", 7),
            ilst_int8_item(*b"hdvd", 1),
            ilst_int8_item(*b"pcst", 1),
            ilst_int8_item(*b"pgap", 0),
            ilst_int8_item(*b"rtng", 4),
            ilst_int8_item(*b"stik", 10),
            ilst_padded_int8_item(*b"tves", 5),
            ilst_padded_int8_item(*b"tvsn", 3),
        ]);
        let bytes = mp4_with_moov_extra_box(box_(*UDTA_ID, &meta_box(ilst)));

        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let metadata = demuxer.info().metadata();

        assert_eq!(metadata.get("account_type"), Some("2"));
        assert_eq!(metadata.get("compilation"), Some("1"));
        assert_eq!(metadata.get("episode_uid"), Some("7"));
        assert_eq!(metadata.get("hd_video"), Some("1"));
        assert_eq!(metadata.get("podcast"), Some("1"));
        assert_eq!(metadata.get("gapless_playback"), Some("0"));
        assert_eq!(metadata.get("rating"), Some("4"));
        assert_eq!(metadata.get("media_type"), Some("10"));
        assert_eq!(metadata.get("episode_sort"), Some("5"));
        assert_eq!(metadata.get("season_number"), Some("3"));
    }

    #[test]
    fn extracts_freeform_metadata_from_ilst_atoms() {
        let ilst = ilst_box(&[
            ilst_freeform_item("com.apple.iTunes", "iTunSMPB", " 00000000 00000840"),
            ilst_freeform_item("com.apple.iTunes", "cdec", "ignored"),
        ]);
        let bytes = mp4_with_moov_extra_box(box_(*UDTA_ID, &meta_box(ilst)));

        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let metadata = demuxer.info().metadata();

        assert_eq!(metadata.get("iTunSMPB"), Some(" 00000000 00000840"));
        assert_eq!(metadata.get("cdec"), None);
    }

    #[test]
    fn extracts_cover_art_metadata_from_ilst_atoms() {
        let png = b"\x89PNG\r\n\x1a\npayload";
        let jpeg = b"\xff\xd8\xff\xe0JFIF payload";
        let unknown = b"ignored cover";
        let ilst = ilst_box(&[
            ilst_cover_art_item(METADATA_DATA_TYPE_JPEG, png),
            ilst_cover_art_item(METADATA_DATA_TYPE_PNG, jpeg),
            ilst_cover_art_item(0xffff, unknown),
        ]);
        let bytes = mp4_with_moov_extra_box(box_(*UDTA_ID, &meta_box(ilst)));

        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let cover_art = demuxer.info().cover_art();

        assert_eq!(cover_art.len(), 2);
        assert_eq!(cover_art[0].data_type(), METADATA_DATA_TYPE_JPEG);
        assert_eq!(cover_art[0].codec(), "png");
        assert_eq!(cover_art[0].mime_type(), "image/png");
        assert_eq!(cover_art[0].data(), png);
        assert_eq!(cover_art[1].data_type(), METADATA_DATA_TYPE_PNG);
        assert_eq!(cover_art[1].codec(), "mjpeg");
        assert_eq!(cover_art[1].mime_type(), "image/jpeg");
        assert_eq!(cover_art[1].data(), jpeg);
    }

    #[test]
    fn extracts_track_metadata_from_track_udta() {
        let track_udta = box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[ilst_utf8_item(
                [0xa9, b'n', b'a', b'm'],
                "Video Track",
            )])),
        );
        let mdia = box_(*MDIA_ID, &mdhd_v0(90_000, 450_000));
        let trak = box_(
            *TRAK_ID,
            &[tkhd_v0(1, 5_000, 1_920, 1_080), track_udta, mdia].concat(),
        );
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &[mvhd_v0(1_000, 5_000), trak].concat()));

        let demuxer = MovDemuxer::open(&out).unwrap();

        assert!(demuxer.info().metadata().is_empty());
        assert_eq!(
            demuxer.info().tracks()[0].metadata().get("title"),
            Some("Video Track")
        );
    }

    #[test]
    fn rejects_malformed_movie_metadata_atom_boundaries() {
        let bad_data_box = box_with_declared_size(*b"data", 16, b"\0");
        let item = box_(*b"name", &bad_data_box);
        let ilst = box_(*ILST_ID, &item);
        let meta = box_(*META_ID, &full_box(0, &ilst));
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(*UDTA_ID, &meta))).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_text = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_UTF8, b"\xff"),
        );
        let item = box_([0xa9, b'n', b'a', b'm'], &bad_text);
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_utf16 = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_UTF16, &[0xd8, 0x00]),
        );
        let item = box_([0xa9, b'n', b'a', b'm'], &bad_utf16);
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_genre = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_RESERVED, &[0x00]),
        );
        let item = box_(*b"gnre", &bad_genre);
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_genre = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_RESERVED, &999_u16.to_be_bytes()),
        );
        let item = box_(*b"gnre", &bad_genre);
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_int8 = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_SIGNED_INTEGER, &[]),
        );
        let item = box_(*b"cpil", &bad_int8);
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_padded_int8 = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_SIGNED_INTEGER, &[0, 0, 0]),
        );
        let item = box_(*b"tves", &bad_padded_int8);
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_freeform_name = box_(*NAME_ID, &full_box(0, b"\xff"));
        let item = box_(
            *FREEFORM_ID,
            &[
                freeform_text_child(*MEAN_ID, "com.apple.iTunes"),
                bad_freeform_name,
            ]
            .concat(),
        );
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_freeform_data = box_(*DATA_ID, &full_box(0, b"\xff"));
        let item = box_(
            *FREEFORM_ID,
            &[
                freeform_text_child(*MEAN_ID, "com.apple.iTunes"),
                freeform_text_child(*NAME_ID, "iTunSMPB"),
                bad_freeform_data,
            ]
            .concat(),
        );
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_cover = box_(*DATA_ID, &full_box(0, b"\0"));
        let item = box_(*COVR_ID, &bad_cover);
        let err = MovDemuxer::open(&mp4_with_moov_extra_box(box_(
            *UDTA_ID,
            &meta_box(ilst_box(&[item])),
        )))
        .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
    }

    #[test]
    fn registers_mov_probe_descriptor_for_signature_extension_and_mime() {
        let mut registry = ProbeRegistry::new();
        register_mov_probe(&mut registry).unwrap();

        let descriptor = mov_probe_descriptor().unwrap();
        assert_eq!(descriptor.name(), MOV_PROBE_NAME);
        let expected_extensions = MOV_PROBE_EXTENSIONS
            .iter()
            .map(|extension| extension.to_string())
            .collect::<Vec<_>>();
        assert_eq!(descriptor.extensions(), expected_extensions.as_slice());

        let ftyp = ftyp_box();
        let signature_match = registry
            .probe(crate::probe::ProbeRequest::new(&ftyp).with_extension("clip.bin"))
            .unwrap();
        assert_eq!(signature_match.descriptor().name(), MOV_PROBE_NAME);
        assert_eq!(signature_match.score(), crate::probe::ProbeScore::SIGNATURE);

        let extension_match = registry
            .probe(crate::probe::ProbeRequest::new(b"not mov").with_extension("clip.MP4"))
            .unwrap();
        assert_eq!(extension_match.descriptor().name(), MOV_PROBE_NAME);
        assert_eq!(extension_match.score(), crate::probe::ProbeScore::EXTENSION);

        let mime_match = registry
            .probe(crate::probe::ProbeRequest::new(b"").with_mime_type("Video/QuickTime"))
            .unwrap();
        assert_eq!(mime_match.descriptor().name(), MOV_PROBE_NAME);
        assert_eq!(mime_match.score(), crate::probe::ProbeScore::MIME_TYPE);

        assert!(registry
            .probe(crate::probe::ProbeRequest::new(b"RIFF....AVI ").with_extension("clip.avi"))
            .is_none());
    }

    #[test]
    fn rejects_fragmented_movie_boxes_explicitly() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_movie_extends())
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_movie_fragment())
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_edit_lists_explicitly() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_edit_list()).unwrap_err().kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_multiple_populated_tracks_explicitly() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_two_populated_tracks())
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn reads_packets_from_simple_sample_table() {
        let bytes = mp4_with_samples(
            false,
            &[b"abc".as_slice(), b"defg".as_slice()],
            &[1_000, 2_000],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let track = &demuxer.info().tracks()[0];
        assert_eq!(track.codec_tag(), Some("raw "));
        let codec_parameters = track.codec_parameters().unwrap();
        assert_eq!(codec_parameters.codec_tag(), "raw ");
        assert_eq!(codec_parameters.data_reference_index(), 1);
        assert_eq!(codec_parameters.extra_data(), b"");
        assert_eq!(track.sample_count(), 2);

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.stream_index(), 0);
        assert_eq!(first.data(), b"abc");
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.duration(), 1_000);
        assert!(first.flags().contains(avutil::PacketFlags::KEY));
        assert_eq!(first.side_data()[0].kind(), "mov_track_id");
        assert_eq!(first.side_data()[0].data(), &1_u32.to_be_bytes());
        assert_eq!(first.side_data()[1].kind(), "mov_codec_tag");
        assert_eq!(first.side_data()[1].data(), b"raw ");

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), b"defg");
        assert_eq!(second.pts(), Some(1_000));
        assert_eq!(second.dts(), Some(1_000));
        assert_eq!(second.duration(), 2_000);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn parses_sample_description_codec_parameters() {
        let bytes = mp4_with_sample_description_entry(b"zzzz", 2, b"\x01\x64\x00\x1f");
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let track = &demuxer.info().tracks()[0];
        let codec_parameters = track.codec_parameters().unwrap();
        assert_eq!(track.codec_tag(), Some("zzzz"));
        assert_eq!(codec_parameters.codec_tag(), "zzzz");
        assert_eq!(codec_parameters.data_reference_index(), 2);
        assert_eq!(codec_parameters.extra_data(), b"\x01\x64\x00\x1f");
        assert_eq!(codec_parameters.details(), &MovSampleEntryDetails::Generic);
    }

    #[test]
    fn parses_audio_sample_entry_codec_parameters() {
        let esds = box_(*b"esds", &full_box(0, b"\x03\x01\x00"));
        let btrt = box_(*BTRT_ID, &bit_rate_box_payload(4_096, 192_000, 128_000));
        let children = [esds.as_slice(), btrt.as_slice()].concat();
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &children);
        let bytes = mp4_with_sample_description_entry(b"mp4a", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "mp4a");
        assert_eq!(codec_parameters.extra_data(), extra_data.as_slice());
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.version(), 0);
        assert_eq!(audio.revision_level(), 0);
        assert_eq!(audio.vendor(), 0);
        assert_eq!(audio.channel_count(), 2);
        assert_eq!(audio.effective_channel_count(), 2);
        assert_eq!(audio.sample_size(), 16);
        assert_eq!(audio.effective_bits_per_sample(), 16);
        assert_eq!(audio.compression_id(), 0);
        assert_eq!(audio.packet_size(), 0);
        assert_eq!(audio.sample_rate(), 48_000);
        assert_eq!(audio.sample_rate_fixed_16_16(), 48_000 << 16);
        assert!(matches!(
            audio.version_fields(),
            MovAudioSampleEntryVersionFields::Version0
        ));
        assert!(audio.version1_fields().is_none());
        assert!(audio.version2_fields().is_none());
        assert!(audio.ac3_specific().is_none());
        assert!(audio.ec3_specific().is_none());
        assert!(audio.opus_specific().is_none());
        assert!(audio.flac_specific().is_none());
        assert!(audio.alac_specific().is_none());
        assert!(audio.wave_extension().is_none());
        assert!(audio.channel_layout().is_none());
        let direct_esds = audio.elementary_stream_descriptor().unwrap();
        assert_eq!(direct_esds.version(), 0);
        assert_eq!(direct_esds.flags(), [0, 0, 0]);
        assert_eq!(direct_esds.descriptor(), b"\x03\x01\x00");
        let bit_rate = audio.bit_rate().unwrap();
        assert_eq!(bit_rate.buffer_size_db(), 4_096);
        assert_eq!(bit_rate.max_bitrate(), 192_000);
        assert_eq!(bit_rate.avg_bitrate(), 128_000);
        assert_eq!(audio.child_boxes().len(), 2);
        assert_eq!(audio.child_boxes()[0].box_type(), "esds");
        let expected_esds_payload = full_box(0, b"\x03\x01\x00");
        assert_eq!(
            audio.child_boxes()[0].payload(),
            expected_esds_payload.as_slice()
        );
        assert_eq!(audio.child_boxes()[1].box_type(), "btrt");
        assert_eq!(
            audio.child_boxes()[1].payload(),
            bit_rate_box_payload(4_096, 192_000, 128_000).as_slice()
        );
    }

    #[test]
    fn parses_amr_audio_sample_entry_specific_box() {
        let damr = box_(
            *DAMR_ID,
            &amr_specific_box_payload(*b"rust", 1, 0x0085, 2, 2),
        );
        let extra_data = audio_sample_entry_extra_data(1, 16, 8_000, &damr);
        let bytes = mp4_with_sample_description_entry(b"samr", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "samr");
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.channel_count(), 1);
        assert_eq!(audio.sample_rate(), 8_000);
        assert!(audio.elementary_stream_descriptor().is_none());
        assert!(audio.bit_rate().is_none());
        assert!(audio.ac3_specific().is_none());
        assert!(audio.ec3_specific().is_none());
        assert!(audio.opus_specific().is_none());
        assert!(audio.flac_specific().is_none());
        assert!(audio.alac_specific().is_none());
        assert!(audio.wave_extension().is_none());
        assert!(audio.channel_layout().is_none());
        let amr = audio.amr_specific().unwrap();
        assert_eq!(amr.vendor(), "rust");
        assert_eq!(amr.decoder_version(), 1);
        assert_eq!(amr.mode_set(), 0x0085);
        assert_eq!(amr.mode_change_period(), 2);
        assert_eq!(amr.frames_per_sample(), 2);
        assert_eq!(audio.child_boxes().len(), 1);
        assert_eq!(audio.child_boxes()[0].box_type(), "damr");
        assert_eq!(
            audio.child_boxes()[0].payload(),
            amr_specific_box_payload(*b"rust", 1, 0x0085, 2, 2).as_slice()
        );
    }

    #[test]
    fn parses_ac3_audio_sample_entry_specific_box() {
        let dac3_payload = ac3_specific_box_payload(0, 8, 0, 7, true, 10, 0);
        let dac3 = box_(*DAC3_ID, &dac3_payload);
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &dac3);
        let bytes = mp4_with_sample_description_entry(b"ac-3", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "ac-3");
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.channel_count(), 2);
        assert_eq!(audio.sample_size(), 16);
        assert_eq!(audio.sample_rate(), 48_000);
        assert!(audio.elementary_stream_descriptor().is_none());
        assert!(audio.bit_rate().is_none());
        assert!(audio.amr_specific().is_none());
        assert!(audio.ec3_specific().is_none());
        assert!(audio.opus_specific().is_none());
        assert!(audio.flac_specific().is_none());
        assert!(audio.alac_specific().is_none());
        assert!(audio.wave_extension().is_none());
        assert!(audio.channel_layout().is_none());
        let ac3 = audio.ac3_specific().unwrap();
        assert_eq!(ac3.fscod(), 0);
        assert_eq!(ac3.bsid(), 8);
        assert_eq!(ac3.bsmod(), 0);
        assert_eq!(ac3.acmod(), 7);
        assert!(ac3.lfeon());
        assert_eq!(ac3.bit_rate_code(), 10);
        assert_eq!(audio.child_boxes().len(), 1);
        assert_eq!(audio.child_boxes()[0].box_type(), "dac3");
        assert_eq!(audio.child_boxes()[0].payload(), dac3_payload.as_slice());
    }

    #[test]
    fn parses_ec3_audio_sample_entry_specific_box() {
        let first = ec3_substream_payload(0, 16, false, 0, 7, true, 0, None);
        let second = ec3_substream_payload(1, 15, true, 2, 2, false, 1, Some(0x101));
        let dec3_payload = ec3_specific_box_payload(768, 1, &[first, second], &[0xaa, 0x55]);
        let dec3 = box_(*DEC3_ID, &dec3_payload);
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &dec3);
        let bytes = mp4_with_sample_description_entry(b"ec-3", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "ec-3");
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.channel_count(), 2);
        assert_eq!(audio.sample_size(), 16);
        assert_eq!(audio.sample_rate(), 48_000);
        assert!(audio.elementary_stream_descriptor().is_none());
        assert!(audio.bit_rate().is_none());
        assert!(audio.amr_specific().is_none());
        assert!(audio.ac3_specific().is_none());
        assert!(audio.opus_specific().is_none());
        assert!(audio.flac_specific().is_none());
        assert!(audio.alac_specific().is_none());
        assert!(audio.wave_extension().is_none());
        assert!(audio.channel_layout().is_none());
        let ec3 = audio.ec3_specific().unwrap();
        assert_eq!(ec3.data_rate(), 768);
        assert_eq!(ec3.num_ind_sub(), 1);
        assert_eq!(ec3.trailing_reserved_bytes(), &[0xaa, 0x55]);
        assert_eq!(ec3.substreams().len(), 2);
        assert_eq!(ec3.substreams()[0].fscod(), 0);
        assert_eq!(ec3.substreams()[0].bsid(), 16);
        assert!(!ec3.substreams()[0].asvc());
        assert_eq!(ec3.substreams()[0].bsmod(), 0);
        assert_eq!(ec3.substreams()[0].acmod(), 7);
        assert!(ec3.substreams()[0].lfeon());
        assert_eq!(ec3.substreams()[0].num_dep_sub(), 0);
        assert_eq!(ec3.substreams()[0].chan_loc(), None);
        assert_eq!(ec3.substreams()[1].fscod(), 1);
        assert_eq!(ec3.substreams()[1].bsid(), 15);
        assert!(ec3.substreams()[1].asvc());
        assert_eq!(ec3.substreams()[1].bsmod(), 2);
        assert_eq!(ec3.substreams()[1].acmod(), 2);
        assert!(!ec3.substreams()[1].lfeon());
        assert_eq!(ec3.substreams()[1].num_dep_sub(), 1);
        assert_eq!(ec3.substreams()[1].chan_loc(), Some(0x101));
        assert_eq!(audio.child_boxes().len(), 1);
        assert_eq!(audio.child_boxes()[0].box_type(), "dec3");
        assert_eq!(audio.child_boxes()[0].payload(), dec3_payload.as_slice());
    }

    #[test]
    fn parses_opus_audio_sample_entry_specific_box() {
        let dops_payload = opus_specific_box_payload(2, 312, 48_000, -256, 0, None);
        let dops = box_(*DOPS_ID, &dops_payload);
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &dops);
        let bytes = mp4_with_sample_description_entry(b"Opus", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "Opus");
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.channel_count(), 2);
        assert_eq!(audio.sample_size(), 16);
        assert_eq!(audio.sample_rate(), 48_000);
        assert!(audio.elementary_stream_descriptor().is_none());
        assert!(audio.bit_rate().is_none());
        assert!(audio.amr_specific().is_none());
        assert!(audio.ac3_specific().is_none());
        assert!(audio.ec3_specific().is_none());
        assert!(audio.flac_specific().is_none());
        assert!(audio.alac_specific().is_none());
        assert!(audio.wave_extension().is_none());
        assert!(audio.channel_layout().is_none());
        let opus = audio.opus_specific().unwrap();
        assert_eq!(opus.version(), 0);
        assert_eq!(opus.output_channel_count(), 2);
        assert_eq!(opus.pre_skip(), 312);
        assert_eq!(opus.input_sample_rate(), 48_000);
        assert_eq!(opus.output_gain(), -256);
        assert_eq!(opus.channel_mapping_family(), 0);
        assert_eq!(opus.stream_count(), None);
        assert_eq!(opus.coupled_count(), None);
        assert!(opus.channel_mapping().is_empty());
        assert_eq!(audio.child_boxes().len(), 1);
        assert_eq!(audio.child_boxes()[0].box_type(), "dOps");
        assert_eq!(audio.child_boxes()[0].payload(), dops_payload.as_slice());

        let dops_payload = opus_specific_box_payload(2, 312, 48_000, -10, 1, Some((1, 1, &[0, 1])));
        let dops = box_(*DOPS_ID, &dops_payload);
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &dops);
        let bytes = mp4_with_sample_description_entry(b"Opus", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        let opus = audio.opus_specific().unwrap();
        assert_eq!(opus.channel_mapping_family(), 1);
        assert_eq!(opus.stream_count(), Some(1));
        assert_eq!(opus.coupled_count(), Some(1));
        assert_eq!(opus.channel_mapping(), &[0, 1]);
    }

    #[test]
    fn parses_flac_audio_sample_entry_specific_box() {
        let streaminfo = flac_streaminfo_block_data(96_000, 2, 24, 12_345);
        let streaminfo_block = flac_metadata_block(false, 0, &streaminfo);
        let padding_block = flac_metadata_block(true, 1, &[0xaa, 0xbb]);
        let dfla_payload =
            flac_specific_box_payload(&[streaminfo_block.as_slice(), padding_block.as_slice()]);
        let dfla = box_(*DFLA_ID, &dfla_payload);
        let extra_data = audio_sample_entry_extra_data(2, 24, 48_000, &dfla);
        let bytes = mp4_with_sample_description_entry(b"fLaC", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "fLaC");
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.channel_count(), 2);
        assert_eq!(audio.effective_channel_count(), 2);
        assert_eq!(audio.sample_size(), 24);
        assert_eq!(audio.effective_bits_per_sample(), 24);
        assert_eq!(audio.sample_rate_fixed_16_16(), 48_000 << 16);
        assert_eq!(audio.sample_rate(), 96_000);
        assert!(audio.elementary_stream_descriptor().is_none());
        assert!(audio.bit_rate().is_none());
        assert!(audio.amr_specific().is_none());
        assert!(audio.ac3_specific().is_none());
        assert!(audio.ec3_specific().is_none());
        assert!(audio.opus_specific().is_none());
        assert!(audio.alac_specific().is_none());
        assert!(audio.wave_extension().is_none());
        assert!(audio.channel_layout().is_none());

        let flac = audio.flac_specific().unwrap();
        assert_eq!(flac.version(), 0);
        assert_eq!(flac.flags(), [0, 0, 0]);
        assert_eq!(flac.metadata_blocks().len(), 2);
        assert!(!flac.metadata_blocks()[0].last());
        assert_eq!(flac.metadata_blocks()[0].block_type(), 0);
        assert_eq!(flac.metadata_blocks()[0].data(), streaminfo.as_slice());
        assert!(flac.metadata_blocks()[1].last());
        assert_eq!(flac.metadata_blocks()[1].block_type(), 1);
        assert_eq!(flac.metadata_blocks()[1].data(), &[0xaa, 0xbb]);
        let streaminfo = flac.streaminfo();
        assert_eq!(streaminfo.min_block_size(), 16);
        assert_eq!(streaminfo.max_block_size(), 4_096);
        assert_eq!(streaminfo.min_frame_size(), 0);
        assert_eq!(streaminfo.max_frame_size(), 0);
        assert_eq!(streaminfo.sample_rate(), 96_000);
        assert_eq!(streaminfo.channels(), 2);
        assert_eq!(streaminfo.bits_per_sample(), 24);
        assert_eq!(streaminfo.total_samples(), 12_345);
        assert_eq!(
            streaminfo.md5(),
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
        assert_eq!(audio.child_boxes().len(), 1);
        assert_eq!(audio.child_boxes()[0].box_type(), "dfLa");
        assert_eq!(audio.child_boxes()[0].payload(), dfla_payload.as_slice());
    }

    #[test]
    fn parses_alac_audio_sample_entry_specific_box() {
        let alac_payload = alac_specific_box_payload(
            4_096,
            0,
            24,
            40,
            14,
            10,
            2,
            255,
            8_192,
            320_000,
            44_100,
            Some(0x0065_0002),
        );
        let alac = box_(*ALAC_ID, &alac_payload);
        let extra_data = audio_sample_entry_extra_data(2, 24, 44_100, &alac);
        let bytes = mp4_with_sample_description_entry(b"alac", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "alac");
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.channel_count(), 2);
        assert_eq!(audio.effective_channel_count(), 2);
        assert_eq!(audio.sample_size(), 24);
        assert_eq!(audio.effective_bits_per_sample(), 24);
        assert_eq!(audio.sample_rate(), 44_100);
        assert_eq!(audio.sample_rate_fixed_16_16(), 44_100 << 16);
        assert!(audio.elementary_stream_descriptor().is_none());
        assert!(audio.bit_rate().is_none());
        assert!(audio.amr_specific().is_none());
        assert!(audio.ac3_specific().is_none());
        assert!(audio.ec3_specific().is_none());
        assert!(audio.opus_specific().is_none());
        assert!(audio.flac_specific().is_none());
        assert!(audio.wave_extension().is_none());
        assert!(audio.channel_layout().is_none());

        let alac = audio.alac_specific().unwrap();
        assert_eq!(alac.version(), 0);
        assert_eq!(alac.flags(), [0, 0, 0]);
        let config = alac.config();
        assert_eq!(config.frame_length(), 4_096);
        assert_eq!(config.compatible_version(), 0);
        assert_eq!(config.bit_depth(), 24);
        assert_eq!(config.pb(), 40);
        assert_eq!(config.mb(), 14);
        assert_eq!(config.kb(), 10);
        assert_eq!(config.num_channels(), 2);
        assert_eq!(config.max_run(), 255);
        assert_eq!(config.max_frame_bytes(), 8_192);
        assert_eq!(config.avg_bit_rate(), 320_000);
        assert_eq!(config.sample_rate(), 44_100);
        let channel_layout = alac.channel_layout().unwrap();
        assert_eq!(channel_layout.channel_layout_tag(), 0x0065_0002);
        assert_eq!(channel_layout.reserved(), [0, 0]);
        assert_eq!(audio.child_boxes().len(), 1);
        assert_eq!(audio.child_boxes()[0].box_type(), "alac");
        assert_eq!(audio.child_boxes()[0].payload(), alac_payload.as_slice());

        let wave_alac_payload =
            alac_specific_box_payload(4_096, 0, 16, 40, 14, 10, 1, 255, 0, 128_000, 44_100, None);
        let wave_payload = [
            box_(*FRMA_ID, ALAC_ID),
            box_(*ALAC_ID, &wave_alac_payload),
            box_(*TERMINATOR_ID, &[]),
        ]
        .concat();
        let wave = box_(*WAVE_ID, &wave_payload);
        let extra_data = audio_sample_entry_extra_data(1, 16, 44_100, &wave);
        let bytes = mp4_with_sample_description_entry(b"alac", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();
        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };

        assert_eq!(audio.channel_count(), 1);
        assert_eq!(audio.effective_channel_count(), 1);
        assert_eq!(audio.effective_bits_per_sample(), 16);
        assert!(audio.wave_extension().unwrap().has_terminator());
        assert_eq!(
            audio.wave_extension().unwrap().original_format(),
            Some("alac")
        );
        assert!(audio.wave_extension().unwrap().alac_specific().is_some());
        assert_eq!(
            audio.alac_specific().unwrap().config().avg_bit_rate(),
            128_000
        );
    }

    #[test]
    fn parses_audio_sample_entry_version_one_fields() {
        let wave_payload = wave_extension_payload(*b"mp4a", b"\x03\x01\x00");
        let wave = box_(*WAVE_ID, &wave_payload);
        let extra_data = audio_sample_entry_v1_extra_data(2, 16, 44_100, 1_024, 0, 4, 2, &wave);
        let bytes = mp4_with_sample_description_entry(b"mp4a", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.version(), 1);
        assert_eq!(audio.channel_count(), 2);
        assert_eq!(audio.effective_channel_count(), 2);
        assert_eq!(audio.sample_size(), 16);
        assert_eq!(audio.effective_bits_per_sample(), 16);
        assert_eq!(audio.sample_rate(), 44_100);
        let fields = audio.version1_fields().unwrap();
        assert_eq!(fields.samples_per_packet(), 1_024);
        assert_eq!(fields.bytes_per_packet(), 0);
        assert_eq!(fields.bytes_per_frame(), 4);
        assert_eq!(fields.bytes_per_sample(), 2);
        assert!(audio.version2_fields().is_none());
        assert_eq!(audio.child_boxes().len(), 1);
        assert_eq!(audio.child_boxes()[0].box_type(), "wave");
        assert_eq!(audio.child_boxes()[0].payload(), wave_payload.as_slice());
        let wave_extension = audio.wave_extension().unwrap();
        assert_eq!(wave_extension.original_format(), Some("mp4a"));
        assert!(wave_extension.alac_specific().is_none());
        assert!(wave_extension.has_terminator());
        assert_eq!(wave_extension.child_boxes().len(), 3);
        assert_eq!(wave_extension.child_boxes()[0].box_type(), "frma");
        assert_eq!(wave_extension.child_boxes()[1].box_type(), "esds");
        assert_eq!(
            wave_extension.child_boxes()[2].box_type().as_bytes(),
            TERMINATOR_ID
        );
        let esds = wave_extension.elementary_stream_descriptor().unwrap();
        assert_eq!(esds.version(), 0);
        assert_eq!(esds.flags(), [0, 0, 0]);
        assert_eq!(esds.descriptor(), b"\x03\x01\x00");
    }

    #[test]
    fn parses_audio_sample_entry_version_two_fields() {
        let chan_payload = audio_channel_layout_payload(
            0x0065_0006,
            0x0000_0003,
            &[(1, 0, [0.0, 0.5, -1.0]), (2, 0x2, [1.0, 0.0, 0.25])],
        );
        let chan = box_(*CHAN_ID, &chan_payload);
        let extra_data = audio_sample_entry_v2_extra_data(96_000.0, 6, 24, 0x09, 24, 1, &chan);
        let bytes = mp4_with_sample_description_entry(b"lpcm", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        let MovSampleEntryDetails::Audio(audio) = codec_parameters.details() else {
            panic!("expected audio sample entry details");
        };
        assert_eq!(audio.version(), 2);
        assert_eq!(audio.channel_count(), 3);
        assert_eq!(audio.effective_channel_count(), 6);
        assert_eq!(audio.sample_size(), 16);
        assert_eq!(audio.effective_bits_per_sample(), 24);
        assert_eq!(audio.compression_id(), -2);
        assert_eq!(audio.packet_size(), 0);
        assert_eq!(audio.sample_rate_fixed_16_16(), 65_536);
        assert_eq!(audio.sample_rate(), 96_000);
        let fields = audio.version2_fields().unwrap();
        assert_eq!(fields.size_of_struct_only(), 72);
        assert_eq!(fields.audio_sample_rate(), 96_000.0);
        assert_eq!(fields.num_audio_channels(), 6);
        assert_eq!(fields.always_7f000000(), 0x7f00_0000);
        assert_eq!(fields.const_bits_per_channel(), 24);
        assert_eq!(fields.format_specific_flags(), 0x09);
        assert_eq!(fields.const_bytes_per_audio_packet(), 24);
        assert_eq!(fields.const_lpcm_frames_per_audio_packet(), 1);
        assert!(audio.version1_fields().is_none());
        assert_eq!(audio.child_boxes().len(), 1);
        assert_eq!(audio.child_boxes()[0].box_type(), "chan");
        assert_eq!(audio.child_boxes()[0].payload(), chan_payload.as_slice());
        let channel_layout = audio.channel_layout().unwrap();
        assert_eq!(channel_layout.version(), 0);
        assert_eq!(channel_layout.flags(), [0, 0, 0]);
        assert_eq!(channel_layout.channel_layout_tag(), 0x0065_0006);
        assert_eq!(channel_layout.channel_bitmap(), 0x0000_0003);
        assert_eq!(channel_layout.channel_descriptions().len(), 2);
        assert_eq!(channel_layout.channel_descriptions()[0].channel_label(), 1);
        assert_eq!(channel_layout.channel_descriptions()[0].channel_flags(), 0);
        assert_eq!(
            channel_layout.channel_descriptions()[0].coordinate_bits(),
            [0.0_f32.to_bits(), 0.5_f32.to_bits(), (-1.0_f32).to_bits()]
        );
        assert_eq!(channel_layout.channel_descriptions()[1].channel_label(), 2);
        assert_eq!(
            channel_layout.channel_descriptions()[1].channel_flags(),
            0x2
        );
        assert_eq!(
            channel_layout.channel_descriptions()[1].coordinates(),
            [1.0, 0.0, 0.25]
        );
    }

    #[test]
    fn parses_timed_text_sample_entry_codec_parameters() {
        let ftab = box_(*b"ftab", &full_box(0, b"\0\x01\0\x01\0\x04Rust"));
        let extra_data = tx3g_sample_entry_extra_data(&ftab);
        let bytes = mp4_with_sample_description_entry(b"tx3g", 5, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "tx3g");
        assert_eq!(codec_parameters.data_reference_index(), 5);
        assert_eq!(codec_parameters.extra_data(), extra_data.as_slice());
        let MovSampleEntryDetails::Subtitle(subtitle) = codec_parameters.details() else {
            panic!("expected subtitle sample entry details");
        };
        let timed_text = subtitle.timed_text().unwrap();
        assert_eq!(timed_text.display_flags(), 0x0000_0200);
        assert_eq!(timed_text.horizontal_justification(), -1);
        assert_eq!(timed_text.vertical_justification(), 1);
        assert_eq!(timed_text.background_color_rgba(), [1, 2, 3, 4]);
        assert_eq!(timed_text.default_text_box().top(), -10);
        assert_eq!(timed_text.default_text_box().left(), -20);
        assert_eq!(timed_text.default_text_box().bottom(), 30);
        assert_eq!(timed_text.default_text_box().right(), 40);
        assert_eq!(timed_text.default_style().start_char(), 0);
        assert_eq!(timed_text.default_style().end_char(), 5);
        assert_eq!(timed_text.default_style().font_id(), 2);
        assert_eq!(timed_text.default_style().face_style_flags(), 1);
        assert_eq!(timed_text.default_style().font_size(), 18);
        assert_eq!(
            timed_text.default_style().text_color_rgba(),
            [10, 20, 30, 255]
        );
        assert_eq!(timed_text.child_boxes().len(), 1);
        assert_eq!(timed_text.child_boxes()[0].box_type(), "ftab");
        assert_eq!(
            timed_text.child_boxes()[0].payload(),
            full_box(0, b"\0\x01\0\x01\0\x04Rust").as_slice()
        );
    }

    #[test]
    fn parses_metadata_sample_entry_codec_parameters() {
        let metx_extra = xml_metadata_sample_entry_extra_data("utf-8", "urn:rust", "rust.xsd");
        let bytes = mp4_with_sample_description_entry(b"metx", 1, &metx_extra);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();
        let MovSampleEntryDetails::Data(data) = codec_parameters.details() else {
            panic!("expected data sample entry details");
        };
        let xml = data.xml_metadata().unwrap();
        assert_eq!(xml.content_encoding(), "utf-8");
        assert_eq!(xml.namespace(), "urn:rust");
        assert_eq!(xml.schema_location(), "rust.xsd");
        assert!(data.text_metadata().is_none());

        let mett_extra = text_metadata_sample_entry_extra_data("utf-8", "application/json");
        let bytes = mp4_with_sample_description_entry(b"mett", 2, &mett_extra);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();
        assert_eq!(codec_parameters.data_reference_index(), 2);
        let MovSampleEntryDetails::Data(data) = codec_parameters.details() else {
            panic!("expected data sample entry details");
        };
        let text = data.text_metadata().unwrap();
        assert_eq!(text.content_encoding(), "utf-8");
        assert_eq!(text.mime_format(), "application/json");
        assert!(data.xml_metadata().is_none());
    }

    #[test]
    fn parses_visual_sample_entry_codec_parameters() {
        let avcc = avcc_payload(
            100,
            0,
            31,
            4,
            &[b"\x67\x64".as_slice()],
            &[b"\x68".as_slice()],
        );
        let child_box = box_(*AVCC_ID, &avcc);
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &child_box);
        let bytes = mp4_with_sample_description_entry(b"avc1", 2, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "avc1");
        assert_eq!(codec_parameters.data_reference_index(), 2);
        assert_eq!(codec_parameters.extra_data(), extra_data.as_slice());
        let MovSampleEntryDetails::Video(video) = codec_parameters.details() else {
            panic!("expected visual sample entry details");
        };
        assert_eq!(video.width(), 640);
        assert_eq!(video.height(), 360);
        assert_eq!(video.frame_count(), 1);
        assert_eq!(video.compressor_name(), "Rust AVC");
        assert_eq!(video.depth(), 24);
        assert_eq!(video.child_boxes().len(), 1);
        assert_eq!(video.child_boxes()[0].box_type(), "avcC");
        assert_eq!(video.child_boxes()[0].payload(), avcc.as_slice());
        assert_eq!(
            video.avc_decoder_configuration_record(),
            Some(avcc.as_slice())
        );
        let configuration = video.avc_decoder_configuration().unwrap();
        assert_eq!(configuration.configuration_version(), 1);
        assert_eq!(configuration.profile_indication(), 100);
        assert_eq!(configuration.profile_compatibility(), 0);
        assert_eq!(configuration.level_indication(), 31);
        assert_eq!(configuration.nal_length_size(), 4);
        assert_eq!(
            configuration.sequence_parameter_sets(),
            &[b"\x67\x64".to_vec()]
        );
        assert_eq!(configuration.picture_parameter_sets(), &[b"\x68".to_vec()]);
        assert!(configuration.extension_data().is_empty());
        assert_eq!(video.hevc_decoder_configuration_record(), None);
    }

    #[test]
    fn parses_raw_visual_sample_entry_codec_parameters() {
        let extra_data = visual_sample_entry_extra_data(320, 240, "Rust Raw", 16, &[]);
        let bytes = mp4_with_sample_description_entry(b"raw ", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "raw ");
        assert_eq!(codec_parameters.extra_data(), extra_data.as_slice());
        let MovSampleEntryDetails::Video(video) = codec_parameters.details() else {
            panic!("expected raw visual sample entry details");
        };
        assert_eq!(video.width(), 320);
        assert_eq!(video.height(), 240);
        assert_eq!(video.compressor_name(), "Rust Raw");
        assert_eq!(video.depth(), 16);
        assert!(video.child_boxes().is_empty());
    }

    #[test]
    fn parses_hevc_sample_entry_codec_parameters() {
        let hvcc = hvcc_payload(
            &[b"\x40\x01".as_slice()],
            &[b"\x42\x01".as_slice()],
            &[b"\x44".as_slice()],
        );
        let child_box = box_(*HVCC_ID, &hvcc);
        let extra_data = visual_sample_entry_extra_data(1920, 1080, "Rust HEVC", 24, &child_box);
        let bytes = mp4_with_sample_description_entry(b"hvc1", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "hvc1");
        let MovSampleEntryDetails::Video(video) = codec_parameters.details() else {
            panic!("expected visual sample entry details");
        };
        assert_eq!(video.width(), 1920);
        assert_eq!(video.height(), 1080);
        assert_eq!(video.compressor_name(), "Rust HEVC");
        assert_eq!(video.child_boxes().len(), 1);
        assert_eq!(video.child_boxes()[0].box_type(), "hvcC");
        assert_eq!(
            video.hevc_decoder_configuration_record(),
            Some(hvcc.as_slice())
        );
        assert_eq!(video.avc_decoder_configuration_record(), None);
        assert!(video.avc_decoder_configuration().is_none());

        let configuration = video.hevc_decoder_configuration().unwrap();
        assert_eq!(configuration.configuration_version(), 1);
        assert_eq!(configuration.general_profile_space(), 0);
        assert!(!configuration.general_tier_flag());
        assert_eq!(configuration.general_profile_idc(), 1);
        assert_eq!(
            configuration.general_profile_compatibility_flags(),
            0x6000_0000
        );
        assert_eq!(configuration.general_constraint_indicator_flags(), 0x90);
        assert_eq!(configuration.general_level_idc(), 120);
        assert_eq!(configuration.min_spatial_segmentation_idc(), 0);
        assert_eq!(configuration.parallelism_type(), 0);
        assert_eq!(configuration.chroma_format(), 1);
        assert_eq!(configuration.bit_depth_luma(), 8);
        assert_eq!(configuration.bit_depth_chroma(), 8);
        assert_eq!(configuration.average_frame_rate(), 0);
        assert_eq!(configuration.constant_frame_rate(), 0);
        assert_eq!(configuration.num_temporal_layers(), 1);
        assert!(configuration.temporal_id_nested());
        assert_eq!(configuration.nal_length_size(), 4);

        let arrays = configuration.arrays();
        assert_eq!(arrays.len(), 3);
        assert!(arrays[0].array_completeness());
        assert_eq!(arrays[0].nal_unit_type(), 32);
        assert_eq!(arrays[0].nal_units(), &[b"\x40\x01".to_vec()]);
        assert_eq!(arrays[1].nal_unit_type(), 33);
        assert_eq!(arrays[1].nal_units(), &[b"\x42\x01".to_vec()]);
        assert_eq!(arrays[2].nal_unit_type(), 34);
        assert_eq!(arrays[2].nal_units(), &[b"\x44".to_vec()]);
    }

    #[test]
    fn parses_visual_sample_entry_pixel_aspect_and_color_information() {
        let child_boxes = [
            pasp_box(4, 3),
            colr_nclx_box(1, 13, 6, true),
            box_(
                *AVCC_ID,
                &avcc_payload(100, 0, 31, 4, &[b"\x67".as_slice()], &[b"\x68".as_slice()]),
            ),
        ]
        .concat();
        let extra_data = visual_sample_entry_extra_data(720, 576, "PAL AVC", 24, &child_boxes);
        let bytes = mp4_with_sample_description_entry(b"avc1", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        let MovSampleEntryDetails::Video(video) = codec_parameters.details() else {
            panic!("expected visual sample entry details");
        };
        let pixel_aspect_ratio = video.pixel_aspect_ratio().unwrap();
        assert_eq!(pixel_aspect_ratio.horizontal_spacing(), 4);
        assert_eq!(pixel_aspect_ratio.vertical_spacing(), 3);

        let color_information = video.color_information().unwrap();
        assert_eq!(color_information.color_type(), "nclx");
        assert_eq!(color_information.color_primaries(), Some(1));
        assert_eq!(color_information.transfer_characteristics(), Some(13));
        assert_eq!(color_information.matrix_coefficients(), Some(6));
        assert_eq!(color_information.full_range(), Some(true));
        assert_eq!(color_information.icc_profile(), None);
        assert_eq!(video.child_boxes().len(), 3);
    }

    #[test]
    fn parses_visual_sample_entry_nclc_color_information() {
        let child_box = colr_nclc_box(9, 16, 9);
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &child_box);
        let bytes = mp4_with_sample_description_entry(b"avc1", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        let MovSampleEntryDetails::Video(video) = codec_parameters.details() else {
            panic!("expected visual sample entry details");
        };
        let color_information = video.color_information().unwrap();
        assert_eq!(color_information.color_type(), "nclc");
        let parameters = color_information.color_parameters().unwrap();
        assert_eq!(parameters.color_primaries(), 9);
        assert_eq!(parameters.transfer_characteristics(), 16);
        assert_eq!(parameters.matrix_coefficients(), 9);
        assert!(!parameters.full_range());
        assert_eq!(color_information.full_range(), Some(false));
        assert_eq!(color_information.icc_profile(), None);
    }

    #[test]
    fn parses_visual_sample_entry_icc_color_information() {
        let child_box = colr_icc_profile_box(*RICC_ID, b"rust-icc-profile");
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &child_box);
        let bytes = mp4_with_sample_description_entry(b"avc1", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        let MovSampleEntryDetails::Video(video) = codec_parameters.details() else {
            panic!("expected visual sample entry details");
        };
        let color_information = video.color_information().unwrap();
        assert_eq!(color_information.color_type(), "rICC");
        assert_eq!(color_information.color_parameters(), None);
        assert_eq!(color_information.color_primaries(), None);
        assert_eq!(color_information.transfer_characteristics(), None);
        assert_eq!(color_information.matrix_coefficients(), None);
        assert_eq!(color_information.full_range(), None);
        assert_eq!(
            color_information.icc_profile(),
            Some(b"rust-icc-profile".as_slice())
        );

        let child_box = colr_icc_profile_box(*PROF_ID, b"rust-prof-profile");
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &child_box);
        let bytes = mp4_with_sample_description_entry(b"avc1", 1, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();
        let MovSampleEntryDetails::Video(video) = codec_parameters.details() else {
            panic!("expected visual sample entry details");
        };
        let color_information = video.color_information().unwrap();
        assert_eq!(color_information.color_type(), "prof");
        assert_eq!(
            color_information.icc_profile(),
            Some(b"rust-prof-profile".as_slice())
        );
    }

    #[test]
    fn rejects_malformed_visual_sample_entry_child_box() {
        let child_box = box_with_declared_size(*AVCC_ID, 12, b"\x01");
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &child_box);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"avc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_pasp = pasp_box(0, 1);
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &bad_pasp);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"avc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_colr = box_(*COLR_ID, b"nclx\0\x01");
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &bad_colr);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"avc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_colr = box_(*COLR_ID, b"nclc\0\x01");
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &bad_colr);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"avc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_colr = box_(*COLR_ID, PROF_ID);
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &bad_colr);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"avc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_avcc = box_(
            *AVCC_ID,
            &avcc_payload(100, 0, 31, 4, &[b"".as_slice()], &[b"\x68".as_slice()]),
        );
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &bad_avcc);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"avc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mut unsupported_avcc =
            avcc_payload(100, 0, 31, 4, &[b"\x67".as_slice()], &[b"\x68".as_slice()]);
        unsupported_avcc[0] = 2;
        let unsupported_avcc = box_(*AVCC_ID, &unsupported_avcc);
        let extra_data =
            visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &unsupported_avcc);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"avc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let bad_hvcc = box_(*HVCC_ID, b"\x01\x01");
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust HEVC", 24, &bad_hvcc);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"hvc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_hvcc = box_(
            *HVCC_ID,
            &hvcc_payload(
                &[b"".as_slice()],
                &[b"\x42".as_slice()],
                &[b"\x44".as_slice()],
            ),
        );
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust HEVC", 24, &bad_hvcc);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"hvc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mut unsupported_hvcc = hvcc_payload(
            &[b"\x40".as_slice()],
            &[b"\x42".as_slice()],
            &[b"\x44".as_slice()],
        );
        unsupported_hvcc[0] = 2;
        let unsupported_hvcc = box_(*HVCC_ID, &unsupported_hvcc);
        let extra_data =
            visual_sample_entry_extra_data(640, 360, "Rust HEVC", 24, &unsupported_hvcc);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"hvc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::Unsupported);
    }

    #[test]
    fn rejects_malformed_audio_sample_entry() {
        let err =
            MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, b"\0\0")).unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let mut missing_v1_fields = audio_sample_entry_extra_data(2, 16, 44_100, &[]);
        missing_v1_fields[1] = 1;
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"mp4a",
            1,
            &missing_v1_fields,
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let mut unknown_version = audio_sample_entry_extra_data(2, 16, 44_100, &[]);
        unknown_version[1] = 3;
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"mp4a",
            1,
            &unknown_version,
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let mut bad_v2_offset = audio_sample_entry_v2_extra_data(48_000.0, 2, 16, 0, 4, 1, &[]);
        bad_v2_offset[20..24].copy_from_slice(&68_u32.to_be_bytes());
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"lpcm",
            1,
            &bad_v2_offset,
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_v2_rate = audio_sample_entry_v2_extra_data(f64::NAN, 2, 16, 0, 4, 1, &[]);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"lpcm", 1, &bad_v2_rate))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_chan_version = box_(*CHAN_ID, &full_box(1, b"\0\0\0\0\0\0\0\0\0\0\0\0"));
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_chan_version);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let bad_chan_flags = {
            let mut payload = audio_channel_layout_payload(1, 0, &[]);
            payload[3] = 1;
            box_(*CHAN_ID, &payload)
        };
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_chan_flags);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mut bad_chan_count = audio_channel_layout_payload(1, 0, &[]);
        bad_chan_count[12..16].copy_from_slice(&1_u32.to_be_bytes());
        let bad_chan_count = box_(*CHAN_ID, &bad_chan_count);
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_chan_count);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_direct_esds = box_(*ESDS_ID, &full_box(1, b"\x03"));
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_direct_esds);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let empty_direct_esds = box_(*ESDS_ID, &full_box(0, &[]));
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &empty_direct_esds);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let truncated_btrt = box_(*BTRT_ID, b"\0\0");
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &truncated_btrt);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let mut oversized_btrt = bit_rate_box_payload(4_096, 192_000, 128_000);
        oversized_btrt.push(0);
        let oversized_btrt = box_(*BTRT_ID, &oversized_btrt);
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &oversized_btrt);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let truncated_damr = box_(*DAMR_ID, b"\0\0");
        let extra_data = audio_sample_entry_extra_data(1, 16, 8_000, &truncated_damr);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"samr", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let mut oversized_damr = amr_specific_box_payload(*b"rust", 1, 0x0085, 2, 2);
        oversized_damr.push(0);
        let oversized_damr = box_(*DAMR_ID, &oversized_damr);
        let extra_data = audio_sample_entry_extra_data(1, 16, 8_000, &oversized_damr);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"samr", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let invalid_zero_frames = box_(
            *DAMR_ID,
            &amr_specific_box_payload(*b"rust", 1, 0x0085, 0, 0),
        );
        let extra_data = audio_sample_entry_extra_data(1, 16, 8_000, &invalid_zero_frames);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"samr", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let invalid_many_frames = box_(
            *DAMR_ID,
            &amr_specific_box_payload(*b"rust", 1, 0x0085, 0, 16),
        );
        let extra_data = audio_sample_entry_extra_data(1, 16, 8_000, &invalid_many_frames);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"samr", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let missing_dac3 = audio_sample_entry_extra_data(2, 16, 48_000, &[]);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"ac-3",
            1,
            &missing_dac3,
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let truncated_dac3 = box_(*DAC3_ID, b"\0\0");
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &truncated_dac3);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ac-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let mut oversized_dac3 = ac3_specific_box_payload(0, 8, 0, 7, true, 10, 0);
        oversized_dac3.push(0);
        let oversized_dac3 = box_(*DAC3_ID, &oversized_dac3);
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &oversized_dac3);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ac-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let reserved_fscod = box_(*DAC3_ID, &ac3_specific_box_payload(3, 8, 0, 7, true, 10, 0));
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &reserved_fscod);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ac-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let invalid_bit_rate_code =
            box_(*DAC3_ID, &ac3_specific_box_payload(0, 8, 0, 7, true, 19, 0));
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &invalid_bit_rate_code);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ac-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let invalid_reserved_bits =
            box_(*DAC3_ID, &ac3_specific_box_payload(0, 8, 0, 7, true, 10, 1));
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &invalid_reserved_bits);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ac-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let duplicate_dac3 = [
            box_(*DAC3_ID, &ac3_specific_box_payload(0, 8, 0, 7, true, 10, 0)),
            box_(*DAC3_ID, &ac3_specific_box_payload(0, 8, 0, 7, true, 10, 0)),
        ]
        .concat();
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &duplicate_dac3);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ac-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let missing_dec3 = audio_sample_entry_extra_data(2, 16, 48_000, &[]);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"ec-3",
            1,
            &missing_dec3,
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let truncated_dec3 = box_(*DEC3_ID, b"\0\0");
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &truncated_dec3);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ec-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let incomplete_dec3_substream = box_(
            *DEC3_ID,
            &ec3_specific_box_payload(
                256,
                1,
                &[ec3_substream_payload(0, 16, false, 0, 7, true, 0, None)],
                &[],
            ),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &incomplete_dec3_substream);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ec-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let reserved_dec3_bit = box_(
            *DEC3_ID,
            &ec3_specific_box_payload(
                256,
                0,
                &[ec3_substream_payload_with_reserved_bits(
                    0, 16, true, false, 0, 7, true, 0, 0, None, false,
                )],
                &[],
            ),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &reserved_dec3_bit);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ec-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let reserved_dec3_bits = box_(
            *DEC3_ID,
            &ec3_specific_box_payload(
                256,
                0,
                &[ec3_substream_payload_with_reserved_bits(
                    0, 16, false, false, 0, 7, true, 1, 0, None, false,
                )],
                &[],
            ),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &reserved_dec3_bits);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ec-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let reserved_dec3_no_dep = box_(
            *DEC3_ID,
            &ec3_specific_box_payload(
                256,
                0,
                &[ec3_substream_payload_with_reserved_bits(
                    0, 16, false, false, 0, 7, true, 0, 0, None, true,
                )],
                &[],
            ),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &reserved_dec3_no_dep);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ec-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let missing_dec3_chan_loc = box_(
            *DEC3_ID,
            &ec3_specific_box_payload(
                256,
                0,
                &[ec3_substream_payload_with_reserved_bits(
                    0, 16, false, false, 0, 7, true, 0, 1, None, false,
                )],
                &[],
            ),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &missing_dec3_chan_loc);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ec-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let duplicate_dec3 = [
            box_(
                *DEC3_ID,
                &ec3_specific_box_payload(
                    256,
                    0,
                    &[ec3_substream_payload(0, 16, false, 0, 7, true, 0, None)],
                    &[],
                ),
            ),
            box_(
                *DEC3_ID,
                &ec3_specific_box_payload(
                    256,
                    0,
                    &[ec3_substream_payload(0, 16, false, 0, 7, true, 0, None)],
                    &[],
                ),
            ),
        ]
        .concat();
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &duplicate_dec3);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"ec-3", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let missing_dops = audio_sample_entry_extra_data(2, 16, 48_000, &[]);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"Opus",
            1,
            &missing_dops,
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let truncated_dops = box_(*DOPS_ID, b"\0\0");
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &truncated_dops);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"Opus", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let unsupported_dops = box_(*DOPS_ID, b"\x01");
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &unsupported_dops);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"Opus", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let zero_output_channels = box_(
            *DOPS_ID,
            &opus_specific_box_payload(0, 312, 48_000, 0, 0, None),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &zero_output_channels);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"Opus", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mut oversized_family_zero_dops = opus_specific_box_payload(2, 312, 48_000, 0, 0, None);
        oversized_family_zero_dops.push(0);
        let oversized_family_zero_dops = box_(*DOPS_ID, &oversized_family_zero_dops);
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &oversized_family_zero_dops);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"Opus", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let missing_channel_mapping_table = box_(
            *DOPS_ID,
            &opus_specific_box_payload(2, 312, 48_000, 0, 1, None),
        );
        let extra_data =
            audio_sample_entry_extra_data(2, 16, 48_000, &missing_channel_mapping_table);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"Opus", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let invalid_stream_count = box_(
            *DOPS_ID,
            &opus_specific_box_payload(2, 312, 48_000, 0, 1, Some((0, 0, &[0, 1]))),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &invalid_stream_count);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"Opus", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let invalid_coupled_count = box_(
            *DOPS_ID,
            &opus_specific_box_payload(2, 312, 48_000, 0, 1, Some((1, 2, &[0, 1]))),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &invalid_coupled_count);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"Opus", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let duplicate_dops = [
            box_(
                *DOPS_ID,
                &opus_specific_box_payload(2, 312, 48_000, 0, 0, None),
            ),
            box_(
                *DOPS_ID,
                &opus_specific_box_payload(2, 312, 48_000, 0, 0, None),
            ),
        ]
        .concat();
        let extra_data = audio_sample_entry_extra_data(2, 16, 48_000, &duplicate_dops);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"Opus", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let valid_alac_payload =
            alac_specific_box_payload(4_096, 0, 16, 40, 14, 10, 2, 255, 0, 128_000, 44_100, None);
        let missing_alac = audio_sample_entry_extra_data(2, 16, 44_100, &[]);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"alac",
            1,
            &missing_alac,
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let truncated_alac = box_(*ALAC_ID, b"\0\0");
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &truncated_alac);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let unsupported_alac = box_(*ALAC_ID, &full_box(1, &[]));
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &unsupported_alac);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let mut flagged_alac_payload = valid_alac_payload.clone();
        flagged_alac_payload[3] = 1;
        let flagged_alac = box_(*ALAC_ID, &flagged_alac_payload);
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &flagged_alac);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let invalid_alac_frame_length = box_(
            *ALAC_ID,
            &alac_specific_box_payload(0, 0, 16, 40, 14, 10, 2, 255, 0, 128_000, 44_100, None),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &invalid_alac_frame_length);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let unsupported_alac_compatible_version = box_(
            *ALAC_ID,
            &alac_specific_box_payload(4_096, 1, 16, 40, 14, 10, 2, 255, 0, 128_000, 44_100, None),
        );
        let extra_data =
            audio_sample_entry_extra_data(2, 16, 44_100, &unsupported_alac_compatible_version);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let invalid_alac_bit_depth = box_(
            *ALAC_ID,
            &alac_specific_box_payload(4_096, 0, 33, 40, 14, 10, 2, 255, 0, 128_000, 44_100, None),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &invalid_alac_bit_depth);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let zero_alac_channels = box_(
            *ALAC_ID,
            &alac_specific_box_payload(4_096, 0, 16, 40, 14, 10, 0, 255, 0, 128_000, 44_100, None),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &zero_alac_channels);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let zero_alac_rate = box_(
            *ALAC_ID,
            &alac_specific_box_payload(4_096, 0, 16, 40, 14, 10, 2, 255, 0, 128_000, 0, None),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &zero_alac_rate);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mismatched_alac_channels = box_(
            *ALAC_ID,
            &alac_specific_box_payload(4_096, 0, 16, 40, 14, 10, 1, 255, 0, 128_000, 44_100, None),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &mismatched_alac_channels);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mismatched_alac_size = box_(
            *ALAC_ID,
            &alac_specific_box_payload(4_096, 0, 24, 40, 14, 10, 2, 255, 0, 128_000, 44_100, None),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &mismatched_alac_size);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mismatched_alac_rate = box_(
            *ALAC_ID,
            &alac_specific_box_payload(4_096, 0, 16, 40, 14, 10, 2, 255, 0, 128_000, 48_000, None),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &mismatched_alac_rate);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let duplicate_alac = [
            box_(*ALAC_ID, &valid_alac_payload),
            box_(*ALAC_ID, &valid_alac_payload),
        ]
        .concat();
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &duplicate_alac);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mut truncated_alac_chan_payload = valid_alac_payload.clone();
        truncated_alac_chan_payload.extend_from_slice(&[0; 8]);
        let truncated_alac_chan = box_(*ALAC_ID, &truncated_alac_chan_payload);
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &truncated_alac_chan);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let mut invalid_alac_chan_payload = valid_alac_payload.clone();
        invalid_alac_chan_payload.extend_from_slice(&alac_channel_layout_info_payload(
            0x0065_0002,
            [0, 1],
            *CHAN_ID,
            0,
            [0, 0, 0],
            24,
        ));
        let invalid_alac_chan = box_(*ALAC_ID, &invalid_alac_chan_payload);
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &invalid_alac_chan);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let wave_alac_payload = [
            box_(*FRMA_ID, ALAC_ID),
            box_(*ALAC_ID, &valid_alac_payload),
            box_(*TERMINATOR_ID, &[]),
        ]
        .concat();
        let direct_and_wave_alac = [
            box_(*ALAC_ID, &valid_alac_payload),
            box_(*WAVE_ID, &wave_alac_payload),
        ]
        .concat();
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &direct_and_wave_alac);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"alac", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let flac_streaminfo = flac_streaminfo_block_data(44_100, 2, 16, 1_000);
        let flac_streaminfo_block = flac_metadata_block(true, 0, &flac_streaminfo);
        let missing_dfla = audio_sample_entry_extra_data(2, 16, 44_100, &[]);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"fLaC",
            1,
            &missing_dfla,
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let truncated_dfla = box_(*DFLA_ID, b"\0\0");
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &truncated_dfla);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let unsupported_dfla = box_(*DFLA_ID, &full_box(1, &[]));
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &unsupported_dfla);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let mut flagged_dfla_payload =
            flac_specific_box_payload(&[flac_streaminfo_block.as_slice()]);
        flagged_dfla_payload[3] = 1;
        let flagged_dfla = box_(*DFLA_ID, &flagged_dfla_payload);
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &flagged_dfla);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_first_dfla = box_(
            *DFLA_ID,
            &flac_specific_box_payload(&[flac_metadata_block(true, 1, &[]).as_slice()]),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_first_dfla);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_streaminfo_len = box_(
            *DFLA_ID,
            &flac_specific_box_payload(&[flac_metadata_block(true, 0, &[0; 33]).as_slice()]),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_streaminfo_len);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let duplicate_streaminfo = box_(
            *DFLA_ID,
            &flac_specific_box_payload(&[
                flac_metadata_block(false, 0, &flac_streaminfo).as_slice(),
                flac_metadata_block(true, 0, &flac_streaminfo).as_slice(),
            ]),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &duplicate_streaminfo);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let missing_final_flac_flag = box_(
            *DFLA_ID,
            &flac_specific_box_payload(&[
                flac_metadata_block(false, 0, &flac_streaminfo).as_slice()
            ]),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &missing_final_flac_flag);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let early_final_flac_flag = box_(
            *DFLA_ID,
            &flac_specific_box_payload(&[
                flac_metadata_block(true, 0, &flac_streaminfo).as_slice(),
                flac_metadata_block(true, 1, &[]).as_slice(),
            ]),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &early_final_flac_flag);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let forbidden_flac_block = box_(
            *DFLA_ID,
            &flac_specific_box_payload(&[
                flac_metadata_block(false, 0, &flac_streaminfo).as_slice(),
                flac_metadata_block(true, 127, &[]).as_slice(),
            ]),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &forbidden_flac_block);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let mismatched_flac_channels = box_(
            *DFLA_ID,
            &flac_specific_box_payload(&[flac_metadata_block(
                true,
                0,
                &flac_streaminfo_block_data(44_100, 1, 16, 1_000),
            )
            .as_slice()]),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &mismatched_flac_channels);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let zero_rate_flac = box_(
            *DFLA_ID,
            &flac_specific_box_payload(&[flac_metadata_block(
                true,
                0,
                &flac_streaminfo_block_data(0, 2, 16, 1_000),
            )
            .as_slice()]),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &zero_rate_flac);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"fLaC", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let truncated_wave = box_(*WAVE_ID, b"\0\0");
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &truncated_wave);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_wave_frma = box_(*WAVE_ID, &box_(*FRMA_ID, b"mp4"));
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_wave_frma);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_wave_esds = box_(*WAVE_ID, &box_(*ESDS_ID, &full_box(1, b"\x03")));
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_wave_esds);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::Unsupported);

        let bad_wave_terminator = box_(
            *WAVE_ID,
            &[box_(*TERMINATOR_ID, &[]), box_(*FRMA_ID, b"mp4a")].concat(),
        );
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_wave_terminator);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let bad_child = box_with_declared_size(*b"esds", 12, b"\x01");
        let extra_data = audio_sample_entry_extra_data(2, 16, 44_100, &bad_child);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"mp4a", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
    }

    #[test]
    fn rejects_malformed_subtitle_and_data_sample_entries() {
        let err =
            MovDemuxer::open(&mp4_with_sample_description_entry(b"tx3g", 1, b"\0")).unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let bad_child = box_with_declared_size(*b"ftab", 12, b"\0");
        let extra_data = tx3g_sample_entry_extra_data(&bad_child);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"tx3g", 1, &extra_data))
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);

        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"metx",
            1,
            b"utf-8\0urn",
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);

        let err = MovDemuxer::open(&mp4_with_sample_description_entry(
            b"mett",
            1,
            b"utf-8\0\xff\0",
        ))
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
    }

    #[test]
    fn rejects_sample_description_indexes_other_than_one() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_sample_description_index(2))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_multiple_sample_description_entries_explicitly() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_multiple_sample_description_entries())
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn reads_packets_from_co64_offsets() {
        let bytes = mp4_with_samples(true, &[b"xy".as_slice()], &[42]);
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let packet = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(demuxer.info().tracks()[0].sample_count(), 1);
        assert_eq!(packet.data(), b"xy");
        assert_eq!(packet.duration(), 42);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_packets_from_multiple_chunks_and_stsc_entries() {
        let bytes = mp4_with_chunk_layout(
            false,
            &[
                b"aa".as_slice(),
                b"bbb".as_slice(),
                b"c".as_slice(),
                b"dddd".as_slice(),
                b"ee".as_slice(),
            ],
            &[10, 11, 12, 13, 14],
            &[(1, 2, 1), (3, 1, 1)],
            &[2, 2, 1],
            &[b"gap".as_slice(), b"x".as_slice()],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().tracks()[0].sample_count(), 5);
        for (data, pts, duration) in [
            (b"aa".as_slice(), 0, 10),
            (b"bbb".as_slice(), 10, 11),
            (b"c".as_slice(), 21, 12),
            (b"dddd".as_slice(), 33, 13),
            (b"ee".as_slice(), 46, 14),
        ] {
            let packet = demuxer.read_packet().unwrap().unwrap();
            assert_eq!(packet.data(), data);
            assert_eq!(packet.pts(), Some(pts));
            assert_eq!(packet.dts(), Some(pts));
            assert_eq!(packet.duration(), duration);
        }
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_packets_from_multiple_mdat_ranges() {
        let bytes = mp4_with_multiple_mdat_ranges();
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        assert!(demuxer.info().has_media_data());
        assert_eq!(demuxer.info().tracks()[0].sample_count(), 2);

        let first = demuxer.read_packet().unwrap().unwrap();
        let second = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(first.data(), b"aa");
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.duration(), 10);
        assert_eq!(second.data(), b"bbb");
        assert_eq!(second.pts(), Some(10));
        assert_eq!(second.duration(), 11);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_sync_sample_table_for_key_flags() {
        let bytes = mp4_with_samples_and_sync(
            false,
            &[b"aa".as_slice(), b"bb".as_slice(), b"cc".as_slice()],
            &[10, 20, 30],
            &[1, 3],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let first = demuxer.read_packet().unwrap().unwrap();
        let second = demuxer.read_packet().unwrap().unwrap();
        let third = demuxer.read_packet().unwrap().unwrap();

        assert!(first.flags().contains(avutil::PacketFlags::KEY));
        assert!(!second.flags().contains(avutil::PacketFlags::KEY));
        assert!(third.flags().contains(avutil::PacketFlags::KEY));
        assert_eq!(third.pts(), Some(30));
        assert_eq!(third.duration(), 30);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_composition_offsets_for_packet_pts() {
        let bytes = mp4_with_samples_and_ctts(
            false,
            &[b"aa".as_slice(), b"bb".as_slice(), b"cc".as_slice()],
            &[10, 20, 30],
            0,
            &[(2, 5), (1, 7)],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let first = demuxer.read_packet().unwrap().unwrap();
        let second = demuxer.read_packet().unwrap().unwrap();
        let third = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.pts(), Some(5));
        assert_eq!(second.dts(), Some(10));
        assert_eq!(second.pts(), Some(15));
        assert_eq!(third.dts(), Some(30));
        assert_eq!(third.pts(), Some(37));
        assert_eq!(third.duration(), 30);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_signed_composition_offsets_for_packet_pts() {
        let bytes = mp4_with_samples_and_ctts(
            false,
            &[b"aa".as_slice(), b"bb".as_slice()],
            &[10, 10],
            1,
            &[(1, -2), (1, 0)],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let first = demuxer.read_packet().unwrap().unwrap();
        let second = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.pts(), Some(-2));
        assert_eq!(second.dts(), Some(10));
        assert_eq!(second.pts(), Some(10));
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_sample_tables_and_truncated_mdat_payloads() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_mismatched_sample_counts())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_sample_description_index(2))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_short_mdat_payload())
                .unwrap_err()
                .kind(),
            AvErrorKind::EndOfFile
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_sync_samples(&[3]))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_sync_samples(&[2, 2]))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_composition_offsets(0, &[(1, 0)]))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_composition_offsets(0, &[(0, 0)]))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_composition_offsets(2, &[(2, 0)]))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_chunk_layout(
                false,
                &[b"aa".as_slice(), b"bb".as_slice()],
                &[1_000, 1_000],
                &[(1, 1, 1)],
                &[1],
                &[],
            ))
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
    }

    fn mp4_v0_fixture() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_v0_box_payload()));
        out.extend_from_slice(&box_(*MDAT_ID, &[]));
        out
    }

    fn mp4_v1_fixture_with_extended_free_box() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&box_extended(*b"free", b"skip"));
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(
            *MOOV_ID,
            &moov_v1_box_payload(7_000_000_000, 7, u64::MAX, 96_000),
        ));
        out
    }

    fn mp4_without_track_header() -> Vec<u8> {
        let mut out = Vec::new();
        let moov_payload = [
            mvhd_v0(1_000, 5_000),
            box_(*TRAK_ID, &box_(*MDIA_ID, &mdhd_v0(1, 1))),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_mvhd_timescale(timescale: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let moov_payload = [
            mvhd_v0(timescale, 5_000),
            trak_v0(1, 5_000, 90_000, 450_000),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_track_id(track_id: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let moov_payload = [
            mvhd_v0(1_000, 5_000),
            trak_v0(track_id, 5_000, 90_000, 450_000),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_mvhd_version(version: u8) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(version);
        payload.extend_from_slice(&[0, 0, 0]);
        payload.extend_from_slice(&[0; 16]);

        let mut out = Vec::new();
        let moov_payload = [box_(*MVHD_ID, &payload), trak_v0(1, 5_000, 90_000, 450_000)].concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_movie_extends() -> Vec<u8> {
        let mut out = Vec::new();
        let moov_payload = [
            mvhd_v0(1_000, 5_000),
            box_(*MVEX_ID, &[]),
            trak_v0(1, 5_000, 90_000, 450_000),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_movie_fragment() -> Vec<u8> {
        let mut out = mp4_with_samples(false, &[b"aa".as_slice()], &[10]);
        out.extend_from_slice(&box_(*MOOF_ID, &box_(*b"traf", &[])));
        out
    }

    fn mp4_with_moov_extra_box(extra_box: Vec<u8>) -> Vec<u8> {
        let mut moov_payload = moov_v0_box_payload();
        moov_payload.extend_from_slice(&extra_box);
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn meta_box(ilst: Vec<u8>) -> Vec<u8> {
        box_(*META_ID, &full_box(0, &ilst))
    }

    fn ilst_box(items: &[Vec<u8>]) -> Vec<u8> {
        box_(*ILST_ID, &items.concat())
    }

    fn ilst_utf8_item(kind: [u8; 4], value: &str) -> Vec<u8> {
        let data = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_UTF8, value.as_bytes()),
        );
        box_(kind, &data)
    }

    fn ilst_utf16_be_item(kind: [u8; 4], value: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for unit in value.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        let data = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_UTF16, &bytes),
        );
        box_(kind, &data)
    }

    fn ilst_utf16_le_bom_item(kind: [u8; 4], value: &str) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xfe];
        for unit in value.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let data = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_UTF16, &bytes),
        );
        box_(kind, &data)
    }

    fn ilst_genre_index_item(one_based_index: u16) -> Vec<u8> {
        let data = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_RESERVED, &one_based_index.to_be_bytes()),
        );
        box_(*b"gnre", &data)
    }

    fn ilst_int8_item(kind: [u8; 4], value: u8) -> Vec<u8> {
        let data = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_SIGNED_INTEGER, &[value]),
        );
        box_(kind, &data)
    }

    fn ilst_padded_int8_item(kind: [u8; 4], value: u8) -> Vec<u8> {
        let data = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_SIGNED_INTEGER, &[0, 0, 0, value]),
        );
        box_(kind, &data)
    }

    fn ilst_freeform_item(mean: &str, name: &str, value: &str) -> Vec<u8> {
        box_(
            *FREEFORM_ID,
            &[
                freeform_text_child(*MEAN_ID, mean),
                freeform_text_child(*NAME_ID, name),
                freeform_data_child(value),
            ]
            .concat(),
        )
    }

    fn freeform_text_child(kind: [u8; 4], value: &str) -> Vec<u8> {
        box_(kind, &full_box(0, value.as_bytes()))
    }

    fn freeform_data_child(value: &str) -> Vec<u8> {
        box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_UTF8, value.as_bytes()),
        )
    }

    fn ilst_cover_art_item(data_type: u32, value: &[u8]) -> Vec<u8> {
        box_(
            *COVR_ID,
            &box_(*DATA_ID, &metadata_data_box_payload(data_type, value)),
        )
    }

    fn ilst_number_pair_item(kind: [u8; 4], current: u16, total: u16) -> Vec<u8> {
        let mut value = Vec::new();
        value.extend_from_slice(&0_u16.to_be_bytes());
        value.extend_from_slice(&current.to_be_bytes());
        value.extend_from_slice(&total.to_be_bytes());
        value.extend_from_slice(&0_u16.to_be_bytes());
        let data = box_(
            *DATA_ID,
            &metadata_data_box_payload(METADATA_DATA_TYPE_RESERVED, &value),
        );
        box_(kind, &data)
    }

    fn metadata_data_box_payload(data_type: u32, value: &[u8]) -> Vec<u8> {
        let flags = data_type.to_be_bytes();
        let mut out = Vec::new();
        out.push(0);
        out.extend_from_slice(&flags[1..]);
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(value);
        out
    }

    fn mp4_with_edit_list() -> Vec<u8> {
        let mut edit_entry = Vec::new();
        edit_entry.extend_from_slice(&1_u32.to_be_bytes());
        edit_entry.extend_from_slice(&1_000_u32.to_be_bytes());
        edit_entry.extend_from_slice(&0_i32.to_be_bytes());
        edit_entry.extend_from_slice(&1_i16.to_be_bytes());
        edit_entry.extend_from_slice(&0_u16.to_be_bytes());
        let edts = box_(*EDTS_ID, &box_(*b"elst", &full_box(0, &edit_entry)));
        let mdia = box_(*MDIA_ID, &mdhd_v0(90_000, 450_000));
        let trak = box_(
            *TRAK_ID,
            &[tkhd_v0(1, 5_000, 1_920, 1_080), edts, mdia].concat(),
        );
        let moov_payload = [mvhd_v0(1_000, 5_000), trak].concat();

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_two_populated_tracks() -> Vec<u8> {
        let mut out = Vec::new();
        let sample_sizes = [2];
        let durations = [1_000];
        let moov_payload = [
            mvhd_v0(1_000, 1_000),
            trak_v0_with_stbl(
                1,
                1_000,
                90_000,
                1_000,
                0,
                &sample_sizes,
                &durations,
                false,
                1,
                None,
                None,
            ),
            trak_v0_with_stbl(
                2,
                1_000,
                48_000,
                1_000,
                0,
                &sample_sizes,
                &durations,
                false,
                1,
                None,
                None,
            ),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_samples(use_co64: bool, samples: &[&[u8]], durations: &[u32]) -> Vec<u8> {
        mp4_with_sample_tables(use_co64, samples, durations, samples, 1, None, None)
    }

    fn mp4_with_samples_and_sync(
        use_co64: bool,
        samples: &[&[u8]],
        durations: &[u32],
        sync_sample_numbers: &[u32],
    ) -> Vec<u8> {
        mp4_with_sample_tables(
            use_co64,
            samples,
            durations,
            samples,
            1,
            Some(sync_sample_numbers),
            None,
        )
    }

    fn mp4_with_samples_and_ctts(
        use_co64: bool,
        samples: &[&[u8]],
        durations: &[u32],
        ctts_version: u8,
        composition_entries: &[(u32, i32)],
    ) -> Vec<u8> {
        mp4_with_sample_tables(
            use_co64,
            samples,
            durations,
            samples,
            1,
            None,
            Some((ctts_version, composition_entries)),
        )
    }

    fn mp4_with_chunk_layout(
        use_co64: bool,
        samples: &[&[u8]],
        durations: &[u32],
        stsc_entries: &[(u32, u32, u32)],
        chunk_sample_counts: &[u32],
        chunk_gaps: &[&[u8]],
    ) -> Vec<u8> {
        let ftyp = ftyp_box();
        let declared_sizes = samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let placeholder_offsets = vec![0; chunk_sample_counts.len()];
        let placeholder_moov = box_(
            *MOOV_ID,
            &moov_v0_with_chunk_layout_payload(
                &placeholder_offsets,
                &declared_sizes,
                durations,
                use_co64,
                stsc_entries,
            ),
        );
        let mdat_start = ftyp.len() + placeholder_moov.len() + 8;
        let (chunk_offsets, mdat_payload) =
            chunked_mdat_payload(samples, chunk_sample_counts, chunk_gaps, mdat_start);
        let moov = box_(
            *MOOV_ID,
            &moov_v0_with_chunk_layout_payload(
                &chunk_offsets,
                &declared_sizes,
                durations,
                use_co64,
                stsc_entries,
            ),
        );

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&moov);
        out.extend_from_slice(&box_(*MDAT_ID, &mdat_payload));
        out
    }

    fn mp4_with_multiple_mdat_ranges() -> Vec<u8> {
        let ftyp = ftyp_box();
        let samples = [b"aa".as_slice(), b"bbb".as_slice()];
        let durations = [10, 11];
        let declared_sizes = samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let stsc_entries = [(1, 1, 1)];
        let placeholder_offsets = [0, 0];
        let placeholder_moov = box_(
            *MOOV_ID,
            &moov_v0_with_chunk_layout_payload(
                &placeholder_offsets,
                &declared_sizes,
                &durations,
                false,
                &stsc_entries,
            ),
        );
        let first_mdat = box_(*MDAT_ID, samples[0]);
        let separator = box_(*b"free", b"skip");
        let first_offset = u64::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let second_offset = u64::try_from(
            ftyp.len() + placeholder_moov.len() + first_mdat.len() + separator.len() + 8,
        )
        .unwrap();
        let moov = box_(
            *MOOV_ID,
            &moov_v0_with_chunk_layout_payload(
                &[first_offset, second_offset],
                &declared_sizes,
                &durations,
                false,
                &stsc_entries,
            ),
        );
        let second_mdat = box_(*MDAT_ID, samples[1]);

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&moov);
        out.extend_from_slice(&first_mdat);
        out.extend_from_slice(&separator);
        out.extend_from_slice(&second_mdat);
        out
    }

    fn mp4_with_mismatched_sample_counts() -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"aa".as_slice(), b"bb".as_slice()],
            &[1_000],
            &[b"aa".as_slice(), b"bb".as_slice()],
            1,
            None,
            None,
        )
    }

    fn mp4_with_sample_description_index(sample_description_index: u32) -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"aa".as_slice()],
            &[1_000],
            &[b"aa".as_slice()],
            sample_description_index,
            None,
            None,
        )
    }

    fn mp4_with_sample_description_entry(
        codec_tag: &[u8; 4],
        data_reference_index: u16,
        extra_data: &[u8],
    ) -> Vec<u8> {
        let ftyp = ftyp_box();
        let sample_sizes = [2];
        let durations = [1_000];
        let sample_description = stsd_box_with_entry(codec_tag, data_reference_index, extra_data);
        let placeholder_moov_payload = moov_v0_with_custom_stsd_payload(
            0,
            &sample_sizes,
            &durations,
            sample_description.clone(),
        );
        let placeholder_moov = box_(*MOOV_ID, &placeholder_moov_payload);
        let chunk_offset = u64::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let moov = moov_v0_with_custom_stsd_payload(
            chunk_offset,
            &sample_sizes,
            &durations,
            sample_description,
        );

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&box_(*MOOV_ID, &moov));
        out.extend_from_slice(&box_(*MDAT_ID, b"aa"));
        out
    }

    fn mp4_with_multiple_sample_description_entries() -> Vec<u8> {
        let moov =
            moov_v0_with_custom_stsd_payload(0, &[2], &[1_000], stsd_box_with_multiple_entries());
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov));
        out
    }

    fn mp4_with_short_mdat_payload() -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"abcd".as_slice()],
            &[1_000],
            &[b"abc".as_slice()],
            1,
            None,
            None,
        )
    }

    fn mp4_with_sync_samples(sync_sample_numbers: &[u32]) -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"aa".as_slice(), b"bb".as_slice()],
            &[1_000, 1_000],
            &[b"aa".as_slice(), b"bb".as_slice()],
            1,
            Some(sync_sample_numbers),
            None,
        )
    }

    fn mp4_with_composition_offsets(
        ctts_version: u8,
        composition_entries: &[(u32, i32)],
    ) -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"aa".as_slice(), b"bb".as_slice()],
            &[1_000, 1_000],
            &[b"aa".as_slice(), b"bb".as_slice()],
            1,
            None,
            Some((ctts_version, composition_entries)),
        )
    }

    fn mp4_with_sample_tables(
        use_co64: bool,
        declared_samples: &[&[u8]],
        durations: &[u32],
        mdat_samples: &[&[u8]],
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        let ftyp = ftyp_box();
        let mdat_payload = mdat_samples.concat();
        let declared_sizes = declared_samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let placeholder_moov = box_(
            *MOOV_ID,
            &moov_v0_with_stbl_payload(
                0,
                &declared_sizes,
                durations,
                use_co64,
                sample_description_index,
                sync_sample_numbers,
                composition_offsets,
            ),
        );
        let chunk_offset = u64::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let moov = box_(
            *MOOV_ID,
            &moov_v0_with_stbl_payload(
                chunk_offset,
                &declared_sizes,
                durations,
                use_co64,
                sample_description_index,
                sync_sample_numbers,
                composition_offsets,
            ),
        );

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&moov);
        out.extend_from_slice(&box_(*MDAT_ID, &mdat_payload));
        out
    }

    fn moov_v0_box_payload() -> Vec<u8> {
        [mvhd_v0(1_000, 5_000), trak_v0(1, 5_000, 90_000, 450_000)].concat()
    }

    fn moov_v0_with_stbl_payload(
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        [
            mvhd_v0(1_000, media_duration),
            trak_v0_with_stbl(
                1,
                media_duration,
                90_000,
                media_duration,
                chunk_offset,
                sample_sizes,
                durations,
                use_co64,
                sample_description_index,
                sync_sample_numbers,
                composition_offsets,
            ),
        ]
        .concat()
    }

    fn moov_v0_with_custom_stsd_payload(
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        sample_description_box: Vec<u8>,
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        let stbl = stbl_box_with_stsd(
            sample_description_box,
            chunk_offset,
            sample_sizes,
            durations,
            false,
            1,
            None,
            None,
        );
        let minf = box_(*MINF_ID, &stbl);
        let mdia = box_(*MDIA_ID, &[mdhd_v0(90_000, media_duration), minf].concat());
        let trak = box_(
            *TRAK_ID,
            &[tkhd_v0(1, media_duration, 1_920, 1_080), mdia].concat(),
        );
        [mvhd_v0(1_000, media_duration), trak].concat()
    }

    fn moov_v0_with_chunk_layout_payload(
        chunk_offsets: &[u64],
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        stsc_entries: &[(u32, u32, u32)],
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        [
            mvhd_v0(1_000, media_duration),
            trak_v0_with_chunk_layout(
                chunk_offsets,
                sample_sizes,
                durations,
                use_co64,
                stsc_entries,
            ),
        ]
        .concat()
    }

    fn moov_v1_box_payload(
        movie_duration: u64,
        track_id: u32,
        track_duration: u64,
        media_duration: u64,
    ) -> Vec<u8> {
        [
            mvhd_v1(1_000, movie_duration),
            trak_v1(track_id, track_duration, 48_000, media_duration),
        ]
        .concat()
    }

    fn trak_v0(track_id: u32, track_duration: u32, timescale: u32, media_duration: u32) -> Vec<u8> {
        let payload = [
            tkhd_v0(track_id, track_duration, 1_920, 1_080),
            box_(*MDIA_ID, &mdhd_v0(timescale, media_duration)),
        ]
        .concat();
        box_(*TRAK_ID, &payload)
    }

    fn trak_v0_with_chunk_layout(
        chunk_offsets: &[u64],
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        stsc_entries: &[(u32, u32, u32)],
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        let stbl = stbl_box_with_chunk_layout(
            chunk_offsets,
            sample_sizes,
            durations,
            use_co64,
            stsc_entries,
        );
        let minf = box_(*MINF_ID, &stbl);
        let mdia = box_(*MDIA_ID, &[mdhd_v0(90_000, media_duration), minf].concat());
        let payload = [tkhd_v0(1, media_duration, 1_920, 1_080), mdia].concat();
        box_(*TRAK_ID, &payload)
    }

    #[allow(clippy::too_many_arguments)]
    fn trak_v0_with_stbl(
        track_id: u32,
        track_duration: u32,
        timescale: u32,
        media_duration: u32,
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        let stbl = stbl_box(
            chunk_offset,
            sample_sizes,
            durations,
            use_co64,
            sample_description_index,
            sync_sample_numbers,
            composition_offsets,
        );
        let minf = box_(*MINF_ID, &stbl);
        let mdia = box_(
            *MDIA_ID,
            &[mdhd_v0(timescale, media_duration), minf].concat(),
        );
        let payload = [tkhd_v0(track_id, track_duration, 1_920, 1_080), mdia].concat();
        box_(*TRAK_ID, &payload)
    }

    fn trak_v1(track_id: u32, track_duration: u64, timescale: u32, media_duration: u64) -> Vec<u8> {
        let payload = [
            tkhd_v1(track_id, track_duration, 3_840, 2_160),
            box_(*MDIA_ID, &mdhd_v1(timescale, media_duration)),
        ]
        .concat();
        box_(*TRAK_ID, &payload)
    }

    fn mp4_with_media_handler_name(handler_name: &[u8]) -> Vec<u8> {
        mp4_with_media_handler(*b"vide", handler_name)
    }

    fn mp4_with_media_handler(handler_type: [u8; 4], handler_name: &[u8]) -> Vec<u8> {
        let mdia = box_(
            *MDIA_ID,
            &[
                mdhd_v0(90_000, 450_000),
                hdlr_box(handler_type, handler_name),
            ]
            .concat(),
        );
        let moov = box_(
            *MOOV_ID,
            &[
                mvhd_v0(1_000, 5_000),
                box_(*TRAK_ID, &[tkhd_v0(1, 5_000, 1_920, 1_080), mdia].concat()),
            ]
            .concat(),
        );
        [ftyp_box(), moov, box_(*MDAT_ID, &[])].concat()
    }

    fn ftyp_box() -> Vec<u8> {
        box_(*FTYP_ID, &ftyp_payload())
    }

    fn ftyp_payload() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"isom");
        out.extend_from_slice(&512_u32.to_be_bytes());
        out.extend_from_slice(b"isom");
        out.extend_from_slice(b"iso2");
        out.extend_from_slice(b"avc1");
        out
    }

    fn stbl_box(
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        stbl_box_with_stsd(
            stsd_box(),
            chunk_offset,
            sample_sizes,
            durations,
            use_co64,
            sample_description_index,
            sync_sample_numbers,
            composition_offsets,
        )
    }

    fn mp4_with_media_language(language: &str) -> Vec<u8> {
        let mdia = box_(*MDIA_ID, &mdhd_v0_with_language(90_000, 450_000, language));
        let moov = box_(
            *MOOV_ID,
            &[
                mvhd_v0(1_000, 5_000),
                box_(*TRAK_ID, &[tkhd_v0(1, 5_000, 1_920, 1_080), mdia].concat()),
            ]
            .concat(),
        );
        [ftyp_box(), moov].concat()
    }

    fn mp4_with_truncated_mdhd() -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&90_000_u32.to_be_bytes());
        body.extend_from_slice(&450_000_u32.to_be_bytes());
        let mdia = box_(*MDIA_ID, &box_(*MDHD_ID, &full_box(0, &body)));
        let moov = box_(
            *MOOV_ID,
            &[
                mvhd_v0(1_000, 5_000),
                box_(*TRAK_ID, &[tkhd_v0(1, 5_000, 1_920, 1_080), mdia].concat()),
            ]
            .concat(),
        );
        [ftyp_box(), moov].concat()
    }

    #[allow(clippy::too_many_arguments)]
    fn stbl_box_with_stsd(
        sample_description_box: Vec<u8>,
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        let chunk_offset_box = if use_co64 {
            co64_box(chunk_offset)
        } else {
            stco_box(u32::try_from(chunk_offset).unwrap())
        };
        let mut boxes = vec![
            sample_description_box,
            stts_box(durations),
            stsc_box(
                u32::try_from(sample_sizes.len()).unwrap(),
                sample_description_index,
            ),
            stsz_box(sample_sizes),
        ];
        if let Some((version, composition_entries)) = composition_offsets {
            boxes.push(ctts_box(version, composition_entries));
        }
        if let Some(sync_sample_numbers) = sync_sample_numbers {
            boxes.push(stss_box(sync_sample_numbers));
        }
        boxes.push(chunk_offset_box);
        box_(*STBL_ID, &boxes.concat())
    }

    fn stbl_box_with_chunk_layout(
        chunk_offsets: &[u64],
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        stsc_entries: &[(u32, u32, u32)],
    ) -> Vec<u8> {
        let chunk_offset_box = if use_co64 {
            co64_offsets_box(chunk_offsets)
        } else {
            stco_offsets_box(chunk_offsets)
        };
        box_(
            *STBL_ID,
            &[
                stsd_box(),
                stts_box(durations),
                stsc_entries_box(stsc_entries),
                stsz_box(sample_sizes),
                chunk_offset_box,
            ]
            .concat(),
        )
    }

    fn stsd_box() -> Vec<u8> {
        stsd_box_with_entry(b"raw ", 1, &[])
    }

    fn stsd_box_with_entry(
        codec_tag: &[u8; 4],
        data_reference_index: u16,
        extra_data: &[u8],
    ) -> Vec<u8> {
        let sample_entry = sample_entry(codec_tag, data_reference_index, extra_data);
        let mut body = Vec::new();
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&sample_entry);
        box_(*STSD_ID, &full_box(0, &body))
    }

    fn stsd_box_with_multiple_entries() -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&2_u32.to_be_bytes());
        body.extend_from_slice(&sample_entry(b"raw ", 1, &[]));
        body.extend_from_slice(&sample_entry(b"avc1", 1, b"\x01\x64"));
        box_(*STSD_ID, &full_box(0, &body))
    }

    fn sample_entry(codec_tag: &[u8; 4], data_reference_index: u16, extra_data: &[u8]) -> Vec<u8> {
        let mut sample_entry = Vec::new();
        let entry_size = u32::try_from(16 + extra_data.len()).unwrap();
        sample_entry.extend_from_slice(&entry_size.to_be_bytes());
        sample_entry.extend_from_slice(codec_tag);
        sample_entry.extend_from_slice(&[0; 6]);
        sample_entry.extend_from_slice(&data_reference_index.to_be_bytes());
        sample_entry.extend_from_slice(extra_data);
        sample_entry
    }

    fn tx3g_sample_entry_extra_data(child_boxes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0x0000_0200_u32.to_be_bytes());
        out.push((-1_i8) as u8);
        out.push(1);
        out.extend_from_slice(&[1, 2, 3, 4]);
        for value in [-10_i16, -20, 30, 40] {
            out.extend_from_slice(&value.to_be_bytes());
        }
        out.extend_from_slice(&0_u16.to_be_bytes());
        out.extend_from_slice(&5_u16.to_be_bytes());
        out.extend_from_slice(&2_u16.to_be_bytes());
        out.push(1);
        out.push(18);
        out.extend_from_slice(&[10, 20, 30, 255]);
        out.extend_from_slice(child_boxes);
        out
    }

    fn xml_metadata_sample_entry_extra_data(
        content_encoding: &str,
        namespace: &str,
        schema_location: &str,
    ) -> Vec<u8> {
        [
            content_encoding.as_bytes(),
            b"\0",
            namespace.as_bytes(),
            b"\0",
            schema_location.as_bytes(),
            b"\0",
        ]
        .concat()
    }

    fn text_metadata_sample_entry_extra_data(content_encoding: &str, mime_format: &str) -> Vec<u8> {
        [
            content_encoding.as_bytes(),
            b"\0",
            mime_format.as_bytes(),
            b"\0",
        ]
        .concat()
    }

    fn audio_channel_layout_payload(
        channel_layout_tag: u32,
        channel_bitmap: u32,
        descriptions: &[(u32, u32, [f32; 3])],
    ) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&channel_layout_tag.to_be_bytes());
        body.extend_from_slice(&channel_bitmap.to_be_bytes());
        body.extend_from_slice(&(u32::try_from(descriptions.len()).unwrap()).to_be_bytes());
        for (channel_label, channel_flags, coordinates) in descriptions {
            body.extend_from_slice(&channel_label.to_be_bytes());
            body.extend_from_slice(&channel_flags.to_be_bytes());
            for coordinate in coordinates {
                body.extend_from_slice(&coordinate.to_bits().to_be_bytes());
            }
        }
        full_box(0, &body)
    }

    fn bit_rate_box_payload(buffer_size_db: u32, max_bitrate: u32, avg_bitrate: u32) -> Vec<u8> {
        [
            buffer_size_db.to_be_bytes(),
            max_bitrate.to_be_bytes(),
            avg_bitrate.to_be_bytes(),
        ]
        .concat()
    }

    fn amr_specific_box_payload(
        vendor: [u8; 4],
        decoder_version: u8,
        mode_set: u16,
        mode_change_period: u8,
        frames_per_sample: u8,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&vendor);
        payload.push(decoder_version);
        payload.extend_from_slice(&mode_set.to_be_bytes());
        payload.push(mode_change_period);
        payload.push(frames_per_sample);
        payload
    }

    fn ac3_specific_box_payload(
        fscod: u8,
        bsid: u8,
        bsmod: u8,
        acmod: u8,
        lfeon: bool,
        bit_rate_code: u8,
        reserved: u8,
    ) -> Vec<u8> {
        let bits = (u32::from(fscod & 0x03) << 22)
            | (u32::from(bsid & 0x1f) << 17)
            | (u32::from(bsmod & 0x07) << 14)
            | (u32::from(acmod & 0x07) << 11)
            | (u32::from(u8::from(lfeon)) << 10)
            | (u32::from(bit_rate_code & 0x1f) << 5)
            | u32::from(reserved & 0x1f);
        vec![
            ((bits >> 16) & 0xff) as u8,
            ((bits >> 8) & 0xff) as u8,
            (bits & 0xff) as u8,
        ]
    }

    #[derive(Clone, Copy)]
    struct Ec3SpecificSubstreamPayload {
        fscod: u8,
        bsid: u8,
        reserved: bool,
        asvc: bool,
        bsmod: u8,
        acmod: u8,
        lfeon: bool,
        reserved3: u8,
        num_dep_sub: u8,
        chan_loc: Option<u16>,
        no_dep_reserved: bool,
    }

    #[allow(clippy::too_many_arguments)]
    fn ec3_substream_payload(
        fscod: u8,
        bsid: u8,
        asvc: bool,
        bsmod: u8,
        acmod: u8,
        lfeon: bool,
        num_dep_sub: u8,
        chan_loc: Option<u16>,
    ) -> Ec3SpecificSubstreamPayload {
        ec3_substream_payload_with_reserved_bits(
            fscod,
            bsid,
            false,
            asvc,
            bsmod,
            acmod,
            lfeon,
            0,
            num_dep_sub,
            chan_loc,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn ec3_substream_payload_with_reserved_bits(
        fscod: u8,
        bsid: u8,
        reserved: bool,
        asvc: bool,
        bsmod: u8,
        acmod: u8,
        lfeon: bool,
        reserved3: u8,
        num_dep_sub: u8,
        chan_loc: Option<u16>,
        no_dep_reserved: bool,
    ) -> Ec3SpecificSubstreamPayload {
        Ec3SpecificSubstreamPayload {
            fscod,
            bsid,
            reserved,
            asvc,
            bsmod,
            acmod,
            lfeon,
            reserved3,
            num_dep_sub,
            chan_loc,
            no_dep_reserved,
        }
    }

    fn ec3_specific_box_payload(
        data_rate: u16,
        num_ind_sub: u8,
        substreams: &[Ec3SpecificSubstreamPayload],
        trailing_reserved_bytes: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut bit_len = 0_usize;
        append_bits(&mut out, &mut bit_len, u64::from(data_rate & 0x1fff), 13);
        append_bits(&mut out, &mut bit_len, u64::from(num_ind_sub & 0x07), 3);
        for substream in substreams {
            append_bits(&mut out, &mut bit_len, u64::from(substream.fscod & 0x03), 2);
            append_bits(&mut out, &mut bit_len, u64::from(substream.bsid & 0x1f), 5);
            append_bits(
                &mut out,
                &mut bit_len,
                u64::from(u8::from(substream.reserved)),
                1,
            );
            append_bits(
                &mut out,
                &mut bit_len,
                u64::from(u8::from(substream.asvc)),
                1,
            );
            append_bits(&mut out, &mut bit_len, u64::from(substream.bsmod & 0x07), 3);
            append_bits(&mut out, &mut bit_len, u64::from(substream.acmod & 0x07), 3);
            append_bits(
                &mut out,
                &mut bit_len,
                u64::from(u8::from(substream.lfeon)),
                1,
            );
            append_bits(
                &mut out,
                &mut bit_len,
                u64::from(substream.reserved3 & 0x07),
                3,
            );
            append_bits(
                &mut out,
                &mut bit_len,
                u64::from(substream.num_dep_sub & 0x0f),
                4,
            );
            if substream.num_dep_sub > 0 {
                if let Some(chan_loc) = substream.chan_loc {
                    append_bits(&mut out, &mut bit_len, u64::from(chan_loc & 0x01ff), 9);
                }
            } else {
                append_bits(
                    &mut out,
                    &mut bit_len,
                    u64::from(u8::from(substream.no_dep_reserved)),
                    1,
                );
            }
        }
        out.extend_from_slice(trailing_reserved_bytes);
        out
    }

    fn append_bits(out: &mut Vec<u8>, bit_len: &mut usize, value: u64, count: u8) {
        for shift in (0..count).rev() {
            if *bit_len % 8 == 0 {
                out.push(0);
            }
            if ((value >> shift) & 1) != 0 {
                let bit_offset = *bit_len % 8;
                *out.last_mut().unwrap() |= 1 << (7 - bit_offset);
            }
            *bit_len += 1;
        }
    }

    fn opus_specific_box_payload(
        output_channel_count: u8,
        pre_skip: u16,
        input_sample_rate: u32,
        output_gain: i16,
        channel_mapping_family: u8,
        mapping_table: Option<(u8, u8, &[u8])>,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(0);
        payload.push(output_channel_count);
        payload.extend_from_slice(&pre_skip.to_be_bytes());
        payload.extend_from_slice(&input_sample_rate.to_be_bytes());
        payload.extend_from_slice(&output_gain.to_be_bytes());
        payload.push(channel_mapping_family);
        if let Some((stream_count, coupled_count, channel_mapping)) = mapping_table {
            payload.push(stream_count);
            payload.push(coupled_count);
            payload.extend_from_slice(channel_mapping);
        }
        payload
    }

    #[allow(clippy::too_many_arguments)]
    fn alac_specific_box_payload(
        frame_length: u32,
        compatible_version: u8,
        bit_depth: u8,
        pb: u8,
        mb: u8,
        kb: u8,
        num_channels: u8,
        max_run: u16,
        max_frame_bytes: u32,
        avg_bit_rate: u32,
        sample_rate: u32,
        channel_layout_tag: Option<u32>,
    ) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&frame_length.to_be_bytes());
        body.push(compatible_version);
        body.push(bit_depth);
        body.push(pb);
        body.push(mb);
        body.push(kb);
        body.push(num_channels);
        body.extend_from_slice(&max_run.to_be_bytes());
        body.extend_from_slice(&max_frame_bytes.to_be_bytes());
        body.extend_from_slice(&avg_bit_rate.to_be_bytes());
        body.extend_from_slice(&sample_rate.to_be_bytes());
        if let Some(channel_layout_tag) = channel_layout_tag {
            body.extend_from_slice(&alac_channel_layout_info_payload(
                channel_layout_tag,
                [0, 0],
                *CHAN_ID,
                0,
                [0, 0, 0],
                24,
            ));
        }
        full_box(0, &body)
    }

    fn alac_channel_layout_info_payload(
        channel_layout_tag: u32,
        reserved: [u32; 2],
        atom_type: [u8; 4],
        version: u8,
        flags: [u8; 3],
        declared_size: u32,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&declared_size.to_be_bytes());
        payload.extend_from_slice(&atom_type);
        payload.push(version);
        payload.extend_from_slice(&flags);
        payload.extend_from_slice(&channel_layout_tag.to_be_bytes());
        payload.extend_from_slice(&reserved[0].to_be_bytes());
        payload.extend_from_slice(&reserved[1].to_be_bytes());
        payload
    }

    fn flac_specific_box_payload(blocks: &[&[u8]]) -> Vec<u8> {
        let mut payload = full_box(0, &[]);
        for block in blocks {
            payload.extend_from_slice(block);
        }
        payload
    }

    fn flac_metadata_block(last: bool, block_type: u8, data: &[u8]) -> Vec<u8> {
        let length = u32::try_from(data.len()).unwrap();
        assert!(length <= 0x00ff_ffff);
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.push(if last { 0x80 } else { 0 } | (block_type & 0x7f));
        payload.push(((length >> 16) & 0xff) as u8);
        payload.push(((length >> 8) & 0xff) as u8);
        payload.push((length & 0xff) as u8);
        payload.extend_from_slice(data);
        payload
    }

    fn flac_streaminfo_block_data(
        sample_rate: u32,
        channels: u8,
        bits_per_sample: u8,
        total_samples: u64,
    ) -> Vec<u8> {
        assert!((1..=8).contains(&channels));
        assert!((1..=32).contains(&bits_per_sample));
        let mut payload = Vec::new();
        payload.extend_from_slice(&16_u16.to_be_bytes());
        payload.extend_from_slice(&4_096_u16.to_be_bytes());
        payload.extend_from_slice(&[0, 0, 0]);
        payload.extend_from_slice(&[0, 0, 0]);
        let mut packed = Vec::new();
        let mut bit_len = 0_usize;
        append_bits(
            &mut packed,
            &mut bit_len,
            u64::from(sample_rate & 0x000f_ffff),
            20,
        );
        append_bits(
            &mut packed,
            &mut bit_len,
            u64::from((channels - 1) & 0x07),
            3,
        );
        append_bits(
            &mut packed,
            &mut bit_len,
            u64::from((bits_per_sample - 1) & 0x1f),
            5,
        );
        append_bits(
            &mut packed,
            &mut bit_len,
            total_samples & ((1_u64 << 36) - 1),
            36,
        );
        assert_eq!(packed.len(), 8);
        payload.extend_from_slice(&packed);
        payload.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        payload
    }

    fn wave_extension_payload(original_format: [u8; 4], esds_descriptor: &[u8]) -> Vec<u8> {
        [
            box_(*FRMA_ID, &original_format),
            box_(*ESDS_ID, &full_box(0, esds_descriptor)),
            box_(*TERMINATOR_ID, &[]),
        ]
        .concat()
    }

    fn audio_sample_entry_extra_data(
        channel_count: u16,
        sample_size: u16,
        sample_rate: u32,
        child_boxes: &[u8],
    ) -> Vec<u8> {
        let mut out = audio_sample_entry_base_extra_data(
            0,
            channel_count,
            sample_size,
            0,
            0,
            sample_rate << 16,
        );
        out.extend_from_slice(child_boxes);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn audio_sample_entry_v1_extra_data(
        channel_count: u16,
        sample_size: u16,
        sample_rate: u32,
        samples_per_packet: u32,
        bytes_per_packet: u32,
        bytes_per_frame: u32,
        bytes_per_sample: u32,
        child_boxes: &[u8],
    ) -> Vec<u8> {
        let mut out = audio_sample_entry_base_extra_data(
            1,
            channel_count,
            sample_size,
            0,
            0,
            sample_rate << 16,
        );
        out.extend_from_slice(&samples_per_packet.to_be_bytes());
        out.extend_from_slice(&bytes_per_packet.to_be_bytes());
        out.extend_from_slice(&bytes_per_frame.to_be_bytes());
        out.extend_from_slice(&bytes_per_sample.to_be_bytes());
        out.extend_from_slice(child_boxes);
        out
    }

    fn audio_sample_entry_v2_extra_data(
        audio_sample_rate: f64,
        num_audio_channels: u32,
        const_bits_per_channel: u32,
        format_specific_flags: u32,
        const_bytes_per_audio_packet: u32,
        const_lpcm_frames_per_audio_packet: u32,
        child_boxes: &[u8],
    ) -> Vec<u8> {
        let mut out = audio_sample_entry_base_extra_data(2, 3, 16, -2, 0, 65_536);
        out.extend_from_slice(&72_u32.to_be_bytes());
        out.extend_from_slice(&audio_sample_rate.to_bits().to_be_bytes());
        out.extend_from_slice(&num_audio_channels.to_be_bytes());
        out.extend_from_slice(&0x7f00_0000_u32.to_be_bytes());
        out.extend_from_slice(&const_bits_per_channel.to_be_bytes());
        out.extend_from_slice(&format_specific_flags.to_be_bytes());
        out.extend_from_slice(&const_bytes_per_audio_packet.to_be_bytes());
        out.extend_from_slice(&const_lpcm_frames_per_audio_packet.to_be_bytes());
        out.extend_from_slice(child_boxes);
        out
    }

    fn audio_sample_entry_base_extra_data(
        version: u16,
        channel_count: u16,
        sample_size: u16,
        compression_id: i16,
        packet_size: u16,
        sample_rate_fixed_16_16: u32,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&version.to_be_bytes());
        out.extend_from_slice(&0_u16.to_be_bytes());
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(&channel_count.to_be_bytes());
        out.extend_from_slice(&sample_size.to_be_bytes());
        out.extend_from_slice(&compression_id.to_be_bytes());
        out.extend_from_slice(&packet_size.to_be_bytes());
        out.extend_from_slice(&sample_rate_fixed_16_16.to_be_bytes());
        out
    }

    fn visual_sample_entry_extra_data(
        width: u16,
        height: u16,
        compressor_name: &str,
        depth: u16,
        child_boxes: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[0; 16]);
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&0x0048_0000_u32.to_be_bytes());
        out.extend_from_slice(&0x0048_0000_u32.to_be_bytes());
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(&1_u16.to_be_bytes());

        let name_bytes = compressor_name.as_bytes();
        let name_len = name_bytes.len().min(31);
        out.push(u8::try_from(name_len).unwrap());
        out.extend_from_slice(&name_bytes[..name_len]);
        out.resize(out.len() + (31 - name_len), 0);

        out.extend_from_slice(&depth.to_be_bytes());
        out.extend_from_slice(&u16::MAX.to_be_bytes());
        out.extend_from_slice(child_boxes);
        out
    }

    fn avcc_payload(
        profile_indication: u8,
        profile_compatibility: u8,
        level_indication: u8,
        nal_length_size: u8,
        sequence_parameter_sets: &[&[u8]],
        picture_parameter_sets: &[&[u8]],
    ) -> Vec<u8> {
        let mut out = vec![
            1,
            profile_indication,
            profile_compatibility,
            level_indication,
            0b1111_1100 | (nal_length_size - 1),
            0b1110_0000 | u8::try_from(sequence_parameter_sets.len()).unwrap(),
        ];
        for parameter_set in sequence_parameter_sets {
            out.extend_from_slice(&u16::try_from(parameter_set.len()).unwrap().to_be_bytes());
            out.extend_from_slice(parameter_set);
        }
        out.push(u8::try_from(picture_parameter_sets.len()).unwrap());
        for parameter_set in picture_parameter_sets {
            out.extend_from_slice(&u16::try_from(parameter_set.len()).unwrap().to_be_bytes());
            out.extend_from_slice(parameter_set);
        }
        out
    }

    fn hvcc_payload(
        video_parameter_sets: &[&[u8]],
        sequence_parameter_sets: &[&[u8]],
        picture_parameter_sets: &[&[u8]],
    ) -> Vec<u8> {
        let mut out = vec![1, 1];
        out.extend_from_slice(&0x6000_0000_u32.to_be_bytes());
        out.extend_from_slice(&0x90_u64.to_be_bytes()[2..]);
        out.push(120);
        out.extend_from_slice(&0xf000_u16.to_be_bytes());
        out.push(0xfc);
        out.push(0xfd);
        out.push(0xf8);
        out.push(0xf8);
        out.extend_from_slice(&0_u16.to_be_bytes());
        out.push(0x0f);
        out.push(3);

        for (nal_unit_type, nal_units) in [
            (32_u8, video_parameter_sets),
            (33_u8, sequence_parameter_sets),
            (34_u8, picture_parameter_sets),
        ] {
            out.push(0x80 | nal_unit_type);
            out.extend_from_slice(&u16::try_from(nal_units.len()).unwrap().to_be_bytes());
            for nal_unit in nal_units {
                out.extend_from_slice(&u16::try_from(nal_unit.len()).unwrap().to_be_bytes());
                out.extend_from_slice(nal_unit);
            }
        }
        out
    }

    fn pasp_box(horizontal_spacing: u32, vertical_spacing: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&horizontal_spacing.to_be_bytes());
        body.extend_from_slice(&vertical_spacing.to_be_bytes());
        box_(*PASP_ID, &body)
    }

    fn colr_nclx_box(
        color_primaries: u16,
        transfer_characteristics: u16,
        matrix_coefficients: u16,
        full_range: bool,
    ) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(NCLX_ID);
        body.extend_from_slice(&color_primaries.to_be_bytes());
        body.extend_from_slice(&transfer_characteristics.to_be_bytes());
        body.extend_from_slice(&matrix_coefficients.to_be_bytes());
        body.push(if full_range { 0x80 } else { 0 });
        box_(*COLR_ID, &body)
    }

    fn colr_nclc_box(
        color_primaries: u16,
        transfer_characteristics: u16,
        matrix_coefficients: u16,
    ) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(NCLC_ID);
        body.extend_from_slice(&color_primaries.to_be_bytes());
        body.extend_from_slice(&transfer_characteristics.to_be_bytes());
        body.extend_from_slice(&matrix_coefficients.to_be_bytes());
        box_(*COLR_ID, &body)
    }

    fn colr_icc_profile_box(color_type: [u8; 4], profile: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&color_type);
        body.extend_from_slice(profile);
        box_(*COLR_ID, &body)
    }

    fn stts_box(durations: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(durations.len()).unwrap()).to_be_bytes());
        for duration in durations {
            body.extend_from_slice(&1_u32.to_be_bytes());
            body.extend_from_slice(&duration.to_be_bytes());
        }
        box_(*STTS_ID, &full_box(0, &body))
    }

    fn ctts_box(version: u8, composition_entries: &[(u32, i32)]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(composition_entries.len()).unwrap()).to_be_bytes());
        for (sample_count, sample_offset) in composition_entries {
            body.extend_from_slice(&sample_count.to_be_bytes());
            if version == 0 {
                body.extend_from_slice(
                    &u32::try_from(*sample_offset)
                        .expect("version 0 ctts fixture offsets must be non-negative")
                        .to_be_bytes(),
                );
            } else {
                body.extend_from_slice(&sample_offset.to_be_bytes());
            }
        }
        box_(*CTTS_ID, &full_box(version, &body))
    }

    fn stsc_box(samples_per_chunk: u32, sample_description_index: u32) -> Vec<u8> {
        stsc_entries_box(&[(1, samples_per_chunk, sample_description_index)])
    }

    fn stsc_entries_box(entries: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(entries.len()).unwrap()).to_be_bytes());
        for (first_chunk, samples_per_chunk, sample_description_index) in entries {
            body.extend_from_slice(&first_chunk.to_be_bytes());
            body.extend_from_slice(&samples_per_chunk.to_be_bytes());
            body.extend_from_slice(&sample_description_index.to_be_bytes());
        }
        box_(*STSC_ID, &full_box(0, &body))
    }

    fn stsz_box(sample_sizes: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&(u32::try_from(sample_sizes.len()).unwrap()).to_be_bytes());
        for sample_size in sample_sizes {
            body.extend_from_slice(&sample_size.to_be_bytes());
        }
        box_(*STSZ_ID, &full_box(0, &body))
    }

    fn stss_box(sync_sample_numbers: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(sync_sample_numbers.len()).unwrap()).to_be_bytes());
        for sample_number in sync_sample_numbers {
            body.extend_from_slice(&sample_number.to_be_bytes());
        }
        box_(*STSS_ID, &full_box(0, &body))
    }

    fn stco_box(chunk_offset: u32) -> Vec<u8> {
        stco_offsets_box(&[u64::from(chunk_offset)])
    }

    fn stco_offsets_box(chunk_offsets: &[u64]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(chunk_offsets.len()).unwrap()).to_be_bytes());
        for chunk_offset in chunk_offsets {
            body.extend_from_slice(&u32::try_from(*chunk_offset).unwrap().to_be_bytes());
        }
        box_(*STCO_ID, &full_box(0, &body))
    }

    fn co64_box(chunk_offset: u64) -> Vec<u8> {
        co64_offsets_box(&[chunk_offset])
    }

    fn co64_offsets_box(chunk_offsets: &[u64]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(chunk_offsets.len()).unwrap()).to_be_bytes());
        for chunk_offset in chunk_offsets {
            body.extend_from_slice(&chunk_offset.to_be_bytes());
        }
        box_(*CO64_ID, &full_box(0, &body))
    }

    fn chunked_mdat_payload(
        samples: &[&[u8]],
        chunk_sample_counts: &[u32],
        chunk_gaps: &[&[u8]],
        mdat_start: usize,
    ) -> (Vec<u64>, Vec<u8>) {
        let mut chunk_offsets = Vec::new();
        let mut payload = Vec::new();
        let mut sample_index = 0;

        for (chunk_index, sample_count) in chunk_sample_counts.iter().copied().enumerate() {
            chunk_offsets.push(u64::try_from(mdat_start + payload.len()).unwrap());
            for _ in 0..sample_count {
                payload.extend_from_slice(samples[sample_index]);
                sample_index += 1;
            }
            if let Some(gap) = chunk_gaps.get(chunk_index) {
                payload.extend_from_slice(gap);
            }
        }

        (chunk_offsets, payload)
    }

    fn mvhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        box_(*MVHD_ID, &full_box(0, &body))
    }

    fn mvhd_v1(timescale: u32, duration: u64) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        box_(*MVHD_ID, &full_box(1, &body))
    }

    fn tkhd_v0(track_id: u32, duration: u32, width: u32, height: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&track_id.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        write_tkhd_tail(&mut body, width, height);
        box_(*TKHD_ID, &full_box(0, &body))
    }

    fn tkhd_v1(track_id: u32, duration: u64, width: u32, height: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&track_id.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        write_tkhd_tail(&mut body, width, height);
        box_(*TKHD_ID, &full_box(1, &body))
    }

    fn write_tkhd_tail(body: &mut Vec<u8>, width: u32, height: u32) {
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&0_i16.to_be_bytes());
        body.extend_from_slice(&0_i16.to_be_bytes());
        body.extend_from_slice(&0_i16.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        for _ in 0..9 {
            body.extend_from_slice(&0_u32.to_be_bytes());
        }
        body.extend_from_slice(&(width << 16).to_be_bytes());
        body.extend_from_slice(&(height << 16).to_be_bytes());
    }

    fn mdhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
        mdhd_v0_with_language(timescale, duration, "")
    }

    fn mdhd_v0_with_language(timescale: u32, duration: u32, language: &str) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        body.extend_from_slice(&packed_mdhd_language(language).to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        box_(*MDHD_ID, &full_box(0, &body))
    }

    fn mdhd_v1(timescale: u32, duration: u64) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        box_(*MDHD_ID, &full_box(1, &body))
    }

    fn packed_mdhd_language(language: &str) -> u16 {
        if language.is_empty() {
            return 0;
        }
        let bytes = language.as_bytes();
        assert_eq!(bytes.len(), 3);
        bytes.iter().fold(0_u16, |packed, byte| {
            assert!(byte.is_ascii_lowercase());
            (packed << 5) | u16::from(*byte - b'a' + 1)
        })
    }

    fn hdlr_box(handler_type: [u8; 4], handler_name: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&handler_type);
        body.extend_from_slice(&[0; 12]);
        body.extend_from_slice(handler_name);
        box_(*HDLR_ID, &full_box(0, &body))
    }

    fn full_box(version: u8, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(version);
        out.extend_from_slice(&[0, 0, 0]);
        out.extend_from_slice(body);
        out
    }

    fn box_(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(8 + payload.len()).unwrap();
        let mut out = ByteWriter::new();
        out.write_u32_be(size);
        out.write_all(&kind);
        out.write_all(payload);
        out.into_inner()
    }

    fn box_with_declared_size(kind: [u8; 4], declared_size: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = ByteWriter::new();
        out.write_u32_be(declared_size);
        out.write_all(&kind);
        out.write_all(payload);
        out.into_inner()
    }

    fn box_extended(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u64::try_from(16 + payload.len()).unwrap();
        let mut out = ByteWriter::new();
        out.write_u32_be(1);
        out.write_all(&kind);
        out.write_u64_be(size);
        out.write_all(payload);
        out.into_inner()
    }

    fn box_size_zero(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = ByteWriter::new();
        out.write_u32_be(0);
        out.write_all(&kind);
        out.write_all(payload);
        out.into_inner()
    }
}
