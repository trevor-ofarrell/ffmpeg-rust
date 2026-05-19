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
pub struct FramePanScan {
    id: i32,
    width: i32,
    height: i32,
    position: [[i16; Self::COORDINATES]; Self::POSITIONS],
}

impl FramePanScan {
    pub const POSITIONS: usize = 3;
    pub const COORDINATES: usize = 2;
    pub const DATA_LEN: usize = 12 + Self::POSITIONS * Self::COORDINATES * 2;

    pub const fn new(
        id: i32,
        width: i32,
        height: i32,
        position: [[i16; Self::COORDINATES]; Self::POSITIONS],
    ) -> Self {
        Self {
            id,
            width,
            height,
            position,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "pan-scan frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let id = Self::read_i32(data, 0);
        let width = Self::read_i32(data, 4);
        let height = Self::read_i32(data, 8);
        let mut position = [[0; Self::COORDINATES]; Self::POSITIONS];
        let mut offset = 12;
        for field in &mut position {
            for coordinate in field {
                *coordinate = Self::read_i16(data, offset);
                offset += 2;
            }
        }

        Ok(Self {
            id,
            width,
            height,
            position,
        })
    }

    pub const fn id(self) -> i32 {
        self.id
    }

    pub const fn width(self) -> i32 {
        self.width
    }

    pub const fn height(self) -> i32 {
        self.height
    }

    pub const fn position(self) -> [[i16; Self::COORDINATES]; Self::POSITIONS] {
        self.position
    }

    pub const fn field_position(self, index: usize) -> Option<[i16; Self::COORDINATES]> {
        if index < Self::POSITIONS {
            Some(self.position[index])
        } else {
            None
        }
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[0..4].copy_from_slice(&self.id.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.width.to_ne_bytes());
        bytes[8..12].copy_from_slice(&self.height.to_ne_bytes());
        let mut offset = 12;
        for field in &self.position {
            for coordinate in field {
                bytes[offset..offset + 2].copy_from_slice(&coordinate.to_ne_bytes());
                offset += 2;
            }
        }
        bytes
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_i16(data: &[u8], offset: usize) -> i16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&data[offset..offset + 2]);
        i16::from_ne_bytes(raw)
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
pub struct FrameHdrVivid3SplineParams<'a> {
    data: &'a [u8],
}

impl<'a> FrameHdrVivid3SplineParams<'a> {
    pub const DATA_LEN: usize = 44;
    const TH_MODE_OFFSET: usize = 0;
    const TH_ENABLE_MB_OFFSET: usize = 4;
    const TH_ENABLE_OFFSET: usize = 12;
    const TH_DELTA1_OFFSET: usize = 20;
    const TH_DELTA2_OFFSET: usize = 28;
    const ENABLE_STRENGTH_OFFSET: usize = 36;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "HDR Vivid three-spline params require exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let params = Self { data };
        if !(0..=3).contains(&params.th_mode()) {
            return Err(AvError::invalid_data(format!(
                "HDR Vivid three-spline mode {} is outside 0..=3",
                params.th_mode()
            )));
        }

        Ok(params)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn th_mode(self) -> i32 {
        Self::read_i32(self.data, Self::TH_MODE_OFFSET)
    }

    pub fn th_enable_mb(self) -> Rational {
        Self::read_rational(self.data, Self::TH_ENABLE_MB_OFFSET)
    }

    pub fn th_enable(self) -> Rational {
        Self::read_rational(self.data, Self::TH_ENABLE_OFFSET)
    }

    pub fn th_delta1(self) -> Rational {
        Self::read_rational(self.data, Self::TH_DELTA1_OFFSET)
    }

    pub fn th_delta2(self) -> Rational {
        Self::read_rational(self.data, Self::TH_DELTA2_OFFSET)
    }

    pub fn enable_strength(self) -> Rational {
        Self::read_rational(self.data, Self::ENABLE_STRENGTH_OFFSET)
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHdrVividColorToneMappingParams<'a> {
    data: &'a [u8],
}

impl<'a> FrameHdrVividColorToneMappingParams<'a> {
    pub const DATA_LEN: usize = 172;
    pub const MAX_THREE_SPLINES: usize = 2;
    const TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET: usize = 0;
    const BASE_ENABLE_FLAG_OFFSET: usize = 8;
    const BASE_PARAM_M_P_OFFSET: usize = 12;
    const BASE_PARAM_M_M_OFFSET: usize = 20;
    const BASE_PARAM_M_A_OFFSET: usize = 28;
    const BASE_PARAM_M_B_OFFSET: usize = 36;
    const BASE_PARAM_M_N_OFFSET: usize = 44;
    const BASE_PARAM_K1_OFFSET: usize = 52;
    const BASE_PARAM_K2_OFFSET: usize = 56;
    const BASE_PARAM_K3_OFFSET: usize = 60;
    const BASE_PARAM_DELTA_ENABLE_MODE_OFFSET: usize = 64;
    const BASE_PARAM_DELTA_OFFSET: usize = 68;
    const THREE_SPLINE_ENABLE_FLAG_OFFSET: usize = 76;
    const THREE_SPLINE_NUM_OFFSET: usize = 80;
    const THREE_SPLINE_OFFSET: usize = 84;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "HDR Vivid tone-mapping params require exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let params = Self { data };
        validate_i32_bool(
            params.base_enable_flag(),
            "HDR Vivid tone-mapping base enable flag",
        )?;
        validate_i32_bool(
            params.three_spline_enable_flag(),
            "HDR Vivid three-spline enable flag",
        )?;

        if params.three_spline_enable_flag() == 1 {
            let count = params.three_spline_num();
            if !(1..=Self::MAX_THREE_SPLINES).contains(&count) {
                return Err(AvError::invalid_data(format!(
                    "HDR Vivid three-spline count {count} is outside 1..={}",
                    Self::MAX_THREE_SPLINES
                )));
            }
            for index in 0..count {
                params.three_spline(index).unwrap()?.validate()?;
            }
        }

        Ok(params)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn targeted_system_display_maximum_luminance(self) -> Rational {
        Self::read_rational(
            self.data,
            Self::TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET,
        )
    }

    pub fn base_enable_flag(self) -> i32 {
        Self::read_i32(self.data, Self::BASE_ENABLE_FLAG_OFFSET)
    }

    pub fn base_param_m_p(self) -> Rational {
        Self::read_rational(self.data, Self::BASE_PARAM_M_P_OFFSET)
    }

    pub fn base_param_m_m(self) -> Rational {
        Self::read_rational(self.data, Self::BASE_PARAM_M_M_OFFSET)
    }

    pub fn base_param_m_a(self) -> Rational {
        Self::read_rational(self.data, Self::BASE_PARAM_M_A_OFFSET)
    }

    pub fn base_param_m_b(self) -> Rational {
        Self::read_rational(self.data, Self::BASE_PARAM_M_B_OFFSET)
    }

    pub fn base_param_m_n(self) -> Rational {
        Self::read_rational(self.data, Self::BASE_PARAM_M_N_OFFSET)
    }

    pub fn base_param_k1(self) -> i32 {
        Self::read_i32(self.data, Self::BASE_PARAM_K1_OFFSET)
    }

    pub fn base_param_k2(self) -> i32 {
        Self::read_i32(self.data, Self::BASE_PARAM_K2_OFFSET)
    }

    pub fn base_param_k3(self) -> i32 {
        Self::read_i32(self.data, Self::BASE_PARAM_K3_OFFSET)
    }

    pub fn base_param_delta_enable_mode(self) -> i32 {
        Self::read_i32(self.data, Self::BASE_PARAM_DELTA_ENABLE_MODE_OFFSET)
    }

    pub fn base_param_delta(self) -> Rational {
        Self::read_rational(self.data, Self::BASE_PARAM_DELTA_OFFSET)
    }

    pub fn three_spline_enable_flag(self) -> i32 {
        Self::read_i32(self.data, Self::THREE_SPLINE_ENABLE_FLAG_OFFSET)
    }

    pub fn three_spline_num(self) -> usize {
        Self::read_count(self.data, Self::THREE_SPLINE_NUM_OFFSET)
    }

    pub fn three_spline(self, index: usize) -> Option<AvResult<FrameHdrVivid3SplineParams<'a>>> {
        if self.three_spline_enable_flag() != 1 || index >= self.three_spline_num() {
            return None;
        }
        let offset = Self::THREE_SPLINE_OFFSET + index * FrameHdrVivid3SplineParams::DATA_LEN;
        Some(FrameHdrVivid3SplineParams::parse(
            &self.data[offset..offset + FrameHdrVivid3SplineParams::DATA_LEN],
        ))
    }

    fn validate(self) -> AvResult<()> {
        Self::parse(self.data).map(|_| ())
    }

    fn read_rational(data: &[u8], offset: usize) -> Rational {
        Rational::from_raw(
            Self::read_i32(data, offset),
            Self::read_i32(data, offset + 4),
        )
    }

    fn read_count(data: &[u8], offset: usize) -> usize {
        usize::try_from(Self::read_i32(data, offset)).unwrap_or(usize::MAX)
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }
}

impl<'a> FrameHdrVivid3SplineParams<'a> {
    fn validate(self) -> AvResult<()> {
        Self::parse(self.data).map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHdrVividColorTransformParams<'a> {
    data: &'a [u8],
}

impl<'a> FrameHdrVividColorTransformParams<'a> {
    pub const DATA_LEN: usize = 456;
    pub const MAX_TONE_MAPPING_PARAMS: usize = 2;
    pub const MAX_COLOR_SATURATION_GAINS: usize = 7;
    const MINIMUM_MAXRGB_OFFSET: usize = 0;
    const AVERAGE_MAXRGB_OFFSET: usize = 8;
    const VARIANCE_MAXRGB_OFFSET: usize = 16;
    const MAXIMUM_MAXRGB_OFFSET: usize = 24;
    const TONE_MAPPING_MODE_FLAG_OFFSET: usize = 32;
    const TONE_MAPPING_PARAM_NUM_OFFSET: usize = 36;
    const TONE_MAPPING_PARAMS_OFFSET: usize = 40;
    const COLOR_SATURATION_MAPPING_FLAG_OFFSET: usize = Self::TONE_MAPPING_PARAMS_OFFSET
        + Self::MAX_TONE_MAPPING_PARAMS * FrameHdrVividColorToneMappingParams::DATA_LEN;
    const COLOR_SATURATION_NUM_OFFSET: usize = Self::COLOR_SATURATION_MAPPING_FLAG_OFFSET + 4;
    const COLOR_SATURATION_GAIN_OFFSET: usize = Self::COLOR_SATURATION_NUM_OFFSET + 4;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "HDR Vivid color-transform params require exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let params = Self { data };
        validate_i32_bool(
            params.tone_mapping_mode_flag(),
            "HDR Vivid tone-mapping mode flag",
        )?;
        validate_i32_bool(
            params.color_saturation_mapping_flag(),
            "HDR Vivid color-saturation mapping flag",
        )?;

        if params.tone_mapping_mode_flag() == 1 {
            let count = params.tone_mapping_param_num();
            if !(1..=Self::MAX_TONE_MAPPING_PARAMS).contains(&count) {
                return Err(AvError::invalid_data(format!(
                    "HDR Vivid tone-mapping parameter count {count} is outside 1..={}",
                    Self::MAX_TONE_MAPPING_PARAMS
                )));
            }
            for index in 0..count {
                params.tone_mapping_params(index).unwrap().validate()?;
            }
        }

        if params.color_saturation_mapping_flag() == 1
            && params.color_saturation_num() > Self::MAX_COLOR_SATURATION_GAINS
        {
            return Err(AvError::invalid_data(format!(
                "HDR Vivid color-saturation count {} exceeds {}",
                params.color_saturation_num(),
                Self::MAX_COLOR_SATURATION_GAINS
            )));
        }

        Ok(params)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn minimum_maxrgb(self) -> Rational {
        Self::read_rational(self.data, Self::MINIMUM_MAXRGB_OFFSET)
    }

    pub fn average_maxrgb(self) -> Rational {
        Self::read_rational(self.data, Self::AVERAGE_MAXRGB_OFFSET)
    }

    pub fn variance_maxrgb(self) -> Rational {
        Self::read_rational(self.data, Self::VARIANCE_MAXRGB_OFFSET)
    }

    pub fn maximum_maxrgb(self) -> Rational {
        Self::read_rational(self.data, Self::MAXIMUM_MAXRGB_OFFSET)
    }

    pub fn tone_mapping_mode_flag(self) -> i32 {
        Self::read_i32(self.data, Self::TONE_MAPPING_MODE_FLAG_OFFSET)
    }

    pub fn tone_mapping_param_num(self) -> usize {
        Self::read_count(self.data, Self::TONE_MAPPING_PARAM_NUM_OFFSET)
    }

    pub fn tone_mapping_params(
        self,
        index: usize,
    ) -> Option<FrameHdrVividColorToneMappingParams<'a>> {
        if self.tone_mapping_mode_flag() != 1 || index >= self.tone_mapping_param_num() {
            return None;
        }
        let offset = Self::TONE_MAPPING_PARAMS_OFFSET
            + index * FrameHdrVividColorToneMappingParams::DATA_LEN;
        Some(FrameHdrVividColorToneMappingParams {
            data: &self.data[offset..offset + FrameHdrVividColorToneMappingParams::DATA_LEN],
        })
    }

    pub fn color_saturation_mapping_flag(self) -> i32 {
        Self::read_i32(self.data, Self::COLOR_SATURATION_MAPPING_FLAG_OFFSET)
    }

    pub fn color_saturation_num(self) -> usize {
        Self::read_count(self.data, Self::COLOR_SATURATION_NUM_OFFSET)
    }

    pub fn color_saturation_gain(self, index: usize) -> Option<Rational> {
        (self.color_saturation_mapping_flag() == 1 && index < self.color_saturation_num())
            .then(|| Self::read_rational(self.data, Self::COLOR_SATURATION_GAIN_OFFSET + index * 8))
    }

    fn validate(self) -> AvResult<()> {
        Self::parse(self.data).map(|_| ())
    }

    fn read_rational(data: &[u8], offset: usize) -> Rational {
        Rational::from_raw(
            Self::read_i32(data, offset),
            Self::read_i32(data, offset + 4),
        )
    }

    fn read_count(data: &[u8], offset: usize) -> usize {
        usize::try_from(Self::read_i32(data, offset)).unwrap_or(usize::MAX)
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDynamicHdrVivid<'a> {
    data: &'a [u8],
    num_windows: usize,
}

impl<'a> FrameDynamicHdrVivid<'a> {
    pub const MIN_SYSTEM_START_CODE: u8 = 0x01;
    pub const MAX_SYSTEM_START_CODE: u8 = 0x07;
    pub const MAX_WINDOWS: usize = 3;
    const PARAMS_OFFSET: usize = 4;
    pub const DATA_LEN: usize =
        Self::PARAMS_OFFSET + Self::MAX_WINDOWS * FrameHdrVividColorTransformParams::DATA_LEN;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "HDR Vivid frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        if !(Self::MIN_SYSTEM_START_CODE..=Self::MAX_SYSTEM_START_CODE).contains(&data[0]) {
            return Err(AvError::invalid_data(format!(
                "HDR Vivid system start code 0x{:02X} is outside 0x{:02X}..=0x{:02X}",
                data[0],
                Self::MIN_SYSTEM_START_CODE,
                Self::MAX_SYSTEM_START_CODE
            )));
        }

        let num_windows = usize::from(data[1]);
        if !(1..=Self::MAX_WINDOWS).contains(&num_windows) {
            return Err(AvError::invalid_data(format!(
                "HDR Vivid window count {num_windows} is outside 1..={}",
                Self::MAX_WINDOWS
            )));
        }

        let parsed = Self { data, num_windows };
        for index in 0..num_windows {
            parsed.color_transform_params(index).unwrap().validate()?;
        }

        Ok(parsed)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn system_start_code(self) -> u8 {
        self.data[0]
    }

    pub const fn num_windows(self) -> usize {
        self.num_windows
    }

    pub fn color_transform_params(
        self,
        index: usize,
    ) -> Option<FrameHdrVividColorTransformParams<'a>> {
        (index < self.num_windows).then(|| {
            let offset = Self::PARAMS_OFFSET + index * FrameHdrVividColorTransformParams::DATA_LEN;
            FrameHdrVividColorTransformParams {
                data: &self.data[offset..offset + FrameHdrVividColorTransformParams::DATA_LEN],
            }
        })
    }
}

