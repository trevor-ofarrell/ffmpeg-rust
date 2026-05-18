use crate::{
    AvError, AvResult, BufferRef, ChannelLayout, Dictionary, PixelFormat, Rational, SampleFormat,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum FrameData {
    #[default]
    Empty,
    Video(VideoFrame),
    Audio(AudioFrame),
}

impl FrameData {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn is_writable(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Video(frame) => frame.is_writable(),
            Self::Audio(frame) => frame.is_writable(),
        }
    }

    pub fn make_writable(&mut self) {
        match self {
            Self::Empty => {}
            Self::Video(frame) => frame.make_writable(),
            Self::Audio(frame) => frame.make_writable(),
        }
    }

    pub fn set_plane_visible_data(&mut self, index: usize, data: &[u8]) -> AvResult<()> {
        match self {
            Self::Empty => Err(AvError::invalid_argument(format!(
                "empty frame has no plane {index} for {} visible bytes",
                data.len()
            ))),
            Self::Video(frame) => frame.set_plane_visible_data(index, data),
            Self::Audio(frame) => frame.set_plane_visible_data(index, data),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pts: Option<i64>,
    data: FrameData,
    hw_frames_context: Option<BufferRef>,
    side_data: Vec<FrameSideData>,
}

impl Frame {
    pub fn empty() -> Self {
        Self {
            pts: None,
            data: FrameData::Empty,
            hw_frames_context: None,
            side_data: Vec::new(),
        }
    }

    pub fn video(frame: VideoFrame) -> Self {
        Self {
            pts: None,
            data: FrameData::Video(frame),
            hw_frames_context: None,
            side_data: Vec::new(),
        }
    }

    pub fn audio(frame: AudioFrame) -> Self {
        Self {
            pts: None,
            data: FrameData::Audio(frame),
            hw_frames_context: None,
            side_data: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pts.is_none()
            && self.data.is_empty()
            && self.hw_frames_context.is_none()
            && self.side_data.is_empty()
    }

    pub fn unref(&mut self) {
        *self = Self::empty();
    }

    pub fn ref_from(&mut self, source: &Self) {
        *self = source.clone();
    }

    pub fn move_ref_from(&mut self, source: &mut Self) {
        *self = std::mem::take(source);
    }

    pub fn pts(&self) -> Option<i64> {
        self.pts
    }

    pub fn set_pts(&mut self, pts: Option<i64>) {
        self.pts = pts;
    }

    pub fn data(&self) -> &FrameData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut FrameData {
        &mut self.data
    }

    pub fn is_writable(&self) -> bool {
        self.data.is_writable()
    }

    pub fn make_writable(&mut self) {
        self.data.make_writable();
    }

    pub fn side_data_is_writable(&self) -> bool {
        self.side_data.iter().all(FrameSideData::is_writable)
    }

    pub fn make_side_data_writable(&mut self) {
        for side_data in &mut self.side_data {
            side_data.make_writable();
        }
    }

    pub fn hw_frames_context_is_writable(&self) -> Option<bool> {
        self.hw_frames_context.as_ref().map(BufferRef::is_writable)
    }

    pub fn make_hw_frames_context_writable(&mut self) -> bool {
        let Some(context) = self.hw_frames_context.as_mut() else {
            return false;
        };
        context.make_mut();
        true
    }

    pub fn all_references_are_writable(&self) -> bool {
        self.is_writable()
            && self.side_data_is_writable()
            && self.hw_frames_context_is_writable().unwrap_or(true)
    }

    pub fn make_all_references_writable(&mut self) {
        self.make_writable();
        self.make_side_data_writable();
        self.make_hw_frames_context_writable();
    }

    pub fn set_plane_visible_data(&mut self, index: usize, data: &[u8]) -> AvResult<()> {
        self.data.set_plane_visible_data(index, data)
    }

    pub fn hw_frames_context(&self) -> Option<&BufferRef> {
        self.hw_frames_context.as_ref()
    }

    pub fn with_hw_frames_context(mut self, context: BufferRef) -> Self {
        self.hw_frames_context = Some(context);
        self
    }

    pub fn set_hw_frames_context(&mut self, context: Option<BufferRef>) {
        self.hw_frames_context = context;
    }

    pub fn take_hw_frames_context(&mut self) -> Option<BufferRef> {
        self.hw_frames_context.take()
    }

    pub fn side_data(&self) -> &[FrameSideData] {
        &self.side_data
    }

    pub fn push_side_data(&mut self, side_data: FrameSideData) {
        self.side_data.push(side_data);
    }

    pub fn set_side_data(&mut self, side_data: FrameSideData) -> Vec<FrameSideData> {
        let kind = side_data.kind_id().clone();
        let Some(first_index) = self
            .side_data
            .iter()
            .position(|existing| existing.kind_id() == &kind)
        else {
            self.side_data.push(side_data);
            return Vec::new();
        };

        let mut removed = vec![std::mem::replace(
            &mut self.side_data[first_index],
            side_data,
        )];
        let mut index = first_index + 1;
        while index < self.side_data.len() {
            if self.side_data[index].kind_id() == &kind {
                removed.push(self.side_data.remove(index));
            } else {
                index += 1;
            }
        }
        removed
    }

    pub fn set_side_data_payload(
        &mut self,
        kind: impl Into<String>,
        data: Vec<u8>,
    ) -> AvResult<Vec<FrameSideData>> {
        let side_data = FrameSideData::new(kind, data)?;
        Ok(self.set_side_data(side_data))
    }

    pub fn set_side_data_buffer(
        &mut self,
        kind: impl Into<String>,
        buffer: BufferRef,
    ) -> AvResult<Vec<FrameSideData>> {
        let side_data = FrameSideData::new_with_buffer_ref(kind, buffer)?;
        Ok(self.set_side_data(side_data))
    }

    pub fn set_side_data_kind(
        &mut self,
        kind: FrameSideDataKind,
        data: Vec<u8>,
    ) -> AvResult<Vec<FrameSideData>> {
        let side_data = FrameSideData::new_with_kind(kind, data)?;
        Ok(self.set_side_data(side_data))
    }

    pub fn set_side_data_kind_buffer(
        &mut self,
        kind: FrameSideDataKind,
        buffer: BufferRef,
    ) -> AvResult<Vec<FrameSideData>> {
        let side_data = FrameSideData::new_with_kind_and_buffer_ref(kind, buffer)?;
        Ok(self.set_side_data(side_data))
    }

    pub fn add_side_data(
        &mut self,
        kind: impl Into<String>,
        data: Vec<u8>,
    ) -> AvResult<&mut FrameSideData> {
        let side_data = FrameSideData::new(kind, data)?;
        self.side_data.push(side_data);
        Ok(self
            .side_data
            .last_mut()
            .expect("side data was just inserted"))
    }

    pub fn add_side_data_buffer(
        &mut self,
        kind: impl Into<String>,
        buffer: BufferRef,
    ) -> AvResult<&mut FrameSideData> {
        let side_data = FrameSideData::new_with_buffer_ref(kind, buffer)?;
        self.side_data.push(side_data);
        Ok(self
            .side_data
            .last_mut()
            .expect("side data was just inserted"))
    }

    pub fn add_side_data_kind(
        &mut self,
        kind: FrameSideDataKind,
        data: Vec<u8>,
    ) -> AvResult<&mut FrameSideData> {
        let side_data = FrameSideData::new_with_kind(kind, data)?;
        self.side_data.push(side_data);
        Ok(self
            .side_data
            .last_mut()
            .expect("side data was just inserted"))
    }

    pub fn add_side_data_kind_buffer(
        &mut self,
        kind: FrameSideDataKind,
        buffer: BufferRef,
    ) -> AvResult<&mut FrameSideData> {
        let side_data = FrameSideData::new_with_kind_and_buffer_ref(kind, buffer)?;
        self.side_data.push(side_data);
        Ok(self
            .side_data
            .last_mut()
            .expect("side data was just inserted"))
    }

    pub fn side_data_by_kind(&self, kind: &FrameSideDataKind) -> Option<&FrameSideData> {
        self.side_data
            .iter()
            .find(|side_data| side_data.kind_id() == kind)
    }

    pub fn side_data_by_kind_mut(
        &mut self,
        kind: &FrameSideDataKind,
    ) -> Option<&mut FrameSideData> {
        self.side_data
            .iter_mut()
            .find(|side_data| side_data.kind_id() == kind)
    }

    pub fn remove_side_data(&mut self, kind: &str) -> Option<FrameSideData> {
        let Ok(kind) = FrameSideDataKind::from_name(kind) else {
            return None;
        };
        self.remove_side_data_kind(&kind)
    }

    pub fn remove_side_data_kind(&mut self, kind: &FrameSideDataKind) -> Option<FrameSideData> {
        self.side_data
            .iter()
            .position(|side_data| side_data.kind_id() == kind)
            .map(|index| self.side_data.remove(index))
    }

    pub fn remove_side_data_by_properties(
        &mut self,
        properties: FrameSideDataProperties,
    ) -> Vec<FrameSideData> {
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.side_data.len() {
            if self.side_data[index].properties().intersects(properties) {
                removed.push(self.side_data.remove(index));
            } else {
                index += 1;
            }
        }
        removed
    }

    pub fn take_side_data(&mut self) -> Vec<FrameSideData> {
        std::mem::take(&mut self.side_data)
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameSideDataProperties(u32);

impl FrameSideDataProperties {
    pub const EMPTY: Self = Self(0);
    pub const GLOBAL: Self = Self(1 << 0);
    pub const MULTI: Self = Self(1 << 1);
    pub const SIZE_DEPENDENT: Self = Self(1 << 2);
    pub const COLOR_DEPENDENT: Self = Self(1 << 3);
    pub const CHANNEL_DEPENDENT: Self = Self(1 << 4);
    pub const ALL: Self = Self(
        Self::GLOBAL.0
            | Self::MULTI.0
            | Self::SIZE_DEPENDENT.0
            | Self::COLOR_DEPENDENT.0
            | Self::CHANNEL_DEPENDENT.0,
    );

    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self(bits & Self::ALL.0)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self::from_bits_truncate(self.0 | other.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSideDataDescriptor {
    name: &'static str,
    properties: FrameSideDataProperties,
}

impl FrameSideDataDescriptor {
    pub const fn new(name: &'static str, properties: FrameSideDataProperties) -> Self {
        Self { name, properties }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn properties(self) -> FrameSideDataProperties {
        self.properties
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDisplayMatrix {
    elements: [i32; Self::ELEMENTS],
}

impl FrameDisplayMatrix {
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
                "display matrix frame side data requires exactly {} bytes, got {}",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FrameMatrixEncoding {
    None = 0,
    Dolby = 1,
    DolbyProLogicIi = 2,
    DolbyProLogicIiX = 3,
    DolbyProLogicIiZ = 4,
    DolbyEx = 5,
    DolbyHeadphone = 6,
}

impl FrameMatrixEncoding {
    pub const DATA_LEN: usize = 4;

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "matrix encoding frame side data requires exactly {} bytes, got {}",
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
            0 => Ok(Self::None),
            1 => Ok(Self::Dolby),
            2 => Ok(Self::DolbyProLogicIi),
            3 => Ok(Self::DolbyProLogicIiX),
            4 => Ok(Self::DolbyProLogicIiZ),
            5 => Ok(Self::DolbyEx),
            6 => Ok(Self::DolbyHeadphone),
            _ => Err(AvError::invalid_data(format!(
                "invalid matrix encoding value {value}"
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
            Self::None => "AV_MATRIX_ENCODING_NONE",
            Self::Dolby => "AV_MATRIX_ENCODING_DOLBY",
            Self::DolbyProLogicIi => "AV_MATRIX_ENCODING_DPLII",
            Self::DolbyProLogicIiX => "AV_MATRIX_ENCODING_DPLIIX",
            Self::DolbyProLogicIiZ => "AV_MATRIX_ENCODING_DPLIIZ",
            Self::DolbyEx => "AV_MATRIX_ENCODING_DOLBYEX",
            Self::DolbyHeadphone => "AV_MATRIX_ENCODING_DOLBYHEADPHONE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FrameDownmixType {
    Unknown = 0,
    LoRo = 1,
    LtRt = 2,
    DolbyProLogicIi = 3,
}

impl FrameDownmixType {
    pub fn from_raw(value: i32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::LoRo),
            2 => Ok(Self::LtRt),
            3 => Ok(Self::DolbyProLogicIi),
            _ => Err(AvError::invalid_data(format!(
                "invalid downmix type value {value}"
            ))),
        }
    }

    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::Unknown => "AV_DOWNMIX_TYPE_UNKNOWN",
            Self::LoRo => "AV_DOWNMIX_TYPE_LORO",
            Self::LtRt => "AV_DOWNMIX_TYPE_LTRT",
            Self::DolbyProLogicIi => "AV_DOWNMIX_TYPE_DPLII",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDownmixInfo {
    preferred_downmix_type: FrameDownmixType,
    level_bits: [u64; Self::LEVELS],
}

impl FrameDownmixInfo {
    pub const LEVELS: usize = 5;
    pub const DATA_LEN: usize = 8 + Self::LEVELS * 8;

    pub fn new(
        preferred_downmix_type: FrameDownmixType,
        center_mix_level: f64,
        center_mix_level_ltrt: f64,
        surround_mix_level: f64,
        surround_mix_level_ltrt: f64,
        lfe_mix_level: f64,
    ) -> Self {
        Self::from_level_bits(
            preferred_downmix_type,
            [
                center_mix_level.to_bits(),
                center_mix_level_ltrt.to_bits(),
                surround_mix_level.to_bits(),
                surround_mix_level_ltrt.to_bits(),
                lfe_mix_level.to_bits(),
            ],
        )
    }

    pub const fn from_level_bits(
        preferred_downmix_type: FrameDownmixType,
        level_bits: [u64; Self::LEVELS],
    ) -> Self {
        Self {
            preferred_downmix_type,
            level_bits,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "downmix info frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut raw_type = [0; 4];
        raw_type.copy_from_slice(&data[..4]);
        let preferred_downmix_type = FrameDownmixType::from_raw(i32::from_ne_bytes(raw_type))?;

        let mut level_bits = [0; Self::LEVELS];
        for (bits, chunk) in level_bits
            .iter_mut()
            .zip(data[8..].chunks_exact(std::mem::size_of::<f64>()))
        {
            let mut raw = [0; 8];
            raw.copy_from_slice(chunk);
            *bits = u64::from_ne_bytes(raw);
        }

        Ok(Self {
            preferred_downmix_type,
            level_bits,
        })
    }

    pub const fn preferred_downmix_type(self) -> FrameDownmixType {
        self.preferred_downmix_type
    }

    pub const fn level_bits(self) -> [u64; Self::LEVELS] {
        self.level_bits
    }

    pub fn levels(self) -> [f64; Self::LEVELS] {
        self.level_bits.map(f64::from_bits)
    }

    pub fn center_mix_level(self) -> f64 {
        f64::from_bits(self.level_bits[0])
    }

    pub fn center_mix_level_ltrt(self) -> f64 {
        f64::from_bits(self.level_bits[1])
    }

    pub fn surround_mix_level(self) -> f64 {
        f64::from_bits(self.level_bits[2])
    }

    pub fn surround_mix_level_ltrt(self) -> f64 {
        f64::from_bits(self.level_bits[3])
    }

    pub fn lfe_mix_level(self) -> f64 {
        f64::from_bits(self.level_bits[4])
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[..4].copy_from_slice(&self.preferred_downmix_type.as_raw().to_ne_bytes());
        for (bits, chunk) in self
            .level_bits
            .iter()
            .zip(bytes[8..].chunks_exact_mut(std::mem::size_of::<f64>()))
        {
            chunk.copy_from_slice(&bits.to_ne_bytes());
        }
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameReplayGain {
    track_gain: i32,
    track_peak: u32,
    album_gain: i32,
    album_peak: u32,
}

impl FrameReplayGain {
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
                "replaygain frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut track_gain = [0; 4];
        track_gain.copy_from_slice(&data[0..4]);
        let mut track_peak = [0; 4];
        track_peak.copy_from_slice(&data[4..8]);
        let mut album_gain = [0; 4];
        album_gain.copy_from_slice(&data[8..12]);
        let mut album_peak = [0; 4];
        album_peak.copy_from_slice(&data[12..16]);

        Ok(Self {
            track_gain: i32::from_ne_bytes(track_gain),
            track_peak: u32::from_ne_bytes(track_peak),
            album_gain: i32::from_ne_bytes(album_gain),
            album_peak: u32::from_ne_bytes(album_peak),
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
#[repr(u8)]
pub enum FrameActiveFormatDescription {
    Same = 8,
    FourThree = 9,
    SixteenNine = 10,
    FourteenNine = 11,
    FourThreeProtectedFourteenNine = 13,
    SixteenNineProtectedFourteenNine = 14,
    ProtectedFourThree = 15,
}

impl FrameActiveFormatDescription {
    pub const DATA_LEN: usize = 1;

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "active format description frame side data requires exactly {} byte, got {}",
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameMotionVector {
    source: i32,
    w: u8,
    h: u8,
    src_x: i16,
    src_y: i16,
    dst_x: i16,
    dst_y: i16,
    flags: u64,
    motion_x: i32,
    motion_y: i32,
    motion_scale: u16,
}

impl FrameMotionVector {
    pub const DATA_LEN: usize = 40;

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        source: i32,
        w: u8,
        h: u8,
        src_x: i16,
        src_y: i16,
        dst_x: i16,
        dst_y: i16,
        flags: u64,
        motion_x: i32,
        motion_y: i32,
        motion_scale: u16,
    ) -> Self {
        Self {
            source,
            w,
            h,
            src_x,
            src_y,
            dst_x,
            dst_y,
            flags,
            motion_x,
            motion_y,
            motion_scale,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "motion vector frame side data record requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut source = [0; 4];
        source.copy_from_slice(&data[0..4]);
        let mut src_x = [0; 2];
        src_x.copy_from_slice(&data[6..8]);
        let mut src_y = [0; 2];
        src_y.copy_from_slice(&data[8..10]);
        let mut dst_x = [0; 2];
        dst_x.copy_from_slice(&data[10..12]);
        let mut dst_y = [0; 2];
        dst_y.copy_from_slice(&data[12..14]);
        let mut flags = [0; 8];
        flags.copy_from_slice(&data[16..24]);
        let mut motion_x = [0; 4];
        motion_x.copy_from_slice(&data[24..28]);
        let mut motion_y = [0; 4];
        motion_y.copy_from_slice(&data[28..32]);
        let mut motion_scale = [0; 2];
        motion_scale.copy_from_slice(&data[32..34]);

        Ok(Self {
            source: i32::from_ne_bytes(source),
            w: data[4],
            h: data[5],
            src_x: i16::from_ne_bytes(src_x),
            src_y: i16::from_ne_bytes(src_y),
            dst_x: i16::from_ne_bytes(dst_x),
            dst_y: i16::from_ne_bytes(dst_y),
            flags: u64::from_ne_bytes(flags),
            motion_x: i32::from_ne_bytes(motion_x),
            motion_y: i32::from_ne_bytes(motion_y),
            motion_scale: u16::from_ne_bytes(motion_scale),
        })
    }

    pub const fn source(self) -> i32 {
        self.source
    }

    pub const fn width(self) -> u8 {
        self.w
    }

    pub const fn height(self) -> u8 {
        self.h
    }

    pub const fn src_x(self) -> i16 {
        self.src_x
    }

    pub const fn src_y(self) -> i16 {
        self.src_y
    }

    pub const fn dst_x(self) -> i16 {
        self.dst_x
    }

    pub const fn dst_y(self) -> i16 {
        self.dst_y
    }

    pub const fn flags(self) -> u64 {
        self.flags
    }

    pub const fn motion_x(self) -> i32 {
        self.motion_x
    }

    pub const fn motion_y(self) -> i32 {
        self.motion_y
    }

    pub const fn motion_scale(self) -> u16 {
        self.motion_scale
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[0..4].copy_from_slice(&self.source.to_ne_bytes());
        bytes[4] = self.w;
        bytes[5] = self.h;
        bytes[6..8].copy_from_slice(&self.src_x.to_ne_bytes());
        bytes[8..10].copy_from_slice(&self.src_y.to_ne_bytes());
        bytes[10..12].copy_from_slice(&self.dst_x.to_ne_bytes());
        bytes[12..14].copy_from_slice(&self.dst_y.to_ne_bytes());
        bytes[16..24].copy_from_slice(&self.flags.to_ne_bytes());
        bytes[24..28].copy_from_slice(&self.motion_x.to_ne_bytes());
        bytes[28..32].copy_from_slice(&self.motion_y.to_ne_bytes());
        bytes[32..34].copy_from_slice(&self.motion_scale.to_ne_bytes());
        bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMotionVectors {
    vectors: Vec<FrameMotionVector>,
}

impl FrameMotionVectors {
    pub fn new(vectors: Vec<FrameMotionVector>) -> AvResult<Self> {
        if vectors.is_empty() {
            return Err(AvError::invalid_data(
                "motion vectors frame side data requires at least one record",
            ));
        }

        Ok(Self { vectors })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.is_empty()
            || !data
                .chunks_exact(FrameMotionVector::DATA_LEN)
                .remainder()
                .is_empty()
        {
            return Err(AvError::invalid_data(format!(
                "motion vectors frame side data requires a non-empty multiple of {} bytes, got {}",
                FrameMotionVector::DATA_LEN,
                data.len()
            )));
        }

        let vectors = data
            .chunks_exact(FrameMotionVector::DATA_LEN)
            .map(FrameMotionVector::parse)
            .collect::<AvResult<Vec<_>>>()?;
        Self::new(vectors)
    }

    pub fn vectors(&self) -> &[FrameMotionVector] {
        &self.vectors
    }

    pub fn into_vectors(self) -> Vec<FrameMotionVector> {
        self.vectors
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.vectors.len() * FrameMotionVector::DATA_LEN);
        for vector in &self.vectors {
            bytes.extend_from_slice(&vector.to_bytes());
        }
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameSkipSamplesReason {
    PaddingSilence = 0,
    Convergence = 1,
}

impl FrameSkipSamplesReason {
    pub fn from_byte(value: u8) -> AvResult<Self> {
        match value {
            0 => Ok(Self::PaddingSilence),
            1 => Ok(Self::Convergence),
            _ => Err(AvError::invalid_data(format!(
                "invalid skip samples reason value {value}"
            ))),
        }
    }

    pub const fn as_byte(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSkipSamples {
    start: u32,
    end: u32,
    start_reason: FrameSkipSamplesReason,
    end_reason: FrameSkipSamplesReason,
}

impl FrameSkipSamples {
    pub const DATA_LEN: usize = 10;

    pub const fn new(
        start: u32,
        end: u32,
        start_reason: FrameSkipSamplesReason,
        end_reason: FrameSkipSamplesReason,
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
                "skip samples frame side data requires exactly {} bytes, got {}",
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
            start_reason: FrameSkipSamplesReason::from_byte(data[8])?,
            end_reason: FrameSkipSamplesReason::from_byte(data[9])?,
        })
    }

    pub const fn start(self) -> u32 {
        self.start
    }

    pub const fn end(self) -> u32 {
        self.end
    }

    pub const fn start_reason(self) -> FrameSkipSamplesReason {
        self.start_reason
    }

    pub const fn end_reason(self) -> FrameSkipSamplesReason {
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
#[repr(i32)]
pub enum FrameAudioServiceType {
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

impl FrameAudioServiceType {
    pub const DATA_LEN: usize = 4;

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() < Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "audio service type frame side data requires at least {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut raw = [0; Self::DATA_LEN];
        raw.copy_from_slice(&data[..Self::DATA_LEN]);
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
                "invalid audio service type value {value}"
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
pub struct FrameMasteringDisplayMetadata {
    display_primaries: [[Rational; Self::COORDINATES]; Self::PRIMARIES],
    white_point: [Rational; Self::COORDINATES],
    min_luminance: Rational,
    max_luminance: Rational,
    has_primaries: i32,
    has_luminance: i32,
}

impl FrameMasteringDisplayMetadata {
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
                "mastering display metadata frame side data requires exactly {} bytes, got {}",
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
        for primary in &self.display_primaries {
            for coordinate in primary {
                Self::write_rational(&mut bytes, &mut offset, *coordinate);
            }
        }
        for coordinate in &self.white_point {
            Self::write_rational(&mut bytes, &mut offset, *coordinate);
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
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[*offset..*offset + 4]);
        *offset += 4;
        i32::from_ne_bytes(raw)
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
pub struct FrameGopTimecode {
    value: i64,
}

impl FrameGopTimecode {
    pub const DATA_LEN: usize = 8;
    pub const MAX_VALUE: i64 = (1 << 25) - 1;

    pub fn new(value: u32) -> AvResult<Self> {
        Self::from_raw_i64(i64::from(value))
    }

    pub fn from_raw_i64(value: i64) -> AvResult<Self> {
        if !(0..=Self::MAX_VALUE).contains(&value) {
            return Err(AvError::invalid_data(format!(
                "invalid GOP timecode 25-bit value {value}"
            )));
        }

        Ok(Self { value })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "GOP timecode frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut raw = [0; Self::DATA_LEN];
        raw.copy_from_slice(data);
        Self::from_raw_i64(i64::from_ne_bytes(raw))
    }

    pub const fn as_raw_i64(self) -> i64 {
        self.value
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        self.value.to_ne_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FrameSphericalProjection {
    Equirectangular = 0,
    Cubemap = 1,
    EquirectangularTile = 2,
    HalfEquirectangular = 3,
    Rectilinear = 4,
    Fisheye = 5,
    ParametricImmersive = 6,
}

impl FrameSphericalProjection {
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
                "invalid spherical projection value {value}"
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
pub struct FrameSphericalMapping {
    projection: FrameSphericalProjection,
    yaw: i32,
    pitch: i32,
    roll: i32,
    bounds: [u32; Self::BOUNDS],
    padding: u32,
}

impl FrameSphericalMapping {
    pub const BOUNDS: usize = 4;
    pub const DATA_LEN: usize = 36;

    pub const fn new(
        projection: FrameSphericalProjection,
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
                "spherical mapping frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut offset = 0;
        let projection = FrameSphericalProjection::from_raw(Self::read_i32(data, &mut offset))?;
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

    pub const fn projection(self) -> FrameSphericalProjection {
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
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[*offset..*offset + 4]);
        *offset += 4;
        i32::from_ne_bytes(raw)
    }

    fn read_u32(data: &[u8], offset: &mut usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[*offset..*offset + 4]);
        *offset += 4;
        u32::from_ne_bytes(raw)
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
pub struct FrameContentLightMetadata {
    max_content_light_level: u32,
    max_average_light_level: u32,
}

impl FrameContentLightMetadata {
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
                "content light metadata frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut max_content_light_level = [0; 4];
        max_content_light_level.copy_from_slice(&data[0..4]);
        let mut max_average_light_level = [0; 4];
        max_average_light_level.copy_from_slice(&data[4..8]);
        Ok(Self {
            max_content_light_level: u32::from_ne_bytes(max_content_light_level),
            max_average_light_level: u32::from_ne_bytes(max_average_light_level),
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
pub struct FrameIccProfile<'a> {
    data: &'a [u8],
    name: Option<&'a str>,
    declared_size: u32,
    tag_count: u32,
}

impl<'a> FrameIccProfile<'a> {
    pub const HEADER_LEN: usize = 128;
    pub const TAG_COUNT_LEN: usize = 4;
    pub const TAG_RECORD_LEN: usize = 12;
    pub const MIN_DATA_LEN: usize = Self::HEADER_LEN + Self::TAG_COUNT_LEN;
    pub const PROFILE_SIZE_OFFSET: usize = 0;
    pub const SIGNATURE_OFFSET: usize = 36;
    pub const TAG_COUNT_OFFSET: usize = Self::HEADER_LEN;
    pub const ICC_SIGNATURE: [u8; 4] = *b"acsp";

    pub fn parse(data: &'a [u8], metadata: &'a Dictionary) -> AvResult<Self> {
        if data.len() < Self::MIN_DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "ICC profile frame side data requires at least {} bytes, got {}",
                Self::MIN_DATA_LEN,
                data.len()
            )));
        }

        let declared_size = Self::read_be_u32(data, Self::PROFILE_SIZE_OFFSET);
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

        let tag_count = Self::read_be_u32(data, Self::TAG_COUNT_OFFSET);
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
            name: metadata.get("name"),
            declared_size,
            tag_count,
        })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn name(self) -> Option<&'a str> {
        self.name
    }

    pub const fn declared_size(self) -> u32 {
        self.declared_size
    }

    pub const fn tag_count(self) -> u32 {
        self.tag_count
    }

    pub fn profile_version_raw(self) -> u32 {
        Self::read_be_u32(self.data, 8)
    }

    pub fn device_class(self) -> [u8; 4] {
        Self::read_fourcc(self.data, 12)
    }

    pub fn color_space(self) -> [u8; 4] {
        Self::read_fourcc(self.data, 16)
    }

    pub fn profile_connection_space(self) -> [u8; 4] {
        Self::read_fourcc(self.data, 20)
    }

    fn read_be_u32(data: &[u8], offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        u32::from_be_bytes(raw)
    }

    fn read_fourcc(data: &[u8], offset: usize) -> [u8; 4] {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameS12mTimecode {
    words: [u32; Self::WORDS],
}

impl FrameS12mTimecode {
    pub const WORDS: usize = 4;
    pub const DATA_LEN: usize = Self::WORDS * 4;
    pub const MIN_TIMECODES: usize = 1;
    pub const MAX_TIMECODES: usize = 3;

    pub fn new(timecodes: &[u32]) -> AvResult<Self> {
        if !(Self::MIN_TIMECODES..=Self::MAX_TIMECODES).contains(&timecodes.len()) {
            return Err(AvError::invalid_argument(format!(
                "S12M timecode side data requires {} to {} timecodes, got {}",
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
                "S12M timecode frame side data requires exactly {} bytes, got {}",
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
#[repr(i32)]
pub enum FrameHdrPlusOverlapProcessOption {
    WeightedAveraging = 0,
    Layering = 1,
}

impl FrameHdrPlusOverlapProcessOption {
    pub fn from_raw(value: i32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::WeightedAveraging),
            1 => Ok(Self::Layering),
            _ => Err(AvError::invalid_data(format!(
                "invalid HDR10+ overlap process option {value}"
            ))),
        }
    }

    pub const fn as_raw(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHdrPlusPercentile {
    percentage: u8,
    percentile: Rational,
}

impl FrameHdrPlusPercentile {
    pub const fn new(percentage: u8, percentile: Rational) -> Self {
        Self {
            percentage,
            percentile,
        }
    }

    pub const fn percentage(self) -> u8 {
        self.percentage
    }

    pub const fn percentile(self) -> Rational {
        self.percentile
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHdrPlusColorTransformParams<'a> {
    data: &'a [u8],
}

impl<'a> FrameHdrPlusColorTransformParams<'a> {
    pub const DATA_LEN: usize = 428;
    pub const MAX_DISTRIBUTION_MAXRGB_PERCENTILES: usize = 15;
    pub const MAX_BEZIER_CURVE_ANCHORS: usize = 15;
    const WINDOW_UPPER_LEFT_CORNER_X_OFFSET: usize = 0;
    const WINDOW_UPPER_LEFT_CORNER_Y_OFFSET: usize = 8;
    const WINDOW_LOWER_RIGHT_CORNER_X_OFFSET: usize = 16;
    const WINDOW_LOWER_RIGHT_CORNER_Y_OFFSET: usize = 24;
    const CENTER_OF_ELLIPSE_X_OFFSET: usize = 32;
    const CENTER_OF_ELLIPSE_Y_OFFSET: usize = 34;
    const ROTATION_ANGLE_OFFSET: usize = 36;
    const SEMIMAJOR_AXIS_INTERNAL_ELLIPSE_OFFSET: usize = 38;
    const SEMIMAJOR_AXIS_EXTERNAL_ELLIPSE_OFFSET: usize = 40;
    const SEMIMINOR_AXIS_EXTERNAL_ELLIPSE_OFFSET: usize = 42;
    const OVERLAP_PROCESS_OPTION_OFFSET: usize = 44;
    const MAXSCL_OFFSET: usize = 48;
    const AVERAGE_MAXRGB_OFFSET: usize = 72;
    const NUM_DISTRIBUTION_MAXRGB_PERCENTILES_OFFSET: usize = 80;
    const DISTRIBUTION_MAXRGB_OFFSET: usize = 84;
    const PERCENTILE_LEN: usize = 12;
    const FRACTION_BRIGHT_PIXELS_OFFSET: usize = 264;
    const TONE_MAPPING_FLAG_OFFSET: usize = 272;
    const KNEE_POINT_X_OFFSET: usize = 276;
    const KNEE_POINT_Y_OFFSET: usize = 284;
    const NUM_BEZIER_CURVE_ANCHORS_OFFSET: usize = 292;
    const BEZIER_CURVE_ANCHORS_OFFSET: usize = 296;
    const COLOR_SATURATION_MAPPING_FLAG_OFFSET: usize = 416;
    const COLOR_SATURATION_WEIGHT_OFFSET: usize = 420;

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        let params = Self { data };
        params.overlap_process_option()?;
        if params.num_distribution_maxrgb_percentiles() > Self::MAX_DISTRIBUTION_MAXRGB_PERCENTILES
        {
            return Err(AvError::invalid_data(format!(
                "HDR10+ distribution percentile count {} exceeds {}",
                params.num_distribution_maxrgb_percentiles(),
                Self::MAX_DISTRIBUTION_MAXRGB_PERCENTILES
            )));
        }
        if params.tone_mapping_flag() > 1 {
            return Err(AvError::invalid_data(format!(
                "HDR10+ tone mapping flag {} is not boolean",
                params.tone_mapping_flag()
            )));
        }
        if params.num_bezier_curve_anchors() > Self::MAX_BEZIER_CURVE_ANCHORS {
            return Err(AvError::invalid_data(format!(
                "HDR10+ Bezier anchor count {} exceeds {}",
                params.num_bezier_curve_anchors(),
                Self::MAX_BEZIER_CURVE_ANCHORS
            )));
        }
        if params.color_saturation_mapping_flag() > 1 {
            return Err(AvError::invalid_data(format!(
                "HDR10+ color saturation mapping flag {} is not boolean",
                params.color_saturation_mapping_flag()
            )));
        }
        Ok(params)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn window_upper_left_corner_x(self) -> Rational {
        Self::read_rational(self.data, Self::WINDOW_UPPER_LEFT_CORNER_X_OFFSET)
    }

    pub fn window_upper_left_corner_y(self) -> Rational {
        Self::read_rational(self.data, Self::WINDOW_UPPER_LEFT_CORNER_Y_OFFSET)
    }

    pub fn window_lower_right_corner_x(self) -> Rational {
        Self::read_rational(self.data, Self::WINDOW_LOWER_RIGHT_CORNER_X_OFFSET)
    }

    pub fn window_lower_right_corner_y(self) -> Rational {
        Self::read_rational(self.data, Self::WINDOW_LOWER_RIGHT_CORNER_Y_OFFSET)
    }

    pub fn center_of_ellipse_x(self) -> u16 {
        Self::read_u16(self.data, Self::CENTER_OF_ELLIPSE_X_OFFSET)
    }

    pub fn center_of_ellipse_y(self) -> u16 {
        Self::read_u16(self.data, Self::CENTER_OF_ELLIPSE_Y_OFFSET)
    }

    pub fn rotation_angle(self) -> u8 {
        self.data[Self::ROTATION_ANGLE_OFFSET]
    }

    pub fn semimajor_axis_internal_ellipse(self) -> u16 {
        Self::read_u16(self.data, Self::SEMIMAJOR_AXIS_INTERNAL_ELLIPSE_OFFSET)
    }

    pub fn semimajor_axis_external_ellipse(self) -> u16 {
        Self::read_u16(self.data, Self::SEMIMAJOR_AXIS_EXTERNAL_ELLIPSE_OFFSET)
    }

    pub fn semiminor_axis_external_ellipse(self) -> u16 {
        Self::read_u16(self.data, Self::SEMIMINOR_AXIS_EXTERNAL_ELLIPSE_OFFSET)
    }

    pub fn overlap_process_option(self) -> AvResult<FrameHdrPlusOverlapProcessOption> {
        FrameHdrPlusOverlapProcessOption::from_raw(Self::read_i32(
            self.data,
            Self::OVERLAP_PROCESS_OPTION_OFFSET,
        ))
    }

    pub fn maxscl(self, channel: usize) -> Option<Rational> {
        (channel < 3).then(|| Self::read_rational(self.data, Self::MAXSCL_OFFSET + channel * 8))
    }

    pub fn average_maxrgb(self) -> Rational {
        Self::read_rational(self.data, Self::AVERAGE_MAXRGB_OFFSET)
    }

    pub fn num_distribution_maxrgb_percentiles(self) -> usize {
        usize::from(self.data[Self::NUM_DISTRIBUTION_MAXRGB_PERCENTILES_OFFSET])
    }

    pub fn distribution_maxrgb(self, index: usize) -> Option<FrameHdrPlusPercentile> {
        (index < self.num_distribution_maxrgb_percentiles()).then(|| {
            let offset = Self::DISTRIBUTION_MAXRGB_OFFSET + index * Self::PERCENTILE_LEN;
            FrameHdrPlusPercentile::new(
                self.data[offset],
                Self::read_rational(self.data, offset + 4),
            )
        })
    }

    pub fn fraction_bright_pixels(self) -> Rational {
        Self::read_rational(self.data, Self::FRACTION_BRIGHT_PIXELS_OFFSET)
    }

    pub fn tone_mapping_flag(self) -> u8 {
        self.data[Self::TONE_MAPPING_FLAG_OFFSET]
    }

    pub fn knee_point_x(self) -> Rational {
        Self::read_rational(self.data, Self::KNEE_POINT_X_OFFSET)
    }

    pub fn knee_point_y(self) -> Rational {
        Self::read_rational(self.data, Self::KNEE_POINT_Y_OFFSET)
    }

    pub fn num_bezier_curve_anchors(self) -> usize {
        usize::from(self.data[Self::NUM_BEZIER_CURVE_ANCHORS_OFFSET])
    }

    pub fn bezier_curve_anchor(self, index: usize) -> Option<Rational> {
        (index < self.num_bezier_curve_anchors())
            .then(|| Self::read_rational(self.data, Self::BEZIER_CURVE_ANCHORS_OFFSET + index * 8))
    }

    pub fn color_saturation_mapping_flag(self) -> u8 {
        self.data[Self::COLOR_SATURATION_MAPPING_FLAG_OFFSET]
    }

    pub fn color_saturation_weight(self) -> Rational {
        Self::read_rational(self.data, Self::COLOR_SATURATION_WEIGHT_OFFSET)
    }

    fn read_rational(data: &[u8], offset: usize) -> Rational {
        Rational::from_raw(
            Self::read_i32(data, offset),
            Self::read_i32(data, offset + 4),
        )
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_u16(data: &[u8], offset: usize) -> u16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&data[offset..offset + 2]);
        u16::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDynamicHdrPlus<'a> {
    data: &'a [u8],
    num_windows: usize,
}

impl<'a> FrameDynamicHdrPlus<'a> {
    pub const ITU_T_T35_COUNTRY_CODE: u8 = 0xB5;
    pub const APPLICATION_VERSION: u8 = 0;
    pub const MAX_WINDOWS: usize = 3;
    pub const MAX_PEAK_LUMINANCE_ROWS: usize = 25;
    pub const MAX_PEAK_LUMINANCE_COLS: usize = 25;
    const PARAMS_OFFSET: usize = 4;
    const TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET: usize =
        Self::PARAMS_OFFSET + Self::MAX_WINDOWS * FrameHdrPlusColorTransformParams::DATA_LEN;
    const TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET: usize =
        Self::TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET + 8;
    const NUM_ROWS_TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET: usize =
        Self::TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET + 1;
    const NUM_COLS_TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET: usize =
        Self::NUM_ROWS_TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET + 1;
    const TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET: usize =
        Self::TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET + 12;
    const PEAK_LUMINANCE_TABLE_LEN: usize =
        Self::MAX_PEAK_LUMINANCE_ROWS * Self::MAX_PEAK_LUMINANCE_COLS * 8;
    const MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET: usize =
        Self::TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET + Self::PEAK_LUMINANCE_TABLE_LEN;
    const NUM_ROWS_MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET: usize =
        Self::MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET + 1;
    const NUM_COLS_MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET: usize =
        Self::NUM_ROWS_MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET + 1;
    const MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET: usize =
        Self::MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET + 4;
    pub const DATA_LEN: usize =
        Self::MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET + Self::PEAK_LUMINANCE_TABLE_LEN;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "dynamic HDR10+ frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }
        if data[0] != Self::ITU_T_T35_COUNTRY_CODE {
            return Err(AvError::invalid_data(format!(
                "dynamic HDR10+ country code 0x{:02X} does not match 0x{:02X}",
                data[0],
                Self::ITU_T_T35_COUNTRY_CODE
            )));
        }
        if data[1] != Self::APPLICATION_VERSION {
            return Err(AvError::invalid_data(format!(
                "dynamic HDR10+ application version {} does not match {}",
                data[1],
                Self::APPLICATION_VERSION
            )));
        }

        let num_windows = usize::from(data[2]);
        if !(1..=Self::MAX_WINDOWS).contains(&num_windows) {
            return Err(AvError::invalid_data(format!(
                "dynamic HDR10+ window count {num_windows} is outside 1..={}",
                Self::MAX_WINDOWS
            )));
        }

        let parsed = Self { data, num_windows };
        for index in 0..num_windows {
            parsed.color_transform_params(index).unwrap().validate()?;
        }
        parsed.validate_peak_luminance_grid(
            parsed.targeted_system_display_actual_peak_luminance_flag(),
            parsed.num_rows_targeted_system_display_actual_peak_luminance(),
            parsed.num_cols_targeted_system_display_actual_peak_luminance(),
            "targeted system display",
        )?;
        parsed.validate_peak_luminance_grid(
            parsed.mastering_display_actual_peak_luminance_flag(),
            parsed.num_rows_mastering_display_actual_peak_luminance(),
            parsed.num_cols_mastering_display_actual_peak_luminance(),
            "mastering display",
        )?;
        Ok(parsed)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn itu_t_t35_country_code(self) -> u8 {
        self.data[0]
    }

    pub fn application_version(self) -> u8 {
        self.data[1]
    }

    pub const fn num_windows(self) -> usize {
        self.num_windows
    }

    pub fn color_transform_params(
        self,
        index: usize,
    ) -> Option<FrameHdrPlusColorTransformParams<'a>> {
        (index < self.num_windows).then(|| {
            let offset = Self::PARAMS_OFFSET + index * FrameHdrPlusColorTransformParams::DATA_LEN;
            FrameHdrPlusColorTransformParams {
                data: &self.data[offset..offset + FrameHdrPlusColorTransformParams::DATA_LEN],
            }
        })
    }

    pub fn targeted_system_display_maximum_luminance(self) -> Rational {
        Self::read_rational(
            self.data,
            Self::TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET,
        )
    }

    pub fn targeted_system_display_actual_peak_luminance_flag(self) -> u8 {
        self.data[Self::TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET]
    }

    pub fn num_rows_targeted_system_display_actual_peak_luminance(self) -> usize {
        usize::from(self.data[Self::NUM_ROWS_TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET])
    }

    pub fn num_cols_targeted_system_display_actual_peak_luminance(self) -> usize {
        usize::from(self.data[Self::NUM_COLS_TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET])
    }

    pub fn targeted_system_display_actual_peak_luminance(
        self,
        row: usize,
        col: usize,
    ) -> Option<Rational> {
        self.peak_luminance_value(
            self.targeted_system_display_actual_peak_luminance_flag(),
            self.num_rows_targeted_system_display_actual_peak_luminance(),
            self.num_cols_targeted_system_display_actual_peak_luminance(),
            Self::TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET,
            row,
            col,
        )
    }

    pub fn mastering_display_actual_peak_luminance_flag(self) -> u8 {
        self.data[Self::MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET]
    }

    pub fn num_rows_mastering_display_actual_peak_luminance(self) -> usize {
        usize::from(self.data[Self::NUM_ROWS_MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET])
    }

    pub fn num_cols_mastering_display_actual_peak_luminance(self) -> usize {
        usize::from(self.data[Self::NUM_COLS_MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET])
    }

    pub fn mastering_display_actual_peak_luminance(
        self,
        row: usize,
        col: usize,
    ) -> Option<Rational> {
        self.peak_luminance_value(
            self.mastering_display_actual_peak_luminance_flag(),
            self.num_rows_mastering_display_actual_peak_luminance(),
            self.num_cols_mastering_display_actual_peak_luminance(),
            Self::MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET,
            row,
            col,
        )
    }

    fn validate_peak_luminance_grid(
        self,
        flag: u8,
        rows: usize,
        cols: usize,
        name: &str,
    ) -> AvResult<()> {
        if flag > 1 {
            return Err(AvError::invalid_data(format!(
                "dynamic HDR10+ {name} actual peak luminance flag {flag} is not boolean"
            )));
        }
        if flag == 1
            && (!(2..=Self::MAX_PEAK_LUMINANCE_ROWS).contains(&rows)
                || !(2..=Self::MAX_PEAK_LUMINANCE_COLS).contains(&cols))
        {
            return Err(AvError::invalid_data(format!(
                "dynamic HDR10+ {name} actual peak luminance grid {rows}x{cols} is outside 2..=25"
            )));
        }
        Ok(())
    }

    fn peak_luminance_value(
        self,
        flag: u8,
        rows: usize,
        cols: usize,
        table_offset: usize,
        row: usize,
        col: usize,
    ) -> Option<Rational> {
        (flag == 1 && row < rows && col < cols).then(|| {
            Self::read_rational(
                self.data,
                table_offset + (row * Self::MAX_PEAK_LUMINANCE_COLS + col) * 8,
            )
        })
    }

    fn read_rational(data: &[u8], offset: usize) -> Rational {
        Rational::from_raw(
            FrameHdrPlusColorTransformParams::read_i32(data, offset),
            FrameHdrPlusColorTransformParams::read_i32(data, offset + 4),
        )
    }
}

impl<'a> FrameHdrPlusColorTransformParams<'a> {
    fn validate(self) -> AvResult<()> {
        Self::parse(self.data).map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSeiUnregistered<'a> {
    uuid: [u8; 16],
    user_data: &'a [u8],
}

impl<'a> FrameSeiUnregistered<'a> {
    pub const UUID_LEN: usize = 16;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::UUID_LEN {
            return Err(AvError::invalid_data(format!(
                "SEI unregistered frame side data requires at least {} UUID bytes, got {}",
                Self::UUID_LEN,
                data.len()
            )));
        }

        let mut uuid = [0; Self::UUID_LEN];
        uuid.copy_from_slice(&data[..Self::UUID_LEN]);
        Ok(Self {
            uuid,
            user_data: &data[Self::UUID_LEN..],
        })
    }

    pub const fn uuid(&self) -> [u8; 16] {
        self.uuid
    }

    pub fn user_data(&self) -> &'a [u8] {
        self.user_data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameSideDataKind {
    PanScan,
    A53ClosedCaptions,
    Stereo3d,
    MatrixEncoding,
    DownmixInfo,
    ReplayGain,
    DisplayMatrix,
    ActiveFormatDescription,
    MotionVectors,
    SkipSamples,
    AudioServiceType,
    MasteringDisplayMetadata,
    GopTimecode,
    Spherical,
    ContentLightLevel,
    IccProfile,
    S12mTimecode,
    DynamicHdrPlus,
    RegionsOfInterest,
    VideoEncParams,
    SeiUnregistered,
    FilmGrainParams,
    DetectionBboxes,
    DolbyVisionRpuBuffer,
    DolbyVisionMetadata,
    DynamicHdrVivid,
    AmbientViewingEnvironment,
    VideoHint,
    Lcevc,
    ViewId,
    ThreeDReferenceDisplays,
    Exif,
    Unknown(String),
}

impl FrameSideDataKind {
    pub const KNOWN: &'static [Self] = &[
        Self::PanScan,
        Self::A53ClosedCaptions,
        Self::Stereo3d,
        Self::MatrixEncoding,
        Self::DownmixInfo,
        Self::ReplayGain,
        Self::DisplayMatrix,
        Self::ActiveFormatDescription,
        Self::MotionVectors,
        Self::SkipSamples,
        Self::AudioServiceType,
        Self::MasteringDisplayMetadata,
        Self::GopTimecode,
        Self::Spherical,
        Self::ContentLightLevel,
        Self::IccProfile,
        Self::S12mTimecode,
        Self::DynamicHdrPlus,
        Self::RegionsOfInterest,
        Self::VideoEncParams,
        Self::SeiUnregistered,
        Self::FilmGrainParams,
        Self::DetectionBboxes,
        Self::DolbyVisionRpuBuffer,
        Self::DolbyVisionMetadata,
        Self::DynamicHdrVivid,
        Self::AmbientViewingEnvironment,
        Self::VideoHint,
        Self::Lcevc,
        Self::ViewId,
        Self::ThreeDReferenceDisplays,
        Self::Exif,
    ];

    pub fn from_name(name: impl Into<String>) -> AvResult<Self> {
        let name = validate_frame_side_data_kind(name.into())?;
        Ok(Self::known_from_name(&name).unwrap_or(Self::Unknown(name)))
    }

    pub fn name(&self) -> &str {
        match self {
            Self::PanScan => "pan_scan",
            Self::A53ClosedCaptions => "a53_cc",
            Self::Stereo3d => "stereo3d",
            Self::MatrixEncoding => "matrix_encoding",
            Self::DownmixInfo => "downmix_info",
            Self::ReplayGain => "replaygain",
            Self::DisplayMatrix => "displaymatrix",
            Self::ActiveFormatDescription => "afd",
            Self::MotionVectors => "motion_vectors",
            Self::SkipSamples => "skip_samples",
            Self::AudioServiceType => "audio_service_type",
            Self::MasteringDisplayMetadata => "mastering_display_metadata",
            Self::GopTimecode => "gop_timecode",
            Self::Spherical => "spherical",
            Self::ContentLightLevel => "content_light_level",
            Self::IccProfile => "icc_profile",
            Self::S12mTimecode => "s12m_timecode",
            Self::DynamicHdrPlus => "dynamic_hdr_plus",
            Self::RegionsOfInterest => "regions_of_interest",
            Self::VideoEncParams => "video_enc_params",
            Self::SeiUnregistered => "sei_unregistered",
            Self::FilmGrainParams => "film_grain_params",
            Self::DetectionBboxes => "detection_bboxes",
            Self::DolbyVisionRpuBuffer => "dolby_vision_rpu_buffer",
            Self::DolbyVisionMetadata => "dolby_vision_metadata",
            Self::DynamicHdrVivid => "dynamic_hdr_vivid",
            Self::AmbientViewingEnvironment => "ambient_viewing_environment",
            Self::VideoHint => "video_hint",
            Self::Lcevc => "lcevc",
            Self::ViewId => "view_id",
            Self::ThreeDReferenceDisplays => "3d_reference_displays",
            Self::Exif => "exif",
            Self::Unknown(name) => name.as_str(),
        }
    }

    pub fn ffmpeg_constant(&self) -> Option<&'static str> {
        match self {
            Self::PanScan => Some("AV_FRAME_DATA_PANSCAN"),
            Self::A53ClosedCaptions => Some("AV_FRAME_DATA_A53_CC"),
            Self::Stereo3d => Some("AV_FRAME_DATA_STEREO3D"),
            Self::MatrixEncoding => Some("AV_FRAME_DATA_MATRIXENCODING"),
            Self::DownmixInfo => Some("AV_FRAME_DATA_DOWNMIX_INFO"),
            Self::ReplayGain => Some("AV_FRAME_DATA_REPLAYGAIN"),
            Self::DisplayMatrix => Some("AV_FRAME_DATA_DISPLAYMATRIX"),
            Self::ActiveFormatDescription => Some("AV_FRAME_DATA_AFD"),
            Self::MotionVectors => Some("AV_FRAME_DATA_MOTION_VECTORS"),
            Self::SkipSamples => Some("AV_FRAME_DATA_SKIP_SAMPLES"),
            Self::AudioServiceType => Some("AV_FRAME_DATA_AUDIO_SERVICE_TYPE"),
            Self::MasteringDisplayMetadata => Some("AV_FRAME_DATA_MASTERING_DISPLAY_METADATA"),
            Self::GopTimecode => Some("AV_FRAME_DATA_GOP_TIMECODE"),
            Self::Spherical => Some("AV_FRAME_DATA_SPHERICAL"),
            Self::ContentLightLevel => Some("AV_FRAME_DATA_CONTENT_LIGHT_LEVEL"),
            Self::IccProfile => Some("AV_FRAME_DATA_ICC_PROFILE"),
            Self::S12mTimecode => Some("AV_FRAME_DATA_S12M_TIMECODE"),
            Self::DynamicHdrPlus => Some("AV_FRAME_DATA_DYNAMIC_HDR_PLUS"),
            Self::RegionsOfInterest => Some("AV_FRAME_DATA_REGIONS_OF_INTEREST"),
            Self::VideoEncParams => Some("AV_FRAME_DATA_VIDEO_ENC_PARAMS"),
            Self::SeiUnregistered => Some("AV_FRAME_DATA_SEI_UNREGISTERED"),
            Self::FilmGrainParams => Some("AV_FRAME_DATA_FILM_GRAIN_PARAMS"),
            Self::DetectionBboxes => Some("AV_FRAME_DATA_DETECTION_BBOXES"),
            Self::DolbyVisionRpuBuffer => Some("AV_FRAME_DATA_DOVI_RPU_BUFFER"),
            Self::DolbyVisionMetadata => Some("AV_FRAME_DATA_DOVI_METADATA"),
            Self::DynamicHdrVivid => Some("AV_FRAME_DATA_DYNAMIC_HDR_VIVID"),
            Self::AmbientViewingEnvironment => Some("AV_FRAME_DATA_AMBIENT_VIEWING_ENVIRONMENT"),
            Self::VideoHint => Some("AV_FRAME_DATA_VIDEO_HINT"),
            Self::Lcevc => Some("AV_FRAME_DATA_LCEVC"),
            Self::ViewId => Some("AV_FRAME_DATA_VIEW_ID"),
            Self::ThreeDReferenceDisplays => Some("AV_FRAME_DATA_3D_REFERENCE_DISPLAYS"),
            Self::Exif => Some("AV_FRAME_DATA_EXIF"),
            Self::Unknown(_) => None,
        }
    }

    pub fn descriptor(&self) -> Option<FrameSideDataDescriptor> {
        use FrameSideDataProperties as Props;

        Some(match self {
            Self::PanScan => FrameSideDataDescriptor::new("AVPanScan", Props::SIZE_DEPENDENT),
            Self::A53ClosedCaptions => {
                FrameSideDataDescriptor::new("ATSC A53 Part 4 Closed Captions", Props::EMPTY)
            }
            Self::Stereo3d => FrameSideDataDescriptor::new("Stereo 3D", Props::GLOBAL),
            Self::MatrixEncoding => {
                FrameSideDataDescriptor::new("AVMatrixEncoding", Props::CHANNEL_DEPENDENT)
            }
            Self::DownmixInfo => FrameSideDataDescriptor::new(
                "Metadata relevant to a downmix procedure",
                Props::CHANNEL_DEPENDENT,
            ),
            Self::ReplayGain => FrameSideDataDescriptor::new("AVReplayGain", Props::GLOBAL),
            Self::DisplayMatrix => FrameSideDataDescriptor::new("3x3 displaymatrix", Props::GLOBAL),
            Self::ActiveFormatDescription => {
                FrameSideDataDescriptor::new("Active format description", Props::EMPTY)
            }
            Self::MotionVectors => {
                FrameSideDataDescriptor::new("Motion vectors", Props::SIZE_DEPENDENT)
            }
            Self::SkipSamples => FrameSideDataDescriptor::new("Skip samples", Props::EMPTY),
            Self::AudioServiceType => {
                FrameSideDataDescriptor::new("Audio service type", Props::GLOBAL)
            }
            Self::MasteringDisplayMetadata => FrameSideDataDescriptor::new(
                "Mastering display metadata",
                Props::GLOBAL.union(Props::COLOR_DEPENDENT),
            ),
            Self::GopTimecode => FrameSideDataDescriptor::new("GOP timecode", Props::EMPTY),
            Self::Spherical => FrameSideDataDescriptor::new(
                "Spherical Mapping",
                Props::GLOBAL.union(Props::SIZE_DEPENDENT),
            ),
            Self::ContentLightLevel => FrameSideDataDescriptor::new(
                "Content light level metadata",
                Props::GLOBAL.union(Props::COLOR_DEPENDENT),
            ),
            Self::IccProfile => FrameSideDataDescriptor::new(
                "ICC profile",
                Props::GLOBAL.union(Props::COLOR_DEPENDENT),
            ),
            Self::S12mTimecode => FrameSideDataDescriptor::new("SMPTE 12-1 timecode", Props::EMPTY),
            Self::DynamicHdrPlus => FrameSideDataDescriptor::new(
                "HDR Dynamic Metadata SMPTE2094-40 (HDR10+)",
                Props::COLOR_DEPENDENT,
            ),
            Self::RegionsOfInterest => {
                FrameSideDataDescriptor::new("Regions Of Interest", Props::SIZE_DEPENDENT)
            }
            Self::VideoEncParams => {
                FrameSideDataDescriptor::new("Video encoding parameters", Props::EMPTY)
            }
            Self::SeiUnregistered => FrameSideDataDescriptor::new(
                "H.26[45] User Data Unregistered SEI message",
                Props::MULTI,
            ),
            Self::FilmGrainParams => {
                FrameSideDataDescriptor::new("Film grain parameters", Props::EMPTY)
            }
            Self::DetectionBboxes => FrameSideDataDescriptor::new(
                "Bounding boxes for object detection and classification",
                Props::SIZE_DEPENDENT,
            ),
            Self::DolbyVisionRpuBuffer => {
                FrameSideDataDescriptor::new("Dolby Vision RPU Data", Props::COLOR_DEPENDENT)
            }
            Self::DolbyVisionMetadata => {
                FrameSideDataDescriptor::new("Dolby Vision Metadata", Props::COLOR_DEPENDENT)
            }
            Self::DynamicHdrVivid => FrameSideDataDescriptor::new(
                "HDR Dynamic Metadata CUVA 005.1 2021 (Vivid)",
                Props::COLOR_DEPENDENT,
            ),
            Self::AmbientViewingEnvironment => {
                FrameSideDataDescriptor::new("Ambient viewing environment", Props::GLOBAL)
            }
            Self::VideoHint => {
                FrameSideDataDescriptor::new("Encoding video hint", Props::SIZE_DEPENDENT)
            }
            Self::Lcevc => FrameSideDataDescriptor::new("LCEVC NAL data", Props::SIZE_DEPENDENT),
            Self::ViewId => FrameSideDataDescriptor::new("View ID", Props::EMPTY),
            Self::ThreeDReferenceDisplays => {
                FrameSideDataDescriptor::new("3D Reference Displays Information", Props::GLOBAL)
            }
            Self::Exif => FrameSideDataDescriptor::new("EXIF metadata", Props::GLOBAL),
            Self::Unknown(_) => return None,
        })
    }

    pub fn descriptor_name(&self) -> Option<&'static str> {
        self.descriptor().map(FrameSideDataDescriptor::name)
    }

    pub fn properties(&self) -> FrameSideDataProperties {
        self.descriptor()
            .map(FrameSideDataDescriptor::properties)
            .unwrap_or(FrameSideDataProperties::EMPTY)
    }

    pub fn supports_multiple_instances(&self) -> bool {
        self.properties().contains(FrameSideDataProperties::MULTI)
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    fn known_from_name(name: &str) -> Option<Self> {
        let normalized = normalize_frame_side_data_name(name);
        let normalized = normalized
            .strip_prefix("av_frame_data_")
            .unwrap_or(normalized.as_str());
        match normalized {
            "panscan" | "pan_scan" | "avpanscan" => Some(Self::PanScan),
            "a53cc" | "a53_cc" | "a53_closed_captions" | "atsc_a53_closed_captions" => {
                Some(Self::A53ClosedCaptions)
            }
            "stereo3d" | "stereo_3d" => Some(Self::Stereo3d),
            "matrixencoding" | "matrix_encoding" => Some(Self::MatrixEncoding),
            "downmixinfo" | "downmix_info" => Some(Self::DownmixInfo),
            "replaygain" | "replay_gain" => Some(Self::ReplayGain),
            "displaymatrix" | "display_matrix" => Some(Self::DisplayMatrix),
            "afd" | "active_format_description" => Some(Self::ActiveFormatDescription),
            "motionvectors" | "motion_vectors" => Some(Self::MotionVectors),
            "skipsamples" | "skip_samples" => Some(Self::SkipSamples),
            "audioservicetype" | "audio_service_type" => Some(Self::AudioServiceType),
            "masteringdisplaymetadata" | "mastering_display_metadata" => {
                Some(Self::MasteringDisplayMetadata)
            }
            "goptimecode" | "gop_timecode" => Some(Self::GopTimecode),
            "spherical" => Some(Self::Spherical),
            "contentlightlevel" | "content_light_level" => Some(Self::ContentLightLevel),
            "iccprofile" | "icc_profile" => Some(Self::IccProfile),
            "s12mtimecode" | "s12m_timecode" => Some(Self::S12mTimecode),
            "dynamichdrplus" | "dynamic_hdr_plus" | "hdr_plus" => Some(Self::DynamicHdrPlus),
            "roi" | "region_of_interest" | "regions_of_interest" => Some(Self::RegionsOfInterest),
            "videoencparams" | "video_enc_params" => Some(Self::VideoEncParams),
            "seiunregistered" | "sei_unregistered" => Some(Self::SeiUnregistered),
            "filmgrainparams" | "film_grain_params" => Some(Self::FilmGrainParams),
            "detectionbboxes" | "detection_bboxes" => Some(Self::DetectionBboxes),
            "dolbyvisionrpubuffer" | "dolby_vision_rpu_buffer" | "dovi_rpu_buffer" => {
                Some(Self::DolbyVisionRpuBuffer)
            }
            "dolbyvisionmetadata" | "dolby_vision_metadata" | "dovi_metadata" => {
                Some(Self::DolbyVisionMetadata)
            }
            "dynamichdrvivid" | "dynamic_hdr_vivid" | "hdr_vivid" => Some(Self::DynamicHdrVivid),
            "ambientviewingenvironment" | "ambient_viewing_environment" => {
                Some(Self::AmbientViewingEnvironment)
            }
            "videohint" | "video_hint" => Some(Self::VideoHint),
            "lcevc" => Some(Self::Lcevc),
            "viewid" | "view_id" => Some(Self::ViewId),
            "3dreferencedisplays" | "3d_reference_displays" | "three_d_reference_displays" => {
                Some(Self::ThreeDReferenceDisplays)
            }
            "exif" => Some(Self::Exif),
            _ => None,
        }
    }
}

impl TryFrom<&str> for FrameSideDataKind {
    type Error = AvError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_name(value)
    }
}

impl TryFrom<String> for FrameSideDataKind {
    type Error = AvError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_name(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSideData {
    kind: FrameSideDataKind,
    buffer: BufferRef,
    metadata: Dictionary,
}

impl FrameSideData {
    pub fn new(kind: impl Into<String>, data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_buffer_ref(kind, BufferRef::from_vec(data))
    }

    pub fn new_active_format_description(value: FrameActiveFormatDescription) -> AvResult<Self> {
        Self::new_with_kind(
            FrameSideDataKind::ActiveFormatDescription,
            vec![value.as_byte()],
        )
    }

    pub fn new_motion_vectors(value: FrameMotionVectors) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::MotionVectors, value.to_bytes())
    }

    pub fn new_display_matrix(value: FrameDisplayMatrix) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::DisplayMatrix, value.to_bytes().to_vec())
    }

    pub fn new_matrix_encoding(value: FrameMatrixEncoding) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::MatrixEncoding, value.to_bytes().to_vec())
    }

    pub fn new_downmix_info(value: FrameDownmixInfo) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::DownmixInfo, value.to_bytes().to_vec())
    }

    pub fn new_replay_gain(value: FrameReplayGain) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::ReplayGain, value.to_bytes().to_vec())
    }

    pub fn new_skip_samples(value: FrameSkipSamples) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::SkipSamples, value.to_bytes().to_vec())
    }

    pub fn new_audio_service_type(value: FrameAudioServiceType) -> AvResult<Self> {
        Self::new_with_kind(
            FrameSideDataKind::AudioServiceType,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_mastering_display_metadata(value: FrameMasteringDisplayMetadata) -> AvResult<Self> {
        Self::new_with_kind(
            FrameSideDataKind::MasteringDisplayMetadata,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_gop_timecode(value: FrameGopTimecode) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::GopTimecode, value.to_bytes().to_vec())
    }

    pub fn new_spherical_mapping(value: FrameSphericalMapping) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::Spherical, value.to_bytes().to_vec())
    }

    pub fn new_content_light_metadata(value: FrameContentLightMetadata) -> AvResult<Self> {
        Self::new_with_kind(
            FrameSideDataKind::ContentLightLevel,
            value.to_bytes().to_vec(),
        )
    }

    pub fn new_icc_profile(data: Vec<u8>, name: Option<&str>) -> AvResult<Self> {
        let mut side_data = Self::new_with_kind(FrameSideDataKind::IccProfile, data)?;
        if let Some(name) = name {
            side_data.metadata_mut().set("name", name)?;
        }
        FrameIccProfile::parse(side_data.data(), side_data.metadata())?;
        Ok(side_data)
    }

    pub fn new_s12m_timecode(value: FrameS12mTimecode) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::S12mTimecode, value.to_bytes().to_vec())
    }

    pub fn new_dynamic_hdr_plus(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(FrameSideDataKind::DynamicHdrPlus, data)?;
        FrameDynamicHdrPlus::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_sei_unregistered(uuid: [u8; 16], user_data: Vec<u8>) -> AvResult<Self> {
        let total_len = FrameSeiUnregistered::UUID_LEN
            .checked_add(user_data.len())
            .ok_or_else(|| AvError::invalid_argument("SEI unregistered payload length overflow"))?;
        let mut data = Vec::with_capacity(total_len);
        data.extend_from_slice(&uuid);
        data.extend_from_slice(&user_data);
        Self::new_with_kind(FrameSideDataKind::SeiUnregistered, data)
    }

    pub fn new_with_buffer_ref(kind: impl Into<String>, buffer: BufferRef) -> AvResult<Self> {
        Self::new_with_kind_and_buffer_ref(FrameSideDataKind::from_name(kind)?, buffer)
    }

    pub fn new_with_kind(kind: FrameSideDataKind, data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_kind_and_buffer_ref(kind, BufferRef::from_vec(data))
    }

    pub fn new_with_kind_and_buffer_ref(
        kind: FrameSideDataKind,
        buffer: BufferRef,
    ) -> AvResult<Self> {
        validate_frame_side_data_kind(kind.name().to_string())?;
        Ok(Self {
            kind,
            buffer,
            metadata: Dictionary::new(),
        })
    }

    pub fn kind(&self) -> &str {
        self.kind.name()
    }

    pub fn kind_id(&self) -> &FrameSideDataKind {
        &self.kind
    }

    pub fn is_known_kind(&self) -> bool {
        self.kind.is_known()
    }

    pub fn descriptor(&self) -> Option<FrameSideDataDescriptor> {
        self.kind.descriptor()
    }

    pub fn descriptor_name(&self) -> Option<&'static str> {
        self.kind.descriptor_name()
    }

    pub fn properties(&self) -> FrameSideDataProperties {
        self.kind.properties()
    }

    pub fn supports_multiple_instances(&self) -> bool {
        self.kind.supports_multiple_instances()
    }

    pub fn display_matrix(&self) -> AvResult<Option<FrameDisplayMatrix>> {
        if self.kind != FrameSideDataKind::DisplayMatrix {
            return Ok(None);
        }

        FrameDisplayMatrix::parse(self.data()).map(Some)
    }

    pub fn matrix_encoding(&self) -> AvResult<Option<FrameMatrixEncoding>> {
        if self.kind != FrameSideDataKind::MatrixEncoding {
            return Ok(None);
        }

        FrameMatrixEncoding::parse(self.data()).map(Some)
    }

    pub fn downmix_info(&self) -> AvResult<Option<FrameDownmixInfo>> {
        if self.kind != FrameSideDataKind::DownmixInfo {
            return Ok(None);
        }

        FrameDownmixInfo::parse(self.data()).map(Some)
    }

    pub fn replay_gain(&self) -> AvResult<Option<FrameReplayGain>> {
        if self.kind != FrameSideDataKind::ReplayGain {
            return Ok(None);
        }

        FrameReplayGain::parse(self.data()).map(Some)
    }

    pub fn active_format_description(&self) -> AvResult<Option<FrameActiveFormatDescription>> {
        if self.kind != FrameSideDataKind::ActiveFormatDescription {
            return Ok(None);
        }

        FrameActiveFormatDescription::parse(self.data()).map(Some)
    }

    pub fn motion_vectors(&self) -> AvResult<Option<FrameMotionVectors>> {
        if self.kind != FrameSideDataKind::MotionVectors {
            return Ok(None);
        }

        FrameMotionVectors::parse(self.data()).map(Some)
    }

    pub fn skip_samples(&self) -> AvResult<Option<FrameSkipSamples>> {
        if self.kind != FrameSideDataKind::SkipSamples {
            return Ok(None);
        }

        FrameSkipSamples::parse(self.data()).map(Some)
    }

    pub fn audio_service_type(&self) -> AvResult<Option<FrameAudioServiceType>> {
        if self.kind != FrameSideDataKind::AudioServiceType {
            return Ok(None);
        }

        FrameAudioServiceType::parse(self.data()).map(Some)
    }

    pub fn mastering_display_metadata(&self) -> AvResult<Option<FrameMasteringDisplayMetadata>> {
        if self.kind != FrameSideDataKind::MasteringDisplayMetadata {
            return Ok(None);
        }

        FrameMasteringDisplayMetadata::parse(self.data()).map(Some)
    }

    pub fn gop_timecode(&self) -> AvResult<Option<FrameGopTimecode>> {
        if self.kind != FrameSideDataKind::GopTimecode {
            return Ok(None);
        }

        FrameGopTimecode::parse(self.data()).map(Some)
    }

    pub fn spherical_mapping(&self) -> AvResult<Option<FrameSphericalMapping>> {
        if self.kind != FrameSideDataKind::Spherical {
            return Ok(None);
        }

        FrameSphericalMapping::parse(self.data()).map(Some)
    }

    pub fn content_light_metadata(&self) -> AvResult<Option<FrameContentLightMetadata>> {
        if self.kind != FrameSideDataKind::ContentLightLevel {
            return Ok(None);
        }

        FrameContentLightMetadata::parse(self.data()).map(Some)
    }

    pub fn icc_profile(&self) -> AvResult<Option<FrameIccProfile<'_>>> {
        if self.kind != FrameSideDataKind::IccProfile {
            return Ok(None);
        }

        FrameIccProfile::parse(self.data(), self.metadata()).map(Some)
    }

    pub fn s12m_timecode(&self) -> AvResult<Option<FrameS12mTimecode>> {
        if self.kind != FrameSideDataKind::S12mTimecode {
            return Ok(None);
        }

        FrameS12mTimecode::parse(self.data()).map(Some)
    }

    pub fn dynamic_hdr_plus(&self) -> AvResult<Option<FrameDynamicHdrPlus<'_>>> {
        if self.kind != FrameSideDataKind::DynamicHdrPlus {
            return Ok(None);
        }

        FrameDynamicHdrPlus::parse(self.data()).map(Some)
    }

    pub fn sei_unregistered(&self) -> AvResult<Option<FrameSeiUnregistered<'_>>> {
        if self.kind != FrameSideDataKind::SeiUnregistered {
            return Ok(None);
        }

        FrameSeiUnregistered::parse(self.data()).map(Some)
    }

    pub fn data(&self) -> &[u8] {
        self.buffer.as_slice()
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        self.buffer.make_mut()
    }

    pub fn buffer(&self) -> &BufferRef {
        &self.buffer
    }

    pub fn is_writable(&self) -> bool {
        self.buffer.is_writable()
    }

    pub fn make_writable(&mut self) {
        self.buffer.make_mut();
    }

    pub fn metadata(&self) -> &Dictionary {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut Dictionary {
        &mut self.metadata
    }
}

fn validate_frame_side_data_kind(kind: String) -> AvResult<String> {
    if kind.trim().is_empty() {
        return Err(AvError::invalid_argument(
            "frame side data kind must not be empty",
        ));
    }
    if kind.contains('\0') {
        return Err(AvError::invalid_argument(
            "frame side data kind must not contain NUL",
        ));
    }

    Ok(kind)
}

fn normalize_frame_side_data_name(name: &str) -> String {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFrame {
    width: usize,
    height: usize,
    pixel_format: PixelFormat,
    line_sizes: Vec<usize>,
    planes: Vec<Vec<u8>>,
    plane_buffers: Vec<BufferRef>,
}

impl VideoFrame {
    pub fn aligned_line_sizes(
        pixel_format: PixelFormat,
        width: usize,
        height: usize,
        alignment: usize,
    ) -> AvResult<Vec<usize>> {
        align_line_sizes(
            video_line_sizes(pixel_format, width, height)?,
            alignment,
            "video frame line size",
        )
    }

    pub fn new(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        planes: Vec<Vec<u8>>,
    ) -> AvResult<Self> {
        let plane_buffers = planes.into_iter().map(BufferRef::from_vec).collect();
        Self::new_with_buffer_refs(width, height, pixel_format, plane_buffers)
    }

    pub fn new_with_buffer_refs(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        plane_buffers: Vec<BufferRef>,
    ) -> AvResult<Self> {
        let line_sizes = video_line_sizes(pixel_format, width, height)?;
        Self::new_with_buffer_refs_and_line_sizes(
            width,
            height,
            pixel_format,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_aligned_line_sizes(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        planes: Vec<Vec<u8>>,
        alignment: usize,
    ) -> AvResult<Self> {
        let line_sizes = Self::aligned_line_sizes(pixel_format, width, height, alignment)?;
        Self::new_with_line_sizes(width, height, pixel_format, planes, line_sizes)
    }

    pub fn new_with_line_sizes(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        planes: Vec<Vec<u8>>,
        line_sizes: Vec<usize>,
    ) -> AvResult<Self> {
        let plane_buffers = planes.into_iter().map(BufferRef::from_vec).collect();
        Self::new_with_buffer_refs_and_line_sizes(
            width,
            height,
            pixel_format,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_buffer_refs_and_aligned_line_sizes(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        plane_buffers: Vec<BufferRef>,
        alignment: usize,
    ) -> AvResult<Self> {
        let line_sizes = Self::aligned_line_sizes(pixel_format, width, height, alignment)?;
        Self::new_with_buffer_refs_and_line_sizes(
            width,
            height,
            pixel_format,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_buffer_refs_and_line_sizes(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        plane_buffers: Vec<BufferRef>,
        line_sizes: Vec<usize>,
    ) -> AvResult<Self> {
        if width == 0 || height == 0 {
            return Err(AvError::invalid_argument(
                "video frame dimensions must be non-zero",
            ));
        }

        let plane_shapes = video_plane_shapes(pixel_format, width, height)?;
        let planes = snapshot_video_plane_buffers(
            &plane_buffers,
            &plane_shapes,
            &line_sizes,
            pixel_format.name(),
        )?;

        Ok(Self {
            width,
            height,
            pixel_format,
            line_sizes,
            planes,
            plane_buffers,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    pub fn pixel_format_name(&self) -> &'static str {
        self.pixel_format.name()
    }

    pub fn line_sizes(&self) -> &[usize] {
        &self.line_sizes
    }

    pub fn planes(&self) -> &[Vec<u8>] {
        &self.planes
    }

    pub fn plane_buffers(&self) -> &[BufferRef] {
        &self.plane_buffers
    }

    pub fn is_writable(&self) -> bool {
        self.plane_buffers.iter().all(BufferRef::is_writable)
    }

    pub fn make_writable(&mut self) {
        for plane in &mut self.plane_buffers {
            plane.make_mut();
        }
    }

    pub fn set_plane_visible_data(&mut self, index: usize, data: &[u8]) -> AvResult<()> {
        let plane_shapes = video_plane_shapes(self.pixel_format, self.width, self.height)?;
        let Some(shape) = plane_shapes.get(index).copied() else {
            return Err(AvError::invalid_argument(format!(
                "{} video frame plane index {index} is out of range",
                self.pixel_format.name()
            )));
        };
        let expected_visible = shape.row_bytes.checked_mul(shape.rows).ok_or_else(|| {
            AvError::invalid_argument(format!(
                "{} video frame plane {index} visible size overflow",
                self.pixel_format.name()
            ))
        })?;
        if data.len() != expected_visible {
            return Err(AvError::invalid_data(format!(
                "{} video frame plane {index} visible data has {} bytes, expected {expected_visible}",
                self.pixel_format.name(),
                data.len()
            )));
        }

        let line_size = self.line_sizes[index];
        let storage = self.plane_buffers[index].make_mut();
        for row in 0..shape.rows {
            let src_start = row * shape.row_bytes;
            let src_end = src_start + shape.row_bytes;
            let dst_start = row * line_size;
            let dst_end = dst_start + shape.row_bytes;
            storage[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
        }
        self.planes[index].clear();
        self.planes[index].extend_from_slice(data);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFrame {
    sample_rate: u32,
    channels: u16,
    channel_layout: Option<ChannelLayout>,
    sample_format: SampleFormat,
    samples_per_channel: usize,
    line_sizes: Vec<usize>,
    planes: Vec<Vec<u8>>,
    plane_buffers: Vec<BufferRef>,
}

impl AudioFrame {
    pub fn aligned_line_sizes(
        sample_format: SampleFormat,
        samples_per_channel: usize,
        channels: u16,
        alignment: usize,
    ) -> AvResult<Vec<usize>> {
        align_line_sizes(
            sample_format.plane_sizes(samples_per_channel, channels)?,
            alignment,
            "audio frame line size",
        )
    }

    pub fn new(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        planes: Vec<Vec<u8>>,
    ) -> AvResult<Self> {
        let plane_buffers = planes.into_iter().map(BufferRef::from_vec).collect();
        Self::new_with_buffer_refs(
            sample_rate,
            channels,
            sample_format,
            samples_per_channel,
            plane_buffers,
        )
    }

    pub fn new_with_buffer_refs(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        plane_buffers: Vec<BufferRef>,
    ) -> AvResult<Self> {
        let line_sizes = sample_format.plane_sizes(samples_per_channel, channels)?;
        Self::new_with_buffer_refs_and_line_sizes(
            sample_rate,
            channels,
            sample_format,
            samples_per_channel,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_aligned_line_sizes(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        planes: Vec<Vec<u8>>,
        alignment: usize,
    ) -> AvResult<Self> {
        let line_sizes =
            Self::aligned_line_sizes(sample_format, samples_per_channel, channels, alignment)?;
        Self::new_with_line_sizes(
            sample_rate,
            channels,
            sample_format,
            samples_per_channel,
            planes,
            line_sizes,
        )
    }

    pub fn new_with_line_sizes(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        planes: Vec<Vec<u8>>,
        line_sizes: Vec<usize>,
    ) -> AvResult<Self> {
        let plane_buffers = planes.into_iter().map(BufferRef::from_vec).collect();
        Self::new_with_buffer_refs_and_line_sizes(
            sample_rate,
            channels,
            sample_format,
            samples_per_channel,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_buffer_refs_and_aligned_line_sizes(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        plane_buffers: Vec<BufferRef>,
        alignment: usize,
    ) -> AvResult<Self> {
        let line_sizes =
            Self::aligned_line_sizes(sample_format, samples_per_channel, channels, alignment)?;
        Self::new_with_buffer_refs_and_line_sizes(
            sample_rate,
            channels,
            sample_format,
            samples_per_channel,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_buffer_refs_and_line_sizes(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        plane_buffers: Vec<BufferRef>,
        line_sizes: Vec<usize>,
    ) -> AvResult<Self> {
        Self::new_inner(
            sample_rate,
            channels,
            ChannelLayout::default_for_count(channels),
            sample_format,
            samples_per_channel,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_channel_layout(
        sample_rate: u32,
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        planes: Vec<Vec<u8>>,
    ) -> AvResult<Self> {
        let plane_buffers = planes.into_iter().map(BufferRef::from_vec).collect();
        Self::new_with_channel_layout_and_buffer_refs(
            sample_rate,
            channel_layout,
            sample_format,
            samples_per_channel,
            plane_buffers,
        )
    }

    pub fn new_with_channel_layout_and_buffer_refs(
        sample_rate: u32,
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        plane_buffers: Vec<BufferRef>,
    ) -> AvResult<Self> {
        let channels = channel_layout.channel_count();
        let line_sizes = sample_format.plane_sizes(samples_per_channel, channels)?;
        Self::new_with_channel_layout_and_buffer_refs_and_line_sizes(
            sample_rate,
            channel_layout,
            sample_format,
            samples_per_channel,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_channel_layout_and_aligned_line_sizes(
        sample_rate: u32,
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        planes: Vec<Vec<u8>>,
        alignment: usize,
    ) -> AvResult<Self> {
        let line_sizes = Self::aligned_line_sizes(
            sample_format,
            samples_per_channel,
            channel_layout.channel_count(),
            alignment,
        )?;
        Self::new_with_channel_layout_and_line_sizes(
            sample_rate,
            channel_layout,
            sample_format,
            samples_per_channel,
            planes,
            line_sizes,
        )
    }

    pub fn new_with_channel_layout_and_line_sizes(
        sample_rate: u32,
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        planes: Vec<Vec<u8>>,
        line_sizes: Vec<usize>,
    ) -> AvResult<Self> {
        let plane_buffers = planes.into_iter().map(BufferRef::from_vec).collect();
        Self::new_with_channel_layout_and_buffer_refs_and_line_sizes(
            sample_rate,
            channel_layout,
            sample_format,
            samples_per_channel,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_channel_layout_and_buffer_refs_and_aligned_line_sizes(
        sample_rate: u32,
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        plane_buffers: Vec<BufferRef>,
        alignment: usize,
    ) -> AvResult<Self> {
        let line_sizes = Self::aligned_line_sizes(
            sample_format,
            samples_per_channel,
            channel_layout.channel_count(),
            alignment,
        )?;
        Self::new_with_channel_layout_and_buffer_refs_and_line_sizes(
            sample_rate,
            channel_layout,
            sample_format,
            samples_per_channel,
            plane_buffers,
            line_sizes,
        )
    }

    pub fn new_with_channel_layout_and_buffer_refs_and_line_sizes(
        sample_rate: u32,
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        plane_buffers: Vec<BufferRef>,
        line_sizes: Vec<usize>,
    ) -> AvResult<Self> {
        let channels = channel_layout.channel_count();
        Self::new_inner(
            sample_rate,
            channels,
            Some(channel_layout),
            sample_format,
            samples_per_channel,
            plane_buffers,
            line_sizes,
        )
    }

    fn new_inner(
        sample_rate: u32,
        channels: u16,
        channel_layout: Option<ChannelLayout>,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        plane_buffers: Vec<BufferRef>,
        line_sizes: Vec<usize>,
    ) -> AvResult<Self> {
        if sample_rate == 0 {
            return Err(AvError::invalid_argument(
                "audio sample rate must be non-zero",
            ));
        }

        if channels == 0 {
            return Err(AvError::invalid_argument(
                "audio channel count must be non-zero",
            ));
        }

        if let Some(layout) = channel_layout {
            layout.validate_channel_count(channels)?;
        }

        let visible_plane_sizes = sample_format.plane_sizes(samples_per_channel, channels)?;
        let planes = snapshot_audio_plane_buffers(
            &plane_buffers,
            &visible_plane_sizes,
            &line_sizes,
            sample_format.name(),
        )?;

        Ok(Self {
            sample_rate,
            channels,
            channel_layout,
            sample_format,
            samples_per_channel,
            line_sizes,
            planes,
            plane_buffers,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn channel_layout(&self) -> Option<ChannelLayout> {
        self.channel_layout
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    pub fn sample_format_name(&self) -> &'static str {
        self.sample_format.name()
    }

    pub fn samples_per_channel(&self) -> usize {
        self.samples_per_channel
    }

    pub fn line_sizes(&self) -> &[usize] {
        &self.line_sizes
    }

    pub fn planes(&self) -> &[Vec<u8>] {
        &self.planes
    }

    pub fn plane_buffers(&self) -> &[BufferRef] {
        &self.plane_buffers
    }

    pub fn is_writable(&self) -> bool {
        self.plane_buffers.iter().all(BufferRef::is_writable)
    }

    pub fn make_writable(&mut self) {
        for plane in &mut self.plane_buffers {
            plane.make_mut();
        }
    }

    pub fn set_plane_visible_data(&mut self, index: usize, data: &[u8]) -> AvResult<()> {
        let visible_plane_sizes = self
            .sample_format
            .plane_sizes(self.samples_per_channel, self.channels)?;
        let Some(expected_visible) = visible_plane_sizes.get(index).copied() else {
            return Err(AvError::invalid_argument(format!(
                "{} audio frame plane index {index} is out of range",
                self.sample_format.name()
            )));
        };
        if data.len() != expected_visible {
            return Err(AvError::invalid_data(format!(
                "{} audio frame plane {index} visible data has {} bytes, expected {expected_visible}",
                self.sample_format.name(),
                data.len()
            )));
        }

        self.plane_buffers[index].make_mut()[..expected_visible].copy_from_slice(data);
        self.planes[index].clear();
        self.planes[index].extend_from_slice(data);
        Ok(())
    }
}

fn snapshot_audio_plane_buffers(
    plane_buffers: &[BufferRef],
    visible_plane_sizes: &[usize],
    line_sizes: &[usize],
    format_name: &str,
) -> AvResult<Vec<Vec<u8>>> {
    if plane_buffers.len() != visible_plane_sizes.len() {
        return Err(AvError::invalid_argument(format!(
            "{format_name} audio frame expects {} planes, got {}",
            visible_plane_sizes.len(),
            plane_buffers.len()
        )));
    }
    if line_sizes.len() != visible_plane_sizes.len() {
        return Err(AvError::invalid_argument(format!(
            "{format_name} audio frame expects {} line sizes, got {}",
            visible_plane_sizes.len(),
            line_sizes.len()
        )));
    }

    let mut planes = Vec::with_capacity(plane_buffers.len());
    for (index, ((plane, visible_size), line_size)) in plane_buffers
        .iter()
        .zip(visible_plane_sizes)
        .zip(line_sizes)
        .enumerate()
    {
        if *line_size < *visible_size {
            return Err(AvError::invalid_argument(format!(
                "{format_name} audio frame plane {index} line size {line_size} is smaller than visible bytes {visible_size}"
            )));
        }
        if plane.len() != *line_size {
            return Err(AvError::invalid_data(format!(
                "{format_name} audio frame plane {index} has {} bytes, expected {line_size}",
                plane.len()
            )));
        }
        planes.push(plane.as_slice()[..*visible_size].to_vec());
    }
    Ok(planes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VideoPlaneShape {
    row_bytes: usize,
    rows: usize,
}

fn snapshot_video_plane_buffers(
    plane_buffers: &[BufferRef],
    plane_shapes: &[VideoPlaneShape],
    line_sizes: &[usize],
    format_name: &str,
) -> AvResult<Vec<Vec<u8>>> {
    if plane_buffers.len() != plane_shapes.len() {
        return Err(AvError::invalid_argument(format!(
            "{format_name} video frame expects {} planes, got {}",
            plane_shapes.len(),
            plane_buffers.len()
        )));
    }
    if line_sizes.len() != plane_shapes.len() {
        return Err(AvError::invalid_argument(format!(
            "{format_name} video frame expects {} line sizes, got {}",
            plane_shapes.len(),
            line_sizes.len()
        )));
    }

    let mut planes = Vec::with_capacity(plane_buffers.len());
    for (index, ((plane, shape), line_size)) in plane_buffers
        .iter()
        .zip(plane_shapes)
        .zip(line_sizes)
        .enumerate()
    {
        if *line_size < shape.row_bytes {
            return Err(AvError::invalid_argument(format!(
                "{format_name} video frame plane {index} line size {line_size} is smaller than visible row bytes {}",
                shape.row_bytes
            )));
        }
        let expected_storage = line_size.checked_mul(shape.rows).ok_or_else(|| {
            AvError::invalid_argument(format!(
                "{format_name} video frame plane {index} line size overflow"
            ))
        })?;
        if plane.len() != expected_storage {
            return Err(AvError::invalid_data(format!(
                "{format_name} video frame plane {index} has {} bytes, expected {expected_storage}",
                plane.len()
            )));
        }

        let mut visible =
            Vec::with_capacity(shape.row_bytes.checked_mul(shape.rows).ok_or_else(|| {
                AvError::invalid_argument(format!(
                    "{format_name} video frame plane {index} visible size overflow"
                ))
            })?);
        for row in 0..shape.rows {
            let start = row * line_size;
            let end = start + shape.row_bytes;
            visible.extend_from_slice(&plane.as_slice()[start..end]);
        }
        planes.push(visible);
    }
    Ok(planes)
}

fn video_plane_shapes(
    pixel_format: PixelFormat,
    width: usize,
    height: usize,
) -> AvResult<Vec<VideoPlaneShape>> {
    pixel_format.plane_sizes(width, height)?;

    match pixel_format {
        PixelFormat::Gray8 => Ok(vec![VideoPlaneShape {
            row_bytes: width,
            rows: height,
        }]),
        PixelFormat::Rgb24 => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 3, "rgb24 video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Rgba => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 4, "rgba video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Yuv420p => Ok(vec![
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
            VideoPlaneShape {
                row_bytes: width / 2,
                rows: height / 2,
            },
            VideoPlaneShape {
                row_bytes: width / 2,
                rows: height / 2,
            },
        ]),
    }
}

fn video_line_sizes(
    pixel_format: PixelFormat,
    width: usize,
    height: usize,
) -> AvResult<Vec<usize>> {
    Ok(video_plane_shapes(pixel_format, width, height)?
        .into_iter()
        .map(|shape| shape.row_bytes)
        .collect())
}

fn align_line_sizes(
    line_sizes: Vec<usize>,
    alignment: usize,
    context: &'static str,
) -> AvResult<Vec<usize>> {
    line_sizes
        .into_iter()
        .map(|line_size| align_size(line_size, alignment, context))
        .collect()
}

fn align_size(value: usize, alignment: usize, context: &'static str) -> AvResult<usize> {
    if alignment == 0 {
        return Err(AvError::invalid_argument(format!(
            "{context} alignment must be non-zero"
        )));
    }

    let remainder = value % alignment;
    if remainder == 0 {
        return Ok(value);
    }

    value
        .checked_add(alignment - remainder)
        .ok_or_else(|| AvError::invalid_argument(format!("{context} alignment overflow")))
}

fn checked_mul(value: usize, factor: usize, context: &'static str) -> AvResult<usize> {
    value
        .checked_mul(factor)
        .ok_or_else(|| AvError::invalid_argument(format!("{context} overflow")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    fn minimal_icc_profile() -> Vec<u8> {
        let mut data = vec![0; FrameIccProfile::MIN_DATA_LEN];
        data[0..4].copy_from_slice(&(FrameIccProfile::MIN_DATA_LEN as u32).to_be_bytes());
        data[8..12].copy_from_slice(&0x0430_0000u32.to_be_bytes());
        data[12..16].copy_from_slice(b"mntr");
        data[16..20].copy_from_slice(b"RGB ");
        data[20..24].copy_from_slice(b"XYZ ");
        data[36..40].copy_from_slice(&FrameIccProfile::ICC_SIGNATURE);
        data[128..132].copy_from_slice(&0u32.to_be_bytes());
        data
    }

    fn minimal_dynamic_hdr_plus() -> Vec<u8> {
        let mut data = vec![0; FrameDynamicHdrPlus::DATA_LEN];
        data[0] = FrameDynamicHdrPlus::ITU_T_T35_COUNTRY_CODE;
        data[1] = FrameDynamicHdrPlus::APPLICATION_VERSION;
        data[2] = 1;

        let params = FrameDynamicHdrPlus::PARAMS_OFFSET;
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::WINDOW_UPPER_LEFT_CORNER_X_OFFSET,
            Rational::from_raw(0, 1),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::WINDOW_UPPER_LEFT_CORNER_Y_OFFSET,
            Rational::from_raw(0, 1),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::WINDOW_LOWER_RIGHT_CORNER_X_OFFSET,
            Rational::from_raw(1, 1),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::WINDOW_LOWER_RIGHT_CORNER_Y_OFFSET,
            Rational::from_raw(1, 1),
        );
        write_ne_u16(
            &mut data,
            params + FrameHdrPlusColorTransformParams::CENTER_OF_ELLIPSE_X_OFFSET,
            640,
        );
        write_ne_u16(
            &mut data,
            params + FrameHdrPlusColorTransformParams::CENTER_OF_ELLIPSE_Y_OFFSET,
            360,
        );
        data[params + FrameHdrPlusColorTransformParams::ROTATION_ANGLE_OFFSET] = 45;
        write_ne_u16(
            &mut data,
            params + FrameHdrPlusColorTransformParams::SEMIMAJOR_AXIS_INTERNAL_ELLIPSE_OFFSET,
            10,
        );
        write_ne_u16(
            &mut data,
            params + FrameHdrPlusColorTransformParams::SEMIMAJOR_AXIS_EXTERNAL_ELLIPSE_OFFSET,
            20,
        );
        write_ne_u16(
            &mut data,
            params + FrameHdrPlusColorTransformParams::SEMIMINOR_AXIS_EXTERNAL_ELLIPSE_OFFSET,
            12,
        );
        write_ne_i32(
            &mut data,
            params + FrameHdrPlusColorTransformParams::OVERLAP_PROCESS_OPTION_OFFSET,
            FrameHdrPlusOverlapProcessOption::Layering.as_raw(),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::MAXSCL_OFFSET,
            Rational::from_raw(1, 100_000),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::MAXSCL_OFFSET + 8,
            Rational::from_raw(2, 100_000),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::MAXSCL_OFFSET + 16,
            Rational::from_raw(3, 100_000),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::AVERAGE_MAXRGB_OFFSET,
            Rational::from_raw(4, 100_000),
        );
        data[params
            + FrameHdrPlusColorTransformParams::NUM_DISTRIBUTION_MAXRGB_PERCENTILES_OFFSET] = 2;
        let distribution = params + FrameHdrPlusColorTransformParams::DISTRIBUTION_MAXRGB_OFFSET;
        data[distribution] = 50;
        write_ne_rational(&mut data, distribution + 4, Rational::from_raw(5, 100_000));
        data[distribution + FrameHdrPlusColorTransformParams::PERCENTILE_LEN] = 99;
        write_ne_rational(
            &mut data,
            distribution + FrameHdrPlusColorTransformParams::PERCENTILE_LEN + 4,
            Rational::from_raw(9, 100_000),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::FRACTION_BRIGHT_PIXELS_OFFSET,
            Rational::from_raw(1, 1000),
        );
        data[params + FrameHdrPlusColorTransformParams::TONE_MAPPING_FLAG_OFFSET] = 1;
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::KNEE_POINT_X_OFFSET,
            Rational::from_raw(1, 4095),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::KNEE_POINT_Y_OFFSET,
            Rational::from_raw(2, 4095),
        );
        data[params + FrameHdrPlusColorTransformParams::NUM_BEZIER_CURVE_ANCHORS_OFFSET] = 1;
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::BEZIER_CURVE_ANCHORS_OFFSET,
            Rational::from_raw(3, 1023),
        );
        data[params + FrameHdrPlusColorTransformParams::COLOR_SATURATION_MAPPING_FLAG_OFFSET] = 1;
        write_ne_rational(
            &mut data,
            params + FrameHdrPlusColorTransformParams::COLOR_SATURATION_WEIGHT_OFFSET,
            Rational::from_raw(8, 8),
        );

        write_ne_rational(
            &mut data,
            FrameDynamicHdrPlus::TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET,
            Rational::from_raw(1000, 1),
        );
        data[FrameDynamicHdrPlus::TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET] = 1;
        data[FrameDynamicHdrPlus::NUM_ROWS_TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET] =
            2;
        data[FrameDynamicHdrPlus::NUM_COLS_TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET] =
            2;
        write_ne_rational(
            &mut data,
            FrameDynamicHdrPlus::TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET,
            Rational::from_raw(1, 15),
        );
        write_ne_rational(
            &mut data,
            FrameDynamicHdrPlus::TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET + 26 * 8,
            Rational::from_raw(2, 15),
        );
        data
    }

    fn write_ne_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_ne_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_ne_rational(data: &mut [u8], offset: usize, value: Rational) {
        write_ne_i32(data, offset, value.num());
        write_ne_i32(data, offset + 4, value.den());
    }

    #[test]
    fn video_frame_validates_required_shape() {
        assert!(VideoFrame::new(0, 1, PixelFormat::Yuv420p, vec![vec![0]]).is_err());
        assert!(VideoFrame::new(1, 1, PixelFormat::Gray8, Vec::new()).is_err());
        assert!(VideoFrame::new(2, 2, PixelFormat::Gray8, vec![vec![0; 3]]).is_err());

        let frame = VideoFrame::new(2, 2, PixelFormat::Gray8, vec![vec![0, 1, 2, 3]]).unwrap();
        assert_eq!(frame.width(), 2);
        assert_eq!(frame.height(), 2);
        assert_eq!(frame.pixel_format(), PixelFormat::Gray8);
        assert_eq!(frame.pixel_format_name(), "gray");
        assert_eq!(frame.line_sizes(), &[2]);
    }

    #[test]
    fn audio_frame_validates_required_shape() {
        assert!(AudioFrame::new(0, 2, SampleFormat::S16, 1, vec![vec![0]]).is_err());
        assert!(AudioFrame::new(48_000, 0, SampleFormat::S16, 1, vec![vec![0]]).is_err());
        assert!(AudioFrame::new(48_000, 2, SampleFormat::S16, 1, vec![vec![0]]).is_err());

        let frame =
            AudioFrame::new(48_000, 2, SampleFormat::S16, 1024, vec![vec![0; 4096]]).unwrap();
        assert_eq!(frame.sample_rate(), 48_000);
        assert_eq!(frame.channels(), 2);
        assert_eq!(frame.channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(frame.sample_format(), SampleFormat::S16);
        assert_eq!(frame.sample_format_name(), "s16");
        assert_eq!(frame.samples_per_channel(), 1024);
        assert_eq!(frame.line_sizes(), &[4096]);

        let mono = AudioFrame::new_with_channel_layout(
            44_100,
            ChannelLayout::mono(),
            SampleFormat::S16,
            1,
            vec![vec![0; 2]],
        )
        .unwrap();
        assert_eq!(mono.channels(), 1);
        assert_eq!(mono.channel_layout(), Some(ChannelLayout::mono()));
        assert_eq!(mono.line_sizes(), &[2]);
    }

    #[test]
    fn video_frame_retains_buffer_ref_planes_and_visible_bytes() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let release_capture = std::sync::Arc::clone(&released);
        let plane = BufferRef::from_external_slice_with_len_and_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![1, 2, 3, 99]),
            3,
            String::from("video-plane"),
            move |opaque| {
                release_capture.lock().unwrap().push(opaque);
            },
        )
        .unwrap();
        let frame = VideoFrame::new_with_buffer_refs(3, 1, PixelFormat::Gray8, vec![plane.clone()])
            .unwrap();
        let cloned = frame.clone();

        assert_eq!(frame.planes(), &[vec![1, 2, 3]]);
        assert_eq!(frame.plane_buffers()[0].as_slice(), &[1, 2, 3]);
        assert_eq!(frame.plane_buffers()[0].padding_slice(), &[99]);
        assert!(frame.plane_buffers()[0].shares_storage(&plane));
        assert!(cloned.plane_buffers()[0].shares_storage(&frame.plane_buffers()[0]));

        drop(plane);
        assert!(released.lock().unwrap().is_empty());
        drop(frame);
        assert!(released.lock().unwrap().is_empty());
        drop(cloned);
        assert_eq!(*released.lock().unwrap(), vec![String::from("video-plane")]);
    }

    #[test]
    fn video_frame_buffer_refs_validate_plane_count_and_size() {
        assert_eq!(
            VideoFrame::new_with_buffer_refs(2, 2, PixelFormat::Gray8, Vec::new())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            VideoFrame::new_with_buffer_refs(
                2,
                2,
                PixelFormat::Gray8,
                vec![BufferRef::copy_from_slice(&[0; 3])],
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn video_frame_accepts_strided_planes_and_packs_visible_rows() {
        let gray = VideoFrame::new_with_line_sizes(
            3,
            2,
            PixelFormat::Gray8,
            vec![vec![1, 2, 3, 99, 99, 4, 5, 6, 88, 88]],
            vec![5],
        )
        .unwrap();
        assert_eq!(gray.line_sizes(), &[5]);
        assert_eq!(gray.planes(), &[vec![1, 2, 3, 4, 5, 6]]);
        assert_eq!(
            gray.plane_buffers()[0].as_slice(),
            &[1, 2, 3, 99, 99, 4, 5, 6, 88, 88]
        );

        let yuv = VideoFrame::new_with_line_sizes(
            4,
            2,
            PixelFormat::Yuv420p,
            vec![
                vec![0, 1, 2, 3, 90, 91, 4, 5, 6, 7, 92, 93],
                vec![10, 11, 94],
                vec![20, 21, 95],
            ],
            vec![6, 3, 3],
        )
        .unwrap();
        assert_eq!(yuv.line_sizes(), &[6, 3, 3]);
        assert_eq!(
            yuv.planes(),
            &[vec![0, 1, 2, 3, 4, 5, 6, 7], vec![10, 11], vec![20, 21]]
        );
    }

    #[test]
    fn video_frame_computes_aligned_line_sizes_and_validates_storage() {
        assert_eq!(
            VideoFrame::aligned_line_sizes(PixelFormat::Rgb24, 3, 2, 4).unwrap(),
            vec![12]
        );
        assert_eq!(
            VideoFrame::aligned_line_sizes(PixelFormat::Yuv420p, 4, 2, 4).unwrap(),
            vec![4, 4, 4]
        );

        let frame = VideoFrame::new_with_aligned_line_sizes(
            3,
            2,
            PixelFormat::Rgb24,
            vec![vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 0xaa, 0xbb, 0xcc, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                0xdd, 0xee, 0xff,
            ]],
            4,
        )
        .unwrap();
        assert_eq!(frame.line_sizes(), &[12]);
        assert_eq!(
            frame.planes(),
            &[vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18
            ]]
        );
        assert_eq!(frame.plane_buffers()[0].len(), 24);

        let source = BufferRef::copy_from_slice(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0xaa, 0xbb, 0xcc, 10, 11, 12, 13, 14, 15, 16, 17, 18, 0xdd,
            0xee, 0xff,
        ]);
        let buffered = VideoFrame::new_with_buffer_refs_and_aligned_line_sizes(
            3,
            2,
            PixelFormat::Rgb24,
            vec![source.clone()],
            4,
        )
        .unwrap();
        assert!(buffered.plane_buffers()[0].shares_storage(&source));
        assert_eq!(buffered.planes(), frame.planes());

        assert_eq!(
            VideoFrame::new_with_aligned_line_sizes(
                3,
                2,
                PixelFormat::Rgb24,
                vec![vec![0; 23]],
                4,
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn video_frame_rejects_invalid_custom_line_sizes() {
        assert_eq!(
            VideoFrame::new_with_line_sizes(
                3,
                2,
                PixelFormat::Gray8,
                vec![vec![0; 6]],
                Vec::new(),
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            VideoFrame::new_with_line_sizes(3, 2, PixelFormat::Gray8, vec![vec![0; 4]], vec![2],)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            VideoFrame::new_with_line_sizes(3, 2, PixelFormat::Gray8, vec![vec![0; 9]], vec![5],)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            VideoFrame::new_with_line_sizes(
                3,
                2,
                PixelFormat::Yuv420p,
                vec![vec![0; 6], vec![0; 1], vec![0; 1]],
                vec![3, 1, 1],
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn video_frame_mutates_visible_plane_data_with_copy_on_write() {
        let mut frame = VideoFrame::new_with_line_sizes(
            3,
            2,
            PixelFormat::Gray8,
            vec![vec![1, 2, 3, 99, 4, 5, 6, 88]],
            vec![4],
        )
        .unwrap();
        let cloned = frame.clone();
        assert!(!frame.is_writable());

        frame
            .set_plane_visible_data(0, &[10, 11, 12, 13, 14, 15])
            .unwrap();
        assert!(frame.is_writable());
        assert!(!frame.plane_buffers()[0].shares_storage(&cloned.plane_buffers()[0]));
        assert_eq!(frame.planes(), &[vec![10, 11, 12, 13, 14, 15]]);
        assert_eq!(
            frame.plane_buffers()[0].as_slice(),
            &[10, 11, 12, 99, 13, 14, 15, 88]
        );
        assert_eq!(cloned.planes(), &[vec![1, 2, 3, 4, 5, 6]]);
        assert_eq!(
            cloned.plane_buffers()[0].as_slice(),
            &[1, 2, 3, 99, 4, 5, 6, 88]
        );

        assert_eq!(
            frame.set_plane_visible_data(1, &[1]).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            frame
                .set_plane_visible_data(0, &[1, 2, 3])
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn video_frame_make_writable_detaches_readonly_plane_storage() {
        let source = BufferRef::from_vec_readonly(vec![1, 2, 3, 4]);
        let mut frame =
            VideoFrame::new_with_buffer_refs(2, 2, PixelFormat::Gray8, vec![source.clone()])
                .unwrap();
        assert!(!frame.is_writable());

        frame.make_writable();
        assert!(frame.is_writable());
        assert!(!frame.plane_buffers()[0].shares_storage(&source));
        assert_eq!(frame.plane_buffers()[0].as_slice(), &[1, 2, 3, 4]);
        assert_eq!(source.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn audio_frame_retains_buffer_ref_planes_and_visible_bytes() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let release_capture = std::sync::Arc::clone(&released);
        let plane = BufferRef::from_external_slice_with_len_and_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![0, 0, 1, 0, 55]),
            4,
            String::from("audio-plane"),
            move |opaque| {
                release_capture.lock().unwrap().push(opaque);
            },
        )
        .unwrap();
        let frame =
            AudioFrame::new_with_buffer_refs(48_000, 2, SampleFormat::S16, 1, vec![plane.clone()])
                .unwrap();
        let cloned = frame.clone();

        assert_eq!(frame.channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(frame.planes(), &[vec![0, 0, 1, 0]]);
        assert_eq!(frame.plane_buffers()[0].padding_slice(), &[55]);
        assert!(frame.plane_buffers()[0].shares_storage(&plane));
        assert!(cloned.plane_buffers()[0].shares_storage(&frame.plane_buffers()[0]));

        drop(plane);
        assert!(released.lock().unwrap().is_empty());
        drop(frame);
        assert!(released.lock().unwrap().is_empty());
        drop(cloned);
        assert_eq!(*released.lock().unwrap(), vec![String::from("audio-plane")]);
    }

    #[test]
    fn audio_frame_buffer_refs_validate_plane_count_size_and_layout() {
        assert_eq!(
            AudioFrame::new_with_buffer_refs(48_000, 2, SampleFormat::S16, 1, Vec::new())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            AudioFrame::new_with_buffer_refs(
                48_000,
                2,
                SampleFormat::S16,
                1,
                vec![BufferRef::copy_from_slice(&[0; 3])],
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            AudioFrame::new_with_channel_layout_and_buffer_refs(
                48_000,
                ChannelLayout::stereo(),
                SampleFormat::S16,
                1,
                vec![BufferRef::copy_from_slice(&[0; 2])],
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn audio_frame_accepts_custom_line_sizes_and_excludes_padding() {
        let mut frame = AudioFrame::new_with_line_sizes(
            48_000,
            2,
            SampleFormat::S16,
            2,
            vec![vec![0, 0, 1, 0, 2, 0, 3, 0, 0xaa, 0xbb]],
            vec![10],
        )
        .unwrap();
        assert_eq!(frame.channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(frame.line_sizes(), &[10]);
        assert_eq!(frame.planes(), &[vec![0, 0, 1, 0, 2, 0, 3, 0]]);
        assert_eq!(
            frame.plane_buffers()[0].as_slice(),
            &[0, 0, 1, 0, 2, 0, 3, 0, 0xaa, 0xbb]
        );

        let cloned = frame.clone();
        assert!(!frame.is_writable());
        frame
            .set_plane_visible_data(0, &[9, 0, 8, 0, 7, 0, 6, 0])
            .unwrap();
        assert!(frame.is_writable());
        assert!(!frame.plane_buffers()[0].shares_storage(&cloned.plane_buffers()[0]));
        assert_eq!(frame.planes(), &[vec![9, 0, 8, 0, 7, 0, 6, 0]]);
        assert_eq!(
            frame.plane_buffers()[0].as_slice(),
            &[9, 0, 8, 0, 7, 0, 6, 0, 0xaa, 0xbb]
        );
        assert_eq!(cloned.planes(), &[vec![0, 0, 1, 0, 2, 0, 3, 0]]);
        assert_eq!(
            cloned.plane_buffers()[0].as_slice(),
            &[0, 0, 1, 0, 2, 0, 3, 0, 0xaa, 0xbb]
        );

        let source = BufferRef::copy_from_slice(&[4, 0, 5, 0, 0xcc]);
        let buffered = AudioFrame::new_with_buffer_refs_and_line_sizes(
            44_100,
            1,
            SampleFormat::S16,
            2,
            vec![source.clone()],
            vec![5],
        )
        .unwrap();
        assert_eq!(buffered.channel_layout(), Some(ChannelLayout::mono()));
        assert_eq!(buffered.line_sizes(), &[5]);
        assert_eq!(buffered.planes(), &[vec![4, 0, 5, 0]]);
        assert!(buffered.plane_buffers()[0].shares_storage(&source));

        let layout_frame = AudioFrame::new_with_channel_layout_and_line_sizes(
            48_000,
            ChannelLayout::stereo(),
            SampleFormat::S16,
            1,
            vec![vec![1, 0, 2, 0, 0xdd]],
            vec![5],
        )
        .unwrap();
        assert_eq!(layout_frame.channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(layout_frame.planes(), &[vec![1, 0, 2, 0]]);
    }

    #[test]
    fn audio_frame_computes_aligned_line_sizes_and_validates_storage() {
        assert_eq!(
            AudioFrame::aligned_line_sizes(SampleFormat::S16, 3, 2, 8).unwrap(),
            vec![16]
        );
        assert_eq!(
            AudioFrame::aligned_line_sizes(SampleFormat::FltP, 2, 2, 16).unwrap(),
            vec![16, 16]
        );

        let packed = AudioFrame::new_with_aligned_line_sizes(
            48_000,
            2,
            SampleFormat::S16,
            3,
            vec![vec![
                0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 0xaa, 0xbb, 0xcc, 0xdd,
            ]],
            8,
        )
        .unwrap();
        assert_eq!(packed.channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(packed.line_sizes(), &[16]);
        assert_eq!(packed.planes(), &[vec![0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0]]);
        assert_eq!(packed.plane_buffers()[0].len(), 16);

        let planar = AudioFrame::new_with_aligned_line_sizes(
            44_100,
            2,
            SampleFormat::FltP,
            2,
            vec![vec![1; 16], vec![2; 16]],
            16,
        )
        .unwrap();
        assert_eq!(planar.sample_format_name(), "fltp");
        assert_eq!(planar.line_sizes(), &[16, 16]);
        assert_eq!(planar.planes(), &[vec![1; 8], vec![2; 8]]);

        let source = BufferRef::copy_from_slice(&[7; 16]);
        let layout_frame =
            AudioFrame::new_with_channel_layout_and_buffer_refs_and_aligned_line_sizes(
                44_100,
                ChannelLayout::mono(),
                SampleFormat::Flt,
                3,
                vec![source.clone()],
                16,
            )
            .unwrap();
        assert_eq!(layout_frame.channel_layout(), Some(ChannelLayout::mono()));
        assert_eq!(layout_frame.line_sizes(), &[16]);
        assert_eq!(layout_frame.planes(), &[vec![7; 12]]);
        assert!(layout_frame.plane_buffers()[0].shares_storage(&source));

        assert_eq!(
            AudioFrame::new_with_aligned_line_sizes(
                48_000,
                2,
                SampleFormat::S16,
                3,
                vec![vec![0; 15]],
                8,
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_alignment_rejects_zero_alignment_and_overflow() {
        assert_eq!(
            VideoFrame::aligned_line_sizes(PixelFormat::Gray8, 2, 2, 0)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            AudioFrame::aligned_line_sizes(SampleFormat::S16, 1, 1, 0)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            align_size(usize::MAX - 1, usize::MAX - 2, "test frame alignment")
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn audio_frame_supports_planar_sample_planes_and_line_sizes() {
        let planar = AudioFrame::new(
            48_000,
            2,
            SampleFormat::S16P,
            3,
            vec![vec![0, 0, 1, 0, 2, 0], vec![3, 0, 4, 0, 5, 0]],
        )
        .unwrap();
        assert_eq!(planar.channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(planar.sample_format_name(), "s16p");
        assert_eq!(planar.line_sizes(), &[6, 6]);
        assert_eq!(
            planar.planes(),
            &[vec![0, 0, 1, 0, 2, 0], vec![3, 0, 4, 0, 5, 0]]
        );
        assert_eq!(planar.plane_buffers().len(), 2);

        let mut padded = AudioFrame::new_with_line_sizes(
            44_100,
            2,
            SampleFormat::S16P,
            2,
            vec![vec![0, 0, 1, 0, 0xaa], vec![2, 0, 3, 0, 0xbb]],
            vec![5, 5],
        )
        .unwrap();
        assert_eq!(padded.line_sizes(), &[5, 5]);
        assert_eq!(padded.planes(), &[vec![0, 0, 1, 0], vec![2, 0, 3, 0]]);
        assert_eq!(padded.plane_buffers()[0].as_slice(), &[0, 0, 1, 0, 0xaa]);
        assert_eq!(padded.plane_buffers()[1].as_slice(), &[2, 0, 3, 0, 0xbb]);

        let cloned = padded.clone();
        assert!(!padded.is_writable());
        padded.set_plane_visible_data(1, &[4, 0, 5, 0]).unwrap();
        assert!(!padded.is_writable());
        assert!(padded.plane_buffers()[1].is_writable());
        assert!(!padded.plane_buffers()[1].shares_storage(&cloned.plane_buffers()[1]));
        assert!(padded.plane_buffers()[0].shares_storage(&cloned.plane_buffers()[0]));
        assert_eq!(padded.planes(), &[vec![0, 0, 1, 0], vec![4, 0, 5, 0]]);
        assert_eq!(padded.plane_buffers()[1].as_slice(), &[4, 0, 5, 0, 0xbb]);
        assert_eq!(cloned.planes(), &[vec![0, 0, 1, 0], vec![2, 0, 3, 0]]);

        padded.make_writable();
        assert!(padded.is_writable());
        assert!(!padded.plane_buffers()[0].shares_storage(&cloned.plane_buffers()[0]));

        assert_eq!(
            AudioFrame::new(48_000, 2, SampleFormat::S16P, 1, vec![vec![0; 2]])
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            AudioFrame::new(
                48_000,
                2,
                SampleFormat::S16P,
                1,
                vec![vec![0; 2], vec![0; 1]]
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            AudioFrame::new_with_channel_layout(
                48_000,
                ChannelLayout::stereo(),
                SampleFormat::S16P,
                1,
                vec![vec![0; 2], vec![0; 2]],
            )
            .unwrap()
            .channel_layout(),
            Some(ChannelLayout::stereo())
        );
    }

    #[test]
    fn audio_frame_supports_non_s16_sample_plane_sizes() {
        let packed_float =
            AudioFrame::new(48_000, 2, SampleFormat::Flt, 2, vec![vec![0; 16]]).unwrap();
        assert_eq!(packed_float.sample_format_name(), "flt");
        assert_eq!(packed_float.line_sizes(), &[16]);
        assert_eq!(packed_float.planes(), &[vec![0; 16]]);

        let mut planar_float = AudioFrame::new_with_line_sizes(
            44_100,
            2,
            SampleFormat::FltP,
            2,
            vec![vec![0; 8 + 1], vec![1; 8 + 1]],
            vec![9, 9],
        )
        .unwrap();
        assert_eq!(planar_float.sample_format_name(), "fltp");
        assert_eq!(planar_float.line_sizes(), &[9, 9]);
        assert_eq!(planar_float.planes(), &[vec![0; 8], vec![1; 8]]);
        assert_eq!(planar_float.plane_buffers()[0].as_slice(), &[0; 9]);
        assert_eq!(planar_float.plane_buffers()[1].as_slice(), &[1; 9]);

        let cloned = planar_float.clone();
        planar_float.set_plane_visible_data(0, &[2; 8]).unwrap();
        assert_eq!(planar_float.planes(), &[vec![2; 8], vec![1; 8]]);
        assert_eq!(
            planar_float.plane_buffers()[0].as_slice(),
            &[2, 2, 2, 2, 2, 2, 2, 2, 0]
        );
        assert_eq!(cloned.planes(), &[vec![0; 8], vec![1; 8]]);
        assert!(planar_float.plane_buffers()[1].shares_storage(&cloned.plane_buffers()[1]));
        assert!(!planar_float.plane_buffers()[0].shares_storage(&cloned.plane_buffers()[0]));
    }

    #[test]
    fn audio_frame_rejects_invalid_custom_line_sizes() {
        assert_eq!(
            AudioFrame::new_with_line_sizes(
                48_000,
                2,
                SampleFormat::S16,
                2,
                vec![vec![0; 8]],
                Vec::new(),
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            AudioFrame::new_with_line_sizes(
                48_000,
                2,
                SampleFormat::S16,
                2,
                vec![vec![0; 7]],
                vec![7],
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            AudioFrame::new_with_line_sizes(
                48_000,
                2,
                SampleFormat::S16,
                2,
                vec![vec![0; 9]],
                vec![10],
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            AudioFrame::new_with_buffer_refs(
                48_000,
                2,
                SampleFormat::S16,
                2,
                vec![BufferRef::copy_from_slice(&[0; 10])],
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn audio_frame_mutates_visible_plane_data_with_copy_on_write() {
        let mut frame =
            AudioFrame::new(48_000, 2, SampleFormat::S16, 1, vec![vec![0, 0, 1, 0]]).unwrap();
        let cloned = frame.clone();
        assert!(!frame.is_writable());

        frame.set_plane_visible_data(0, &[9, 0, 8, 0]).unwrap();
        assert!(frame.is_writable());
        assert!(!frame.plane_buffers()[0].shares_storage(&cloned.plane_buffers()[0]));
        assert_eq!(frame.planes(), &[vec![9, 0, 8, 0]]);
        assert_eq!(frame.plane_buffers()[0].as_slice(), &[9, 0, 8, 0]);
        assert_eq!(cloned.planes(), &[vec![0, 0, 1, 0]]);

        assert_eq!(
            frame.set_plane_visible_data(1, &[1]).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            frame.set_plane_visible_data(0, &[1, 2]).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn audio_frame_make_writable_detaches_readonly_plane_storage() {
        let source = BufferRef::from_vec_readonly(vec![0, 0, 1, 0]);
        let mut frame =
            AudioFrame::new_with_buffer_refs(48_000, 2, SampleFormat::S16, 1, vec![source.clone()])
                .unwrap();
        assert!(!frame.is_writable());

        frame.make_writable();
        assert!(frame.is_writable());
        assert!(!frame.plane_buffers()[0].shares_storage(&source));
        assert_eq!(frame.plane_buffers()[0].as_slice(), &[0, 0, 1, 0]);
        assert_eq!(source.as_slice(), &[0, 0, 1, 0]);
    }

    #[test]
    fn frames_report_tightly_packed_line_sizes() {
        let rgb = VideoFrame::new(3, 2, PixelFormat::Rgb24, vec![vec![0; 18]]).unwrap();
        assert_eq!(rgb.line_sizes(), &[9]);

        let rgba = VideoFrame::new(3, 2, PixelFormat::Rgba, vec![vec![0; 24]]).unwrap();
        assert_eq!(rgba.line_sizes(), &[12]);

        let yuv = VideoFrame::new(
            4,
            2,
            PixelFormat::Yuv420p,
            vec![vec![0; 8], vec![1; 2], vec![2; 2]],
        )
        .unwrap();
        assert_eq!(yuv.line_sizes(), &[4, 2, 2]);

        let audio = AudioFrame::new(48_000, 2, SampleFormat::S16, 3, vec![vec![0; 12]]).unwrap();
        assert_eq!(audio.line_sizes(), &[12]);
    }

    #[test]
    fn frame_wraps_audio_or_video_with_optional_pts() {
        let video = VideoFrame::new(1, 1, PixelFormat::Gray8, vec![vec![0]]).unwrap();
        let mut frame = Frame::video(video);

        assert_eq!(frame.pts(), None);
        assert!(frame.hw_frames_context().is_none());
        assert!(frame.side_data().is_empty());
        frame.set_pts(Some(42));
        assert_eq!(frame.pts(), Some(42));
        assert!(matches!(frame.data(), FrameData::Video(_)));
    }

    #[test]
    fn empty_frame_unref_clears_data_and_releases_references() {
        let plane_released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let plane_capture = std::sync::Arc::clone(&plane_released);
        let plane = BufferRef::from_external_slice_with_len_and_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![1, 2, 3, 0]),
            3,
            String::from("plane"),
            move |opaque| {
                plane_capture.lock().unwrap().push(opaque);
            },
        )
        .unwrap();
        let side_released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let side_capture = std::sync::Arc::clone(&side_released);
        let side = BufferRef::from_external_slice_with_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![0xAA]),
            String::from("side"),
            move |opaque| {
                side_capture.lock().unwrap().push(opaque);
            },
        );
        let hw_released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let hw_capture = std::sync::Arc::clone(&hw_released);
        let hw_context = BufferRef::from_external_slice_with_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![0xCC]),
            String::from("hw"),
            move |opaque| {
                hw_capture.lock().unwrap().push(opaque);
            },
        );

        let video =
            VideoFrame::new_with_buffer_refs(3, 1, PixelFormat::Gray8, vec![plane]).unwrap();
        let mut frame = Frame::video(video).with_hw_frames_context(hw_context);
        frame.set_pts(Some(99));
        frame
            .set_side_data_kind_buffer(FrameSideDataKind::DisplayMatrix, side)
            .unwrap();

        assert!(!frame.is_empty());
        frame.unref();

        assert!(frame.is_empty());
        assert_eq!(frame.pts(), None);
        assert!(matches!(frame.data(), FrameData::Empty));
        assert!(frame.hw_frames_context().is_none());
        assert!(frame.side_data().is_empty());
        assert!(!frame.is_writable());
        assert_eq!(
            frame.set_plane_visible_data(0, &[]).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(*plane_released.lock().unwrap(), vec![String::from("plane")]);
        assert_eq!(*side_released.lock().unwrap(), vec![String::from("side")]);
        assert_eq!(*hw_released.lock().unwrap(), vec![String::from("hw")]);

        let mut empty = Frame::empty();
        assert!(empty.is_empty());
        empty.unref();
        assert!(empty.is_empty());
    }

    #[test]
    fn frame_ref_from_shares_references_and_replaces_destination() {
        let source_plane = BufferRef::copy_from_slice(&[1, 2, 3]);
        let source_side = BufferRef::copy_from_slice(&[0x44]);
        let source_hw = BufferRef::copy_from_slice(&[0x55]);
        let source_video =
            VideoFrame::new_with_buffer_refs(3, 1, PixelFormat::Gray8, vec![source_plane.clone()])
                .unwrap();
        let mut source = Frame::video(source_video).with_hw_frames_context(source_hw.clone());
        source.set_pts(Some(7));
        source
            .set_side_data_kind_buffer(FrameSideDataKind::DisplayMatrix, source_side.clone())
            .unwrap();

        let old_released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let old_capture = std::sync::Arc::clone(&old_released);
        let old_plane = BufferRef::from_external_slice_with_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![9]),
            String::from("old-plane"),
            move |opaque| {
                old_capture.lock().unwrap().push(opaque);
            },
        );
        let old_video =
            VideoFrame::new_with_buffer_refs(1, 1, PixelFormat::Gray8, vec![old_plane]).unwrap();
        let mut destination = Frame::video(old_video);

        destination.ref_from(&source);

        assert_eq!(
            *old_released.lock().unwrap(),
            vec![String::from("old-plane")]
        );
        assert_eq!(destination.pts(), Some(7));
        assert!(!source.is_empty());
        let (destination_video, source_video) = match (destination.data(), source.data()) {
            (FrameData::Video(destination_video), FrameData::Video(source_video)) => {
                (destination_video, source_video)
            }
            _ => panic!("expected video frames"),
        };
        assert!(destination_video.plane_buffers()[0].shares_storage(&source_plane));
        assert!(
            destination_video.plane_buffers()[0].shares_storage(&source_video.plane_buffers()[0])
        );
        assert!(destination.side_data()[0]
            .buffer()
            .shares_storage(&source_side));
        assert!(destination.side_data()[0]
            .buffer()
            .shares_storage(source.side_data()[0].buffer()));
        assert!(destination
            .hw_frames_context()
            .unwrap()
            .shares_storage(&source_hw));
        assert!(destination
            .hw_frames_context()
            .unwrap()
            .shares_storage(source.hw_frames_context().unwrap()));
    }

    #[test]
    fn frame_move_ref_from_moves_references_and_unrefs_source() {
        let source_released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let source_capture = std::sync::Arc::clone(&source_released);
        let source_plane = BufferRef::from_external_slice_with_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![1]),
            String::from("source-plane"),
            move |opaque| {
                source_capture.lock().unwrap().push(opaque);
            },
        );
        let source_video =
            VideoFrame::new_with_buffer_refs(1, 1, PixelFormat::Gray8, vec![source_plane]).unwrap();
        let mut source = Frame::video(source_video);
        source.set_pts(Some(11));

        let destination_released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let destination_capture = std::sync::Arc::clone(&destination_released);
        let destination_plane = BufferRef::from_external_slice_with_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![9]),
            String::from("destination-plane"),
            move |opaque| {
                destination_capture.lock().unwrap().push(opaque);
            },
        );
        let destination_video =
            VideoFrame::new_with_buffer_refs(1, 1, PixelFormat::Gray8, vec![destination_plane])
                .unwrap();
        let mut destination = Frame::video(destination_video);

        destination.move_ref_from(&mut source);

        assert!(source.is_empty());
        assert_eq!(destination.pts(), Some(11));
        assert!(matches!(destination.data(), FrameData::Video(_)));
        assert_eq!(
            *destination_released.lock().unwrap(),
            vec![String::from("destination-plane")]
        );
        assert!(source_released.lock().unwrap().is_empty());

        drop(destination);
        assert_eq!(
            *source_released.lock().unwrap(),
            vec![String::from("source-plane")]
        );
    }

    #[test]
    fn frame_make_writable_detaches_video_payload_refs() {
        let source = BufferRef::from_vec_readonly(vec![1, 2, 3, 4]);
        let video =
            VideoFrame::new_with_buffer_refs(2, 2, PixelFormat::Gray8, vec![source.clone()])
                .unwrap();
        let side_data = BufferRef::copy_from_slice(&[0xAA, 0xBB]);
        let hw_context = BufferRef::copy_from_slice(&[0xCC]);
        let mut frame = Frame::video(video).with_hw_frames_context(hw_context.clone());
        frame
            .set_side_data_kind_buffer(FrameSideDataKind::DisplayMatrix, side_data.clone())
            .unwrap();
        let cloned = frame.clone();

        assert!(!frame.is_writable());
        frame.make_writable();
        assert!(frame.is_writable());

        let (frame_video, cloned_video) = match (frame.data(), cloned.data()) {
            (FrameData::Video(frame_video), FrameData::Video(cloned_video)) => {
                (frame_video, cloned_video)
            }
            _ => panic!("expected video frames"),
        };
        assert!(!frame_video.plane_buffers()[0].shares_storage(&source));
        assert!(cloned_video.plane_buffers()[0].shares_storage(&source));
        assert_eq!(frame_video.planes(), &[vec![1, 2, 3, 4]]);
        assert_eq!(cloned_video.planes(), &[vec![1, 2, 3, 4]]);
        assert!(frame.side_data()[0].buffer().shares_storage(&side_data));
        assert!(frame.side_data()[0]
            .buffer()
            .shares_storage(cloned.side_data()[0].buffer()));
        assert!(frame
            .hw_frames_context()
            .unwrap()
            .shares_storage(&hw_context));
        assert!(frame
            .hw_frames_context()
            .unwrap()
            .shares_storage(cloned.hw_frames_context().unwrap()));

        frame.set_plane_visible_data(0, &[4, 3, 2, 1]).unwrap();
        let (frame_video, cloned_video) = match (frame.data(), cloned.data()) {
            (FrameData::Video(frame_video), FrameData::Video(cloned_video)) => {
                (frame_video, cloned_video)
            }
            _ => panic!("expected video frames"),
        };
        assert_eq!(frame_video.planes(), &[vec![4, 3, 2, 1]]);
        assert_eq!(frame_video.plane_buffers()[0].as_slice(), &[4, 3, 2, 1]);
        assert_eq!(cloned_video.planes(), &[vec![1, 2, 3, 4]]);
        assert_eq!(source.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn frame_make_all_references_writable_detaches_side_data_and_hw_context() {
        let source = BufferRef::from_vec_readonly(vec![1]);
        let video =
            VideoFrame::new_with_buffer_refs(1, 1, PixelFormat::Gray8, vec![source.clone()])
                .unwrap();
        let side_data = BufferRef::from_vec_readonly(vec![0x10, 0x11]);
        let hw_context = BufferRef::from_vec_readonly(vec![0x20, 0x21]);
        let mut frame = Frame::video(video).with_hw_frames_context(hw_context.clone());
        frame
            .set_side_data_kind_buffer(FrameSideDataKind::DisplayMatrix, side_data.clone())
            .unwrap();
        let cloned = frame.clone();

        assert!(!frame.is_writable());
        assert!(!frame.side_data_is_writable());
        assert_eq!(frame.hw_frames_context_is_writable(), Some(false));
        assert!(!frame.all_references_are_writable());

        frame.make_all_references_writable();

        assert!(frame.is_writable());
        assert!(frame.side_data_is_writable());
        assert_eq!(frame.hw_frames_context_is_writable(), Some(true));
        assert!(frame.all_references_are_writable());

        let (frame_video, cloned_video) = match (frame.data(), cloned.data()) {
            (FrameData::Video(frame_video), FrameData::Video(cloned_video)) => {
                (frame_video, cloned_video)
            }
            _ => panic!("expected video frames"),
        };
        assert!(!frame_video.plane_buffers()[0].shares_storage(&source));
        assert!(cloned_video.plane_buffers()[0].shares_storage(&source));
        assert!(!frame.side_data()[0].buffer().shares_storage(&side_data));
        assert!(cloned.side_data()[0].buffer().shares_storage(&side_data));
        assert!(!frame.side_data()[0]
            .buffer()
            .shares_storage(cloned.side_data()[0].buffer()));
        assert!(!frame
            .hw_frames_context()
            .unwrap()
            .shares_storage(&hw_context));
        assert!(cloned
            .hw_frames_context()
            .unwrap()
            .shares_storage(&hw_context));
        assert!(!frame
            .hw_frames_context()
            .unwrap()
            .shares_storage(cloned.hw_frames_context().unwrap()));

        frame.set_plane_visible_data(0, &[9]).unwrap();
        frame
            .side_data_by_kind_mut(&FrameSideDataKind::DisplayMatrix)
            .unwrap()
            .data_mut()[0] = 0x99;

        let (frame_video, cloned_video) = match (frame.data(), cloned.data()) {
            (FrameData::Video(frame_video), FrameData::Video(cloned_video)) => {
                (frame_video, cloned_video)
            }
            _ => panic!("expected video frames"),
        };
        assert_eq!(frame_video.planes(), &[vec![9]]);
        assert_eq!(cloned_video.planes(), &[vec![1]]);
        assert_eq!(source.as_slice(), &[1]);
        assert_eq!(frame.side_data()[0].data(), &[0x99, 0x11]);
        assert_eq!(cloned.side_data()[0].data(), &[0x10, 0x11]);
        assert_eq!(side_data.as_slice(), &[0x10, 0x11]);
        assert_eq!(frame.hw_frames_context().unwrap().as_slice(), &[0x20, 0x21]);
        assert_eq!(hw_context.as_slice(), &[0x20, 0x21]);

        let mut empty = Frame::empty();
        assert!(empty.side_data_is_writable());
        assert_eq!(empty.hw_frames_context_is_writable(), None);
        assert!(!empty.make_hw_frames_context_writable());
        assert!(!empty.all_references_are_writable());
        empty.make_all_references_writable();
        assert!(empty.is_empty());
    }

    #[test]
    fn frame_data_make_writable_detaches_audio_payload_refs() {
        let source = BufferRef::from_vec_readonly(vec![0, 0, 1, 0]);
        let audio =
            AudioFrame::new_with_buffer_refs(48_000, 2, SampleFormat::S16, 1, vec![source.clone()])
                .unwrap();
        let mut frame = Frame::audio(audio);
        let cloned = frame.clone();

        assert!(!frame.data().is_writable());
        frame.data_mut().make_writable();
        assert!(frame.data().is_writable());
        frame
            .data_mut()
            .set_plane_visible_data(0, &[9, 0, 8, 0])
            .unwrap();

        let (frame_audio, cloned_audio) = match (frame.data(), cloned.data()) {
            (FrameData::Audio(frame_audio), FrameData::Audio(cloned_audio)) => {
                (frame_audio, cloned_audio)
            }
            _ => panic!("expected audio frames"),
        };
        assert!(!frame_audio.plane_buffers()[0].shares_storage(&source));
        assert!(cloned_audio.plane_buffers()[0].shares_storage(&source));
        assert_eq!(frame_audio.planes(), &[vec![9, 0, 8, 0]]);
        assert_eq!(cloned_audio.planes(), &[vec![0, 0, 1, 0]]);
        assert_eq!(source.as_slice(), &[0, 0, 1, 0]);
    }

    #[test]
    fn frame_side_data_retains_buffer_ref_payload_and_metadata() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let release_capture = std::sync::Arc::clone(&released);
        let payload = BufferRef::from_external_slice_with_len_and_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![1, 2, 3, 77]),
            3,
            String::from("displaymatrix"),
            move |opaque| {
                release_capture.lock().unwrap().push(opaque);
            },
        )
        .unwrap();
        let mut side_data =
            FrameSideData::new_with_buffer_ref("displaymatrix", payload.clone()).unwrap();
        side_data.metadata_mut().set("rotation", "90").unwrap();

        let video = VideoFrame::new(1, 1, PixelFormat::Gray8, vec![vec![7]]).unwrap();
        let mut frame = Frame::video(video);
        frame.push_side_data(side_data);
        let cloned = frame.clone();

        assert_eq!(frame.side_data()[0].kind(), "displaymatrix");
        assert_eq!(
            frame.side_data()[0].kind_id(),
            &FrameSideDataKind::DisplayMatrix
        );
        assert!(frame.side_data()[0].is_known_kind());
        assert_eq!(frame.side_data()[0].data(), &[1, 2, 3]);
        assert_eq!(frame.side_data()[0].buffer().padding_slice(), &[77]);
        assert_eq!(frame.side_data()[0].metadata().get("rotation"), Some("90"));
        assert!(frame.side_data()[0].buffer().shares_storage(&payload));
        assert!(cloned.side_data()[0]
            .buffer()
            .shares_storage(frame.side_data()[0].buffer()));

        drop(payload);
        assert!(released.lock().unwrap().is_empty());
        drop(frame);
        assert!(released.lock().unwrap().is_empty());
        drop(cloned);
        assert_eq!(
            *released.lock().unwrap(),
            vec![String::from("displaymatrix")]
        );
    }

    #[test]
    fn frame_side_data_maps_known_kinds_and_preserves_unknown_names() {
        assert_eq!(
            FrameSideDataKind::from_name("Display Matrix").unwrap(),
            FrameSideDataKind::DisplayMatrix
        );
        assert_eq!(
            FrameSideDataKind::from_name("display_matrix").unwrap(),
            FrameSideDataKind::DisplayMatrix
        );
        assert_eq!(
            FrameSideDataKind::from_name("ATSC A53 Closed Captions").unwrap(),
            FrameSideDataKind::A53ClosedCaptions
        );
        assert_eq!(
            FrameSideDataKind::from_name("Dolby Vision RPU Buffer").unwrap(),
            FrameSideDataKind::DolbyVisionRpuBuffer
        );
        assert_eq!(
            FrameSideDataKind::from_name("AV_FRAME_DATA_EXIF").unwrap(),
            FrameSideDataKind::Exif
        );
        assert_eq!(
            FrameSideDataKind::from_name("3D Reference Displays").unwrap(),
            FrameSideDataKind::ThreeDReferenceDisplays
        );
        assert_eq!(FrameSideDataKind::DisplayMatrix.name(), "displaymatrix");
        assert_eq!(
            FrameSideDataKind::DisplayMatrix.ffmpeg_constant(),
            Some("AV_FRAME_DATA_DISPLAYMATRIX")
        );
        assert!(FrameSideDataKind::DisplayMatrix.is_known());
        assert!(FrameSideDataKind::KNOWN.contains(&FrameSideDataKind::DisplayMatrix));

        let unknown = FrameSideDataKind::from_name("vendor.private.side-data").unwrap();
        assert_eq!(
            unknown,
            FrameSideDataKind::Unknown(String::from("vendor.private.side-data"))
        );
        assert_eq!(unknown.name(), "vendor.private.side-data");
        assert_eq!(unknown.ffmpeg_constant(), None);
        assert!(!unknown.is_known());

        assert_eq!(
            FrameSideDataKind::from_name(" \t").unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            FrameSideDataKind::from_name("bad\0kind")
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn frame_side_data_known_inventory_matches_ffmpeg_8_1_1_header() {
        let expected = [
            (FrameSideDataKind::PanScan, "AV_FRAME_DATA_PANSCAN"),
            (FrameSideDataKind::A53ClosedCaptions, "AV_FRAME_DATA_A53_CC"),
            (FrameSideDataKind::Stereo3d, "AV_FRAME_DATA_STEREO3D"),
            (
                FrameSideDataKind::MatrixEncoding,
                "AV_FRAME_DATA_MATRIXENCODING",
            ),
            (FrameSideDataKind::DownmixInfo, "AV_FRAME_DATA_DOWNMIX_INFO"),
            (FrameSideDataKind::ReplayGain, "AV_FRAME_DATA_REPLAYGAIN"),
            (
                FrameSideDataKind::DisplayMatrix,
                "AV_FRAME_DATA_DISPLAYMATRIX",
            ),
            (
                FrameSideDataKind::ActiveFormatDescription,
                "AV_FRAME_DATA_AFD",
            ),
            (
                FrameSideDataKind::MotionVectors,
                "AV_FRAME_DATA_MOTION_VECTORS",
            ),
            (FrameSideDataKind::SkipSamples, "AV_FRAME_DATA_SKIP_SAMPLES"),
            (
                FrameSideDataKind::AudioServiceType,
                "AV_FRAME_DATA_AUDIO_SERVICE_TYPE",
            ),
            (
                FrameSideDataKind::MasteringDisplayMetadata,
                "AV_FRAME_DATA_MASTERING_DISPLAY_METADATA",
            ),
            (FrameSideDataKind::GopTimecode, "AV_FRAME_DATA_GOP_TIMECODE"),
            (FrameSideDataKind::Spherical, "AV_FRAME_DATA_SPHERICAL"),
            (
                FrameSideDataKind::ContentLightLevel,
                "AV_FRAME_DATA_CONTENT_LIGHT_LEVEL",
            ),
            (FrameSideDataKind::IccProfile, "AV_FRAME_DATA_ICC_PROFILE"),
            (
                FrameSideDataKind::S12mTimecode,
                "AV_FRAME_DATA_S12M_TIMECODE",
            ),
            (
                FrameSideDataKind::DynamicHdrPlus,
                "AV_FRAME_DATA_DYNAMIC_HDR_PLUS",
            ),
            (
                FrameSideDataKind::RegionsOfInterest,
                "AV_FRAME_DATA_REGIONS_OF_INTEREST",
            ),
            (
                FrameSideDataKind::VideoEncParams,
                "AV_FRAME_DATA_VIDEO_ENC_PARAMS",
            ),
            (
                FrameSideDataKind::SeiUnregistered,
                "AV_FRAME_DATA_SEI_UNREGISTERED",
            ),
            (
                FrameSideDataKind::FilmGrainParams,
                "AV_FRAME_DATA_FILM_GRAIN_PARAMS",
            ),
            (
                FrameSideDataKind::DetectionBboxes,
                "AV_FRAME_DATA_DETECTION_BBOXES",
            ),
            (
                FrameSideDataKind::DolbyVisionRpuBuffer,
                "AV_FRAME_DATA_DOVI_RPU_BUFFER",
            ),
            (
                FrameSideDataKind::DolbyVisionMetadata,
                "AV_FRAME_DATA_DOVI_METADATA",
            ),
            (
                FrameSideDataKind::DynamicHdrVivid,
                "AV_FRAME_DATA_DYNAMIC_HDR_VIVID",
            ),
            (
                FrameSideDataKind::AmbientViewingEnvironment,
                "AV_FRAME_DATA_AMBIENT_VIEWING_ENVIRONMENT",
            ),
            (FrameSideDataKind::VideoHint, "AV_FRAME_DATA_VIDEO_HINT"),
            (FrameSideDataKind::Lcevc, "AV_FRAME_DATA_LCEVC"),
            (FrameSideDataKind::ViewId, "AV_FRAME_DATA_VIEW_ID"),
            (
                FrameSideDataKind::ThreeDReferenceDisplays,
                "AV_FRAME_DATA_3D_REFERENCE_DISPLAYS",
            ),
            (FrameSideDataKind::Exif, "AV_FRAME_DATA_EXIF"),
        ];
        let expected_kinds = expected
            .iter()
            .map(|(kind, _)| kind.clone())
            .collect::<Vec<_>>();
        assert_eq!(FrameSideDataKind::KNOWN, expected_kinds.as_slice());

        for (kind, ffmpeg_constant) in expected {
            assert_eq!(kind.ffmpeg_constant(), Some(ffmpeg_constant));
            assert_eq!(FrameSideDataKind::from_name(ffmpeg_constant).unwrap(), kind);
            assert_eq!(FrameSideDataKind::from_name(kind.name()).unwrap(), kind);
        }
    }

    #[test]
    fn frame_side_data_descriptors_match_ffmpeg_8_1_1_side_data_table() {
        use FrameSideDataProperties as Props;

        let expected = [
            (
                FrameSideDataKind::PanScan,
                "AVPanScan",
                Props::SIZE_DEPENDENT,
            ),
            (
                FrameSideDataKind::A53ClosedCaptions,
                "ATSC A53 Part 4 Closed Captions",
                Props::EMPTY,
            ),
            (FrameSideDataKind::Stereo3d, "Stereo 3D", Props::GLOBAL),
            (
                FrameSideDataKind::MatrixEncoding,
                "AVMatrixEncoding",
                Props::CHANNEL_DEPENDENT,
            ),
            (
                FrameSideDataKind::DownmixInfo,
                "Metadata relevant to a downmix procedure",
                Props::CHANNEL_DEPENDENT,
            ),
            (FrameSideDataKind::ReplayGain, "AVReplayGain", Props::GLOBAL),
            (
                FrameSideDataKind::DisplayMatrix,
                "3x3 displaymatrix",
                Props::GLOBAL,
            ),
            (
                FrameSideDataKind::ActiveFormatDescription,
                "Active format description",
                Props::EMPTY,
            ),
            (
                FrameSideDataKind::MotionVectors,
                "Motion vectors",
                Props::SIZE_DEPENDENT,
            ),
            (FrameSideDataKind::SkipSamples, "Skip samples", Props::EMPTY),
            (
                FrameSideDataKind::AudioServiceType,
                "Audio service type",
                Props::GLOBAL,
            ),
            (
                FrameSideDataKind::MasteringDisplayMetadata,
                "Mastering display metadata",
                Props::GLOBAL.union(Props::COLOR_DEPENDENT),
            ),
            (FrameSideDataKind::GopTimecode, "GOP timecode", Props::EMPTY),
            (
                FrameSideDataKind::Spherical,
                "Spherical Mapping",
                Props::GLOBAL.union(Props::SIZE_DEPENDENT),
            ),
            (
                FrameSideDataKind::ContentLightLevel,
                "Content light level metadata",
                Props::GLOBAL.union(Props::COLOR_DEPENDENT),
            ),
            (
                FrameSideDataKind::IccProfile,
                "ICC profile",
                Props::GLOBAL.union(Props::COLOR_DEPENDENT),
            ),
            (
                FrameSideDataKind::S12mTimecode,
                "SMPTE 12-1 timecode",
                Props::EMPTY,
            ),
            (
                FrameSideDataKind::DynamicHdrPlus,
                "HDR Dynamic Metadata SMPTE2094-40 (HDR10+)",
                Props::COLOR_DEPENDENT,
            ),
            (
                FrameSideDataKind::RegionsOfInterest,
                "Regions Of Interest",
                Props::SIZE_DEPENDENT,
            ),
            (
                FrameSideDataKind::VideoEncParams,
                "Video encoding parameters",
                Props::EMPTY,
            ),
            (
                FrameSideDataKind::SeiUnregistered,
                "H.26[45] User Data Unregistered SEI message",
                Props::MULTI,
            ),
            (
                FrameSideDataKind::FilmGrainParams,
                "Film grain parameters",
                Props::EMPTY,
            ),
            (
                FrameSideDataKind::DetectionBboxes,
                "Bounding boxes for object detection and classification",
                Props::SIZE_DEPENDENT,
            ),
            (
                FrameSideDataKind::DolbyVisionRpuBuffer,
                "Dolby Vision RPU Data",
                Props::COLOR_DEPENDENT,
            ),
            (
                FrameSideDataKind::DolbyVisionMetadata,
                "Dolby Vision Metadata",
                Props::COLOR_DEPENDENT,
            ),
            (
                FrameSideDataKind::DynamicHdrVivid,
                "HDR Dynamic Metadata CUVA 005.1 2021 (Vivid)",
                Props::COLOR_DEPENDENT,
            ),
            (
                FrameSideDataKind::AmbientViewingEnvironment,
                "Ambient viewing environment",
                Props::GLOBAL,
            ),
            (
                FrameSideDataKind::VideoHint,
                "Encoding video hint",
                Props::SIZE_DEPENDENT,
            ),
            (
                FrameSideDataKind::Lcevc,
                "LCEVC NAL data",
                Props::SIZE_DEPENDENT,
            ),
            (FrameSideDataKind::ViewId, "View ID", Props::EMPTY),
            (
                FrameSideDataKind::ThreeDReferenceDisplays,
                "3D Reference Displays Information",
                Props::GLOBAL,
            ),
            (FrameSideDataKind::Exif, "EXIF metadata", Props::GLOBAL),
        ];

        for (kind, name, properties) in expected {
            let descriptor = kind.descriptor().unwrap();
            assert_eq!(descriptor.name(), name);
            assert_eq!(descriptor.properties(), properties);
            assert_eq!(kind.descriptor_name(), Some(name));
            assert_eq!(kind.properties(), properties);
        }

        let unknown = FrameSideDataKind::Unknown(String::from("vendor.private.side-data"));
        assert_eq!(unknown.descriptor(), None);
        assert_eq!(unknown.descriptor_name(), None);
        assert_eq!(unknown.properties(), Props::EMPTY);
        assert!(!unknown.supports_multiple_instances());
        assert_eq!(
            Props::from_bits_truncate(u32::MAX).bits(),
            Props::ALL.bits()
        );
        assert_eq!(Props::from_bits_truncate(1 << 31), Props::EMPTY);
        assert!(Props::GLOBAL
            .union(Props::COLOR_DEPENDENT)
            .contains(Props::GLOBAL));
        assert!(Props::GLOBAL
            .union(Props::COLOR_DEPENDENT)
            .intersects(Props::COLOR_DEPENDENT));
        assert!(!Props::GLOBAL.intersects(Props::SIZE_DEPENDENT));
    }

    #[test]
    fn frame_side_data_parses_display_matrix_payload() {
        let expected =
            FrameDisplayMatrix::new([1 << 16, 0, 0, 0, 1 << 16, 0, 12 << 16, -34 << 16, 1 << 30]);
        assert_eq!(expected.as_elements()[0], 1 << 16);
        assert_eq!(expected.elements()[8], 1 << 30);
        assert_eq!(
            FrameDisplayMatrix::parse(&expected.to_bytes()).unwrap(),
            expected
        );

        let identity = FrameDisplayMatrix::identity();
        assert_eq!(
            identity.elements(),
            [1 << 16, 0, 0, 0, 1 << 16, 0, 0, 0, 1 << 30]
        );

        let side_data = FrameSideData::new_display_matrix(expected).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::DisplayMatrix);
        assert_eq!(side_data.data(), &expected.to_bytes()[..]);
        assert_eq!(side_data.display_matrix().unwrap(), Some(expected));

        let all_i32_values = FrameDisplayMatrix::new([
            i32::MIN,
            -1,
            0,
            1,
            i32::MAX,
            1 << 30,
            -(1 << 30),
            1 << 16,
            -(1 << 16),
        ]);
        assert_eq!(
            FrameDisplayMatrix::parse(&all_i32_values.to_bytes()).unwrap(),
            all_i32_values
        );

        let motion_vectors = FrameSideData::new_with_kind(
            FrameSideDataKind::MotionVectors,
            expected.to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(motion_vectors.display_matrix().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_display_matrix_payload() {
        for data in [Vec::new(), vec![0; 35], vec![0; 37]] {
            assert_eq!(
                FrameDisplayMatrix::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, data).unwrap();
            assert_eq!(
                side_data.display_matrix().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_display =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 35]).unwrap();
        assert_eq!(non_display.display_matrix().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_matrix_encoding_payload() {
        let expected = [
            (FrameMatrixEncoding::None, 0, "AV_MATRIX_ENCODING_NONE"),
            (FrameMatrixEncoding::Dolby, 1, "AV_MATRIX_ENCODING_DOLBY"),
            (
                FrameMatrixEncoding::DolbyProLogicIi,
                2,
                "AV_MATRIX_ENCODING_DPLII",
            ),
            (
                FrameMatrixEncoding::DolbyProLogicIiX,
                3,
                "AV_MATRIX_ENCODING_DPLIIX",
            ),
            (
                FrameMatrixEncoding::DolbyProLogicIiZ,
                4,
                "AV_MATRIX_ENCODING_DPLIIZ",
            ),
            (
                FrameMatrixEncoding::DolbyEx,
                5,
                "AV_MATRIX_ENCODING_DOLBYEX",
            ),
            (
                FrameMatrixEncoding::DolbyHeadphone,
                6,
                "AV_MATRIX_ENCODING_DOLBYHEADPHONE",
            ),
        ];

        for (value, raw, ffmpeg_constant) in expected {
            assert_eq!(value.as_raw(), raw);
            assert_eq!(value.ffmpeg_constant(), ffmpeg_constant);
            assert_eq!(FrameMatrixEncoding::from_raw(raw).unwrap(), value);
            assert_eq!(
                FrameMatrixEncoding::parse(&raw.to_ne_bytes()).unwrap(),
                value
            );
            let side_data = FrameSideData::new_matrix_encoding(value).unwrap();
            assert_eq!(side_data.kind_id(), &FrameSideDataKind::MatrixEncoding);
            assert_eq!(side_data.data(), &raw.to_ne_bytes()[..]);
            assert_eq!(side_data.matrix_encoding().unwrap(), Some(value));
        }

        let replay_gain = FrameSideData::new_with_kind(
            FrameSideDataKind::ReplayGain,
            0i32.to_ne_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(replay_gain.matrix_encoding().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_matrix_encoding_payload() {
        for data in [Vec::new(), vec![0; 3], vec![0; 5]] {
            assert_eq!(
                FrameMatrixEncoding::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::MatrixEncoding, data).unwrap();
            assert_eq!(
                side_data.matrix_encoding().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for raw in [7, 8, -1] {
            assert_eq!(
                FrameMatrixEncoding::from_raw(raw).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data = FrameSideData::new_with_kind(
                FrameSideDataKind::MatrixEncoding,
                raw.to_ne_bytes().to_vec(),
            )
            .unwrap();
            assert_eq!(
                side_data.matrix_encoding().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 4]).unwrap();
        assert_eq!(non_matrix.matrix_encoding().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_downmix_info_payload() {
        let expected = [
            (FrameDownmixType::Unknown, 0, "AV_DOWNMIX_TYPE_UNKNOWN"),
            (FrameDownmixType::LoRo, 1, "AV_DOWNMIX_TYPE_LORO"),
            (FrameDownmixType::LtRt, 2, "AV_DOWNMIX_TYPE_LTRT"),
            (
                FrameDownmixType::DolbyProLogicIi,
                3,
                "AV_DOWNMIX_TYPE_DPLII",
            ),
        ];
        for (value, raw, ffmpeg_constant) in expected {
            assert_eq!(value.as_raw(), raw);
            assert_eq!(value.ffmpeg_constant(), ffmpeg_constant);
            assert_eq!(FrameDownmixType::from_raw(raw).unwrap(), value);
        }

        let downmix = FrameDownmixInfo::new(
            FrameDownmixType::LtRt,
            std::f64::consts::FRAC_1_SQRT_2,
            0.5,
            0.25,
            0.125,
            0.0,
        );
        assert_eq!(FrameDownmixInfo::DATA_LEN, 48);
        assert_eq!(downmix.preferred_downmix_type(), FrameDownmixType::LtRt);
        assert_eq!(downmix.center_mix_level(), std::f64::consts::FRAC_1_SQRT_2);
        assert_eq!(downmix.center_mix_level_ltrt(), 0.5);
        assert_eq!(downmix.surround_mix_level(), 0.25);
        assert_eq!(downmix.surround_mix_level_ltrt(), 0.125);
        assert_eq!(downmix.lfe_mix_level(), 0.0);
        assert_eq!(
            downmix.levels(),
            [std::f64::consts::FRAC_1_SQRT_2, 0.5, 0.25, 0.125, 0.0]
        );
        assert_eq!(&downmix.to_bytes()[4..8], &[0, 0, 0, 0]);
        assert_eq!(
            FrameDownmixInfo::parse(&downmix.to_bytes()).unwrap(),
            downmix
        );

        let mut nonzero_padding = downmix.to_bytes();
        nonzero_padding[4..8].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(FrameDownmixInfo::parse(&nonzero_padding).unwrap(), downmix);

        let nan_bits = f64::NAN.to_bits();
        let raw_bits = FrameDownmixInfo::from_level_bits(
            FrameDownmixType::DolbyProLogicIi,
            [
                nan_bits,
                1.0f64.to_bits(),
                2.0f64.to_bits(),
                3.0f64.to_bits(),
                4.0f64.to_bits(),
            ],
        );
        assert_eq!(
            FrameDownmixInfo::parse(&raw_bits.to_bytes()).unwrap(),
            raw_bits
        );
        assert_eq!(raw_bits.level_bits()[0], nan_bits);
        assert!(raw_bits.center_mix_level().is_nan());

        let side_data = FrameSideData::new_downmix_info(downmix).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::DownmixInfo);
        assert_eq!(side_data.data(), &downmix.to_bytes()[..]);
        assert_eq!(side_data.downmix_info().unwrap(), Some(downmix));

        let replay_gain = FrameSideData::new_with_kind(
            FrameSideDataKind::ReplayGain,
            vec![0; FrameDownmixInfo::DATA_LEN],
        )
        .unwrap();
        assert_eq!(replay_gain.downmix_info().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_downmix_info_payload() {
        for data in [Vec::new(), vec![0; 47], vec![0; 49]] {
            assert_eq!(
                FrameDownmixInfo::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DownmixInfo, data).unwrap();
            assert_eq!(
                side_data.downmix_info().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for raw in [4, 5, -1] {
            assert_eq!(
                FrameDownmixType::from_raw(raw).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let mut data = [0; FrameDownmixInfo::DATA_LEN];
            data[..4].copy_from_slice(&raw.to_ne_bytes());
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DownmixInfo, data.to_vec())
                    .unwrap();
            assert_eq!(
                side_data.downmix_info().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_downmix =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 48]).unwrap();
        assert_eq!(non_downmix.downmix_info().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_replay_gain_payload() {
        let replay_gain =
            FrameReplayGain::new(-650_000, 100_000, FrameReplayGain::GAIN_UNKNOWN, u32::MAX);
        let mut expected_bytes = [0; FrameReplayGain::DATA_LEN];
        expected_bytes[0..4].copy_from_slice(&(-650_000i32).to_ne_bytes());
        expected_bytes[4..8].copy_from_slice(&100_000u32.to_ne_bytes());
        expected_bytes[8..12].copy_from_slice(&FrameReplayGain::GAIN_UNKNOWN.to_ne_bytes());
        expected_bytes[12..16].copy_from_slice(&u32::MAX.to_ne_bytes());

        assert_eq!(FrameReplayGain::DATA_LEN, 16);
        assert_eq!(FrameReplayGain::PEAK_UNKNOWN, 0);
        assert_eq!(replay_gain.track_gain(), -650_000);
        assert_eq!(replay_gain.track_peak(), 100_000);
        assert_eq!(replay_gain.album_gain(), FrameReplayGain::GAIN_UNKNOWN);
        assert_eq!(replay_gain.album_peak(), u32::MAX);
        assert!(!replay_gain.track_gain_unknown());
        assert!(!replay_gain.track_peak_unknown());
        assert!(replay_gain.album_gain_unknown());
        assert!(!replay_gain.album_peak_unknown());
        assert_eq!(replay_gain.to_bytes(), expected_bytes);
        assert_eq!(
            FrameReplayGain::parse(&expected_bytes).unwrap(),
            replay_gain
        );

        let unknown = FrameReplayGain::new(
            FrameReplayGain::GAIN_UNKNOWN,
            FrameReplayGain::PEAK_UNKNOWN,
            FrameReplayGain::GAIN_UNKNOWN,
            FrameReplayGain::PEAK_UNKNOWN,
        );
        assert!(unknown.track_gain_unknown());
        assert!(unknown.track_peak_unknown());
        assert!(unknown.album_gain_unknown());
        assert!(unknown.album_peak_unknown());

        let side_data = FrameSideData::new_replay_gain(replay_gain).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::ReplayGain);
        assert_eq!(side_data.data(), &expected_bytes[..]);
        assert_eq!(side_data.replay_gain().unwrap(), Some(replay_gain));

        let display_matrix = FrameSideData::new_with_kind(
            FrameSideDataKind::DisplayMatrix,
            vec![0; FrameReplayGain::DATA_LEN],
        )
        .unwrap();
        assert_eq!(display_matrix.replay_gain().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_replay_gain_payload() {
        for data in [Vec::new(), vec![0; 15], vec![0; 17]] {
            assert_eq!(
                FrameReplayGain::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, data).unwrap();
            assert_eq!(
                side_data.replay_gain().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_replay_gain =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 16]).unwrap();
        assert_eq!(non_replay_gain.replay_gain().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_active_format_description_payload() {
        let expected = [
            (FrameActiveFormatDescription::Same, 8, "AV_AFD_SAME"),
            (FrameActiveFormatDescription::FourThree, 9, "AV_AFD_4_3"),
            (FrameActiveFormatDescription::SixteenNine, 10, "AV_AFD_16_9"),
            (
                FrameActiveFormatDescription::FourteenNine,
                11,
                "AV_AFD_14_9",
            ),
            (
                FrameActiveFormatDescription::FourThreeProtectedFourteenNine,
                13,
                "AV_AFD_4_3_SP_14_9",
            ),
            (
                FrameActiveFormatDescription::SixteenNineProtectedFourteenNine,
                14,
                "AV_AFD_16_9_SP_14_9",
            ),
            (
                FrameActiveFormatDescription::ProtectedFourThree,
                15,
                "AV_AFD_SP_4_3",
            ),
        ];

        for (value, byte, ffmpeg_constant) in expected {
            assert_eq!(value.as_byte(), byte);
            assert_eq!(value.ffmpeg_constant(), ffmpeg_constant);
            assert_eq!(
                FrameActiveFormatDescription::from_byte(byte).unwrap(),
                value
            );
            let side_data = FrameSideData::new_active_format_description(value).unwrap();
            assert_eq!(
                side_data.kind_id(),
                &FrameSideDataKind::ActiveFormatDescription
            );
            assert_eq!(side_data.data(), &[byte]);
            assert_eq!(side_data.active_format_description().unwrap(), Some(value));
        }

        let replay_gain =
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![8]).unwrap();
        assert_eq!(replay_gain.active_format_description().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_active_format_description_payload() {
        let bad_lengths: [&[u8]; 2] = [&[], &[8, 9]];
        for data in bad_lengths {
            assert_eq!(
                FrameActiveFormatDescription::parse(data)
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
            let side_data = FrameSideData::new_with_kind(
                FrameSideDataKind::ActiveFormatDescription,
                data.to_vec(),
            )
            .unwrap();
            assert_eq!(
                side_data.active_format_description().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            FrameActiveFormatDescription::from_byte(12)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::ActiveFormatDescription, vec![12])
                .unwrap();
        assert_eq!(
            side_data.active_format_description().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_parses_motion_vectors_payload() {
        let past = FrameMotionVector::new(
            -1,
            16,
            8,
            -20,
            32,
            64,
            -12,
            0x0102_0304_0506_0708,
            1200,
            -3400,
            4,
        );
        let future = FrameMotionVector::new(1, 4, 4, 320, -240, -64, 96, u64::MAX, -128, 256, 2);
        let mut past_bytes = [0; FrameMotionVector::DATA_LEN];
        past_bytes[0..4].copy_from_slice(&(-1i32).to_ne_bytes());
        past_bytes[4] = 16;
        past_bytes[5] = 8;
        past_bytes[6..8].copy_from_slice(&(-20i16).to_ne_bytes());
        past_bytes[8..10].copy_from_slice(&32i16.to_ne_bytes());
        past_bytes[10..12].copy_from_slice(&64i16.to_ne_bytes());
        past_bytes[12..14].copy_from_slice(&(-12i16).to_ne_bytes());
        past_bytes[16..24].copy_from_slice(&0x0102_0304_0506_0708u64.to_ne_bytes());
        past_bytes[24..28].copy_from_slice(&1200i32.to_ne_bytes());
        past_bytes[28..32].copy_from_slice(&(-3400i32).to_ne_bytes());
        past_bytes[32..34].copy_from_slice(&4u16.to_ne_bytes());

        assert_eq!(FrameMotionVector::DATA_LEN, 40);
        assert_eq!(past.source(), -1);
        assert_eq!(past.width(), 16);
        assert_eq!(past.height(), 8);
        assert_eq!(past.src_x(), -20);
        assert_eq!(past.src_y(), 32);
        assert_eq!(past.dst_x(), 64);
        assert_eq!(past.dst_y(), -12);
        assert_eq!(past.flags(), 0x0102_0304_0506_0708);
        assert_eq!(past.motion_x(), 1200);
        assert_eq!(past.motion_y(), -3400);
        assert_eq!(past.motion_scale(), 4);
        assert_eq!(past.to_bytes(), past_bytes);
        assert_eq!(&past_bytes[14..16], &[0, 0]);
        assert_eq!(&past_bytes[34..40], &[0, 0, 0, 0, 0, 0]);
        assert_eq!(FrameMotionVector::parse(&past_bytes).unwrap(), past);

        let mut padded = past_bytes;
        padded[14..16].copy_from_slice(&[0xaa, 0xbb]);
        padded[34..40].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(FrameMotionVector::parse(&padded).unwrap(), past);
        assert_eq!(
            FrameMotionVector::parse(&padded).unwrap().to_bytes(),
            past_bytes
        );

        let vectors = FrameMotionVectors::new(vec![past, future]).unwrap();
        let mut payload = Vec::new();
        payload.extend_from_slice(&past_bytes);
        payload.extend_from_slice(&future.to_bytes());

        assert_eq!(vectors.len(), 2);
        assert!(!vectors.is_empty());
        assert_eq!(vectors.vectors(), &[past, future]);
        assert_eq!(vectors.clone().into_vectors(), vec![past, future]);
        assert_eq!(vectors.to_bytes(), payload);
        assert_eq!(FrameMotionVectors::parse(&payload).unwrap(), vectors);

        let side_data = FrameSideData::new_motion_vectors(vectors.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::MotionVectors);
        assert_eq!(side_data.data(), payload.as_slice());
        assert_eq!(side_data.motion_vectors().unwrap(), Some(vectors));

        let replay_gain = FrameSideData::new_with_kind(
            FrameSideDataKind::ReplayGain,
            vec![0; FrameMotionVector::DATA_LEN],
        )
        .unwrap();
        assert_eq!(replay_gain.motion_vectors().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_motion_vectors_payload() {
        for data in [Vec::new(), vec![0; 39], vec![0; 41], vec![0; 79]] {
            assert_eq!(
                FrameMotionVectors::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, data).unwrap();
            assert_eq!(
                side_data.motion_vectors().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            FrameMotionVector::parse(&[0; 39]).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameMotionVectors::new(Vec::new()).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let replay_gain =
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, Vec::new()).unwrap();
        assert_eq!(replay_gain.motion_vectors().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_skip_samples_payload() {
        let expected = FrameSkipSamples::new(
            0x0102_0304,
            0xA0B0_C0D0,
            FrameSkipSamplesReason::PaddingSilence,
            FrameSkipSamplesReason::Convergence,
        );
        let expected_bytes = [0x04, 0x03, 0x02, 0x01, 0xD0, 0xC0, 0xB0, 0xA0, 0, 1];

        assert_eq!(expected.start(), 0x0102_0304);
        assert_eq!(expected.end(), 0xA0B0_C0D0);
        assert_eq!(
            expected.start_reason(),
            FrameSkipSamplesReason::PaddingSilence
        );
        assert_eq!(expected.end_reason(), FrameSkipSamplesReason::Convergence);
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            FrameSkipSamplesReason::from_byte(0).unwrap(),
            FrameSkipSamplesReason::PaddingSilence
        );
        assert_eq!(
            FrameSkipSamplesReason::from_byte(1).unwrap(),
            FrameSkipSamplesReason::Convergence
        );
        assert_eq!(FrameSkipSamples::parse(&expected_bytes).unwrap(), expected);

        let side_data = FrameSideData::new_skip_samples(expected).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::SkipSamples);
        assert_eq!(side_data.data(), &expected_bytes[..]);
        assert_eq!(side_data.skip_samples().unwrap(), Some(expected));

        let replay_gain =
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, expected_bytes.to_vec())
                .unwrap();
        assert_eq!(replay_gain.skip_samples().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_skip_samples_payload() {
        for data in [Vec::new(), vec![0; 9], vec![0; 11]] {
            assert_eq!(
                FrameSkipSamples::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::SkipSamples, data).unwrap();
            assert_eq!(
                side_data.skip_samples().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            FrameSkipSamplesReason::from_byte(2).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        let mut bad_start_reason = [0; FrameSkipSamples::DATA_LEN];
        bad_start_reason[8] = 2;
        assert_eq!(
            FrameSkipSamples::parse(&bad_start_reason)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::SkipSamples, bad_start_reason.to_vec())
                .unwrap();
        assert_eq!(
            side_data.skip_samples().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_end_reason = [0; FrameSkipSamples::DATA_LEN];
        bad_end_reason[9] = 2;
        assert_eq!(
            FrameSkipSamples::parse(&bad_end_reason).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::SkipSamples, bad_end_reason.to_vec())
                .unwrap();
        assert_eq!(
            side_data.skip_samples().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let non_skip =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 10]).unwrap();
        assert_eq!(non_skip.skip_samples().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_audio_service_type_payload() {
        let expected = [
            (FrameAudioServiceType::Main, 0, "AV_AUDIO_SERVICE_TYPE_MAIN"),
            (
                FrameAudioServiceType::Effects,
                1,
                "AV_AUDIO_SERVICE_TYPE_EFFECTS",
            ),
            (
                FrameAudioServiceType::VisuallyImpaired,
                2,
                "AV_AUDIO_SERVICE_TYPE_VISUALLY_IMPAIRED",
            ),
            (
                FrameAudioServiceType::HearingImpaired,
                3,
                "AV_AUDIO_SERVICE_TYPE_HEARING_IMPAIRED",
            ),
            (
                FrameAudioServiceType::Dialogue,
                4,
                "AV_AUDIO_SERVICE_TYPE_DIALOGUE",
            ),
            (
                FrameAudioServiceType::Commentary,
                5,
                "AV_AUDIO_SERVICE_TYPE_COMMENTARY",
            ),
            (
                FrameAudioServiceType::Emergency,
                6,
                "AV_AUDIO_SERVICE_TYPE_EMERGENCY",
            ),
            (
                FrameAudioServiceType::VoiceOver,
                7,
                "AV_AUDIO_SERVICE_TYPE_VOICE_OVER",
            ),
            (
                FrameAudioServiceType::Karaoke,
                8,
                "AV_AUDIO_SERVICE_TYPE_KARAOKE",
            ),
        ];

        for (value, raw, ffmpeg_constant) in expected {
            assert_eq!(value.as_raw(), raw);
            assert_eq!(value.ffmpeg_constant(), ffmpeg_constant);
            assert_eq!(FrameAudioServiceType::from_raw(raw).unwrap(), value);
            assert_eq!(
                FrameAudioServiceType::parse(&raw.to_ne_bytes()).unwrap(),
                value
            );
            let side_data = FrameSideData::new_audio_service_type(value).unwrap();
            assert_eq!(side_data.kind_id(), &FrameSideDataKind::AudioServiceType);
            assert_eq!(side_data.data(), &raw.to_ne_bytes()[..]);
            assert_eq!(side_data.audio_service_type().unwrap(), Some(value));
        }

        let mut extended = FrameAudioServiceType::Commentary.to_bytes().to_vec();
        extended.extend_from_slice(&[0xAA, 0xBB]);
        assert_eq!(
            FrameAudioServiceType::parse(&extended).unwrap(),
            FrameAudioServiceType::Commentary
        );
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::AudioServiceType, extended).unwrap();
        assert_eq!(
            side_data.audio_service_type().unwrap(),
            Some(FrameAudioServiceType::Commentary)
        );

        let replay_gain = FrameSideData::new_with_kind(
            FrameSideDataKind::ReplayGain,
            0i32.to_ne_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(replay_gain.audio_service_type().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_audio_service_type_payload() {
        for data in [Vec::new(), vec![0; 3]] {
            assert_eq!(
                FrameAudioServiceType::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::AudioServiceType, data).unwrap();
            assert_eq!(
                side_data.audio_service_type().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for raw in [9, -1] {
            assert_eq!(
                FrameAudioServiceType::from_raw(raw).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data = FrameSideData::new_with_kind(
                FrameSideDataKind::AudioServiceType,
                raw.to_ne_bytes().to_vec(),
            )
            .unwrap();
            assert_eq!(
                side_data.audio_service_type().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_audio =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 4]).unwrap();
        assert_eq!(non_audio.audio_service_type().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_mastering_display_metadata_payload() {
        fn write_rational(
            bytes: &mut [u8; FrameMasteringDisplayMetadata::DATA_LEN],
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
        let expected = FrameMasteringDisplayMetadata::new(
            display_primaries,
            white_point,
            Rational::from_raw(50, 10_000),
            Rational::from_raw(1000, 1),
            1,
            2,
        );
        let mut expected_bytes = [0; FrameMasteringDisplayMetadata::DATA_LEN];
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
        expected_bytes[offset..offset + 4].copy_from_slice(&1i32.to_ne_bytes());
        offset += 4;
        expected_bytes[offset..offset + 4].copy_from_slice(&2i32.to_ne_bytes());

        assert_eq!(FrameMasteringDisplayMetadata::DATA_LEN, 88);
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
            FrameMasteringDisplayMetadata::parse(&expected_bytes).unwrap(),
            expected
        );

        let raw_values = FrameMasteringDisplayMetadata::new(
            [[Rational::from_raw(2, 4); FrameMasteringDisplayMetadata::COORDINATES];
                FrameMasteringDisplayMetadata::PRIMARIES],
            [Rational::from_raw(0, 0); FrameMasteringDisplayMetadata::COORDINATES],
            Rational::from_raw(0, 0),
            Rational::from_raw(9, 3),
            0,
            -3,
        );
        let roundtrip = FrameMasteringDisplayMetadata::parse(&raw_values.to_bytes()).unwrap();
        assert_eq!(
            roundtrip.display_primaries()[0][0],
            Rational::from_raw(2, 4)
        );
        assert_eq!(roundtrip.white_point()[0], Rational::from_raw(0, 0));
        assert!(!roundtrip.has_primaries());
        assert!(roundtrip.has_luminance());
        assert_eq!(roundtrip.has_luminance_raw(), -3);

        let side_data = FrameSideData::new_mastering_display_metadata(expected).unwrap();
        assert_eq!(
            side_data.kind_id(),
            &FrameSideDataKind::MasteringDisplayMetadata
        );
        assert_eq!(side_data.data(), &expected_bytes[..]);
        assert_eq!(
            side_data.mastering_display_metadata().unwrap(),
            Some(expected)
        );

        let motion_vectors =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, expected_bytes.to_vec())
                .unwrap();
        assert_eq!(motion_vectors.mastering_display_metadata().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_mastering_display_metadata_payload() {
        for data in [Vec::new(), vec![0; 87], vec![0; 89]] {
            assert_eq!(
                FrameMasteringDisplayMetadata::parse(&data)
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::MasteringDisplayMetadata, data)
                    .unwrap();
            assert_eq!(
                side_data.mastering_display_metadata().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let audio_service =
            FrameSideData::new_with_kind(FrameSideDataKind::AudioServiceType, Vec::new()).unwrap();
        assert_eq!(audio_service.mastering_display_metadata().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_gop_timecode_payload() {
        let expected = FrameGopTimecode::new(0x01FE_DCBA).unwrap();
        assert_eq!(expected.as_raw_i64(), 0x01FE_DCBA);
        assert_eq!(
            FrameGopTimecode::from_raw_i64(expected.as_raw_i64()).unwrap(),
            expected
        );
        assert_eq!(
            FrameGopTimecode::parse(&expected.to_bytes()).unwrap(),
            expected
        );

        let side_data = FrameSideData::new_gop_timecode(expected).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::GopTimecode);
        assert_eq!(side_data.data(), &expected.to_bytes()[..]);
        assert_eq!(side_data.gop_timecode().unwrap(), Some(expected));

        let zero = FrameGopTimecode::from_raw_i64(0).unwrap();
        assert_eq!(zero.as_raw_i64(), 0);
        let max = FrameGopTimecode::from_raw_i64(FrameGopTimecode::MAX_VALUE).unwrap();
        assert_eq!(max.as_raw_i64(), FrameGopTimecode::MAX_VALUE);

        let display_matrix = FrameSideData::new_with_kind(
            FrameSideDataKind::DisplayMatrix,
            expected.to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(display_matrix.gop_timecode().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_gop_timecode_payload() {
        for data in [Vec::new(), vec![0; 7], vec![0; 9]] {
            assert_eq!(
                FrameGopTimecode::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::GopTimecode, data).unwrap();
            assert_eq!(
                side_data.gop_timecode().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for raw in [-1, FrameGopTimecode::MAX_VALUE + 1, i64::MAX] {
            assert_eq!(
                FrameGopTimecode::from_raw_i64(raw).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data = FrameSideData::new_with_kind(
                FrameSideDataKind::GopTimecode,
                raw.to_ne_bytes().to_vec(),
            )
            .unwrap();
            assert_eq!(
                side_data.gop_timecode().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_gop =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 8]).unwrap();
        assert_eq!(non_gop.gop_timecode().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_spherical_mapping_payload() {
        let expected = FrameSphericalMapping::new(
            FrameSphericalProjection::Cubemap,
            90 << 16,
            -15 << 16,
            180 << 16,
            [1, 2, 3, 4],
            12,
        );
        let mut expected_bytes = [0; FrameSphericalMapping::DATA_LEN];
        expected_bytes[0..4].copy_from_slice(&1i32.to_ne_bytes());
        expected_bytes[4..8].copy_from_slice(&(90i32 << 16).to_ne_bytes());
        expected_bytes[8..12].copy_from_slice(&(-15i32 << 16).to_ne_bytes());
        expected_bytes[12..16].copy_from_slice(&(180i32 << 16).to_ne_bytes());
        expected_bytes[16..20].copy_from_slice(&1u32.to_ne_bytes());
        expected_bytes[20..24].copy_from_slice(&2u32.to_ne_bytes());
        expected_bytes[24..28].copy_from_slice(&3u32.to_ne_bytes());
        expected_bytes[28..32].copy_from_slice(&4u32.to_ne_bytes());
        expected_bytes[32..36].copy_from_slice(&12u32.to_ne_bytes());

        assert_eq!(FrameSphericalMapping::DATA_LEN, 36);
        assert_eq!(FrameSphericalMapping::BOUNDS, 4);
        assert_eq!(expected.projection(), FrameSphericalProjection::Cubemap);
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
            FrameSphericalMapping::parse(&expected_bytes).unwrap(),
            expected
        );

        let projections = [
            (
                FrameSphericalProjection::Equirectangular,
                0,
                "AV_SPHERICAL_EQUIRECTANGULAR",
            ),
            (FrameSphericalProjection::Cubemap, 1, "AV_SPHERICAL_CUBEMAP"),
            (
                FrameSphericalProjection::EquirectangularTile,
                2,
                "AV_SPHERICAL_EQUIRECTANGULAR_TILE",
            ),
            (
                FrameSphericalProjection::HalfEquirectangular,
                3,
                "AV_SPHERICAL_HALF_EQUIRECTANGULAR",
            ),
            (
                FrameSphericalProjection::Rectilinear,
                4,
                "AV_SPHERICAL_RECTILINEAR",
            ),
            (FrameSphericalProjection::Fisheye, 5, "AV_SPHERICAL_FISHEYE"),
            (
                FrameSphericalProjection::ParametricImmersive,
                6,
                "AV_SPHERICAL_PARAMETRIC_IMMERSIVE",
            ),
        ];
        assert_eq!(
            FrameSphericalProjection::KNOWN,
            projections.map(|(projection, _, _)| projection)
        );
        for (projection, raw, ffmpeg_constant) in projections {
            assert_eq!(FrameSphericalProjection::from_raw(raw).unwrap(), projection);
            assert_eq!(projection.as_raw(), raw);
            assert_eq!(projection.ffmpeg_constant(), ffmpeg_constant);
        }

        let side_data = FrameSideData::new_spherical_mapping(expected).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::Spherical);
        assert_eq!(side_data.data(), &expected_bytes[..]);
        assert_eq!(side_data.spherical_mapping().unwrap(), Some(expected));

        let raw_bounds = FrameSphericalMapping::new(
            FrameSphericalProjection::EquirectangularTile,
            i32::MIN,
            0,
            i32::MAX,
            [u32::MAX, 0, 0x8000_0000, 42],
            u32::MAX,
        );
        assert_eq!(
            FrameSphericalMapping::parse(&raw_bounds.to_bytes()).unwrap(),
            raw_bounds
        );

        let display_matrix = FrameSideData::new_with_kind(
            FrameSideDataKind::DisplayMatrix,
            expected.to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(display_matrix.spherical_mapping().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_spherical_mapping_payload() {
        for data in [Vec::new(), vec![0; 35], vec![0; 37]] {
            assert_eq!(
                FrameSphericalMapping::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::Spherical, data).unwrap();
            assert_eq!(
                side_data.spherical_mapping().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for raw in [-1, 7, i32::MAX] {
            assert_eq!(
                FrameSphericalProjection::from_raw(raw).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let mut data = [0; FrameSphericalMapping::DATA_LEN];
            data[0..4].copy_from_slice(&raw.to_ne_bytes());
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::Spherical, data.to_vec()).unwrap();
            assert_eq!(
                side_data.spherical_mapping().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_spherical =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 36]).unwrap();
        assert_eq!(non_spherical.spherical_mapping().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_content_light_metadata_payload() {
        let expected = FrameContentLightMetadata::new(1000, 400);
        let mut expected_bytes = [0; FrameContentLightMetadata::DATA_LEN];
        expected_bytes[0..4].copy_from_slice(&1000u32.to_ne_bytes());
        expected_bytes[4..8].copy_from_slice(&400u32.to_ne_bytes());

        assert_eq!(FrameContentLightMetadata::DATA_LEN, 8);
        assert_eq!(expected.max_content_light_level(), 1000);
        assert_eq!(expected.max_average_light_level(), 400);
        assert_eq!(expected.to_bytes(), expected_bytes);
        assert_eq!(
            FrameContentLightMetadata::parse(&expected_bytes).unwrap(),
            expected
        );

        let side_data = FrameSideData::new_content_light_metadata(expected).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::ContentLightLevel);
        assert_eq!(side_data.data(), &expected_bytes[..]);
        assert_eq!(side_data.content_light_metadata().unwrap(), Some(expected));

        let raw_values = FrameContentLightMetadata::new(u32::MAX, 0);
        assert_eq!(
            FrameContentLightMetadata::parse(&raw_values.to_bytes()).unwrap(),
            raw_values
        );

        let display_matrix = FrameSideData::new_with_kind(
            FrameSideDataKind::DisplayMatrix,
            expected.to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(display_matrix.content_light_metadata().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_content_light_metadata_payload() {
        for data in [Vec::new(), vec![0; 7], vec![0; 9]] {
            assert_eq!(
                FrameContentLightMetadata::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::ContentLightLevel, data).unwrap();
            assert_eq!(
                side_data.content_light_metadata().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_content_light =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 8]).unwrap();
        assert_eq!(non_content_light.content_light_metadata().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_icc_profile_payload() {
        let data = minimal_icc_profile();
        let side_data = FrameSideData::new_icc_profile(data.clone(), Some("display-p3")).unwrap();

        assert_eq!(side_data.kind_id(), &FrameSideDataKind::IccProfile);
        assert_eq!(side_data.metadata().get("name"), Some("display-p3"));
        let parsed = side_data.icc_profile().unwrap().unwrap();
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.name(), Some("display-p3"));
        assert_eq!(parsed.declared_size(), FrameIccProfile::MIN_DATA_LEN as u32);
        assert_eq!(parsed.profile_version_raw(), 0x0430_0000);
        assert_eq!(parsed.device_class(), *b"mntr");
        assert_eq!(parsed.color_space(), *b"RGB ");
        assert_eq!(parsed.profile_connection_space(), *b"XYZ ");
        assert_eq!(parsed.tag_count(), 0);

        let mut with_tag = minimal_icc_profile();
        with_tag.resize(
            FrameIccProfile::MIN_DATA_LEN + FrameIccProfile::TAG_RECORD_LEN,
            0,
        );
        let with_tag_len = with_tag.len() as u32;
        with_tag[0..4].copy_from_slice(&with_tag_len.to_be_bytes());
        with_tag[128..132].copy_from_slice(&1u32.to_be_bytes());
        with_tag[132..136].copy_from_slice(b"desc");
        with_tag[136..140].copy_from_slice(&(FrameIccProfile::MIN_DATA_LEN as u32).to_be_bytes());
        with_tag[140..144].copy_from_slice(&0u32.to_be_bytes());
        let side_data = FrameSideData::new_icc_profile(with_tag.clone(), None).unwrap();
        let parsed = side_data.icc_profile().unwrap().unwrap();
        assert_eq!(parsed.data(), with_tag.as_slice());
        assert_eq!(parsed.name(), None);
        assert_eq!(parsed.tag_count(), 1);

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, data).unwrap();
        assert_eq!(display_matrix.icc_profile().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_icc_profile_payload() {
        for data in [Vec::new(), vec![0; FrameIccProfile::MIN_DATA_LEN - 1]] {
            assert_eq!(
                FrameIccProfile::parse(&data, &Dictionary::new())
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::IccProfile, data).unwrap();
            assert_eq!(
                side_data.icc_profile().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut bad_size = minimal_icc_profile();
        bad_size[0..4].copy_from_slice(&999u32.to_be_bytes());
        assert_eq!(
            FrameSideData::new_icc_profile(bad_size, None)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut missing_signature = minimal_icc_profile();
        missing_signature[36..40].copy_from_slice(b"bad!");
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::IccProfile, missing_signature).unwrap();
        assert_eq!(
            side_data.icc_profile().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let mut truncated_tag_table = minimal_icc_profile();
        truncated_tag_table[128..132].copy_from_slice(&1u32.to_be_bytes());
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::IccProfile, truncated_tag_table)
                .unwrap();
        assert_eq!(
            side_data.icc_profile().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let invalid_name = minimal_icc_profile();
        assert_eq!(
            FrameSideData::new_icc_profile(invalid_name.clone(), Some("bad\0name"))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        let non_icc =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, invalid_name).unwrap();
        assert_eq!(non_icc.icc_profile().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_s12m_timecode_payload() {
        let expected = FrameS12mTimecode::new(&[0x0102_0304, 0xA0B0_C0D0]).unwrap();
        assert_eq!(expected.count(), 2);
        assert_eq!(expected.timecodes(), &[0x0102_0304, 0xA0B0_C0D0]);
        assert_eq!(expected.raw_words(), [2, 0x0102_0304, 0xA0B0_C0D0, 0]);

        let side_data = FrameSideData::new_s12m_timecode(expected).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::S12mTimecode);
        assert_eq!(side_data.data(), &expected.to_bytes()[..]);
        assert_eq!(side_data.s12m_timecode().unwrap(), Some(expected));

        let raw_with_unused =
            FrameS12mTimecode::from_raw_words([1, 0x0A0B_0C0D, 0xFEED_C0DE, 0x1234_5678]).unwrap();
        assert_eq!(raw_with_unused.count(), 1);
        assert_eq!(raw_with_unused.timecodes(), &[0x0A0B_0C0D]);
        assert_eq!(
            FrameS12mTimecode::parse(&raw_with_unused.to_bytes()).unwrap(),
            raw_with_unused
        );

        let display_matrix = FrameSideData::new_with_kind(
            FrameSideDataKind::DisplayMatrix,
            expected.to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(display_matrix.s12m_timecode().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_s12m_timecode_payload() {
        assert_eq!(
            FrameS12mTimecode::new(&[]).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            FrameS12mTimecode::new(&[1, 2, 3, 4]).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        for data in [Vec::new(), vec![0; 15], vec![0; 17]] {
            assert_eq!(
                FrameS12mTimecode::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::S12mTimecode, data).unwrap();
            assert_eq!(
                side_data.s12m_timecode().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for count in [0, 4, u32::MAX] {
            let words = [count, 1, 2, 3];
            assert_eq!(
                FrameS12mTimecode::from_raw_words(words).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data = FrameSideData::new_with_kind(
                FrameSideDataKind::S12mTimecode,
                FrameS12mTimecode { words }.to_bytes().to_vec(),
            )
            .unwrap();
            assert_eq!(
                side_data.s12m_timecode().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_s12m =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 16]).unwrap();
        assert_eq!(non_s12m.s12m_timecode().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_dynamic_hdr_plus_payload() {
        let data = minimal_dynamic_hdr_plus();
        let side_data = FrameSideData::new_dynamic_hdr_plus(data.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &FrameSideDataKind::DynamicHdrPlus);
        let parsed = side_data.dynamic_hdr_plus().unwrap().unwrap();
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(parsed.itu_t_t35_country_code(), 0xB5);
        assert_eq!(parsed.application_version(), 0);
        assert_eq!(parsed.num_windows(), 1);
        assert_eq!(parsed.color_transform_params(1), None);
        assert_eq!(
            parsed.targeted_system_display_maximum_luminance(),
            Rational::from_raw(1000, 1)
        );
        assert_eq!(
            parsed.targeted_system_display_actual_peak_luminance_flag(),
            1
        );
        assert_eq!(
            parsed.num_rows_targeted_system_display_actual_peak_luminance(),
            2
        );
        assert_eq!(
            parsed.num_cols_targeted_system_display_actual_peak_luminance(),
            2
        );
        assert_eq!(
            parsed.targeted_system_display_actual_peak_luminance(0, 0),
            Some(Rational::from_raw(1, 15))
        );
        assert_eq!(
            parsed.targeted_system_display_actual_peak_luminance(1, 1),
            Some(Rational::from_raw(2, 15))
        );
        assert_eq!(
            parsed.targeted_system_display_actual_peak_luminance(2, 0),
            None
        );
        assert_eq!(parsed.mastering_display_actual_peak_luminance_flag(), 0);
        assert_eq!(parsed.mastering_display_actual_peak_luminance(0, 0), None);

        let params = parsed.color_transform_params(0).unwrap();
        assert_eq!(
            params.data().len(),
            FrameHdrPlusColorTransformParams::DATA_LEN
        );
        assert_eq!(
            params.window_upper_left_corner_x(),
            Rational::from_raw(0, 1)
        );
        assert_eq!(
            params.window_upper_left_corner_y(),
            Rational::from_raw(0, 1)
        );
        assert_eq!(
            params.window_lower_right_corner_x(),
            Rational::from_raw(1, 1)
        );
        assert_eq!(
            params.window_lower_right_corner_y(),
            Rational::from_raw(1, 1)
        );
        assert_eq!(params.center_of_ellipse_x(), 640);
        assert_eq!(params.center_of_ellipse_y(), 360);
        assert_eq!(params.rotation_angle(), 45);
        assert_eq!(params.semimajor_axis_internal_ellipse(), 10);
        assert_eq!(params.semimajor_axis_external_ellipse(), 20);
        assert_eq!(params.semiminor_axis_external_ellipse(), 12);
        assert_eq!(
            params.overlap_process_option().unwrap(),
            FrameHdrPlusOverlapProcessOption::Layering
        );
        assert_eq!(params.maxscl(0), Some(Rational::from_raw(1, 100_000)));
        assert_eq!(params.maxscl(1), Some(Rational::from_raw(2, 100_000)));
        assert_eq!(params.maxscl(2), Some(Rational::from_raw(3, 100_000)));
        assert_eq!(params.maxscl(3), None);
        assert_eq!(params.average_maxrgb(), Rational::from_raw(4, 100_000));
        assert_eq!(params.num_distribution_maxrgb_percentiles(), 2);
        assert_eq!(
            params.distribution_maxrgb(0),
            Some(FrameHdrPlusPercentile::new(
                50,
                Rational::from_raw(5, 100_000)
            ))
        );
        assert_eq!(
            params.distribution_maxrgb(1),
            Some(FrameHdrPlusPercentile::new(
                99,
                Rational::from_raw(9, 100_000)
            ))
        );
        assert_eq!(params.distribution_maxrgb(2), None);
        assert_eq!(params.fraction_bright_pixels(), Rational::from_raw(1, 1000));
        assert_eq!(params.tone_mapping_flag(), 1);
        assert_eq!(params.knee_point_x(), Rational::from_raw(1, 4095));
        assert_eq!(params.knee_point_y(), Rational::from_raw(2, 4095));
        assert_eq!(params.num_bezier_curve_anchors(), 1);
        assert_eq!(
            params.bezier_curve_anchor(0),
            Some(Rational::from_raw(3, 1023))
        );
        assert_eq!(params.bezier_curve_anchor(1), None);
        assert_eq!(params.color_saturation_mapping_flag(), 1);
        assert_eq!(params.color_saturation_weight(), Rational::from_raw(8, 8));

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, data).unwrap();
        assert_eq!(display_matrix.dynamic_hdr_plus().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_dynamic_hdr_plus_payload() {
        for data in [
            Vec::new(),
            vec![0; FrameDynamicHdrPlus::DATA_LEN - 1],
            vec![0; FrameDynamicHdrPlus::DATA_LEN + 1],
        ] {
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DynamicHdrPlus, data).unwrap();
            assert_eq!(
                side_data.dynamic_hdr_plus().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (0, 0xB4),
            (1, 1),
            (2, 0),
            (2, 4),
            (
                FrameDynamicHdrPlus::PARAMS_OFFSET
                    + FrameHdrPlusColorTransformParams::NUM_DISTRIBUTION_MAXRGB_PERCENTILES_OFFSET,
                16,
            ),
            (
                FrameDynamicHdrPlus::PARAMS_OFFSET
                    + FrameHdrPlusColorTransformParams::TONE_MAPPING_FLAG_OFFSET,
                2,
            ),
            (
                FrameDynamicHdrPlus::PARAMS_OFFSET
                    + FrameHdrPlusColorTransformParams::NUM_BEZIER_CURVE_ANCHORS_OFFSET,
                16,
            ),
            (
                FrameDynamicHdrPlus::PARAMS_OFFSET
                    + FrameHdrPlusColorTransformParams::COLOR_SATURATION_MAPPING_FLAG_OFFSET,
                2,
            ),
            (
                FrameDynamicHdrPlus::TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET,
                2,
            ),
            (
                FrameDynamicHdrPlus::MASTERING_DISPLAY_ACTUAL_PEAK_LUMINANCE_FLAG_OFFSET,
                2,
            ),
        ] {
            let mut bad = minimal_dynamic_hdr_plus();
            bad[offset] = value;
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DynamicHdrPlus, bad).unwrap();
            assert_eq!(
                side_data.dynamic_hdr_plus().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut bad_overlap = minimal_dynamic_hdr_plus();
        write_ne_i32(
            &mut bad_overlap,
            FrameDynamicHdrPlus::PARAMS_OFFSET
                + FrameHdrPlusColorTransformParams::OVERLAP_PROCESS_OPTION_OFFSET,
            2,
        );
        assert_eq!(
            FrameSideData::new_dynamic_hdr_plus(bad_overlap)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_grid = minimal_dynamic_hdr_plus();
        bad_grid
            [FrameDynamicHdrPlus::NUM_ROWS_TARGETED_SYSTEM_DISPLAY_ACTUAL_PEAK_LUMINANCE_OFFSET] =
            1;
        assert_eq!(
            FrameSideData::new_dynamic_hdr_plus(bad_grid)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let non_hdr =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_hdr.dynamic_hdr_plus().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_sei_unregistered_payload() {
        let uuid = [
            0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB,
            0xCD, 0xEF,
        ];
        let side_data = FrameSideData::new_sei_unregistered(uuid, vec![0xAA, 0xBB]).unwrap();

        assert_eq!(side_data.kind_id(), &FrameSideDataKind::SeiUnregistered);
        assert_eq!(
            &side_data.data()[..FrameSeiUnregistered::UUID_LEN],
            uuid.as_slice()
        );
        let parsed = side_data.sei_unregistered().unwrap().unwrap();
        assert_eq!(parsed.uuid(), uuid);
        assert_eq!(parsed.user_data(), &[0xAA, 0xBB]);

        let empty_payload =
            FrameSideData::new_with_kind(FrameSideDataKind::SeiUnregistered, uuid.to_vec())
                .unwrap();
        let parsed = empty_payload.sei_unregistered().unwrap().unwrap();
        assert_eq!(parsed.uuid(), uuid);
        assert!(parsed.user_data().is_empty());

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 15]).unwrap();
        assert_eq!(display_matrix.sei_unregistered().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_sei_unregistered_payload() {
        let short = vec![0; FrameSeiUnregistered::UUID_LEN - 1];

        assert_eq!(
            FrameSeiUnregistered::parse(&short).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::SeiUnregistered, short).unwrap();
        assert_eq!(
            side_data.sei_unregistered().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let non_sei =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_sei.sei_unregistered().unwrap(), None);
    }

    #[test]
    fn frame_side_data_properties_drive_multi_and_removal_helpers() {
        let mut frame =
            Frame::video(VideoFrame::new(1, 1, PixelFormat::Gray8, vec![vec![7]]).unwrap());
        frame
            .add_side_data_kind(FrameSideDataKind::DisplayMatrix, vec![1])
            .unwrap();
        frame
            .add_side_data_kind(FrameSideDataKind::MotionVectors, vec![2])
            .unwrap();
        frame
            .add_side_data_kind(FrameSideDataKind::SeiUnregistered, vec![3])
            .unwrap();
        frame
            .add_side_data("vendor.private.side-data", vec![4])
            .unwrap();

        assert_eq!(
            frame.side_data()[0].descriptor_name(),
            Some("3x3 displaymatrix")
        );
        assert!(frame.side_data()[0]
            .properties()
            .contains(FrameSideDataProperties::GLOBAL));
        assert!(FrameSideDataKind::SeiUnregistered.supports_multiple_instances());
        assert!(frame.side_data()[2].supports_multiple_instances());
        assert!(!frame.side_data()[3]
            .properties()
            .intersects(FrameSideDataProperties::ALL));

        let removed_globals = frame.remove_side_data_by_properties(FrameSideDataProperties::GLOBAL);
        assert_eq!(removed_globals.len(), 1);
        assert_eq!(
            removed_globals[0].kind_id(),
            &FrameSideDataKind::DisplayMatrix
        );
        assert_eq!(frame.side_data().len(), 3);

        let removed_size_or_multi = frame.remove_side_data_by_properties(
            FrameSideDataProperties::SIZE_DEPENDENT.union(FrameSideDataProperties::MULTI),
        );
        assert_eq!(removed_size_or_multi.len(), 2);
        assert_eq!(
            removed_size_or_multi
                .iter()
                .map(FrameSideData::kind_id)
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                FrameSideDataKind::MotionVectors,
                FrameSideDataKind::SeiUnregistered
            ]
        );
        assert_eq!(frame.side_data().len(), 1);
        assert_eq!(frame.side_data()[0].kind(), "vendor.private.side-data");
        assert!(frame
            .remove_side_data_by_properties(FrameSideDataProperties::EMPTY)
            .is_empty());
        assert_eq!(frame.side_data().len(), 1);
    }

    #[test]
    fn frame_side_data_validates_kind_and_supports_remove_and_take() {
        assert_eq!(
            FrameSideData::new(" ", Vec::new()).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            FrameSideData::new("bad\0kind", Vec::new())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );

        let audio = AudioFrame::new(48_000, 1, SampleFormat::S16, 1, vec![vec![0; 2]]).unwrap();
        let mut frame = Frame::audio(audio);

        frame.add_side_data("alpha_info", vec![1, 2]).unwrap();
        let side_data = frame
            .add_side_data_kind_buffer(
                FrameSideDataKind::ReplayGain,
                BufferRef::copy_from_slice(&[9, 8, 7]),
            )
            .unwrap();
        side_data.metadata_mut().set("gain", "-3.0 dB").unwrap();

        assert_eq!(frame.side_data().len(), 2);
        assert_eq!(frame.side_data()[0].data(), &[1, 2]);
        assert_eq!(frame.side_data()[1].metadata().get("gain"), Some("-3.0 dB"));
        assert_eq!(
            frame
                .side_data_by_kind(&FrameSideDataKind::ReplayGain)
                .unwrap()
                .data(),
            &[9, 8, 7]
        );

        let removed = frame.remove_side_data("alpha_info").unwrap();
        assert_eq!(removed.kind(), "alpha_info");
        assert_eq!(removed.data(), &[1, 2]);
        assert!(frame.remove_side_data("missing").is_none());
        assert_eq!(frame.side_data().len(), 1);
        let removed = frame.remove_side_data("replay_gain").unwrap();
        assert_eq!(removed.kind_id(), &FrameSideDataKind::ReplayGain);
        frame.push_side_data(removed);

        let taken = frame.take_side_data();
        assert!(frame.side_data().is_empty());
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].kind(), "replaygain");
    }

    #[test]
    fn frame_set_side_data_replaces_alias_duplicates_and_releases_removed_records() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let make_payload = |label: &'static str, bytes: Vec<u8>| {
            let release_capture = std::sync::Arc::clone(&released);
            BufferRef::from_external_slice_with_opaque_readonly(
                std::sync::Arc::<[u8]>::from(bytes),
                String::from(label),
                move |opaque| {
                    release_capture.lock().unwrap().push(opaque);
                },
            )
        };
        let audio = AudioFrame::new(48_000, 1, SampleFormat::S16, 1, vec![vec![0; 2]]).unwrap();
        let mut frame = Frame::audio(audio);

        let appended = frame
            .set_side_data_kind(FrameSideDataKind::IccProfile, vec![5])
            .unwrap();
        assert!(appended.is_empty());
        assert_eq!(
            frame
                .remove_side_data_kind(&FrameSideDataKind::IccProfile)
                .unwrap()
                .data(),
            &[5]
        );

        frame
            .add_side_data_kind_buffer(
                FrameSideDataKind::DisplayMatrix,
                make_payload("first-display", vec![1, 2]),
            )
            .unwrap();
        frame
            .add_side_data_kind_buffer(
                FrameSideDataKind::ReplayGain,
                make_payload("replaygain", vec![3]),
            )
            .unwrap();
        frame
            .add_side_data_buffer(
                "Display Matrix",
                make_payload("duplicate-display", vec![4, 5]),
            )
            .unwrap();
        assert_eq!(frame.side_data().len(), 3);

        let removed = frame
            .set_side_data_buffer(
                "display_matrix",
                make_payload("replacement-display", vec![9, 8, 7]),
            )
            .unwrap();
        assert_eq!(removed.len(), 2);
        assert!(removed
            .iter()
            .all(|side_data| side_data.kind_id() == &FrameSideDataKind::DisplayMatrix));
        assert_eq!(removed[0].data(), &[1, 2]);
        assert_eq!(removed[1].data(), &[4, 5]);
        assert_eq!(frame.side_data().len(), 2);
        assert_eq!(
            frame.side_data()[0].kind_id(),
            &FrameSideDataKind::DisplayMatrix
        );
        assert_eq!(frame.side_data()[0].data(), &[9, 8, 7]);
        assert_eq!(
            frame.side_data()[1].kind_id(),
            &FrameSideDataKind::ReplayGain
        );

        frame
            .side_data_by_kind_mut(&FrameSideDataKind::DisplayMatrix)
            .unwrap()
            .metadata_mut()
            .set("rotation", "180")
            .unwrap();
        assert_eq!(
            frame
                .side_data_by_kind(&FrameSideDataKind::DisplayMatrix)
                .unwrap()
                .metadata()
                .get("rotation"),
            Some("180")
        );
        assert!(released.lock().unwrap().is_empty());

        drop(removed);
        let mut released_after_removed = released.lock().unwrap().clone();
        released_after_removed.sort();
        assert_eq!(
            released_after_removed,
            vec![
                String::from("duplicate-display"),
                String::from("first-display")
            ]
        );

        drop(frame);
        let mut all_released = released.lock().unwrap().clone();
        all_released.sort();
        assert_eq!(
            all_released,
            vec![
                String::from("duplicate-display"),
                String::from("first-display"),
                String::from("replacement-display"),
                String::from("replaygain")
            ]
        );
    }

    #[test]
    fn frame_hw_context_tracks_buffer_lifetime_and_clone_sharing() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let release_capture = std::sync::Arc::clone(&released);
        let context = BufferRef::from_external_slice_with_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![0]),
            String::from("frames"),
            move |opaque| {
                release_capture.lock().unwrap().push(opaque);
            },
        );
        let video = VideoFrame::new(1, 1, PixelFormat::Gray8, vec![vec![7]]).unwrap();
        let mut frame = Frame::video(video).with_hw_frames_context(context.clone());
        let cloned = frame.clone();

        assert!(frame.hw_frames_context().unwrap().shares_storage(&context));
        assert!(cloned
            .hw_frames_context()
            .unwrap()
            .shares_storage(frame.hw_frames_context().unwrap()));
        assert_eq!(
            context.opaque_ref::<String>().map(String::as_str),
            Some("frames")
        );
        assert_eq!(
            frame
                .hw_frames_context()
                .unwrap()
                .opaque_ref::<String>()
                .map(String::as_str),
            Some("frames")
        );

        drop(context);
        assert!(released.lock().unwrap().is_empty());
        let taken = frame.take_hw_frames_context().unwrap();
        assert!(frame.hw_frames_context().is_none());
        drop(frame);
        assert!(released.lock().unwrap().is_empty());
        drop(taken);
        assert!(released.lock().unwrap().is_empty());
        drop(cloned);
        assert_eq!(*released.lock().unwrap(), vec![String::from("frames")]);
    }

    #[test]
    fn frame_hw_context_replacement_releases_unshared_context() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let first_release = std::sync::Arc::clone(&released);
        let second_release = std::sync::Arc::clone(&released);
        let first = BufferRef::from_external_slice_with_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![1]),
            String::from("first"),
            move |opaque| {
                first_release.lock().unwrap().push(opaque);
            },
        );
        let second = BufferRef::from_external_slice_with_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![2]),
            String::from("second"),
            move |opaque| {
                second_release.lock().unwrap().push(opaque);
            },
        );
        let audio = AudioFrame::new(48_000, 1, SampleFormat::S16, 1, vec![vec![0; 2]]).unwrap();
        let mut frame = Frame::audio(audio);

        frame.set_hw_frames_context(Some(first));
        assert!(released.lock().unwrap().is_empty());
        frame.set_hw_frames_context(Some(second));
        assert_eq!(*released.lock().unwrap(), vec![String::from("first")]);
        frame.set_hw_frames_context(None);
        assert_eq!(
            *released.lock().unwrap(),
            vec![String::from("first"), String::from("second")]
        );
    }
}