fn validate_i32_bool(value: i32, name: &str) -> AvResult<()> {
    if value == 0 || value == 1 {
        return Ok(());
    }

    Err(AvError::invalid_data(format!(
        "{name} {value} is not boolean"
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRegionOfInterest {
    top: i32,
    bottom: i32,
    left: i32,
    right: i32,
    qoffset: Rational,
}

impl FrameRegionOfInterest {
    pub const DATA_LEN: usize = 28;
    pub const SELF_SIZE: u32 = Self::DATA_LEN as u32;

    pub fn new(top: i32, bottom: i32, left: i32, right: i32, qoffset: Rational) -> AvResult<Self> {
        if !Self::qoffset_is_valid(qoffset) {
            return Err(AvError::invalid_argument(format!(
                "region-of-interest qoffset {qoffset} is outside -1..=1"
            )));
        }

        Ok(Self {
            top,
            bottom,
            left,
            right,
            qoffset,
        })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "region-of-interest side-data record requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let self_size = Self::read_u32(data, 0);
        if self_size != Self::SELF_SIZE {
            return Err(AvError::invalid_data(format!(
                "region-of-interest self_size {self_size} does not match {}",
                Self::SELF_SIZE
            )));
        }

        let qoffset = Rational::from_raw(Self::read_i32(data, 20), Self::read_i32(data, 24));
        if !Self::qoffset_is_valid(qoffset) {
            return Err(AvError::invalid_data(format!(
                "region-of-interest qoffset {qoffset} is outside -1..=1"
            )));
        }

        Ok(Self {
            top: Self::read_i32(data, 4),
            bottom: Self::read_i32(data, 8),
            left: Self::read_i32(data, 12),
            right: Self::read_i32(data, 16),
            qoffset,
        })
    }

    pub const fn self_size(self) -> u32 {
        Self::SELF_SIZE
    }

    pub const fn top(self) -> i32 {
        self.top
    }

    pub const fn bottom(self) -> i32 {
        self.bottom
    }

    pub const fn left(self) -> i32 {
        self.left
    }

    pub const fn right(self) -> i32 {
        self.right
    }

    pub const fn qoffset(self) -> Rational {
        self.qoffset
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[0..4].copy_from_slice(&Self::SELF_SIZE.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.top.to_ne_bytes());
        bytes[8..12].copy_from_slice(&self.bottom.to_ne_bytes());
        bytes[12..16].copy_from_slice(&self.left.to_ne_bytes());
        bytes[16..20].copy_from_slice(&self.right.to_ne_bytes());
        bytes[20..24].copy_from_slice(&self.qoffset.num().to_ne_bytes());
        bytes[24..28].copy_from_slice(&self.qoffset.den().to_ne_bytes());
        bytes
    }

    fn qoffset_is_valid(qoffset: Rational) -> bool {
        if qoffset.den() == 0 {
            return false;
        }

        let mut num = i128::from(qoffset.num());
        let mut den = i128::from(qoffset.den());
        if den < 0 {
            num = -num;
            den = -den;
        }

        (-den..=den).contains(&num)
    }

    fn read_u32(data: &[u8], offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        u32::from_ne_bytes(raw)
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameRegionsOfInterest {
    regions: Vec<FrameRegionOfInterest>,
}

impl FrameRegionsOfInterest {
    pub fn new(regions: Vec<FrameRegionOfInterest>) -> AvResult<Self> {
        if regions.is_empty() {
            return Err(AvError::invalid_data(
                "regions-of-interest frame side data requires at least one record",
            ));
        }

        Ok(Self { regions })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.is_empty()
            || !data
                .chunks_exact(FrameRegionOfInterest::DATA_LEN)
                .remainder()
                .is_empty()
        {
            return Err(AvError::invalid_data(format!(
                "regions-of-interest frame side data requires a non-empty multiple of {} bytes, got {}",
                FrameRegionOfInterest::DATA_LEN,
                data.len()
            )));
        }

        let regions = data
            .chunks_exact(FrameRegionOfInterest::DATA_LEN)
            .map(FrameRegionOfInterest::parse)
            .collect::<AvResult<Vec<_>>>()?;
        Self::new(regions)
    }

    pub fn regions(&self) -> &[FrameRegionOfInterest] {
        &self.regions
    }

    pub fn into_regions(self) -> Vec<FrameRegionOfInterest> {
        self.regions
    }

    pub fn len(&self) -> usize {
        self.regions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.regions.len() * FrameRegionOfInterest::DATA_LEN);
        for region in &self.regions {
            bytes.extend_from_slice(&region.to_bytes());
        }
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameVideoEncParamsType {
    None,
    Vp9,
    H264,
    Mpeg2,
}

impl FrameVideoEncParamsType {
    pub const fn as_raw(self) -> i32 {
        match self {
            Self::None => -1,
            Self::Vp9 => 0,
            Self::H264 => 1,
            Self::Mpeg2 => 2,
        }
    }

    pub fn from_raw(raw: i32) -> AvResult<Self> {
        match raw {
            -1 => Ok(Self::None),
            0 => Ok(Self::Vp9),
            1 => Ok(Self::H264),
            2 => Ok(Self::Mpeg2),
            _ => Err(AvError::invalid_data(format!(
                "unknown video encoding parameters type {raw}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameVideoBlockParams {
    src_x: i32,
    src_y: i32,
    width: i32,
    height: i32,
    delta_qp: i32,
}

impl FrameVideoBlockParams {
    pub const DATA_LEN: usize = 20;

    pub fn new(src_x: i32, src_y: i32, width: i32, height: i32, delta_qp: i32) -> AvResult<Self> {
        if width <= 0 || height <= 0 {
            return Err(AvError::invalid_argument(format!(
                "video encoding block dimensions must be positive, got {width}x{height}"
            )));
        }

        Ok(Self {
            src_x,
            src_y,
            width,
            height,
            delta_qp,
        })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "video encoding block parameters require exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let width = Self::read_i32(data, 8);
        let height = Self::read_i32(data, 12);
        if width <= 0 || height <= 0 {
            return Err(AvError::invalid_data(format!(
                "video encoding block dimensions must be positive, got {width}x{height}"
            )));
        }

        Ok(Self {
            src_x: Self::read_i32(data, 0),
            src_y: Self::read_i32(data, 4),
            width,
            height,
            delta_qp: Self::read_i32(data, 16),
        })
    }

    pub const fn src_x(self) -> i32 {
        self.src_x
    }

    pub const fn src_y(self) -> i32 {
        self.src_y
    }

    pub const fn width(self) -> i32 {
        self.width
    }

    pub const fn height(self) -> i32 {
        self.height
    }

    pub const fn delta_qp(self) -> i32 {
        self.delta_qp
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[0..4].copy_from_slice(&self.src_x.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.src_y.to_ne_bytes());
        bytes[8..12].copy_from_slice(&self.width.to_ne_bytes());
        bytes[12..16].copy_from_slice(&self.height.to_ne_bytes());
        bytes[16..20].copy_from_slice(&self.delta_qp.to_ne_bytes());
        bytes
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameVideoEncParams {
    params_type: FrameVideoEncParamsType,
    qp: i32,
    delta_qp: [[i32; 2]; 4],
    blocks: Vec<FrameVideoBlockParams>,
}

impl FrameVideoEncParams {
    const SIZE_T_LEN: usize = core::mem::size_of::<usize>();
    const BLOCKS_OFFSET_OFFSET: usize = if Self::SIZE_T_LEN == 8 { 8 } else { 4 };
    const BLOCK_SIZE_OFFSET: usize = Self::BLOCKS_OFFSET_OFFSET + Self::SIZE_T_LEN;
    const TYPE_OFFSET: usize = Self::BLOCK_SIZE_OFFSET + Self::SIZE_T_LEN;
    const QP_OFFSET: usize = Self::TYPE_OFFSET + 4;
    const DELTA_QP_OFFSET: usize = Self::QP_OFFSET + 4;
    pub const DELTA_QP_PLANES: usize = 4;
    pub const DELTA_QP_COEFFS: usize = 2;
    pub const HEADER_LEN: usize =
        Self::DELTA_QP_OFFSET + Self::DELTA_QP_PLANES * Self::DELTA_QP_COEFFS * 4;
    pub const BLOCK_SIZE: usize = FrameVideoBlockParams::DATA_LEN;

    pub fn new(
        params_type: FrameVideoEncParamsType,
        qp: i32,
        delta_qp: [[i32; Self::DELTA_QP_COEFFS]; Self::DELTA_QP_PLANES],
        blocks: Vec<FrameVideoBlockParams>,
    ) -> AvResult<Self> {
        if blocks.len() > u32::MAX as usize {
            return Err(AvError::invalid_argument(format!(
                "too many video encoding parameter blocks: {}",
                blocks.len()
            )));
        }
        Self::expected_data_len(blocks.len())?;

        Ok(Self {
            params_type,
            qp,
            delta_qp,
            blocks,
        })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "video encoding parameters require at least {} bytes, got {}",
                Self::HEADER_LEN,
                data.len()
            )));
        }

        let nb_blocks = Self::read_u32(data, 0) as usize;
        let blocks_offset = Self::read_usize(data, Self::BLOCKS_OFFSET_OFFSET);
        if blocks_offset != Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "video encoding parameters block offset {blocks_offset} does not match {}",
                Self::HEADER_LEN
            )));
        }

        let block_size = Self::read_usize(data, Self::BLOCK_SIZE_OFFSET);
        if block_size != Self::BLOCK_SIZE {
            return Err(AvError::invalid_data(format!(
                "video encoding parameters block size {block_size} does not match {}",
                Self::BLOCK_SIZE
            )));
        }

        let expected_len = Self::expected_data_len(nb_blocks)?;
        if data.len() != expected_len {
            return Err(AvError::invalid_data(format!(
                "video encoding parameters payload requires exactly {expected_len} bytes, got {}",
                data.len()
            )));
        }

        let params_type =
            FrameVideoEncParamsType::from_raw(Self::read_i32(data, Self::TYPE_OFFSET))?;
        let qp = Self::read_i32(data, Self::QP_OFFSET);
        let mut delta_qp = [[0; Self::DELTA_QP_COEFFS]; Self::DELTA_QP_PLANES];
        for (plane, plane_delta_qp) in delta_qp.iter_mut().enumerate() {
            for (coeff, value) in plane_delta_qp.iter_mut().enumerate() {
                let index = plane * Self::DELTA_QP_COEFFS + coeff;
                *value = Self::read_i32(data, Self::DELTA_QP_OFFSET + index * 4);
            }
        }

        let mut blocks = Vec::with_capacity(nb_blocks);
        for index in 0..nb_blocks {
            let offset = Self::HEADER_LEN + index * Self::BLOCK_SIZE;
            blocks.push(FrameVideoBlockParams::parse(
                &data[offset..offset + Self::BLOCK_SIZE],
            )?);
        }

        Ok(Self {
            params_type,
            qp,
            delta_qp,
            blocks,
        })
    }

    pub const fn params_type(&self) -> FrameVideoEncParamsType {
        self.params_type
    }

    pub const fn qp(&self) -> i32 {
        self.qp
    }

    pub const fn delta_qp(&self) -> &[[i32; Self::DELTA_QP_COEFFS]; Self::DELTA_QP_PLANES] {
        &self.delta_qp
    }

    pub fn blocks(&self) -> &[FrameVideoBlockParams] {
        &self.blocks
    }

    pub fn into_blocks(self) -> Vec<FrameVideoBlockParams> {
        self.blocks
    }

    pub fn nb_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0; Self::HEADER_LEN];
        Self::write_u32(&mut bytes, 0, self.blocks.len() as u32);
        Self::write_usize(&mut bytes, Self::BLOCKS_OFFSET_OFFSET, Self::HEADER_LEN);
        Self::write_usize(&mut bytes, Self::BLOCK_SIZE_OFFSET, Self::BLOCK_SIZE);
        Self::write_i32(&mut bytes, Self::TYPE_OFFSET, self.params_type.as_raw());
        Self::write_i32(&mut bytes, Self::QP_OFFSET, self.qp);
        for plane in 0..Self::DELTA_QP_PLANES {
            for coeff in 0..Self::DELTA_QP_COEFFS {
                let index = plane * Self::DELTA_QP_COEFFS + coeff;
                Self::write_i32(
                    &mut bytes,
                    Self::DELTA_QP_OFFSET + index * 4,
                    self.delta_qp[plane][coeff],
                );
            }
        }
        for block in &self.blocks {
            bytes.extend_from_slice(&block.to_bytes());
        }
        bytes
    }

    fn expected_data_len(nb_blocks: usize) -> AvResult<usize> {
        let blocks_len = nb_blocks.checked_mul(Self::BLOCK_SIZE).ok_or_else(|| {
            AvError::invalid_data("video encoding parameters block data length overflow")
        })?;
        Self::HEADER_LEN.checked_add(blocks_len).ok_or_else(|| {
            AvError::invalid_data("video encoding parameters payload length overflow")
        })
    }

    fn read_u32(data: &[u8], offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        u32::from_ne_bytes(raw)
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_usize(data: &[u8], offset: usize) -> usize {
        let mut raw = [0; Self::SIZE_T_LEN];
        raw.copy_from_slice(&data[offset..offset + Self::SIZE_T_LEN]);
        usize::from_ne_bytes(raw)
    }

    fn write_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_usize(data: &mut [u8], offset: usize, value: usize) {
        data[offset..offset + Self::SIZE_T_LEN].copy_from_slice(&value.to_ne_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameFilmGrainParamsType {
    None,
    Av1,
    H274,
}

impl FrameFilmGrainParamsType {
    pub const fn as_raw(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Av1 => 1,
            Self::H274 => 2,
        }
    }

    pub fn from_raw(raw: i32) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::None),
            1 => Ok(Self::Av1),
            2 => Ok(Self::H274),
            _ => Err(AvError::invalid_data(format!(
                "unknown film grain parameters type {raw}"
            ))),
        }
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::None => "AV_FILM_GRAIN_PARAMS_NONE",
            Self::Av1 => "AV_FILM_GRAIN_PARAMS_AV1",
            Self::H274 => "AV_FILM_GRAIN_PARAMS_H274",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameFilmGrainAomParams<'a> {
    data: &'a [u8],
}

impl<'a> FrameFilmGrainAomParams<'a> {
    pub const Y_POINTS: usize = 14;
    pub const UV_PLANES: usize = 2;
    pub const UV_POINTS: usize = 10;
    pub const AR_COEFFS_Y: usize = 24;
    pub const AR_COEFFS_UV: usize = 25;
    pub const DATA_LEN: usize = 208;
    const NUM_Y_POINTS_OFFSET: usize = 0;
    const Y_POINTS_OFFSET: usize = 4;
    const CHROMA_SCALING_FROM_LUMA_OFFSET: usize = 32;
    const NUM_UV_POINTS_OFFSET: usize = 36;
    const UV_POINTS_OFFSET: usize = 44;
    const SCALING_SHIFT_OFFSET: usize = 84;
    const AR_COEFF_LAG_OFFSET: usize = 88;
    const AR_COEFFS_Y_OFFSET: usize = 92;
    const AR_COEFFS_UV_OFFSET: usize = 116;
    const AR_COEFF_SHIFT_OFFSET: usize = 168;
    const GRAIN_SCALE_SHIFT_OFFSET: usize = 172;
    const UV_MULT_OFFSET: usize = 176;
    const UV_MULT_LUMA_OFFSET: usize = 184;
    const UV_OFFSET_OFFSET: usize = 192;
    const OVERLAP_FLAG_OFFSET: usize = 200;
    const LIMIT_OUTPUT_RANGE_OFFSET: usize = 204;

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "AOM film grain parameters require exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let params = Self { data };
        let num_y_points_raw = params.read_i32(Self::NUM_Y_POINTS_OFFSET);
        if !(0..=Self::Y_POINTS as i32).contains(&num_y_points_raw) {
            return Err(AvError::invalid_data(format!(
                "AOM film grain luma point count {num_y_points_raw} is outside 0..={}",
                Self::Y_POINTS
            )));
        }
        Self::validate_bool(
            params.read_i32(Self::CHROMA_SCALING_FROM_LUMA_OFFSET),
            "AOM chroma scaling from luma",
        )?;
        for plane in 0..Self::UV_PLANES {
            let num_uv_points_raw = params.read_i32(Self::NUM_UV_POINTS_OFFSET + plane * 4);
            if !(0..=Self::UV_POINTS as i32).contains(&num_uv_points_raw) {
                return Err(AvError::invalid_data(format!(
                    "AOM film grain chroma plane {plane} point count {num_uv_points_raw} is outside 0..={}",
                    Self::UV_POINTS
                )));
            }
        }
        if !(8..=11).contains(&params.scaling_shift()) {
            return Err(AvError::invalid_data(format!(
                "AOM film grain scaling shift {} is outside 8..=11",
                params.scaling_shift()
            )));
        }
        if !(0..=3).contains(&params.ar_coeff_lag()) {
            return Err(AvError::invalid_data(format!(
                "AOM film grain AR coefficient lag {} is outside 0..=3",
                params.ar_coeff_lag()
            )));
        }
        if !(6..=9).contains(&params.ar_coeff_shift()) {
            return Err(AvError::invalid_data(format!(
                "AOM film grain AR coefficient shift {} is outside 6..=9",
                params.ar_coeff_shift()
            )));
        }
        if !(0..=3).contains(&params.grain_scale_shift()) {
            return Err(AvError::invalid_data(format!(
                "AOM film grain scale shift {} is outside 0..=3",
                params.grain_scale_shift()
            )));
        }
        for plane in 0..Self::UV_PLANES {
            let uv_offset = params.uv_offset(plane).unwrap();
            if !(-256..=255).contains(&uv_offset) {
                return Err(AvError::invalid_data(format!(
                    "AOM film grain chroma plane {plane} offset {uv_offset} is outside -256..=255"
                )));
            }
        }
        Self::validate_bool(
            params.read_i32(Self::OVERLAP_FLAG_OFFSET),
            "AOM film grain overlap flag",
        )?;
        Self::validate_bool(
            params.read_i32(Self::LIMIT_OUTPUT_RANGE_OFFSET),
            "AOM film grain limit output range",
        )?;

        Ok(params)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn num_y_points(self) -> usize {
        self.read_i32(Self::NUM_Y_POINTS_OFFSET).max(0) as usize
    }

    pub fn y_point(self, index: usize) -> Option<[u8; 2]> {
        if index >= self.num_y_points() {
            return None;
        }
        let offset = Self::Y_POINTS_OFFSET + index * 2;
        Some([self.data[offset], self.data[offset + 1]])
    }

    pub fn chroma_scaling_from_luma(self) -> bool {
        self.read_i32(Self::CHROMA_SCALING_FROM_LUMA_OFFSET) != 0
    }

    pub fn num_uv_points(self, plane: usize) -> Option<usize> {
        if plane >= Self::UV_PLANES {
            return None;
        }
        Some(self.read_i32(Self::NUM_UV_POINTS_OFFSET + plane * 4).max(0) as usize)
    }

    pub fn uv_point(self, plane: usize, index: usize) -> Option<[u8; 2]> {
        if plane >= Self::UV_PLANES || index >= self.num_uv_points(plane).unwrap() {
            return None;
        }
        let offset = Self::UV_POINTS_OFFSET + plane * Self::UV_POINTS * 2 + index * 2;
        Some([self.data[offset], self.data[offset + 1]])
    }

    pub fn scaling_shift(self) -> i32 {
        self.read_i32(Self::SCALING_SHIFT_OFFSET)
    }

    pub fn ar_coeff_lag(self) -> i32 {
        self.read_i32(Self::AR_COEFF_LAG_OFFSET)
    }

    pub fn ar_coeff_count_y(self) -> usize {
        let lag = self.ar_coeff_lag().max(0) as usize;
        2 * lag * (lag + 1)
    }

    pub fn ar_coeff_count_uv(self) -> usize {
        self.ar_coeff_count_y() + usize::from(self.num_y_points() > 0)
    }

    pub fn ar_coeff_y(self, index: usize) -> Option<i8> {
        if index >= self.ar_coeff_count_y() {
            return None;
        }
        Some(self.data[Self::AR_COEFFS_Y_OFFSET + index] as i8)
    }

    pub fn ar_coeff_uv(self, plane: usize, index: usize) -> Option<i8> {
        if plane >= Self::UV_PLANES || index >= self.ar_coeff_count_uv() {
            return None;
        }
        Some(self.data[Self::AR_COEFFS_UV_OFFSET + plane * Self::AR_COEFFS_UV + index] as i8)
    }

    pub fn ar_coeff_shift(self) -> i32 {
        self.read_i32(Self::AR_COEFF_SHIFT_OFFSET)
    }

    pub fn grain_scale_shift(self) -> i32 {
        self.read_i32(Self::GRAIN_SCALE_SHIFT_OFFSET)
    }

    pub fn uv_mult(self, plane: usize) -> Option<i32> {
        if plane >= Self::UV_PLANES {
            return None;
        }
        Some(self.read_i32(Self::UV_MULT_OFFSET + plane * 4))
    }

    pub fn uv_mult_luma(self, plane: usize) -> Option<i32> {
        if plane >= Self::UV_PLANES {
            return None;
        }
        Some(self.read_i32(Self::UV_MULT_LUMA_OFFSET + plane * 4))
    }

    pub fn uv_offset(self, plane: usize) -> Option<i32> {
        if plane >= Self::UV_PLANES {
            return None;
        }
        Some(self.read_i32(Self::UV_OFFSET_OFFSET + plane * 4))
    }

    pub fn overlap_flag(self) -> bool {
        self.read_i32(Self::OVERLAP_FLAG_OFFSET) != 0
    }

    pub fn limit_output_range(self) -> bool {
        self.read_i32(Self::LIMIT_OUTPUT_RANGE_OFFSET) != 0
    }

    fn validate_bool(value: i32, label: &str) -> AvResult<()> {
        if matches!(value, 0 | 1) {
            Ok(())
        } else {
            Err(AvError::invalid_data(format!(
                "{label} value {value} is not boolean"
            )))
        }
    }

    fn read_i32(self, offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&self.data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameFilmGrainH274Params<'a> {
    data: &'a [u8],
}

impl<'a> FrameFilmGrainH274Params<'a> {
    pub const COMPONENTS: usize = 3;
    pub const MAX_INTENSITY_INTERVALS: usize = 256;
    pub const MAX_MODEL_VALUES: usize = 6;
    pub const DATA_LEN: usize = 10_788;
    const MODEL_ID_OFFSET: usize = 0;
    const BLENDING_MODE_ID_OFFSET: usize = 4;
    const LOG2_SCALE_FACTOR_OFFSET: usize = 8;
    const COMPONENT_MODEL_PRESENT_OFFSET: usize = 12;
    const NUM_INTENSITY_INTERVALS_OFFSET: usize = 24;
    const NUM_MODEL_VALUES_OFFSET: usize = 30;
    const INTENSITY_INTERVAL_LOWER_BOUND_OFFSET: usize = 33;
    const INTENSITY_INTERVAL_UPPER_BOUND_OFFSET: usize = Self::INTENSITY_INTERVAL_LOWER_BOUND_OFFSET
        + Self::COMPONENTS * Self::MAX_INTENSITY_INTERVALS;
    const COMP_MODEL_VALUE_OFFSET: usize = 1_570;

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "H.274 film grain parameters require exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let params = Self { data };
        if !matches!(params.model_id(), 0 | 1) {
            return Err(AvError::invalid_data(format!(
                "H.274 film grain model id {} is outside 0..=1",
                params.model_id()
            )));
        }
        if !matches!(params.blending_mode_id(), 0 | 1) {
            return Err(AvError::invalid_data(format!(
                "H.274 film grain blending mode id {} is outside 0..=1",
                params.blending_mode_id()
            )));
        }

        for component in 0..Self::COMPONENTS {
            let present_raw = params.read_i32(Self::COMPONENT_MODEL_PRESENT_OFFSET + component * 4);
            if !matches!(present_raw, 0 | 1) {
                return Err(AvError::invalid_data(format!(
                    "H.274 film grain component {component} present value {present_raw} is not boolean"
                )));
            }

            let intervals = params.num_intensity_intervals(component).unwrap();
            let model_values = params.num_model_values(component).unwrap();
            if present_raw == 0 {
                if intervals != 0 || model_values != 0 {
                    return Err(AvError::invalid_data(format!(
                        "H.274 film grain component {component} is absent but carries interval/model counts"
                    )));
                }
                continue;
            }

            if !(1..=Self::MAX_INTENSITY_INTERVALS).contains(&intervals) {
                return Err(AvError::invalid_data(format!(
                    "H.274 film grain component {component} interval count {intervals} is outside 1..={}",
                    Self::MAX_INTENSITY_INTERVALS
                )));
            }
            if !(1..=Self::MAX_MODEL_VALUES).contains(&model_values) {
                return Err(AvError::invalid_data(format!(
                    "H.274 film grain component {component} model value count {model_values} is outside 1..={}",
                    Self::MAX_MODEL_VALUES
                )));
            }
            for interval in 0..intervals {
                let lower = params
                    .intensity_interval_lower_bound(component, interval)
                    .unwrap();
                let upper = params
                    .intensity_interval_upper_bound(component, interval)
                    .unwrap();
                if lower > upper {
                    return Err(AvError::invalid_data(format!(
                        "H.274 film grain component {component} interval {interval} lower bound {lower} exceeds upper bound {upper}"
                    )));
                }
            }
        }

        Ok(params)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn model_id(self) -> i32 {
        self.read_i32(Self::MODEL_ID_OFFSET)
    }

    pub fn blending_mode_id(self) -> i32 {
        self.read_i32(Self::BLENDING_MODE_ID_OFFSET)
    }

    pub fn log2_scale_factor(self) -> i32 {
        self.read_i32(Self::LOG2_SCALE_FACTOR_OFFSET)
    }

    pub fn component_model_present(self, component: usize) -> Option<bool> {
        if component >= Self::COMPONENTS {
            return None;
        }
        Some(self.read_i32(Self::COMPONENT_MODEL_PRESENT_OFFSET + component * 4) != 0)
    }

    pub fn num_intensity_intervals(self, component: usize) -> Option<usize> {
        if component >= Self::COMPONENTS {
            return None;
        }
        Some(usize::from(self.read_u16(
            Self::NUM_INTENSITY_INTERVALS_OFFSET + component * 2,
        )))
    }

    pub fn num_model_values(self, component: usize) -> Option<usize> {
        if component >= Self::COMPONENTS {
            return None;
        }
        Some(usize::from(
            self.data[Self::NUM_MODEL_VALUES_OFFSET + component],
        ))
    }

    pub fn intensity_interval_lower_bound(self, component: usize, interval: usize) -> Option<u8> {
        if component >= Self::COMPONENTS
            || interval >= self.num_intensity_intervals(component).unwrap()
        {
            return None;
        }
        Some(self.data[Self::INTENSITY_INTERVAL_LOWER_BOUND_OFFSET + component * 256 + interval])
    }

    pub fn intensity_interval_upper_bound(self, component: usize, interval: usize) -> Option<u8> {
        if component >= Self::COMPONENTS
            || interval >= self.num_intensity_intervals(component).unwrap()
        {
            return None;
        }
        Some(self.data[Self::INTENSITY_INTERVAL_UPPER_BOUND_OFFSET + component * 256 + interval])
    }

    pub fn comp_model_value(self, component: usize, interval: usize, value: usize) -> Option<i16> {
        if component >= Self::COMPONENTS
            || interval >= self.num_intensity_intervals(component).unwrap()
            || value >= self.num_model_values(component).unwrap()
        {
            return None;
        }
        let offset = Self::COMP_MODEL_VALUE_OFFSET
            + ((component * Self::MAX_INTENSITY_INTERVALS + interval) * Self::MAX_MODEL_VALUES
                + value)
                * 2;
        Some(self.read_i16(offset))
    }

    fn read_i32(self, offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&self.data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_u16(self, offset: usize) -> u16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&self.data[offset..offset + 2]);
        u16::from_ne_bytes(raw)
    }

    fn read_i16(self, offset: usize) -> i16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&self.data[offset..offset + 2]);
        i16::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameFilmGrainParams<'a> {
    data: &'a [u8],
    params_type: FrameFilmGrainParamsType,
}

impl<'a> FrameFilmGrainParams<'a> {
    const TYPE_OFFSET: usize = 0;
    const SEED_OFFSET: usize = Self::align_up(Self::TYPE_OFFSET + 4, core::mem::align_of::<u64>());
    const WIDTH_OFFSET: usize = Self::SEED_OFFSET + 8;
    const HEIGHT_OFFSET: usize = Self::WIDTH_OFFSET + 4;
    const SUBSAMPLING_X_OFFSET: usize = Self::HEIGHT_OFFSET + 4;
    const SUBSAMPLING_Y_OFFSET: usize = Self::SUBSAMPLING_X_OFFSET + 4;
    const COLOR_RANGE_OFFSET: usize = Self::SUBSAMPLING_Y_OFFSET + 4;
    const COLOR_PRIMARIES_OFFSET: usize = Self::COLOR_RANGE_OFFSET + 4;
    const COLOR_TRC_OFFSET: usize = Self::COLOR_PRIMARIES_OFFSET + 4;
    const COLOR_SPACE_OFFSET: usize = Self::COLOR_TRC_OFFSET + 4;
    const BIT_DEPTH_LUMA_OFFSET: usize = Self::COLOR_SPACE_OFFSET + 4;
    const BIT_DEPTH_CHROMA_OFFSET: usize = Self::BIT_DEPTH_LUMA_OFFSET + 4;
    const CODEC_OFFSET: usize = Self::BIT_DEPTH_CHROMA_OFFSET + 4;
    pub const DATA_LEN: usize = Self::align_up(
        Self::CODEC_OFFSET + FrameFilmGrainH274Params::DATA_LEN,
        core::mem::align_of::<u64>(),
    );

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "film grain parameters frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let params_type =
            FrameFilmGrainParamsType::from_raw(Self::read_i32(data, Self::TYPE_OFFSET))?;
        let params = Self { data, params_type };
        for (label, value) in [
            ("width", params.width()),
            ("height", params.height()),
            ("subsampling_x", params.subsampling_x()),
            ("subsampling_y", params.subsampling_y()),
            ("bit_depth_luma", params.bit_depth_luma()),
            ("bit_depth_chroma", params.bit_depth_chroma()),
        ] {
            if value < 0 {
                return Err(AvError::invalid_data(format!(
                    "film grain parameters {label} is negative: {value}"
                )));
            }
        }

        match params_type {
            FrameFilmGrainParamsType::None => {}
            FrameFilmGrainParamsType::Av1 => {
                FrameFilmGrainAomParams::parse(
                    &data[Self::CODEC_OFFSET
                        ..Self::CODEC_OFFSET + FrameFilmGrainAomParams::DATA_LEN],
                )?;
            }
            FrameFilmGrainParamsType::H274 => {
                FrameFilmGrainH274Params::parse(
                    &data[Self::CODEC_OFFSET
                        ..Self::CODEC_OFFSET + FrameFilmGrainH274Params::DATA_LEN],
                )?;
            }
        }

        Ok(params)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn params_type(self) -> FrameFilmGrainParamsType {
        self.params_type
    }

    pub fn seed(self) -> u64 {
        let mut raw = [0; 8];
        raw.copy_from_slice(&self.data[Self::SEED_OFFSET..Self::SEED_OFFSET + 8]);
        u64::from_ne_bytes(raw)
    }

    pub fn width(self) -> i32 {
        Self::read_i32(self.data, Self::WIDTH_OFFSET)
    }

    pub fn height(self) -> i32 {
        Self::read_i32(self.data, Self::HEIGHT_OFFSET)
    }

    pub fn subsampling_x(self) -> i32 {
        Self::read_i32(self.data, Self::SUBSAMPLING_X_OFFSET)
    }

    pub fn subsampling_y(self) -> i32 {
        Self::read_i32(self.data, Self::SUBSAMPLING_Y_OFFSET)
    }

    pub fn color_range(self) -> i32 {
        Self::read_i32(self.data, Self::COLOR_RANGE_OFFSET)
    }

    pub fn color_primaries(self) -> i32 {
        Self::read_i32(self.data, Self::COLOR_PRIMARIES_OFFSET)
    }

    pub fn color_transfer(self) -> i32 {
        Self::read_i32(self.data, Self::COLOR_TRC_OFFSET)
    }

    pub fn color_space(self) -> i32 {
        Self::read_i32(self.data, Self::COLOR_SPACE_OFFSET)
    }

    pub fn bit_depth_luma(self) -> i32 {
        Self::read_i32(self.data, Self::BIT_DEPTH_LUMA_OFFSET)
    }

    pub fn bit_depth_chroma(self) -> i32 {
        Self::read_i32(self.data, Self::BIT_DEPTH_CHROMA_OFFSET)
    }

    pub fn aom_params(self) -> AvResult<Option<FrameFilmGrainAomParams<'a>>> {
        if self.params_type != FrameFilmGrainParamsType::Av1 {
            return Ok(None);
        }
        FrameFilmGrainAomParams::parse(
            &self.data[Self::CODEC_OFFSET..Self::CODEC_OFFSET + FrameFilmGrainAomParams::DATA_LEN],
        )
        .map(Some)
    }

    pub fn h274_params(self) -> AvResult<Option<FrameFilmGrainH274Params<'a>>> {
        if self.params_type != FrameFilmGrainParamsType::H274 {
            return Ok(None);
        }
        FrameFilmGrainH274Params::parse(
            &self.data[Self::CODEC_OFFSET..Self::CODEC_OFFSET + FrameFilmGrainH274Params::DATA_LEN],
        )
        .map(Some)
    }

    const fn align_up(value: usize, align: usize) -> usize {
        let remainder = value % align;
        if remainder == 0 {
            value
        } else {
            value + align - remainder
        }
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDetectionBbox<'a> {
    data: &'a [u8],
}

impl<'a> FrameDetectionBbox<'a> {
    pub const LABEL_LEN: usize = 64;
    pub const MAX_CLASSIFICATIONS: usize = 4;
    pub const DATA_LEN: usize = 380;
    const X_OFFSET: usize = 0;
    const Y_OFFSET: usize = 4;
    const WIDTH_OFFSET: usize = 8;
    const HEIGHT_OFFSET: usize = 12;
    const DETECT_LABEL_OFFSET: usize = 16;
    const DETECT_CONFIDENCE_OFFSET: usize = Self::DETECT_LABEL_OFFSET + Self::LABEL_LEN;
    const CLASSIFY_COUNT_OFFSET: usize = Self::DETECT_CONFIDENCE_OFFSET + 8;
    const CLASSIFY_LABELS_OFFSET: usize = Self::CLASSIFY_COUNT_OFFSET + 4;
    const CLASSIFY_CONFIDENCES_OFFSET: usize =
        Self::CLASSIFY_LABELS_OFFSET + Self::MAX_CLASSIFICATIONS * Self::LABEL_LEN;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "detection bounding box record requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let bbox = Self { data };
        if bbox.classify_count() > Self::MAX_CLASSIFICATIONS {
            return Err(AvError::invalid_data(format!(
                "detection bounding box classification count {} exceeds {}",
                bbox.classify_count(),
                Self::MAX_CLASSIFICATIONS
            )));
        }

        Ok(bbox)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn x(self) -> i32 {
        self.read_i32(Self::X_OFFSET)
    }

    pub fn y(self) -> i32 {
        self.read_i32(Self::Y_OFFSET)
    }

    pub fn width(self) -> i32 {
        self.read_i32(Self::WIDTH_OFFSET)
    }

    pub fn height(self) -> i32 {
        self.read_i32(Self::HEIGHT_OFFSET)
    }

    pub fn detect_label_raw(self) -> &'a [u8] {
        &self.data[Self::DETECT_LABEL_OFFSET..Self::DETECT_LABEL_OFFSET + Self::LABEL_LEN]
    }

    pub fn detect_label(self) -> &'a [u8] {
        fixed_c_string_bytes(self.detect_label_raw())
    }

    pub fn detect_confidence(self) -> Rational {
        self.read_rational(Self::DETECT_CONFIDENCE_OFFSET)
    }

    pub fn classify_count(self) -> usize {
        self.read_u32(Self::CLASSIFY_COUNT_OFFSET) as usize
    }

    pub fn classify_label_raw(self, index: usize) -> Option<&'a [u8]> {
        if index >= Self::MAX_CLASSIFICATIONS {
            return None;
        }
        let offset = Self::CLASSIFY_LABELS_OFFSET + index * Self::LABEL_LEN;
        Some(&self.data[offset..offset + Self::LABEL_LEN])
    }

    pub fn classify_label(self, index: usize) -> Option<&'a [u8]> {
        if index >= self.classify_count() {
            return None;
        }
        self.classify_label_raw(index).map(fixed_c_string_bytes)
    }

    pub fn classify_confidence(self, index: usize) -> Option<Rational> {
        if index >= self.classify_count() {
            return None;
        }
        Some(self.read_rational(Self::CLASSIFY_CONFIDENCES_OFFSET + index * 8))
    }

    fn read_i32(self, offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&self.data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_u32(self, offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&self.data[offset..offset + 4]);
        u32::from_ne_bytes(raw)
    }

    fn read_rational(self, offset: usize) -> Rational {
        Rational::from_raw(self.read_i32(offset), self.read_i32(offset + 4))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDetectionBboxes<'a> {
    data: &'a [u8],
    nb_bboxes: usize,
}

impl<'a> FrameDetectionBboxes<'a> {
    pub const SOURCE_LEN: usize = 256;
    pub const SIZE_T_LEN: usize = core::mem::size_of::<usize>();
    const NB_BBOXES_OFFSET: usize = Self::SOURCE_LEN;
    const BBOXES_OFFSET_OFFSET: usize =
        Self::align_up(Self::NB_BBOXES_OFFSET + 4, core::mem::align_of::<usize>());
    const BBOX_SIZE_OFFSET: usize = Self::BBOXES_OFFSET_OFFSET + Self::SIZE_T_LEN;
    pub const BBOXES_OFFSET: usize = Self::align_up(
        Self::BBOX_SIZE_OFFSET + Self::SIZE_T_LEN,
        core::mem::align_of::<usize>(),
    );
    pub const HEADER_LEN: usize = Self::BBOXES_OFFSET;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "detection bounding boxes side data requires at least {} header bytes, got {}",
                Self::HEADER_LEN,
                data.len()
            )));
        }

        let nb_bboxes = Self::read_u32(data, Self::NB_BBOXES_OFFSET) as usize;
        let bboxes_offset = Self::read_usize(data, Self::BBOXES_OFFSET_OFFSET);
        let bbox_size = Self::read_usize(data, Self::BBOX_SIZE_OFFSET);
        if bboxes_offset != Self::BBOXES_OFFSET {
            return Err(AvError::invalid_data(format!(
                "detection bounding boxes offset {bboxes_offset} does not match native offset {}",
                Self::BBOXES_OFFSET
            )));
        }
        if bbox_size != FrameDetectionBbox::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "detection bounding box size {bbox_size} does not match native size {}",
                FrameDetectionBbox::DATA_LEN
            )));
        }

        let bbox_bytes = nb_bboxes
            .checked_mul(bbox_size)
            .ok_or_else(|| AvError::invalid_data("detection bounding box array length overflow"))?;
        let expected_len = bboxes_offset.checked_add(bbox_bytes).ok_or_else(|| {
            AvError::invalid_data("detection bounding boxes payload length overflow")
        })?;
        if data.len() != expected_len {
            return Err(AvError::invalid_data(format!(
                "detection bounding boxes side data expected {expected_len} bytes for {nb_bboxes} boxes, got {}",
                data.len()
            )));
        }

        let bboxes = Self { data, nb_bboxes };
        for index in 0..nb_bboxes {
            bboxes.bbox(index).expect("validated bbox index")?;
        }

        Ok(bboxes)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn source_raw(self) -> &'a [u8] {
        &self.data[..Self::SOURCE_LEN]
    }

    pub fn source(self) -> &'a [u8] {
        fixed_c_string_bytes(self.source_raw())
    }

    pub const fn nb_bboxes(self) -> usize {
        self.nb_bboxes
    }

    pub const fn is_empty(self) -> bool {
        self.nb_bboxes == 0
    }

    pub fn bbox(self, index: usize) -> Option<AvResult<FrameDetectionBbox<'a>>> {
        if index >= self.nb_bboxes {
            return None;
        }
        let offset = Self::BBOXES_OFFSET + index * FrameDetectionBbox::DATA_LEN;
        Some(FrameDetectionBbox::parse(
            &self.data[offset..offset + FrameDetectionBbox::DATA_LEN],
        ))
    }

    pub fn bboxes(self) -> impl Iterator<Item = AvResult<FrameDetectionBbox<'a>>> {
        (0..self.nb_bboxes).map(move |index| {
            self.bbox(index)
                .expect("iterator only visits validated bbox indexes")
        })
    }

    const fn align_up(value: usize, align: usize) -> usize {
        let remainder = value % align;
        if remainder == 0 {
            value
        } else {
            value + align - remainder
        }
    }

    fn read_u32(data: &[u8], offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        u32::from_ne_bytes(raw)
    }

    fn read_usize(data: &[u8], offset: usize) -> usize {
        let mut raw = [0; Self::SIZE_T_LEN];
        raw.copy_from_slice(&data[offset..offset + Self::SIZE_T_LEN]);
        usize::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDolbyVisionRpuBuffer<'a> {
    data: &'a [u8],
}

impl<'a> FrameDolbyVisionRpuBuffer<'a> {
    pub const fn parse(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn is_empty(self) -> bool {
        self.data.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDolbyVisionRpuDataHeader<'a> {
    data: &'a [u8],
}

impl<'a> FrameDolbyVisionRpuDataHeader<'a> {
    pub const DATA_LEN: usize = 20;
    const RPU_TYPE_OFFSET: usize = 0;
    const RPU_FORMAT_OFFSET: usize = 2;
    const VDR_RPU_PROFILE_OFFSET: usize = 4;
    const VDR_RPU_LEVEL_OFFSET: usize = 5;
    const CHROMA_RESAMPLING_EXPLICIT_FILTER_FLAG_OFFSET: usize = 6;
    const COEF_DATA_TYPE_OFFSET: usize = 7;
    const COEF_LOG2_DENOM_OFFSET: usize = 8;
    const VDR_RPU_NORMALIZED_IDC_OFFSET: usize = 9;
    const BL_VIDEO_FULL_RANGE_FLAG_OFFSET: usize = 10;
    const BL_BIT_DEPTH_OFFSET: usize = 11;
    const EL_BIT_DEPTH_OFFSET: usize = 12;
    const VDR_BIT_DEPTH_OFFSET: usize = 13;
    const SPATIAL_RESAMPLING_FILTER_FLAG_OFFSET: usize = 14;
    const EL_SPATIAL_RESAMPLING_FILTER_FLAG_OFFSET: usize = 15;
    const DISABLE_RESIDUAL_FLAG_OFFSET: usize = 16;
    const EXT_MAPPING_IDC_0_4_OFFSET: usize = 17;
    const EXT_MAPPING_IDC_5_7_OFFSET: usize = 18;

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision RPU data header requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self { data })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn rpu_type(self) -> u8 {
        self.data[Self::RPU_TYPE_OFFSET]
    }

    pub fn rpu_format(self) -> u16 {
        self.read_u16(Self::RPU_FORMAT_OFFSET)
    }

    pub fn vdr_rpu_profile(self) -> u8 {
        self.data[Self::VDR_RPU_PROFILE_OFFSET]
    }

    pub fn vdr_rpu_level(self) -> u8 {
        self.data[Self::VDR_RPU_LEVEL_OFFSET]
    }

    pub fn chroma_resampling_explicit_filter_flag(self) -> bool {
        self.data[Self::CHROMA_RESAMPLING_EXPLICIT_FILTER_FLAG_OFFSET] != 0
    }

    pub fn coef_data_type(self) -> u8 {
        self.data[Self::COEF_DATA_TYPE_OFFSET]
    }

    pub fn coef_log2_denom(self) -> u8 {
        self.data[Self::COEF_LOG2_DENOM_OFFSET]
    }

    pub fn vdr_rpu_normalized_idc(self) -> u8 {
        self.data[Self::VDR_RPU_NORMALIZED_IDC_OFFSET]
    }

    pub fn bl_video_full_range_flag(self) -> bool {
        self.data[Self::BL_VIDEO_FULL_RANGE_FLAG_OFFSET] != 0
    }

    pub fn bl_bit_depth(self) -> u8 {
        self.data[Self::BL_BIT_DEPTH_OFFSET]
    }

    pub fn el_bit_depth(self) -> u8 {
        self.data[Self::EL_BIT_DEPTH_OFFSET]
    }

    pub fn vdr_bit_depth(self) -> u8 {
        self.data[Self::VDR_BIT_DEPTH_OFFSET]
    }

    pub fn spatial_resampling_filter_flag(self) -> bool {
        self.data[Self::SPATIAL_RESAMPLING_FILTER_FLAG_OFFSET] != 0
    }

    pub fn el_spatial_resampling_filter_flag(self) -> bool {
        self.data[Self::EL_SPATIAL_RESAMPLING_FILTER_FLAG_OFFSET] != 0
    }

    pub fn disable_residual_flag(self) -> bool {
        self.data[Self::DISABLE_RESIDUAL_FLAG_OFFSET] != 0
    }

    pub fn ext_mapping_idc_0_4(self) -> u8 {
        self.data[Self::EXT_MAPPING_IDC_0_4_OFFSET]
    }

    pub fn ext_mapping_idc_5_7(self) -> u8 {
        self.data[Self::EXT_MAPPING_IDC_5_7_OFFSET]
    }

    fn read_u16(self, offset: usize) -> u16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&self.data[offset..offset + 2]);
        u16::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDolbyVisionDataMapping<'a> {
    data: &'a [u8],
}

impl<'a> FrameDolbyVisionDataMapping<'a> {
    const RESHAPING_CURVE_LEN: usize = 1_672;
    const NLQ_PARAMS_LEN: usize = 32;
    const VDR_RPU_ID_OFFSET: usize = 0;
    const MAPPING_COLOR_SPACE_OFFSET: usize = 1;
    const MAPPING_CHROMA_FORMAT_IDC_OFFSET: usize = 2;
    const CURVES_OFFSET: usize = Self::align_up(3, core::mem::align_of::<u64>());
    const NLQ_METHOD_IDC_OFFSET: usize = Self::CURVES_OFFSET + 3 * Self::RESHAPING_CURVE_LEN;
    const NUM_X_PARTITIONS_OFFSET: usize = Self::NLQ_METHOD_IDC_OFFSET + 4;
    const NUM_Y_PARTITIONS_OFFSET: usize = Self::NUM_X_PARTITIONS_OFFSET + 4;
    const NLQ_OFFSET: usize = Self::align_up(
        Self::NUM_Y_PARTITIONS_OFFSET + 4,
        core::mem::align_of::<u64>(),
    );
    const NLQ_PIVOTS_OFFSET: usize = Self::NLQ_OFFSET + 3 * Self::NLQ_PARAMS_LEN;
    pub const DATA_LEN: usize =
        Self::align_up(Self::NLQ_PIVOTS_OFFSET + 4, core::mem::align_of::<u64>());

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision data mapping requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Ok(Self { data })
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn vdr_rpu_id(self) -> u8 {
        self.data[Self::VDR_RPU_ID_OFFSET]
    }

    pub fn mapping_color_space(self) -> u8 {
        self.data[Self::MAPPING_COLOR_SPACE_OFFSET]
    }

    pub fn mapping_chroma_format_idc(self) -> u8 {
        self.data[Self::MAPPING_CHROMA_FORMAT_IDC_OFFSET]
    }

    pub fn nlq_method_idc(self) -> i32 {
        self.read_i32(Self::NLQ_METHOD_IDC_OFFSET)
    }

    pub fn num_x_partitions(self) -> u32 {
        self.read_u32(Self::NUM_X_PARTITIONS_OFFSET)
    }

    pub fn num_y_partitions(self) -> u32 {
        self.read_u32(Self::NUM_Y_PARTITIONS_OFFSET)
    }

    const fn align_up(value: usize, align: usize) -> usize {
        let remainder = value % align;
        if remainder == 0 {
            value
        } else {
            value + align - remainder
        }
    }

    fn read_i32(self, offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&self.data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_u32(self, offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&self.data[offset..offset + 4]);
        u32::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDolbyVisionColorMetadata<'a> {
    data: &'a [u8],
}

impl<'a> FrameDolbyVisionColorMetadata<'a> {
    const RATIONAL_LEN: usize = 8;
    const DM_METADATA_ID_OFFSET: usize = 0;
    const SCENE_REFRESH_FLAG_OFFSET: usize = 1;
    const YCC_TO_RGB_MATRIX_OFFSET: usize = 4;
    const YCC_TO_RGB_OFFSET_OFFSET: usize = Self::YCC_TO_RGB_MATRIX_OFFSET + 9 * Self::RATIONAL_LEN;
    const RGB_TO_LMS_MATRIX_OFFSET: usize = Self::YCC_TO_RGB_OFFSET_OFFSET + 3 * Self::RATIONAL_LEN;
    const SIGNAL_EOTF_OFFSET: usize = Self::RGB_TO_LMS_MATRIX_OFFSET + 9 * Self::RATIONAL_LEN;
    const SIGNAL_EOTF_PARAM0_OFFSET: usize = Self::SIGNAL_EOTF_OFFSET + 2;
    const SIGNAL_EOTF_PARAM1_OFFSET: usize = Self::SIGNAL_EOTF_PARAM0_OFFSET + 2;
    const SIGNAL_EOTF_PARAM2_OFFSET: usize = Self::align_up(
        Self::SIGNAL_EOTF_PARAM1_OFFSET + 2,
        core::mem::align_of::<u32>(),
    );
    const SIGNAL_BIT_DEPTH_OFFSET: usize = Self::SIGNAL_EOTF_PARAM2_OFFSET + 4;
    const SIGNAL_COLOR_SPACE_OFFSET: usize = Self::SIGNAL_BIT_DEPTH_OFFSET + 1;
    const SIGNAL_CHROMA_FORMAT_OFFSET: usize = Self::SIGNAL_COLOR_SPACE_OFFSET + 1;
    const SIGNAL_FULL_RANGE_FLAG_OFFSET: usize = Self::SIGNAL_CHROMA_FORMAT_OFFSET + 1;
    const SOURCE_MIN_PQ_OFFSET: usize = Self::SIGNAL_FULL_RANGE_FLAG_OFFSET + 1;
    const SOURCE_MAX_PQ_OFFSET: usize = Self::SOURCE_MIN_PQ_OFFSET + 2;
    const SOURCE_DIAGONAL_OFFSET: usize = Self::SOURCE_MAX_PQ_OFFSET + 2;
    pub const DATA_LEN: usize = Self::align_up(
        Self::SOURCE_DIAGONAL_OFFSET + 2,
        core::mem::align_of::<i32>(),
    );

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision color metadata requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let metadata = Self { data };
        if metadata.signal_full_range_flag() > 3 {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision signal full range flag {} is outside 0..=3",
                metadata.signal_full_range_flag()
            )));
        }

        Ok(metadata)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn dm_metadata_id(self) -> u8 {
        self.data[Self::DM_METADATA_ID_OFFSET]
    }

    pub fn scene_refresh_flag(self) -> u8 {
        self.data[Self::SCENE_REFRESH_FLAG_OFFSET]
    }

    pub fn ycc_to_rgb_matrix(self, index: usize) -> Option<Rational> {
        if index >= 9 {
            return None;
        }
        Some(self.read_rational(Self::YCC_TO_RGB_MATRIX_OFFSET + index * Self::RATIONAL_LEN))
    }

    pub fn ycc_to_rgb_offset(self, index: usize) -> Option<Rational> {
        if index >= 3 {
            return None;
        }
        Some(self.read_rational(Self::YCC_TO_RGB_OFFSET_OFFSET + index * Self::RATIONAL_LEN))
    }

    pub fn rgb_to_lms_matrix(self, index: usize) -> Option<Rational> {
        if index >= 9 {
            return None;
        }
        Some(self.read_rational(Self::RGB_TO_LMS_MATRIX_OFFSET + index * Self::RATIONAL_LEN))
    }

    pub fn signal_eotf(self) -> u16 {
        self.read_u16(Self::SIGNAL_EOTF_OFFSET)
    }

    pub fn signal_eotf_param0(self) -> u16 {
        self.read_u16(Self::SIGNAL_EOTF_PARAM0_OFFSET)
    }

    pub fn signal_eotf_param1(self) -> u16 {
        self.read_u16(Self::SIGNAL_EOTF_PARAM1_OFFSET)
    }

    pub fn signal_eotf_param2(self) -> u32 {
        self.read_u32(Self::SIGNAL_EOTF_PARAM2_OFFSET)
    }

    pub fn signal_bit_depth(self) -> u8 {
        self.data[Self::SIGNAL_BIT_DEPTH_OFFSET]
    }

    pub fn signal_color_space(self) -> u8 {
        self.data[Self::SIGNAL_COLOR_SPACE_OFFSET]
    }

    pub fn signal_chroma_format(self) -> u8 {
        self.data[Self::SIGNAL_CHROMA_FORMAT_OFFSET]
    }

    pub fn signal_full_range_flag(self) -> u8 {
        self.data[Self::SIGNAL_FULL_RANGE_FLAG_OFFSET]
    }

    pub fn source_min_pq(self) -> u16 {
        self.read_u16(Self::SOURCE_MIN_PQ_OFFSET)
    }

    pub fn source_max_pq(self) -> u16 {
        self.read_u16(Self::SOURCE_MAX_PQ_OFFSET)
    }

    pub fn source_diagonal(self) -> u16 {
        self.read_u16(Self::SOURCE_DIAGONAL_OFFSET)
    }

    const fn align_up(value: usize, align: usize) -> usize {
        let remainder = value % align;
        if remainder == 0 {
            value
        } else {
            value + align - remainder
        }
    }

    fn read_rational(self, offset: usize) -> Rational {
        Rational::from_raw(self.read_i32(offset), self.read_i32(offset + 4))
    }

    fn read_i32(self, offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&self.data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_u16(self, offset: usize) -> u16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&self.data[offset..offset + 2]);
        u16::from_ne_bytes(raw)
    }

    fn read_u32(self, offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&self.data[offset..offset + 4]);
        u32::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDolbyVisionDmData<'a> {
    data: &'a [u8],
}

impl<'a> FrameDolbyVisionDmData<'a> {
    pub const DATA_LEN: usize = 76;
    const LEVEL_OFFSET: usize = 0;
    const UNION_OFFSET: usize = 4;
    const LEVEL1_MIN_PQ_OFFSET: usize = Self::UNION_OFFSET;
    const LEVEL1_MAX_PQ_OFFSET: usize = Self::LEVEL1_MIN_PQ_OFFSET + 2;
    const LEVEL1_AVG_PQ_OFFSET: usize = Self::LEVEL1_MAX_PQ_OFFSET + 2;
    const LEVEL6_MAX_LUMINANCE_OFFSET: usize = Self::UNION_OFFSET;
    const LEVEL6_MIN_LUMINANCE_OFFSET: usize = Self::LEVEL6_MAX_LUMINANCE_OFFSET + 2;
    const LEVEL6_MAX_CLL_OFFSET: usize = Self::LEVEL6_MIN_LUMINANCE_OFFSET + 2;
    const LEVEL6_MAX_FALL_OFFSET: usize = Self::LEVEL6_MAX_CLL_OFFSET + 2;

    fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision display-management block requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let block = Self { data };
        if block.level() == 0 {
            return Err(AvError::invalid_data(
                "Dolby Vision display-management block level must be nonzero",
            ));
        }

        Ok(block)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub fn level(self) -> u8 {
        self.data[Self::LEVEL_OFFSET]
    }

    pub fn level1_min_pq(self) -> Option<u16> {
        (self.level() == 1).then(|| self.read_u16(Self::LEVEL1_MIN_PQ_OFFSET))
    }

    pub fn level1_max_pq(self) -> Option<u16> {
        (self.level() == 1).then(|| self.read_u16(Self::LEVEL1_MAX_PQ_OFFSET))
    }

    pub fn level1_avg_pq(self) -> Option<u16> {
        (self.level() == 1).then(|| self.read_u16(Self::LEVEL1_AVG_PQ_OFFSET))
    }

    pub fn level6_max_luminance(self) -> Option<u16> {
        (self.level() == 6).then(|| self.read_u16(Self::LEVEL6_MAX_LUMINANCE_OFFSET))
    }

    pub fn level6_min_luminance(self) -> Option<u16> {
        (self.level() == 6).then(|| self.read_u16(Self::LEVEL6_MIN_LUMINANCE_OFFSET))
    }

    pub fn level6_max_content_light_level(self) -> Option<u16> {
        (self.level() == 6).then(|| self.read_u16(Self::LEVEL6_MAX_CLL_OFFSET))
    }

    pub fn level6_max_frame_average_light_level(self) -> Option<u16> {
        (self.level() == 6).then(|| self.read_u16(Self::LEVEL6_MAX_FALL_OFFSET))
    }

    fn read_u16(self, offset: usize) -> u16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&self.data[offset..offset + 2]);
        u16::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDolbyVisionMetadata<'a> {
    data: &'a [u8],
    num_ext_blocks: usize,
}

impl<'a> FrameDolbyVisionMetadata<'a> {
    pub const MAX_EXT_BLOCKS: usize = 32;
    pub const SIZE_T_LEN: usize = core::mem::size_of::<usize>();
    const HEADER_OFFSET_OFFSET: usize = 0;
    const MAPPING_OFFSET_OFFSET: usize = Self::HEADER_OFFSET_OFFSET + Self::SIZE_T_LEN;
    const COLOR_OFFSET_OFFSET: usize = Self::MAPPING_OFFSET_OFFSET + Self::SIZE_T_LEN;
    const EXT_BLOCK_OFFSET_OFFSET: usize = Self::COLOR_OFFSET_OFFSET + Self::SIZE_T_LEN;
    const EXT_BLOCK_SIZE_OFFSET: usize = Self::EXT_BLOCK_OFFSET_OFFSET + Self::SIZE_T_LEN;
    const NUM_EXT_BLOCKS_OFFSET: usize = Self::EXT_BLOCK_SIZE_OFFSET + Self::SIZE_T_LEN;
    const METADATA_LEN: usize = Self::align_up(
        Self::NUM_EXT_BLOCKS_OFFSET + 4,
        core::mem::align_of::<usize>(),
    );
    pub const HEADER_OFFSET: usize =
        Self::align_up(Self::METADATA_LEN, core::mem::align_of::<u16>());
    pub const MAPPING_OFFSET: usize = Self::align_up(
        Self::HEADER_OFFSET + FrameDolbyVisionRpuDataHeader::DATA_LEN,
        core::mem::align_of::<u64>(),
    );
    pub const COLOR_OFFSET: usize = Self::align_up(
        Self::MAPPING_OFFSET + FrameDolbyVisionDataMapping::DATA_LEN,
        core::mem::align_of::<i32>(),
    );
    pub const EXT_BLOCK_OFFSET: usize = Self::align_up(
        Self::COLOR_OFFSET + FrameDolbyVisionColorMetadata::DATA_LEN,
        core::mem::align_of::<i32>(),
    );
    pub const EXT_BLOCK_SIZE: usize = FrameDolbyVisionDmData::DATA_LEN;
    pub const DATA_LEN: usize = Self::align_up(
        Self::EXT_BLOCK_OFFSET + Self::MAX_EXT_BLOCKS * Self::EXT_BLOCK_SIZE,
        core::mem::align_of::<u64>(),
    );

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision metadata side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let header_offset = Self::read_usize(data, Self::HEADER_OFFSET_OFFSET);
        let mapping_offset = Self::read_usize(data, Self::MAPPING_OFFSET_OFFSET);
        let color_offset = Self::read_usize(data, Self::COLOR_OFFSET_OFFSET);
        let ext_block_offset = Self::read_usize(data, Self::EXT_BLOCK_OFFSET_OFFSET);
        let ext_block_size = Self::read_usize(data, Self::EXT_BLOCK_SIZE_OFFSET);

        for (label, actual, expected) in [
            ("header", header_offset, Self::HEADER_OFFSET),
            ("mapping", mapping_offset, Self::MAPPING_OFFSET),
            ("color", color_offset, Self::COLOR_OFFSET),
            ("extension block", ext_block_offset, Self::EXT_BLOCK_OFFSET),
        ] {
            if actual != expected {
                return Err(AvError::invalid_data(format!(
                    "Dolby Vision metadata {label} offset {actual} does not match native offset {expected}"
                )));
            }
        }
        if ext_block_size != Self::EXT_BLOCK_SIZE {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision metadata extension block size {ext_block_size} does not match native size {}",
                Self::EXT_BLOCK_SIZE
            )));
        }

        let num_ext_blocks_raw = Self::read_i32(data, Self::NUM_EXT_BLOCKS_OFFSET);
        if !(0..=Self::MAX_EXT_BLOCKS as i32).contains(&num_ext_blocks_raw) {
            return Err(AvError::invalid_data(format!(
                "Dolby Vision metadata extension block count {num_ext_blocks_raw} is outside 0..={}",
                Self::MAX_EXT_BLOCKS
            )));
        }

        let metadata = Self {
            data,
            num_ext_blocks: num_ext_blocks_raw as usize,
        };
        metadata.header()?;
        metadata.mapping()?;
        metadata.color()?;
        for index in 0..metadata.num_ext_blocks {
            metadata
                .ext_block(index)
                .expect("validated extension block index")?;
        }

        Ok(metadata)
    }

    pub const fn data(self) -> &'a [u8] {
        self.data
    }

    pub const fn num_ext_blocks(self) -> usize {
        self.num_ext_blocks
    }

    pub const fn is_empty(self) -> bool {
        self.num_ext_blocks == 0
    }

    pub fn header(self) -> AvResult<FrameDolbyVisionRpuDataHeader<'a>> {
        FrameDolbyVisionRpuDataHeader::parse(
            &self.data[Self::HEADER_OFFSET
                ..Self::HEADER_OFFSET + FrameDolbyVisionRpuDataHeader::DATA_LEN],
        )
    }

    pub fn mapping(self) -> AvResult<FrameDolbyVisionDataMapping<'a>> {
        FrameDolbyVisionDataMapping::parse(
            &self.data[Self::MAPPING_OFFSET
                ..Self::MAPPING_OFFSET + FrameDolbyVisionDataMapping::DATA_LEN],
        )
    }

    pub fn color(self) -> AvResult<FrameDolbyVisionColorMetadata<'a>> {
        FrameDolbyVisionColorMetadata::parse(
            &self.data
                [Self::COLOR_OFFSET..Self::COLOR_OFFSET + FrameDolbyVisionColorMetadata::DATA_LEN],
        )
    }

    pub fn ext_block(self, index: usize) -> Option<AvResult<FrameDolbyVisionDmData<'a>>> {
        if index >= self.num_ext_blocks {
            return None;
        }
        let offset = Self::EXT_BLOCK_OFFSET + index * Self::EXT_BLOCK_SIZE;
        Some(FrameDolbyVisionDmData::parse(
            &self.data[offset..offset + Self::EXT_BLOCK_SIZE],
        ))
    }

    pub fn ext_blocks(self) -> impl Iterator<Item = AvResult<FrameDolbyVisionDmData<'a>>> {
        (0..self.num_ext_blocks).map(move |index| {
            self.ext_block(index)
                .expect("iterator only visits validated extension block indexes")
        })
    }

    pub fn find_level(self, level: u8) -> Option<AvResult<FrameDolbyVisionDmData<'a>>> {
        if level == 0 {
            return None;
        }
        self.ext_blocks().find(|block| match block {
            Ok(block) => block.level() == level,
            Err(_) => true,
        })
    }

    const fn align_up(value: usize, align: usize) -> usize {
        let remainder = value % align;
        if remainder == 0 {
            value
        } else {
            value + align - remainder
        }
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_usize(data: &[u8], offset: usize) -> usize {
        let mut raw = [0; Self::SIZE_T_LEN];
        raw.copy_from_slice(&data[offset..offset + Self::SIZE_T_LEN]);
        usize::from_ne_bytes(raw)
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

    pub fn new_pan_scan(value: FramePanScan) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::PanScan, value.to_bytes().to_vec())
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

    pub fn new_regions_of_interest(value: FrameRegionsOfInterest) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::RegionsOfInterest, value.to_bytes())
    }

    pub fn new_video_enc_params(value: FrameVideoEncParams) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::VideoEncParams, value.to_bytes())
    }

    pub fn new_film_grain_params(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(FrameSideDataKind::FilmGrainParams, data)?;
        FrameFilmGrainParams::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_detection_bboxes(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(FrameSideDataKind::DetectionBboxes, data)?;
        FrameDetectionBboxes::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_dolby_vision_rpu_buffer(data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::DolbyVisionRpuBuffer, data)
    }

    pub fn new_dolby_vision_metadata(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(FrameSideDataKind::DolbyVisionMetadata, data)?;
        FrameDolbyVisionMetadata::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_dynamic_hdr_vivid(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(FrameSideDataKind::DynamicHdrVivid, data)?;
        FrameDynamicHdrVivid::parse(side_data.data())?;
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

    pub fn pan_scan(&self) -> AvResult<Option<FramePanScan>> {
        if self.kind != FrameSideDataKind::PanScan {
            return Ok(None);
        }

        FramePanScan::parse(self.data()).map(Some)
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

    pub fn regions_of_interest(&self) -> AvResult<Option<FrameRegionsOfInterest>> {
        if self.kind != FrameSideDataKind::RegionsOfInterest {
            return Ok(None);
        }

        FrameRegionsOfInterest::parse(self.data()).map(Some)
    }

    pub fn video_enc_params(&self) -> AvResult<Option<FrameVideoEncParams>> {
        if self.kind != FrameSideDataKind::VideoEncParams {
            return Ok(None);
        }

        FrameVideoEncParams::parse(self.data()).map(Some)
    }

    pub fn film_grain_params(&self) -> AvResult<Option<FrameFilmGrainParams<'_>>> {
        if self.kind != FrameSideDataKind::FilmGrainParams {
            return Ok(None);
        }

        FrameFilmGrainParams::parse(self.data()).map(Some)
    }

    pub fn detection_bboxes(&self) -> AvResult<Option<FrameDetectionBboxes<'_>>> {
        if self.kind != FrameSideDataKind::DetectionBboxes {
            return Ok(None);
        }

        FrameDetectionBboxes::parse(self.data()).map(Some)
    }

    pub fn dolby_vision_rpu_buffer(&self) -> Option<FrameDolbyVisionRpuBuffer<'_>> {
        if self.kind != FrameSideDataKind::DolbyVisionRpuBuffer {
            return None;
        }

        Some(FrameDolbyVisionRpuBuffer::parse(self.data()))
    }

    pub fn dolby_vision_metadata(&self) -> AvResult<Option<FrameDolbyVisionMetadata<'_>>> {
        if self.kind != FrameSideDataKind::DolbyVisionMetadata {
            return Ok(None);
        }

        FrameDolbyVisionMetadata::parse(self.data()).map(Some)
    }

    pub fn dynamic_hdr_vivid(&self) -> AvResult<Option<FrameDynamicHdrVivid<'_>>> {
        if self.kind != FrameSideDataKind::DynamicHdrVivid {
            return Ok(None);
        }

        FrameDynamicHdrVivid::parse(self.data()).map(Some)
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

fn fixed_c_string_bytes(bytes: &[u8]) -> &[u8] {
    bytes
        .iter()
        .position(|byte| *byte == 0)
        .map_or(bytes, |end| &bytes[..end])
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

    fn minimal_dynamic_hdr_vivid() -> Vec<u8> {
        let mut data = vec![0; FrameDynamicHdrVivid::DATA_LEN];
        data[0] = FrameDynamicHdrVivid::MIN_SYSTEM_START_CODE;
        data[1] = 1;

        let params = FrameDynamicHdrVivid::PARAMS_OFFSET;
        write_ne_rational(
            &mut data,
            params + FrameHdrVividColorTransformParams::MINIMUM_MAXRGB_OFFSET,
            Rational::from_raw(1, 4095),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrVividColorTransformParams::AVERAGE_MAXRGB_OFFSET,
            Rational::from_raw(2, 4095),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrVividColorTransformParams::VARIANCE_MAXRGB_OFFSET,
            Rational::from_raw(3, 4095),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrVividColorTransformParams::MAXIMUM_MAXRGB_OFFSET,
            Rational::from_raw(4, 4095),
        );
        write_ne_i32(
            &mut data,
            params + FrameHdrVividColorTransformParams::TONE_MAPPING_MODE_FLAG_OFFSET,
            1,
        );
        write_ne_i32(
            &mut data,
            params + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAM_NUM_OFFSET,
            2,
        );

        let tm0 = params + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAMS_OFFSET;
        write_ne_rational(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET,
            Rational::from_raw(100, 4095),
        );
        write_ne_i32(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_ENABLE_FLAG_OFFSET,
            1,
        );
        write_ne_rational(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_M_P_OFFSET,
            Rational::from_raw(10, 16383),
        );
        write_ne_rational(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_M_M_OFFSET,
            Rational::from_raw(11, 10),
        );
        write_ne_rational(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_M_A_OFFSET,
            Rational::from_raw(12, 1023),
        );
        write_ne_rational(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_M_B_OFFSET,
            Rational::from_raw(13, 1023),
        );
        write_ne_rational(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_M_N_OFFSET,
            Rational::from_raw(14, 10),
        );
        write_ne_i32(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_K1_OFFSET,
            1,
        );
        write_ne_i32(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_K2_OFFSET,
            0,
        );
        write_ne_i32(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_K3_OFFSET,
            2,
        );
        write_ne_i32(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_DELTA_ENABLE_MODE_OFFSET,
            1,
        );
        write_ne_rational(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::BASE_PARAM_DELTA_OFFSET,
            Rational::from_raw(7, 127),
        );
        write_ne_i32(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::THREE_SPLINE_ENABLE_FLAG_OFFSET,
            1,
        );
        write_ne_i32(
            &mut data,
            tm0 + FrameHdrVividColorToneMappingParams::THREE_SPLINE_NUM_OFFSET,
            2,
        );

        let spline0 = tm0 + FrameHdrVividColorToneMappingParams::THREE_SPLINE_OFFSET;
        write_ne_i32(
            &mut data,
            spline0 + FrameHdrVivid3SplineParams::TH_MODE_OFFSET,
            0,
        );
        write_ne_rational(
            &mut data,
            spline0 + FrameHdrVivid3SplineParams::TH_ENABLE_MB_OFFSET,
            Rational::from_raw(9, 255),
        );
        write_ne_rational(
            &mut data,
            spline0 + FrameHdrVivid3SplineParams::TH_ENABLE_OFFSET,
            Rational::from_raw(10, 4095),
        );
        write_ne_rational(
            &mut data,
            spline0 + FrameHdrVivid3SplineParams::TH_DELTA1_OFFSET,
            Rational::from_raw(11, 1023),
        );
        write_ne_rational(
            &mut data,
            spline0 + FrameHdrVivid3SplineParams::TH_DELTA2_OFFSET,
            Rational::from_raw(12, 1023),
        );
        write_ne_rational(
            &mut data,
            spline0 + FrameHdrVivid3SplineParams::ENABLE_STRENGTH_OFFSET,
            Rational::from_raw(13, 255),
        );

        let spline1 = spline0 + FrameHdrVivid3SplineParams::DATA_LEN;
        write_ne_i32(
            &mut data,
            spline1 + FrameHdrVivid3SplineParams::TH_MODE_OFFSET,
            3,
        );
        write_ne_rational(
            &mut data,
            spline1 + FrameHdrVivid3SplineParams::TH_ENABLE_OFFSET,
            Rational::from_raw(20, 4095),
        );
        write_ne_rational(
            &mut data,
            spline1 + FrameHdrVivid3SplineParams::TH_DELTA1_OFFSET,
            Rational::from_raw(21, 1023),
        );
        write_ne_rational(
            &mut data,
            spline1 + FrameHdrVivid3SplineParams::TH_DELTA2_OFFSET,
            Rational::from_raw(22, 1023),
        );
        write_ne_rational(
            &mut data,
            spline1 + FrameHdrVivid3SplineParams::ENABLE_STRENGTH_OFFSET,
            Rational::from_raw(23, 255),
        );

        let tm1 = tm0 + FrameHdrVividColorToneMappingParams::DATA_LEN;
        write_ne_rational(
            &mut data,
            tm1 + FrameHdrVividColorToneMappingParams::TARGETED_SYSTEM_DISPLAY_MAXIMUM_LUMINANCE_OFFSET,
            Rational::from_raw(200, 4095),
        );
        write_ne_i32(
            &mut data,
            tm1 + FrameHdrVividColorToneMappingParams::BASE_ENABLE_FLAG_OFFSET,
            0,
        );
        write_ne_i32(
            &mut data,
            tm1 + FrameHdrVividColorToneMappingParams::THREE_SPLINE_ENABLE_FLAG_OFFSET,
            0,
        );

        write_ne_i32(
            &mut data,
            params + FrameHdrVividColorTransformParams::COLOR_SATURATION_MAPPING_FLAG_OFFSET,
            1,
        );
        write_ne_i32(
            &mut data,
            params + FrameHdrVividColorTransformParams::COLOR_SATURATION_NUM_OFFSET,
            2,
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrVividColorTransformParams::COLOR_SATURATION_GAIN_OFFSET,
            Rational::from_raw(1, 128),
        );
        write_ne_rational(
            &mut data,
            params + FrameHdrVividColorTransformParams::COLOR_SATURATION_GAIN_OFFSET + 8,
            Rational::from_raw(2, 128),
        );

        data
    }

    fn minimal_film_grain_av1() -> Vec<u8> {
        let mut data = vec![0; FrameFilmGrainParams::DATA_LEN];
        write_ne_i32(
            &mut data,
            FrameFilmGrainParams::TYPE_OFFSET,
            FrameFilmGrainParamsType::Av1.as_raw(),
        );
        write_ne_u64(
            &mut data,
            FrameFilmGrainParams::SEED_OFFSET,
            0x0102_0304_0506_0708,
        );
        write_ne_i32(&mut data, FrameFilmGrainParams::WIDTH_OFFSET, 1920);
        write_ne_i32(&mut data, FrameFilmGrainParams::HEIGHT_OFFSET, 1080);
        write_ne_i32(&mut data, FrameFilmGrainParams::SUBSAMPLING_X_OFFSET, 1);
        write_ne_i32(&mut data, FrameFilmGrainParams::SUBSAMPLING_Y_OFFSET, 1);
        write_ne_i32(&mut data, FrameFilmGrainParams::COLOR_RANGE_OFFSET, 1);
        write_ne_i32(&mut data, FrameFilmGrainParams::COLOR_PRIMARIES_OFFSET, 9);
        write_ne_i32(&mut data, FrameFilmGrainParams::COLOR_TRC_OFFSET, 16);
        write_ne_i32(&mut data, FrameFilmGrainParams::COLOR_SPACE_OFFSET, 9);
        write_ne_i32(&mut data, FrameFilmGrainParams::BIT_DEPTH_LUMA_OFFSET, 10);
        write_ne_i32(&mut data, FrameFilmGrainParams::BIT_DEPTH_CHROMA_OFFSET, 10);

        let codec = FrameFilmGrainParams::CODEC_OFFSET;
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::NUM_Y_POINTS_OFFSET,
            2,
        );
        data[codec + FrameFilmGrainAomParams::Y_POINTS_OFFSET] = 16;
        data[codec + FrameFilmGrainAomParams::Y_POINTS_OFFSET + 1] = 3;
        data[codec + FrameFilmGrainAomParams::Y_POINTS_OFFSET + 2] = 128;
        data[codec + FrameFilmGrainAomParams::Y_POINTS_OFFSET + 3] = 17;
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::NUM_UV_POINTS_OFFSET,
            1,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::NUM_UV_POINTS_OFFSET + 4,
            1,
        );
        data[codec + FrameFilmGrainAomParams::UV_POINTS_OFFSET] = 32;
        data[codec + FrameFilmGrainAomParams::UV_POINTS_OFFSET + 1] = 4;
        data[codec + FrameFilmGrainAomParams::UV_POINTS_OFFSET + 20] = 64;
        data[codec + FrameFilmGrainAomParams::UV_POINTS_OFFSET + 21] = 5;
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::SCALING_SHIFT_OFFSET,
            10,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::AR_COEFF_LAG_OFFSET,
            1,
        );
        data[codec + FrameFilmGrainAomParams::AR_COEFFS_Y_OFFSET] = (-3i8) as u8;
        data[codec + FrameFilmGrainAomParams::AR_COEFFS_UV_OFFSET] = 2;
        data[codec
            + FrameFilmGrainAomParams::AR_COEFFS_UV_OFFSET
            + FrameFilmGrainAomParams::AR_COEFFS_UV
            + 4] = (-7i8) as u8;
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::AR_COEFF_SHIFT_OFFSET,
            7,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::GRAIN_SCALE_SHIFT_OFFSET,
            2,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::UV_MULT_OFFSET,
            128,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::UV_MULT_OFFSET + 4,
            64,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::UV_MULT_LUMA_OFFSET,
            32,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::UV_MULT_LUMA_OFFSET + 4,
            48,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::UV_OFFSET_OFFSET,
            -64,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::UV_OFFSET_OFFSET + 4,
            96,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainAomParams::OVERLAP_FLAG_OFFSET,
            1,
        );
        data
    }

    fn minimal_film_grain_h274() -> Vec<u8> {
        let mut data = vec![0; FrameFilmGrainParams::DATA_LEN];
        write_ne_i32(
            &mut data,
            FrameFilmGrainParams::TYPE_OFFSET,
            FrameFilmGrainParamsType::H274.as_raw(),
        );
        write_ne_i32(&mut data, FrameFilmGrainParams::WIDTH_OFFSET, 1280);
        write_ne_i32(&mut data, FrameFilmGrainParams::HEIGHT_OFFSET, 720);
        write_ne_i32(&mut data, FrameFilmGrainParams::BIT_DEPTH_LUMA_OFFSET, 8);
        write_ne_i32(&mut data, FrameFilmGrainParams::BIT_DEPTH_CHROMA_OFFSET, 8);

        let codec = FrameFilmGrainParams::CODEC_OFFSET;
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainH274Params::MODEL_ID_OFFSET,
            1,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainH274Params::BLENDING_MODE_ID_OFFSET,
            0,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainH274Params::LOG2_SCALE_FACTOR_OFFSET,
            3,
        );
        write_ne_i32(
            &mut data,
            codec + FrameFilmGrainH274Params::COMPONENT_MODEL_PRESENT_OFFSET,
            1,
        );
        write_ne_u16(
            &mut data,
            codec + FrameFilmGrainH274Params::NUM_INTENSITY_INTERVALS_OFFSET,
            2,
        );
        data[codec + FrameFilmGrainH274Params::NUM_MODEL_VALUES_OFFSET] = 3;
        data[codec + FrameFilmGrainH274Params::INTENSITY_INTERVAL_LOWER_BOUND_OFFSET] = 0;
        data[codec + FrameFilmGrainH274Params::INTENSITY_INTERVAL_UPPER_BOUND_OFFSET] = 63;
        data[codec + FrameFilmGrainH274Params::INTENSITY_INTERVAL_LOWER_BOUND_OFFSET + 1] = 64;
        data[codec + FrameFilmGrainH274Params::INTENSITY_INTERVAL_UPPER_BOUND_OFFSET + 1] = 127;
        write_ne_i16(
            &mut data,
            codec
                + FrameFilmGrainH274Params::COMP_MODEL_VALUE_OFFSET
                + (FrameFilmGrainH274Params::MAX_MODEL_VALUES + 2) * 2,
            -14,
        );
        data
    }

    fn minimal_detection_bboxes() -> Vec<u8> {
        let mut data = vec![0; FrameDetectionBboxes::HEADER_LEN + 2 * FrameDetectionBbox::DATA_LEN];
        write_fixed_bytes(
            &mut data,
            0,
            FrameDetectionBboxes::SOURCE_LEN,
            b"rust-detector",
        );
        write_ne_u32(&mut data, FrameDetectionBboxes::NB_BBOXES_OFFSET, 2);
        write_ne_usize(
            &mut data,
            FrameDetectionBboxes::BBOXES_OFFSET_OFFSET,
            FrameDetectionBboxes::BBOXES_OFFSET,
        );
        write_ne_usize(
            &mut data,
            FrameDetectionBboxes::BBOX_SIZE_OFFSET,
            FrameDetectionBbox::DATA_LEN,
        );

        let first = FrameDetectionBboxes::BBOXES_OFFSET;
        write_ne_i32(&mut data, first + FrameDetectionBbox::X_OFFSET, 10);
        write_ne_i32(&mut data, first + FrameDetectionBbox::Y_OFFSET, 20);
        write_ne_i32(&mut data, first + FrameDetectionBbox::WIDTH_OFFSET, 30);
        write_ne_i32(&mut data, first + FrameDetectionBbox::HEIGHT_OFFSET, 40);
        write_fixed_bytes(
            &mut data,
            first + FrameDetectionBbox::DETECT_LABEL_OFFSET,
            FrameDetectionBbox::LABEL_LEN,
            b"person",
        );
        write_ne_rational(
            &mut data,
            first + FrameDetectionBbox::DETECT_CONFIDENCE_OFFSET,
            Rational::from_raw(9, 10),
        );
        write_ne_u32(
            &mut data,
            first + FrameDetectionBbox::CLASSIFY_COUNT_OFFSET,
            2,
        );
        write_fixed_bytes(
            &mut data,
            first + FrameDetectionBbox::CLASSIFY_LABELS_OFFSET,
            FrameDetectionBbox::LABEL_LEN,
            b"adult",
        );
        write_fixed_bytes(
            &mut data,
            first + FrameDetectionBbox::CLASSIFY_LABELS_OFFSET + FrameDetectionBbox::LABEL_LEN,
            FrameDetectionBbox::LABEL_LEN,
            b"standing",
        );
        write_ne_rational(
            &mut data,
            first + FrameDetectionBbox::CLASSIFY_CONFIDENCES_OFFSET,
            Rational::from_raw(3, 4),
        );
        write_ne_rational(
            &mut data,
            first + FrameDetectionBbox::CLASSIFY_CONFIDENCES_OFFSET + 8,
            Rational::from_raw(5, 6),
        );

        let second = first + FrameDetectionBbox::DATA_LEN;
        write_ne_i32(&mut data, second + FrameDetectionBbox::X_OFFSET, 64);
        write_ne_i32(&mut data, second + FrameDetectionBbox::Y_OFFSET, 72);
        write_ne_i32(&mut data, second + FrameDetectionBbox::WIDTH_OFFSET, 16);
        write_ne_i32(&mut data, second + FrameDetectionBbox::HEIGHT_OFFSET, 18);
        write_fixed_bytes(
            &mut data,
            second + FrameDetectionBbox::DETECT_LABEL_OFFSET,
            FrameDetectionBbox::LABEL_LEN,
            b"ball",
        );
        write_ne_rational(
            &mut data,
            second + FrameDetectionBbox::DETECT_CONFIDENCE_OFFSET,
            Rational::from_raw(7, 8),
        );

        data
    }

    fn minimal_dolby_vision_metadata() -> Vec<u8> {
        let mut data = vec![0; FrameDolbyVisionMetadata::DATA_LEN];
        write_ne_usize(
            &mut data,
            FrameDolbyVisionMetadata::HEADER_OFFSET_OFFSET,
            FrameDolbyVisionMetadata::HEADER_OFFSET,
        );
        write_ne_usize(
            &mut data,
            FrameDolbyVisionMetadata::MAPPING_OFFSET_OFFSET,
            FrameDolbyVisionMetadata::MAPPING_OFFSET,
        );
        write_ne_usize(
            &mut data,
            FrameDolbyVisionMetadata::COLOR_OFFSET_OFFSET,
            FrameDolbyVisionMetadata::COLOR_OFFSET,
        );
        write_ne_usize(
            &mut data,
            FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET_OFFSET,
            FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET,
        );
        write_ne_usize(
            &mut data,
            FrameDolbyVisionMetadata::EXT_BLOCK_SIZE_OFFSET,
            FrameDolbyVisionMetadata::EXT_BLOCK_SIZE,
        );
        write_ne_i32(
            &mut data,
            FrameDolbyVisionMetadata::NUM_EXT_BLOCKS_OFFSET,
            2,
        );

        let header = FrameDolbyVisionMetadata::HEADER_OFFSET;
        data[header + FrameDolbyVisionRpuDataHeader::RPU_TYPE_OFFSET] = 2;
        write_ne_u16(
            &mut data,
            header + FrameDolbyVisionRpuDataHeader::RPU_FORMAT_OFFSET,
            18,
        );
        data[header + FrameDolbyVisionRpuDataHeader::VDR_RPU_PROFILE_OFFSET] = 8;
        data[header + FrameDolbyVisionRpuDataHeader::VDR_RPU_LEVEL_OFFSET] = 6;
        data[header + FrameDolbyVisionRpuDataHeader::COEF_DATA_TYPE_OFFSET] = 1;
        data[header + FrameDolbyVisionRpuDataHeader::COEF_LOG2_DENOM_OFFSET] = 28;
        data[header + FrameDolbyVisionRpuDataHeader::BL_BIT_DEPTH_OFFSET] = 10;
        data[header + FrameDolbyVisionRpuDataHeader::EL_BIT_DEPTH_OFFSET] = 10;
        data[header + FrameDolbyVisionRpuDataHeader::VDR_BIT_DEPTH_OFFSET] = 12;
        data[header + FrameDolbyVisionRpuDataHeader::DISABLE_RESIDUAL_FLAG_OFFSET] = 1;
        data[header + FrameDolbyVisionRpuDataHeader::EXT_MAPPING_IDC_0_4_OFFSET] = 4;

        let mapping = FrameDolbyVisionMetadata::MAPPING_OFFSET;
        data[mapping + FrameDolbyVisionDataMapping::VDR_RPU_ID_OFFSET] = 3;
        data[mapping + FrameDolbyVisionDataMapping::MAPPING_COLOR_SPACE_OFFSET] = 1;
        data[mapping + FrameDolbyVisionDataMapping::MAPPING_CHROMA_FORMAT_IDC_OFFSET] = 2;
        write_ne_i32(
            &mut data,
            mapping + FrameDolbyVisionDataMapping::NLQ_METHOD_IDC_OFFSET,
            0,
        );
        write_ne_u32(
            &mut data,
            mapping + FrameDolbyVisionDataMapping::NUM_X_PARTITIONS_OFFSET,
            1,
        );
        write_ne_u32(
            &mut data,
            mapping + FrameDolbyVisionDataMapping::NUM_Y_PARTITIONS_OFFSET,
            1,
        );

        let color = FrameDolbyVisionMetadata::COLOR_OFFSET;
        data[color + FrameDolbyVisionColorMetadata::DM_METADATA_ID_OFFSET] = 9;
        data[color + FrameDolbyVisionColorMetadata::SCENE_REFRESH_FLAG_OFFSET] = 1;
        write_ne_rational(
            &mut data,
            color + FrameDolbyVisionColorMetadata::YCC_TO_RGB_MATRIX_OFFSET,
            Rational::from_raw(1, 2),
        );
        write_ne_rational(
            &mut data,
            color + FrameDolbyVisionColorMetadata::YCC_TO_RGB_OFFSET_OFFSET,
            Rational::from_raw(3, 4),
        );
        write_ne_rational(
            &mut data,
            color + FrameDolbyVisionColorMetadata::RGB_TO_LMS_MATRIX_OFFSET,
            Rational::from_raw(5, 6),
        );
        write_ne_u16(
            &mut data,
            color + FrameDolbyVisionColorMetadata::SIGNAL_EOTF_OFFSET,
            2084,
        );
        write_ne_u16(
            &mut data,
            color + FrameDolbyVisionColorMetadata::SIGNAL_EOTF_PARAM0_OFFSET,
            1,
        );
        write_ne_u16(
            &mut data,
            color + FrameDolbyVisionColorMetadata::SIGNAL_EOTF_PARAM1_OFFSET,
            2,
        );
        write_ne_u32(
            &mut data,
            color + FrameDolbyVisionColorMetadata::SIGNAL_EOTF_PARAM2_OFFSET,
            3,
        );
        data[color + FrameDolbyVisionColorMetadata::SIGNAL_BIT_DEPTH_OFFSET] = 12;
        data[color + FrameDolbyVisionColorMetadata::SIGNAL_COLOR_SPACE_OFFSET] = 9;
        data[color + FrameDolbyVisionColorMetadata::SIGNAL_CHROMA_FORMAT_OFFSET] = 1;
        data[color + FrameDolbyVisionColorMetadata::SIGNAL_FULL_RANGE_FLAG_OFFSET] = 3;
        write_ne_u16(
            &mut data,
            color + FrameDolbyVisionColorMetadata::SOURCE_MIN_PQ_OFFSET,
            64,
        );
        write_ne_u16(
            &mut data,
            color + FrameDolbyVisionColorMetadata::SOURCE_MAX_PQ_OFFSET,
            4095,
        );
        write_ne_u16(
            &mut data,
            color + FrameDolbyVisionColorMetadata::SOURCE_DIAGONAL_OFFSET,
            42,
        );

        let level1 = FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET;
        data[level1 + FrameDolbyVisionDmData::LEVEL_OFFSET] = 1;
        write_ne_u16(
            &mut data,
            level1 + FrameDolbyVisionDmData::LEVEL1_MIN_PQ_OFFSET,
            10,
        );
        write_ne_u16(
            &mut data,
            level1 + FrameDolbyVisionDmData::LEVEL1_MAX_PQ_OFFSET,
            2048,
        );
        write_ne_u16(
            &mut data,
            level1 + FrameDolbyVisionDmData::LEVEL1_AVG_PQ_OFFSET,
            512,
        );

        let level6 = level1 + FrameDolbyVisionMetadata::EXT_BLOCK_SIZE;
        data[level6 + FrameDolbyVisionDmData::LEVEL_OFFSET] = 6;
        write_ne_u16(
            &mut data,
            level6 + FrameDolbyVisionDmData::LEVEL6_MAX_LUMINANCE_OFFSET,
            1000,
        );
        write_ne_u16(
            &mut data,
            level6 + FrameDolbyVisionDmData::LEVEL6_MIN_LUMINANCE_OFFSET,
            1,
        );
        write_ne_u16(
            &mut data,
            level6 + FrameDolbyVisionDmData::LEVEL6_MAX_CLL_OFFSET,
            800,
        );
        write_ne_u16(
            &mut data,
            level6 + FrameDolbyVisionDmData::LEVEL6_MAX_FALL_OFFSET,
            400,
        );

        data
    }

    fn write_fixed_bytes(data: &mut [u8], offset: usize, len: usize, value: &[u8]) {
        assert!(value.len() < len);
        data[offset..offset + value.len()].copy_from_slice(value);
    }

    fn write_ne_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_ne_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_ne_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_ne_usize(data: &mut [u8], offset: usize, value: usize) {
        data[offset..offset + core::mem::size_of::<usize>()].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_ne_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_ne_i16(data: &mut [u8], offset: usize, value: i16) {
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
    fn frame_side_data_parses_pan_scan_payload() {
        let expected = FramePanScan::new(
            7,
            1920 * 16,
            1080 * 16,
            [[0, 0], [16, -32], [i16::MIN, i16::MAX]],
        );

        assert_eq!(FramePanScan::DATA_LEN, 24);
        assert_eq!(expected.id(), 7);
        assert_eq!(expected.width(), 1920 * 16);
        assert_eq!(expected.height(), 1080 * 16);
        assert_eq!(expected.position()[1], [16, -32]);
        assert_eq!(expected.field_position(2), Some([i16::MIN, i16::MAX]));
        assert_eq!(expected.field_position(3), None);
        assert_eq!(FramePanScan::parse(&expected.to_bytes()).unwrap(), expected);

        let side_data = FrameSideData::new_pan_scan(expected).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::PanScan);
        assert_eq!(side_data.data(), &expected.to_bytes()[..]);
        assert_eq!(side_data.pan_scan().unwrap(), Some(expected));

        let display_matrix = FrameSideData::new_with_kind(
            FrameSideDataKind::DisplayMatrix,
            expected.to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(display_matrix.pan_scan().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_pan_scan_payload() {
        for data in [
            Vec::new(),
            vec![0; FramePanScan::DATA_LEN - 1],
            vec![0; FramePanScan::DATA_LEN + 1],
        ] {
            assert_eq!(
                FramePanScan::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data = FrameSideData::new_with_kind(FrameSideDataKind::PanScan, data).unwrap();
            assert_eq!(
                side_data.pan_scan().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_pan_scan =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 24]).unwrap();
        assert_eq!(non_pan_scan.pan_scan().unwrap(), None);
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
    fn frame_side_data_parses_regions_of_interest_payload() {
        let high_priority =
            FrameRegionOfInterest::new(0, 72, 8, 128, Rational::from_raw(-1, 10)).unwrap();
        let lower_priority =
            FrameRegionOfInterest::new(72, 144, 0, 192, Rational::from_raw(1, -2)).unwrap();
        let regions = FrameRegionsOfInterest::new(vec![high_priority, lower_priority]).unwrap();
        let payload = regions.to_bytes();

        assert_eq!(FrameRegionOfInterest::DATA_LEN, 28);
        assert_eq!(high_priority.self_size(), FrameRegionOfInterest::SELF_SIZE);
        assert_eq!(high_priority.top(), 0);
        assert_eq!(high_priority.bottom(), 72);
        assert_eq!(high_priority.left(), 8);
        assert_eq!(high_priority.right(), 128);
        assert_eq!(high_priority.qoffset(), Rational::from_raw(-1, 10));
        assert_eq!(lower_priority.qoffset(), Rational::from_raw(1, -2));
        assert_eq!(regions.len(), 2);
        assert!(!regions.is_empty());
        assert_eq!(regions.regions(), &[high_priority, lower_priority]);
        assert_eq!(
            regions.clone().into_regions(),
            vec![high_priority, lower_priority]
        );
        assert_eq!(FrameRegionsOfInterest::parse(&payload).unwrap(), regions);

        let side_data = FrameSideData::new_regions_of_interest(regions.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::RegionsOfInterest);
        assert_eq!(side_data.data(), payload.as_slice());
        assert_eq!(side_data.regions_of_interest().unwrap(), Some(regions));

        let replay_gain = FrameSideData::new_with_kind(
            FrameSideDataKind::ReplayGain,
            vec![0; FrameRegionOfInterest::DATA_LEN],
        )
        .unwrap();
        assert_eq!(replay_gain.regions_of_interest().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_regions_of_interest_payload() {
        for data in [
            Vec::new(),
            vec![0; FrameRegionOfInterest::DATA_LEN - 1],
            vec![0; FrameRegionOfInterest::DATA_LEN + 1],
            vec![0; FrameRegionOfInterest::DATA_LEN * 2 - 1],
        ] {
            assert_eq!(
                FrameRegionsOfInterest::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::RegionsOfInterest, data).unwrap();
            assert_eq!(
                side_data.regions_of_interest().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut bad_self_size = FrameRegionOfInterest::new(0, 1, 0, 1, Rational::from_raw(0, 1))
            .unwrap()
            .to_bytes();
        write_ne_u32(&mut bad_self_size, 0, FrameRegionOfInterest::SELF_SIZE + 4);
        assert_eq!(
            FrameRegionsOfInterest::parse(&bad_self_size)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        for qoffset in [
            Rational::from_raw(1, 0),
            Rational::from_raw(2, 1),
            Rational::from_raw(-2, 1),
        ] {
            assert_eq!(
                FrameRegionOfInterest::new(0, 1, 0, 1, qoffset)
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidArgument
            );

            let mut bad_qoffset = FrameRegionOfInterest::new(0, 1, 0, 1, Rational::from_raw(0, 1))
                .unwrap()
                .to_bytes();
            write_ne_rational(&mut bad_qoffset, 20, qoffset);
            assert_eq!(
                FrameRegionsOfInterest::parse(&bad_qoffset)
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            FrameRegionsOfInterest::new(Vec::new()).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let replay_gain =
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, Vec::new()).unwrap();
        assert_eq!(replay_gain.regions_of_interest().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_video_enc_params_payload() {
        let first_block = FrameVideoBlockParams::new(-4, 8, 16, 16, -2).unwrap();
        let second_block = FrameVideoBlockParams::new(12, 24, 32, 16, 3).unwrap();
        let delta_qp = [[0, 1], [2, 3], [4, 5], [6, 7]];
        let params = FrameVideoEncParams::new(
            FrameVideoEncParamsType::H264,
            26,
            delta_qp,
            vec![first_block, second_block],
        )
        .unwrap();
        let payload = params.to_bytes();

        assert_eq!(
            FrameVideoEncParams::HEADER_LEN,
            if core::mem::size_of::<usize>() == 8 {
                64
            } else {
                52
            }
        );
        assert_eq!(FrameVideoEncParams::BLOCK_SIZE, 20);
        assert_eq!(FrameVideoBlockParams::DATA_LEN, 20);
        assert_eq!(first_block.src_x(), -4);
        assert_eq!(first_block.src_y(), 8);
        assert_eq!(first_block.width(), 16);
        assert_eq!(first_block.height(), 16);
        assert_eq!(first_block.delta_qp(), -2);
        assert_eq!(params.params_type(), FrameVideoEncParamsType::H264);
        assert_eq!(params.params_type().as_raw(), 1);
        assert_eq!(params.qp(), 26);
        assert_eq!(params.delta_qp(), &delta_qp);
        assert_eq!(params.nb_blocks(), 2);
        assert!(!params.is_empty());
        assert_eq!(params.blocks(), &[first_block, second_block]);
        assert_eq!(
            params.clone().into_blocks(),
            vec![first_block, second_block]
        );
        assert_eq!(FrameVideoEncParams::parse(&payload).unwrap(), params);

        let zero_blocks =
            FrameVideoEncParams::new(FrameVideoEncParamsType::Vp9, 12, [[0; 2]; 4], Vec::new())
                .unwrap();
        assert!(zero_blocks.is_empty());
        assert_eq!(
            FrameVideoEncParams::parse(&zero_blocks.to_bytes()).unwrap(),
            zero_blocks
        );

        let side_data = FrameSideData::new_video_enc_params(params.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::VideoEncParams);
        assert_eq!(side_data.data(), payload.as_slice());
        assert_eq!(side_data.video_enc_params().unwrap(), Some(params));

        let replay_gain =
            FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, payload).unwrap();
        assert_eq!(replay_gain.video_enc_params().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_video_enc_params_payload() {
        assert_eq!(
            FrameVideoEncParamsType::from_raw(99).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameVideoBlockParams::new(0, 0, 0, 16, 0)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );

        let block = FrameVideoBlockParams::new(0, 0, 16, 16, 0).unwrap();
        let params =
            FrameVideoEncParams::new(FrameVideoEncParamsType::Mpeg2, 18, [[0; 2]; 4], vec![block])
                .unwrap();
        let payload = params.to_bytes();

        for data in [
            vec![0; FrameVideoEncParams::HEADER_LEN - 1],
            {
                let mut data = payload.clone();
                data.push(0);
                data
            },
            {
                let mut data = payload.clone();
                data.pop();
                data
            },
        ] {
            assert_eq!(
                FrameVideoEncParams::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::VideoEncParams, data).unwrap();
            assert_eq!(
                side_data.video_enc_params().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (FrameVideoEncParams::TYPE_OFFSET, 3),
            (FrameVideoEncParams::HEADER_LEN + 8, 0),
            (FrameVideoEncParams::HEADER_LEN + 12, -1),
        ] {
            let mut bad = payload.clone();
            write_ne_i32(&mut bad, offset, value);
            assert_eq!(
                FrameVideoEncParams::parse(&bad).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut bad_block_offset = payload.clone();
        write_ne_usize(
            &mut bad_block_offset,
            FrameVideoEncParams::BLOCKS_OFFSET_OFFSET,
            FrameVideoEncParams::HEADER_LEN - 4,
        );
        assert_eq!(
            FrameVideoEncParams::parse(&bad_block_offset)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_block_size = payload.clone();
        write_ne_usize(
            &mut bad_block_size,
            FrameVideoEncParams::BLOCK_SIZE_OFFSET,
            FrameVideoEncParams::BLOCK_SIZE + 4,
        );
        assert_eq!(
            FrameVideoEncParams::parse(&bad_block_size)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, Vec::new()).unwrap();
        assert_eq!(display_matrix.video_enc_params().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_film_grain_params_payload() {
        let aom_payload = minimal_film_grain_av1();
        let params = FrameFilmGrainParams::parse(&aom_payload).unwrap();
        let aom = params.aom_params().unwrap().unwrap();

        assert_eq!(
            FrameFilmGrainParamsType::from_raw(0).unwrap(),
            FrameFilmGrainParamsType::None
        );
        assert_eq!(
            FrameFilmGrainParamsType::Av1.ffmpeg_constant(),
            "AV_FILM_GRAIN_PARAMS_AV1"
        );
        assert_eq!(FrameFilmGrainParamsType::H274.as_raw(), 2);
        assert_eq!(
            FrameFilmGrainParams::DATA_LEN,
            if core::mem::align_of::<u64>() == 8 {
                10_848
            } else {
                10_840
            }
        );
        assert_eq!(FrameFilmGrainAomParams::DATA_LEN, 208);
        assert_eq!(FrameFilmGrainH274Params::DATA_LEN, 10_788);
        assert_eq!(params.data(), aom_payload.as_slice());
        assert_eq!(params.params_type(), FrameFilmGrainParamsType::Av1);
        assert_eq!(params.seed(), 0x0102_0304_0506_0708);
        assert_eq!(params.width(), 1920);
        assert_eq!(params.height(), 1080);
        assert_eq!(params.subsampling_x(), 1);
        assert_eq!(params.subsampling_y(), 1);
        assert_eq!(params.color_range(), 1);
        assert_eq!(params.color_primaries(), 9);
        assert_eq!(params.color_transfer(), 16);
        assert_eq!(params.color_space(), 9);
        assert_eq!(params.bit_depth_luma(), 10);
        assert_eq!(params.bit_depth_chroma(), 10);
        assert_eq!(
            aom.data(),
            &aom_payload[FrameFilmGrainParams::CODEC_OFFSET..][..208]
        );
        assert_eq!(aom.num_y_points(), 2);
        assert_eq!(aom.y_point(0), Some([16, 3]));
        assert_eq!(aom.y_point(1), Some([128, 17]));
        assert_eq!(aom.y_point(2), None);
        assert!(!aom.chroma_scaling_from_luma());
        assert_eq!(aom.num_uv_points(0), Some(1));
        assert_eq!(aom.num_uv_points(1), Some(1));
        assert_eq!(aom.num_uv_points(2), None);
        assert_eq!(aom.uv_point(0, 0), Some([32, 4]));
        assert_eq!(aom.uv_point(1, 0), Some([64, 5]));
        assert_eq!(aom.uv_point(1, 1), None);
        assert_eq!(aom.scaling_shift(), 10);
        assert_eq!(aom.ar_coeff_lag(), 1);
        assert_eq!(aom.ar_coeff_count_y(), 4);
        assert_eq!(aom.ar_coeff_count_uv(), 5);
        assert_eq!(aom.ar_coeff_y(0), Some(-3));
        assert_eq!(aom.ar_coeff_y(4), None);
        assert_eq!(aom.ar_coeff_uv(0, 0), Some(2));
        assert_eq!(aom.ar_coeff_uv(1, 4), Some(-7));
        assert_eq!(aom.ar_coeff_uv(2, 0), None);
        assert_eq!(aom.ar_coeff_shift(), 7);
        assert_eq!(aom.grain_scale_shift(), 2);
        assert_eq!(aom.uv_mult(0), Some(128));
        assert_eq!(aom.uv_mult_luma(1), Some(48));
        assert_eq!(aom.uv_offset(0), Some(-64));
        assert!(aom.overlap_flag());
        assert!(!aom.limit_output_range());
        assert!(params.h274_params().unwrap().is_none());

        let h274_payload = minimal_film_grain_h274();
        let h274_params = FrameFilmGrainParams::parse(&h274_payload).unwrap();
        let h274 = h274_params.h274_params().unwrap().unwrap();
        assert_eq!(h274_params.params_type(), FrameFilmGrainParamsType::H274);
        assert!(h274_params.aom_params().unwrap().is_none());
        assert_eq!(h274.model_id(), 1);
        assert_eq!(h274.blending_mode_id(), 0);
        assert_eq!(h274.log2_scale_factor(), 3);
        assert_eq!(h274.component_model_present(0), Some(true));
        assert_eq!(h274.component_model_present(1), Some(false));
        assert_eq!(h274.component_model_present(3), None);
        assert_eq!(h274.num_intensity_intervals(0), Some(2));
        assert_eq!(h274.num_model_values(0), Some(3));
        assert_eq!(h274.intensity_interval_lower_bound(0, 1), Some(64));
        assert_eq!(h274.intensity_interval_upper_bound(0, 1), Some(127));
        assert_eq!(h274.comp_model_value(0, 1, 2), Some(-14));
        assert_eq!(h274.comp_model_value(0, 1, 3), None);
        assert_eq!(h274.comp_model_value(3, 0, 0), None);

        let side_data = FrameSideData::new_film_grain_params(aom_payload.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::FilmGrainParams);
        assert_eq!(side_data.data(), aom_payload.as_slice());
        assert_eq!(
            side_data
                .film_grain_params()
                .unwrap()
                .unwrap()
                .params_type(),
            FrameFilmGrainParamsType::Av1
        );

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, aom_payload).unwrap();
        assert_eq!(display_matrix.film_grain_params().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_film_grain_params_payload() {
        assert_eq!(
            FrameFilmGrainParamsType::from_raw(99).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        for data in [
            Vec::new(),
            vec![0; FrameFilmGrainParams::DATA_LEN - 1],
            vec![0; FrameFilmGrainParams::DATA_LEN + 1],
        ] {
            assert_eq!(
                FrameFilmGrainParams::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::FilmGrainParams, data).unwrap();
            assert_eq!(
                side_data.film_grain_params().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (FrameFilmGrainParams::TYPE_OFFSET, 3),
            (FrameFilmGrainParams::WIDTH_OFFSET, -1),
            (FrameFilmGrainParams::HEIGHT_OFFSET, -1),
            (FrameFilmGrainParams::SUBSAMPLING_X_OFFSET, -1),
            (FrameFilmGrainParams::BIT_DEPTH_LUMA_OFFSET, -1),
        ] {
            let mut bad = minimal_film_grain_av1();
            write_ne_i32(&mut bad, offset, value);
            assert_eq!(
                FrameFilmGrainParams::parse(&bad).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::NUM_Y_POINTS_OFFSET,
                -1,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::NUM_Y_POINTS_OFFSET,
                15,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET
                    + FrameFilmGrainAomParams::CHROMA_SCALING_FROM_LUMA_OFFSET,
                2,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::NUM_UV_POINTS_OFFSET,
                -1,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::NUM_UV_POINTS_OFFSET,
                11,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::SCALING_SHIFT_OFFSET,
                7,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::AR_COEFF_LAG_OFFSET,
                4,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::AR_COEFF_SHIFT_OFFSET,
                10,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET
                    + FrameFilmGrainAomParams::GRAIN_SCALE_SHIFT_OFFSET,
                4,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::UV_OFFSET_OFFSET,
                -257,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainAomParams::OVERLAP_FLAG_OFFSET,
                2,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET
                    + FrameFilmGrainAomParams::LIMIT_OUTPUT_RANGE_OFFSET,
                2,
            ),
        ] {
            let mut bad = minimal_film_grain_av1();
            write_ne_i32(&mut bad, offset, value);
            assert_eq!(
                FrameSideData::new_with_kind(FrameSideDataKind::FilmGrainParams, bad)
                    .unwrap()
                    .film_grain_params()
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (
                FrameFilmGrainParams::CODEC_OFFSET + FrameFilmGrainH274Params::MODEL_ID_OFFSET,
                2,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET
                    + FrameFilmGrainH274Params::BLENDING_MODE_ID_OFFSET,
                2,
            ),
            (
                FrameFilmGrainParams::CODEC_OFFSET
                    + FrameFilmGrainH274Params::COMPONENT_MODEL_PRESENT_OFFSET,
                2,
            ),
        ] {
            let mut bad = minimal_film_grain_h274();
            write_ne_i32(&mut bad, offset, value);
            assert_eq!(
                FrameFilmGrainParams::parse(&bad).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut missing_h274_intervals = minimal_film_grain_h274();
        write_ne_u16(
            &mut missing_h274_intervals,
            FrameFilmGrainParams::CODEC_OFFSET
                + FrameFilmGrainH274Params::NUM_INTENSITY_INTERVALS_OFFSET,
            0,
        );
        assert_eq!(
            FrameFilmGrainParams::parse(&missing_h274_intervals)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut too_many_h274_values = minimal_film_grain_h274();
        too_many_h274_values[FrameFilmGrainParams::CODEC_OFFSET
            + FrameFilmGrainH274Params::NUM_MODEL_VALUES_OFFSET] = 7;
        assert_eq!(
            FrameFilmGrainParams::parse(&too_many_h274_values)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut inverted_h274_interval = minimal_film_grain_h274();
        inverted_h274_interval[FrameFilmGrainParams::CODEC_OFFSET
            + FrameFilmGrainH274Params::INTENSITY_INTERVAL_LOWER_BOUND_OFFSET] = 200;
        inverted_h274_interval[FrameFilmGrainParams::CODEC_OFFSET
            + FrameFilmGrainH274Params::INTENSITY_INTERVAL_UPPER_BOUND_OFFSET] = 100;
        assert_eq!(
            FrameSideData::new_film_grain_params(inverted_h274_interval)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut absent_h274_counts = minimal_film_grain_h274();
        write_ne_i32(
            &mut absent_h274_counts,
            FrameFilmGrainParams::CODEC_OFFSET
                + FrameFilmGrainH274Params::COMPONENT_MODEL_PRESENT_OFFSET,
            0,
        );
        assert_eq!(
            FrameFilmGrainParams::parse(&absent_h274_counts)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, Vec::new()).unwrap();
        assert_eq!(display_matrix.film_grain_params().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_detection_bboxes_payload() {
        let payload = minimal_detection_bboxes();
        let parsed = FrameDetectionBboxes::parse(&payload).unwrap();

        assert_eq!(
            FrameDetectionBboxes::HEADER_LEN,
            if core::mem::size_of::<usize>() == 8 {
                280
            } else {
                268
            }
        );
        assert_eq!(FrameDetectionBbox::DATA_LEN, 380);
        assert_eq!(parsed.data(), payload.as_slice());
        assert_eq!(parsed.source(), b"rust-detector");
        assert_eq!(parsed.source_raw().len(), FrameDetectionBboxes::SOURCE_LEN);
        assert_eq!(parsed.nb_bboxes(), 2);
        assert!(!parsed.is_empty());
        assert_eq!(parsed.bboxes().count(), 2);

        let first = parsed.bbox(0).unwrap().unwrap();
        assert_eq!(
            first.data(),
            &payload[FrameDetectionBboxes::BBOXES_OFFSET..][..380]
        );
        assert_eq!(first.x(), 10);
        assert_eq!(first.y(), 20);
        assert_eq!(first.width(), 30);
        assert_eq!(first.height(), 40);
        assert_eq!(first.detect_label(), b"person");
        assert_eq!(
            first.detect_label_raw().len(),
            FrameDetectionBbox::LABEL_LEN
        );
        assert_eq!(first.detect_confidence(), Rational::from_raw(9, 10));
        assert_eq!(first.classify_count(), 2);
        assert_eq!(first.classify_label(0), Some(&b"adult"[..]));
        assert_eq!(first.classify_label(1), Some(&b"standing"[..]));
        assert_eq!(first.classify_label(2), None);
        assert_eq!(
            first.classify_label_raw(FrameDetectionBbox::MAX_CLASSIFICATIONS),
            None
        );
        assert_eq!(first.classify_confidence(0), Some(Rational::from_raw(3, 4)));
        assert_eq!(first.classify_confidence(1), Some(Rational::from_raw(5, 6)));
        assert_eq!(first.classify_confidence(2), None);

        let second = parsed.bbox(1).unwrap().unwrap();
        assert_eq!(second.x(), 64);
        assert_eq!(second.y(), 72);
        assert_eq!(second.width(), 16);
        assert_eq!(second.height(), 18);
        assert_eq!(second.detect_label(), b"ball");
        assert_eq!(second.detect_confidence(), Rational::from_raw(7, 8));
        assert_eq!(second.classify_count(), 0);
        assert!(parsed.bbox(2).is_none());

        let side_data = FrameSideData::new_detection_bboxes(payload.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::DetectionBboxes);
        assert_eq!(side_data.data(), payload.as_slice());
        assert_eq!(
            side_data.detection_bboxes().unwrap().unwrap().source(),
            b"rust-detector"
        );

        let zero_payload = {
            let mut data = vec![0; FrameDetectionBboxes::HEADER_LEN];
            write_ne_u32(&mut data, FrameDetectionBboxes::NB_BBOXES_OFFSET, 0);
            write_ne_usize(
                &mut data,
                FrameDetectionBboxes::BBOXES_OFFSET_OFFSET,
                FrameDetectionBboxes::BBOXES_OFFSET,
            );
            write_ne_usize(
                &mut data,
                FrameDetectionBboxes::BBOX_SIZE_OFFSET,
                FrameDetectionBbox::DATA_LEN,
            );
            data
        };
        let zero = FrameDetectionBboxes::parse(&zero_payload).unwrap();
        assert!(zero.is_empty());
        assert_eq!(zero.nb_bboxes(), 0);

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, payload).unwrap();
        assert_eq!(display_matrix.detection_bboxes().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_detection_bboxes_payload() {
        for data in [
            Vec::new(),
            vec![0; FrameDetectionBboxes::HEADER_LEN - 1],
            {
                let mut data = minimal_detection_bboxes();
                data.pop();
                data
            },
            {
                let mut data = minimal_detection_bboxes();
                data.push(0);
                data
            },
        ] {
            assert_eq!(
                FrameDetectionBboxes::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DetectionBboxes, data).unwrap();
            assert_eq!(
                side_data.detection_bboxes().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (
                FrameDetectionBboxes::BBOXES_OFFSET_OFFSET,
                FrameDetectionBboxes::BBOXES_OFFSET + 4,
            ),
            (
                FrameDetectionBboxes::BBOX_SIZE_OFFSET,
                FrameDetectionBbox::DATA_LEN + 4,
            ),
        ] {
            let mut bad = minimal_detection_bboxes();
            write_ne_usize(&mut bad, offset, value);
            assert_eq!(
                FrameSideData::new_detection_bboxes(bad).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut bad_count = minimal_detection_bboxes();
        write_ne_u32(&mut bad_count, FrameDetectionBboxes::NB_BBOXES_OFFSET, 3);
        assert_eq!(
            FrameDetectionBboxes::parse(&bad_count).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_classify_count = minimal_detection_bboxes();
        write_ne_u32(
            &mut bad_classify_count,
            FrameDetectionBboxes::BBOXES_OFFSET + FrameDetectionBbox::CLASSIFY_COUNT_OFFSET,
            FrameDetectionBbox::MAX_CLASSIFICATIONS as u32 + 1,
        );
        assert_eq!(
            FrameDetectionBboxes::parse(&bad_classify_count)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let non_detection =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_detection.detection_bboxes().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_dolby_vision_payloads() {
        let rpu_bytes = vec![0x7C, 0x01, 0x19, 0xAB];
        let rpu_side_data = FrameSideData::new_dolby_vision_rpu_buffer(rpu_bytes.clone()).unwrap();
        assert_eq!(
            rpu_side_data.kind_id(),
            &FrameSideDataKind::DolbyVisionRpuBuffer
        );
        let rpu = rpu_side_data.dolby_vision_rpu_buffer().unwrap();
        assert_eq!(rpu.data(), rpu_bytes.as_slice());
        assert!(!rpu.is_empty());

        let empty_rpu = FrameSideData::new_dolby_vision_rpu_buffer(Vec::new()).unwrap();
        assert!(empty_rpu.dolby_vision_rpu_buffer().unwrap().is_empty());

        let payload = minimal_dolby_vision_metadata();
        let parsed = FrameDolbyVisionMetadata::parse(&payload).unwrap();
        assert_eq!(
            FrameDolbyVisionMetadata::DATA_LEN,
            if core::mem::size_of::<usize>() == 8 {
                7_848
            } else {
                7_804
            }
        );
        assert_eq!(FrameDolbyVisionRpuDataHeader::DATA_LEN, 20);
        assert_eq!(FrameDolbyVisionDataMapping::DATA_LEN, 5_144);
        assert_eq!(FrameDolbyVisionColorMetadata::DATA_LEN, 196);
        assert_eq!(FrameDolbyVisionDmData::DATA_LEN, 76);
        assert_eq!(parsed.data(), payload.as_slice());
        assert_eq!(parsed.num_ext_blocks(), 2);
        assert!(!parsed.is_empty());

        let header = parsed.header().unwrap();
        assert_eq!(
            header.data(),
            &payload[FrameDolbyVisionMetadata::HEADER_OFFSET..][..20]
        );
        assert_eq!(header.rpu_type(), 2);
        assert_eq!(header.rpu_format(), 18);
        assert_eq!(header.vdr_rpu_profile(), 8);
        assert_eq!(header.vdr_rpu_level(), 6);
        assert_eq!(header.coef_data_type(), 1);
        assert_eq!(header.coef_log2_denom(), 28);
        assert_eq!(header.bl_bit_depth(), 10);
        assert_eq!(header.el_bit_depth(), 10);
        assert_eq!(header.vdr_bit_depth(), 12);
        assert!(header.disable_residual_flag());
        assert_eq!(header.ext_mapping_idc_0_4(), 4);

        let mapping = parsed.mapping().unwrap();
        assert_eq!(mapping.vdr_rpu_id(), 3);
        assert_eq!(mapping.mapping_color_space(), 1);
        assert_eq!(mapping.mapping_chroma_format_idc(), 2);
        assert_eq!(mapping.nlq_method_idc(), 0);
        assert_eq!(mapping.num_x_partitions(), 1);
        assert_eq!(mapping.num_y_partitions(), 1);

        let color = parsed.color().unwrap();
        assert_eq!(color.dm_metadata_id(), 9);
        assert_eq!(color.scene_refresh_flag(), 1);
        assert_eq!(color.ycc_to_rgb_matrix(0), Some(Rational::from_raw(1, 2)));
        assert_eq!(color.ycc_to_rgb_matrix(9), None);
        assert_eq!(color.ycc_to_rgb_offset(0), Some(Rational::from_raw(3, 4)));
        assert_eq!(color.rgb_to_lms_matrix(0), Some(Rational::from_raw(5, 6)));
        assert_eq!(color.signal_eotf(), 2084);
        assert_eq!(color.signal_eotf_param0(), 1);
        assert_eq!(color.signal_eotf_param1(), 2);
        assert_eq!(color.signal_eotf_param2(), 3);
        assert_eq!(color.signal_bit_depth(), 12);
        assert_eq!(color.signal_color_space(), 9);
        assert_eq!(color.signal_chroma_format(), 1);
        assert_eq!(color.signal_full_range_flag(), 3);
        assert_eq!(color.source_min_pq(), 64);
        assert_eq!(color.source_max_pq(), 4095);
        assert_eq!(color.source_diagonal(), 42);

        let level1 = parsed.ext_block(0).unwrap().unwrap();
        assert_eq!(level1.level(), 1);
        assert_eq!(level1.level1_min_pq(), Some(10));
        assert_eq!(level1.level1_max_pq(), Some(2048));
        assert_eq!(level1.level1_avg_pq(), Some(512));
        assert_eq!(level1.level6_max_luminance(), None);

        let level6 = parsed.find_level(6).unwrap().unwrap();
        assert_eq!(level6.level(), 6);
        assert_eq!(level6.level6_max_luminance(), Some(1000));
        assert_eq!(level6.level6_min_luminance(), Some(1));
        assert_eq!(level6.level6_max_content_light_level(), Some(800));
        assert_eq!(level6.level6_max_frame_average_light_level(), Some(400));
        assert!(parsed.find_level(8).is_none());
        assert!(parsed.ext_block(2).is_none());
        assert_eq!(parsed.ext_blocks().count(), 2);

        let side_data = FrameSideData::new_dolby_vision_metadata(payload.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::DolbyVisionMetadata);
        assert_eq!(side_data.data(), payload.as_slice());
        assert_eq!(
            side_data
                .dolby_vision_metadata()
                .unwrap()
                .unwrap()
                .num_ext_blocks(),
            2
        );

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, payload).unwrap();
        assert_eq!(display_matrix.dolby_vision_rpu_buffer(), None);
        assert_eq!(display_matrix.dolby_vision_metadata().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_dolby_vision_metadata_payload() {
        for data in [
            Vec::new(),
            vec![0; FrameDolbyVisionMetadata::DATA_LEN - 1],
            {
                let mut data = minimal_dolby_vision_metadata();
                data.push(0);
                data
            },
        ] {
            assert_eq!(
                FrameDolbyVisionMetadata::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DolbyVisionMetadata, data).unwrap();
            assert_eq!(
                side_data.dolby_vision_metadata().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (
                FrameDolbyVisionMetadata::HEADER_OFFSET_OFFSET,
                FrameDolbyVisionMetadata::HEADER_OFFSET + 4,
            ),
            (
                FrameDolbyVisionMetadata::MAPPING_OFFSET_OFFSET,
                FrameDolbyVisionMetadata::MAPPING_OFFSET + 4,
            ),
            (
                FrameDolbyVisionMetadata::COLOR_OFFSET_OFFSET,
                FrameDolbyVisionMetadata::COLOR_OFFSET + 4,
            ),
            (
                FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET_OFFSET,
                FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET + 4,
            ),
            (
                FrameDolbyVisionMetadata::EXT_BLOCK_SIZE_OFFSET,
                FrameDolbyVisionMetadata::EXT_BLOCK_SIZE + 4,
            ),
        ] {
            let mut bad = minimal_dolby_vision_metadata();
            write_ne_usize(&mut bad, offset, value);
            assert_eq!(
                FrameSideData::new_dolby_vision_metadata(bad)
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
        }

        for count in [-1, FrameDolbyVisionMetadata::MAX_EXT_BLOCKS as i32 + 1] {
            let mut bad = minimal_dolby_vision_metadata();
            write_ne_i32(
                &mut bad,
                FrameDolbyVisionMetadata::NUM_EXT_BLOCKS_OFFSET,
                count,
            );
            assert_eq!(
                FrameDolbyVisionMetadata::parse(&bad).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut bad_level = minimal_dolby_vision_metadata();
        bad_level
            [FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET + FrameDolbyVisionDmData::LEVEL_OFFSET] = 0;
        assert_eq!(
            FrameDolbyVisionMetadata::parse(&bad_level)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_color = minimal_dolby_vision_metadata();
        bad_color[FrameDolbyVisionMetadata::COLOR_OFFSET
            + FrameDolbyVisionColorMetadata::SIGNAL_FULL_RANGE_FLAG_OFFSET] = 4;
        assert_eq!(
            FrameSideData::new_with_kind(FrameSideDataKind::DolbyVisionMetadata, bad_color)
                .unwrap()
                .dolby_vision_metadata()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let non_dovi =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_dovi.dolby_vision_rpu_buffer(), None);
        assert_eq!(non_dovi.dolby_vision_metadata().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_dynamic_hdr_vivid_payload() {
        let data = minimal_dynamic_hdr_vivid();
        let side_data = FrameSideData::new_dynamic_hdr_vivid(data.clone()).unwrap();

        assert_eq!(FrameDynamicHdrVivid::DATA_LEN, 1372);
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::DynamicHdrVivid);
        let parsed = side_data.dynamic_hdr_vivid().unwrap().unwrap();
        assert_eq!(parsed.data(), data.as_slice());
        assert_eq!(
            parsed.system_start_code(),
            FrameDynamicHdrVivid::MIN_SYSTEM_START_CODE
        );
        assert_eq!(parsed.num_windows(), 1);
        assert_eq!(parsed.color_transform_params(1), None);

        let params = parsed.color_transform_params(0).unwrap();
        assert_eq!(
            params.data().len(),
            FrameHdrVividColorTransformParams::DATA_LEN
        );
        assert_eq!(params.minimum_maxrgb(), Rational::from_raw(1, 4095));
        assert_eq!(params.average_maxrgb(), Rational::from_raw(2, 4095));
        assert_eq!(params.variance_maxrgb(), Rational::from_raw(3, 4095));
        assert_eq!(params.maximum_maxrgb(), Rational::from_raw(4, 4095));
        assert_eq!(params.tone_mapping_mode_flag(), 1);
        assert_eq!(params.tone_mapping_param_num(), 2);
        assert_eq!(params.tone_mapping_params(2), None);
        assert_eq!(params.color_saturation_mapping_flag(), 1);
        assert_eq!(params.color_saturation_num(), 2);
        assert_eq!(
            params.color_saturation_gain(0),
            Some(Rational::from_raw(1, 128))
        );
        assert_eq!(
            params.color_saturation_gain(1),
            Some(Rational::from_raw(2, 128))
        );
        assert_eq!(params.color_saturation_gain(2), None);

        let tone_mapping = params.tone_mapping_params(0).unwrap();
        assert_eq!(
            tone_mapping.targeted_system_display_maximum_luminance(),
            Rational::from_raw(100, 4095)
        );
        assert_eq!(tone_mapping.base_enable_flag(), 1);
        assert_eq!(tone_mapping.base_param_m_p(), Rational::from_raw(10, 16383));
        assert_eq!(tone_mapping.base_param_m_m(), Rational::from_raw(11, 10));
        assert_eq!(tone_mapping.base_param_m_a(), Rational::from_raw(12, 1023));
        assert_eq!(tone_mapping.base_param_m_b(), Rational::from_raw(13, 1023));
        assert_eq!(tone_mapping.base_param_m_n(), Rational::from_raw(14, 10));
        assert_eq!(tone_mapping.base_param_k1(), 1);
        assert_eq!(tone_mapping.base_param_k2(), 0);
        assert_eq!(tone_mapping.base_param_k3(), 2);
        assert_eq!(tone_mapping.base_param_delta_enable_mode(), 1);
        assert_eq!(tone_mapping.base_param_delta(), Rational::from_raw(7, 127));
        assert_eq!(tone_mapping.three_spline_enable_flag(), 1);
        assert_eq!(tone_mapping.three_spline_num(), 2);
        assert!(tone_mapping.three_spline(2).is_none());

        let spline0 = tone_mapping.three_spline(0).unwrap().unwrap();
        assert_eq!(spline0.data().len(), FrameHdrVivid3SplineParams::DATA_LEN);
        assert_eq!(spline0.th_mode(), 0);
        assert_eq!(spline0.th_enable_mb(), Rational::from_raw(9, 255));
        assert_eq!(spline0.th_enable(), Rational::from_raw(10, 4095));
        assert_eq!(spline0.th_delta1(), Rational::from_raw(11, 1023));
        assert_eq!(spline0.th_delta2(), Rational::from_raw(12, 1023));
        assert_eq!(spline0.enable_strength(), Rational::from_raw(13, 255));

        let spline1 = tone_mapping.three_spline(1).unwrap().unwrap();
        assert_eq!(spline1.th_mode(), 3);
        assert_eq!(spline1.th_enable(), Rational::from_raw(20, 4095));
        assert_eq!(spline1.th_delta1(), Rational::from_raw(21, 1023));
        assert_eq!(spline1.th_delta2(), Rational::from_raw(22, 1023));
        assert_eq!(spline1.enable_strength(), Rational::from_raw(23, 255));

        let second_tone_mapping = params.tone_mapping_params(1).unwrap();
        assert_eq!(
            second_tone_mapping.targeted_system_display_maximum_luminance(),
            Rational::from_raw(200, 4095)
        );
        assert_eq!(second_tone_mapping.base_enable_flag(), 0);
        assert_eq!(second_tone_mapping.three_spline_enable_flag(), 0);
        assert!(second_tone_mapping.three_spline(0).is_none());

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, data).unwrap();
        assert_eq!(display_matrix.dynamic_hdr_vivid().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_dynamic_hdr_vivid_payload() {
        for data in [
            Vec::new(),
            vec![0; FrameDynamicHdrVivid::DATA_LEN - 1],
            vec![0; FrameDynamicHdrVivid::DATA_LEN + 1],
        ] {
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DynamicHdrVivid, data).unwrap();
            assert_eq!(
                side_data.dynamic_hdr_vivid().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (0, 0),
            (0, FrameDynamicHdrVivid::MAX_SYSTEM_START_CODE + 1),
            (1, 0),
            (1, FrameDynamicHdrVivid::MAX_WINDOWS as u8 + 1),
        ] {
            let mut bad = minimal_dynamic_hdr_vivid();
            bad[offset] = value;
            assert_eq!(
                FrameSideData::new_dynamic_hdr_vivid(bad)
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, value) in [
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::TONE_MAPPING_MODE_FLAG_OFFSET,
                2,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAM_NUM_OFFSET,
                0,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAM_NUM_OFFSET,
                3,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAMS_OFFSET
                    + FrameHdrVividColorToneMappingParams::BASE_ENABLE_FLAG_OFFSET,
                2,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAMS_OFFSET
                    + FrameHdrVividColorToneMappingParams::THREE_SPLINE_ENABLE_FLAG_OFFSET,
                2,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAMS_OFFSET
                    + FrameHdrVividColorToneMappingParams::THREE_SPLINE_NUM_OFFSET,
                0,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAMS_OFFSET
                    + FrameHdrVividColorToneMappingParams::THREE_SPLINE_NUM_OFFSET,
                3,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::TONE_MAPPING_PARAMS_OFFSET
                    + FrameHdrVividColorToneMappingParams::THREE_SPLINE_OFFSET
                    + FrameHdrVivid3SplineParams::TH_MODE_OFFSET,
                4,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::COLOR_SATURATION_MAPPING_FLAG_OFFSET,
                2,
            ),
            (
                FrameDynamicHdrVivid::PARAMS_OFFSET
                    + FrameHdrVividColorTransformParams::COLOR_SATURATION_NUM_OFFSET,
                8,
            ),
        ] {
            let mut bad = minimal_dynamic_hdr_vivid();
            write_ne_i32(&mut bad, offset, value);
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::DynamicHdrVivid, bad).unwrap();
            assert_eq!(
                side_data.dynamic_hdr_vivid().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_hdr =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_hdr.dynamic_hdr_vivid().unwrap(), None);
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
