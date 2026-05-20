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
pub struct FrameA53ClosedCaptions<'a> {
    data: &'a [u8],
}

impl<'a> FrameA53ClosedCaptions<'a> {
    pub const BYTES_PER_CC: usize = 3;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() % Self::BYTES_PER_CC != 0 {
            return Err(AvError::invalid_data(format!(
                "A53 closed-caption frame side data requires whole {}-byte CC entries, got {} bytes",
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
#[repr(i32)]
pub enum FrameStereo3dType {
    TwoDimensional = 0,
    SideBySide = 1,
    TopBottom = 2,
    FrameSequence = 3,
    Checkerboard = 4,
    SideBySideQuincunx = 5,
    Lines = 6,
    Columns = 7,
    Unspecified = 8,
}

impl FrameStereo3dType {
    pub fn from_raw(value: i32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::TwoDimensional),
            1 => Ok(Self::SideBySide),
            2 => Ok(Self::TopBottom),
            3 => Ok(Self::FrameSequence),
            4 => Ok(Self::Checkerboard),
            5 => Ok(Self::SideBySideQuincunx),
            6 => Ok(Self::Lines),
            7 => Ok(Self::Columns),
            8 => Ok(Self::Unspecified),
            _ => Err(AvError::invalid_data(format!(
                "invalid stereo3d type value {value}"
            ))),
        }
    }

    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::TwoDimensional => "AV_STEREO3D_2D",
            Self::SideBySide => "AV_STEREO3D_SIDEBYSIDE",
            Self::TopBottom => "AV_STEREO3D_TOPBOTTOM",
            Self::FrameSequence => "AV_STEREO3D_FRAMESEQUENCE",
            Self::Checkerboard => "AV_STEREO3D_CHECKERBOARD",
            Self::SideBySideQuincunx => "AV_STEREO3D_SIDEBYSIDE_QUINCUNX",
            Self::Lines => "AV_STEREO3D_LINES",
            Self::Columns => "AV_STEREO3D_COLUMNS",
            Self::Unspecified => "AV_STEREO3D_UNSPEC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FrameStereo3dView {
    Packed = 0,
    Left = 1,
    Right = 2,
    Unspecified = 3,
}

impl FrameStereo3dView {
    pub fn from_raw(value: i32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::Packed),
            1 => Ok(Self::Left),
            2 => Ok(Self::Right),
            3 => Ok(Self::Unspecified),
            _ => Err(AvError::invalid_data(format!(
                "invalid stereo3d view value {value}"
            ))),
        }
    }

    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::Packed => "AV_STEREO3D_VIEW_PACKED",
            Self::Left => "AV_STEREO3D_VIEW_LEFT",
            Self::Right => "AV_STEREO3D_VIEW_RIGHT",
            Self::Unspecified => "AV_STEREO3D_VIEW_UNSPEC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FrameStereo3dPrimaryEye {
    None = 0,
    Left = 1,
    Right = 2,
}

impl FrameStereo3dPrimaryEye {
    pub fn from_raw(value: i32) -> AvResult<Self> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Left),
            2 => Ok(Self::Right),
            _ => Err(AvError::invalid_data(format!(
                "invalid stereo3d primary eye value {value}"
            ))),
        }
    }

    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::None => "AV_PRIMARY_EYE_NONE",
            Self::Left => "AV_PRIMARY_EYE_LEFT",
            Self::Right => "AV_PRIMARY_EYE_RIGHT",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameStereo3dFlags(u32);

impl FrameStereo3dFlags {
    pub const EMPTY: Self = Self(0);
    pub const INVERT: Self = Self(1 << 0);
    pub const ALL: Self = Self(Self::INVERT.0);

    pub fn from_bits(bits: u32) -> AvResult<Self> {
        if bits & !Self::ALL.bits() != 0 {
            return Err(AvError::invalid_data(format!(
                "invalid stereo3d flags bits 0x{bits:08x}"
            )));
        }

        Ok(Self(bits))
    }

    pub fn from_raw(value: i32) -> AvResult<Self> {
        let bits = u32::try_from(value).map_err(|_| {
            AvError::invalid_data(format!("invalid negative stereo3d flags value {value}"))
        })?;
        Self::from_bits(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn as_raw(self) -> i32 {
        self.0 as i32
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameStereo3d {
    stereo_type: FrameStereo3dType,
    flags: FrameStereo3dFlags,
    view: FrameStereo3dView,
    primary_eye: FrameStereo3dPrimaryEye,
    baseline: u32,
    horizontal_disparity_adjustment: Rational,
    horizontal_field_of_view: Rational,
}

impl FrameStereo3d {
    pub const RATIONAL_LEN: usize = 8;
    pub const TYPE_OFFSET: usize = 0;
    pub const FLAGS_OFFSET: usize = 4;
    pub const VIEW_OFFSET: usize = 8;
    pub const PRIMARY_EYE_OFFSET: usize = 12;
    pub const BASELINE_OFFSET: usize = 16;
    pub const HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET: usize = 20;
    pub const HORIZONTAL_FIELD_OF_VIEW_OFFSET: usize =
        Self::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET + Self::RATIONAL_LEN;
    pub const DATA_LEN: usize = Self::HORIZONTAL_FIELD_OF_VIEW_OFFSET + Self::RATIONAL_LEN;

    pub fn new(
        stereo_type: FrameStereo3dType,
        flags: FrameStereo3dFlags,
        view: FrameStereo3dView,
        primary_eye: FrameStereo3dPrimaryEye,
        baseline: u32,
        horizontal_disparity_adjustment: Rational,
        horizontal_field_of_view: Rational,
    ) -> AvResult<Self> {
        Self::validate_horizontal_disparity_adjustment(horizontal_disparity_adjustment)?;
        Self::validate_horizontal_field_of_view(horizontal_field_of_view)?;

        Ok(Self {
            stereo_type,
            flags,
            view,
            primary_eye,
            baseline,
            horizontal_disparity_adjustment,
            horizontal_field_of_view,
        })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "stereo3d frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Self::new(
            FrameStereo3dType::from_raw(Self::read_i32(data, Self::TYPE_OFFSET))?,
            FrameStereo3dFlags::from_raw(Self::read_i32(data, Self::FLAGS_OFFSET))?,
            FrameStereo3dView::from_raw(Self::read_i32(data, Self::VIEW_OFFSET))?,
            FrameStereo3dPrimaryEye::from_raw(Self::read_i32(data, Self::PRIMARY_EYE_OFFSET))?,
            Self::read_u32(data, Self::BASELINE_OFFSET),
            Self::read_rational(data, Self::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET),
            Self::read_rational(data, Self::HORIZONTAL_FIELD_OF_VIEW_OFFSET),
        )
    }

    pub const fn stereo_type(self) -> FrameStereo3dType {
        self.stereo_type
    }

    pub const fn flags(self) -> FrameStereo3dFlags {
        self.flags
    }

    pub const fn view(self) -> FrameStereo3dView {
        self.view
    }

    pub const fn primary_eye(self) -> FrameStereo3dPrimaryEye {
        self.primary_eye
    }

    pub const fn baseline(self) -> u32 {
        self.baseline
    }

    pub const fn horizontal_disparity_adjustment(self) -> Rational {
        self.horizontal_disparity_adjustment
    }

    pub const fn horizontal_field_of_view(self) -> Rational {
        self.horizontal_field_of_view
    }

    pub const fn has_inverted_views(self) -> bool {
        self.flags.contains(FrameStereo3dFlags::INVERT)
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        Self::write_i32(&mut bytes, Self::TYPE_OFFSET, self.stereo_type.as_raw());
        Self::write_i32(&mut bytes, Self::FLAGS_OFFSET, self.flags.as_raw());
        Self::write_i32(&mut bytes, Self::VIEW_OFFSET, self.view.as_raw());
        Self::write_i32(
            &mut bytes,
            Self::PRIMARY_EYE_OFFSET,
            self.primary_eye.as_raw(),
        );
        Self::write_u32(&mut bytes, Self::BASELINE_OFFSET, self.baseline);
        Self::write_rational(
            &mut bytes,
            Self::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET,
            self.horizontal_disparity_adjustment,
        );
        Self::write_rational(
            &mut bytes,
            Self::HORIZONTAL_FIELD_OF_VIEW_OFFSET,
            self.horizontal_field_of_view,
        );
        bytes
    }

    fn validate_horizontal_disparity_adjustment(value: Rational) -> AvResult<()> {
        Self::validate_rational_is_set_or_zero("stereo3d horizontal disparity adjustment", value)?;
        if value.den() != 0 && i64::from(value.num()).abs() > i64::from(value.den()).abs() {
            return Err(AvError::invalid_data(format!(
                "stereo3d horizontal disparity adjustment {value} is outside -1..=1"
            )));
        }

        Ok(())
    }

    fn validate_horizontal_field_of_view(value: Rational) -> AvResult<()> {
        Self::validate_rational_is_set_or_zero("stereo3d horizontal field of view", value)?;
        if value.den() != 0
            && value.num() != 0
            && (value.num().is_positive() != value.den().is_positive())
        {
            return Err(AvError::invalid_data(format!(
                "stereo3d horizontal field of view {value} must be nonnegative"
            )));
        }

        Ok(())
    }

    fn validate_rational_is_set_or_zero(name: &str, value: Rational) -> AvResult<()> {
        if value.den() == 0 && value.num() != 0 {
            return Err(AvError::invalid_data(format!(
                "{name} has numerator {} with zero denominator",
                value.num()
            )));
        }

        Ok(())
    }

    fn read_i32(data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        i32::from_ne_bytes(raw)
    }

    fn read_u32(data: &[u8], offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        u32::from_ne_bytes(raw)
    }

    fn read_rational(data: &[u8], offset: usize) -> Rational {
        Rational::from_raw(
            Self::read_i32(data, offset),
            Self::read_i32(data, offset + 4),
        )
    }

    fn write_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_rational(data: &mut [u8], offset: usize, value: Rational) {
        Self::write_i32(data, offset, value.num());
        Self::write_i32(data, offset + 4, value.den());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameAmbientViewingEnvironment {
    ambient_illuminance: Rational,
    ambient_light_x: Rational,
    ambient_light_y: Rational,
}

impl FrameAmbientViewingEnvironment {
    pub const RATIONAL_LEN: usize = 8;
    pub const AMBIENT_ILLUMINANCE_OFFSET: usize = 0;
    pub const AMBIENT_LIGHT_X_OFFSET: usize = Self::AMBIENT_ILLUMINANCE_OFFSET + Self::RATIONAL_LEN;
    pub const AMBIENT_LIGHT_Y_OFFSET: usize = Self::AMBIENT_LIGHT_X_OFFSET + Self::RATIONAL_LEN;
    pub const DATA_LEN: usize = Self::AMBIENT_LIGHT_Y_OFFSET + Self::RATIONAL_LEN;

    pub fn new(
        ambient_illuminance: Rational,
        ambient_light_x: Rational,
        ambient_light_y: Rational,
    ) -> AvResult<Self> {
        Self::validate_nonnegative_rational("ambient illuminance", ambient_illuminance)?;
        Self::validate_unit_interval_rational("ambient light x", ambient_light_x)?;
        Self::validate_unit_interval_rational("ambient light y", ambient_light_y)?;

        Ok(Self {
            ambient_illuminance,
            ambient_light_x,
            ambient_light_y,
        })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "ambient viewing environment frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        Self::new(
            Self::read_rational(data, Self::AMBIENT_ILLUMINANCE_OFFSET),
            Self::read_rational(data, Self::AMBIENT_LIGHT_X_OFFSET),
            Self::read_rational(data, Self::AMBIENT_LIGHT_Y_OFFSET),
        )
    }

    pub const fn ambient_illuminance(self) -> Rational {
        self.ambient_illuminance
    }

    pub const fn ambient_light_x(self) -> Rational {
        self.ambient_light_x
    }

    pub const fn ambient_light_y(self) -> Rational {
        self.ambient_light_y
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        Self::write_rational(
            &mut bytes,
            Self::AMBIENT_ILLUMINANCE_OFFSET,
            self.ambient_illuminance,
        );
        Self::write_rational(
            &mut bytes,
            Self::AMBIENT_LIGHT_X_OFFSET,
            self.ambient_light_x,
        );
        Self::write_rational(
            &mut bytes,
            Self::AMBIENT_LIGHT_Y_OFFSET,
            self.ambient_light_y,
        );
        bytes
    }

    fn validate_nonnegative_rational(name: &str, value: Rational) -> AvResult<()> {
        Self::validate_rational_denominator(name, value)?;
        if value.num() != 0 && (value.num().is_positive() != value.den().is_positive()) {
            return Err(AvError::invalid_data(format!(
                "{name} {value} must be nonnegative"
            )));
        }

        Ok(())
    }

    fn validate_unit_interval_rational(name: &str, value: Rational) -> AvResult<()> {
        Self::validate_nonnegative_rational(name, value)?;
        if i64::from(value.num()).abs() > i64::from(value.den()).abs() {
            return Err(AvError::invalid_data(format!(
                "{name} {value} is outside 0..=1"
            )));
        }

        Ok(())
    }

    fn validate_rational_denominator(name: &str, value: Rational) -> AvResult<()> {
        if value.den() == 0 {
            return Err(AvError::invalid_data(format!(
                "{name} rational denominator must not be zero"
            )));
        }

        Ok(())
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

    fn write_rational(data: &mut [u8], offset: usize, value: Rational) {
        Self::write_i32(data, offset, value.num());
        Self::write_i32(data, offset + 4, value.den());
    }

    fn write_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameVideoHintType {
    Constant,
    Changed,
}

impl FrameVideoHintType {
    pub const fn as_raw(self) -> i32 {
        match self {
            Self::Constant => 0,
            Self::Changed => 1,
        }
    }

    pub fn from_raw(raw: i32) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Constant),
            1 => Ok(Self::Changed),
            _ => Err(AvError::invalid_data(format!(
                "unknown video hint type {raw}"
            ))),
        }
    }

    pub const fn ffmpeg_constant(self) -> &'static str {
        match self {
            Self::Constant => "AV_VIDEO_HINT_TYPE_CONSTANT",
            Self::Changed => "AV_VIDEO_HINT_TYPE_CHANGED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameVideoRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl FrameVideoRect {
    pub const DATA_LEN: usize = 16;

    pub fn new(x: u32, y: u32, width: u32, height: u32) -> AvResult<Self> {
        Self::validate_rect(x, y, width, height, AvError::invalid_argument)?;

        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "video hint rectangle requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let x = Self::read_u32(data, 0);
        let y = Self::read_u32(data, 4);
        let width = Self::read_u32(data, 8);
        let height = Self::read_u32(data, 12);
        Self::validate_rect(x, y, width, height, AvError::invalid_data)?;

        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub const fn x(self) -> u32 {
        self.x
    }

    pub const fn y(self) -> u32 {
        self.y
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        bytes[0..4].copy_from_slice(&self.x.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.y.to_ne_bytes());
        bytes[8..12].copy_from_slice(&self.width.to_ne_bytes());
        bytes[12..16].copy_from_slice(&self.height.to_ne_bytes());
        bytes
    }

    fn validate_rect(
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        make_error: impl FnOnce(String) -> AvError,
    ) -> AvResult<()> {
        if width == 0 || height == 0 {
            return Err(make_error(format!(
                "video hint rectangle dimensions must be positive, got {width}x{height}"
            )));
        }
        if x.checked_add(width).is_none() || y.checked_add(height).is_none() {
            return Err(make_error(format!(
                "video hint rectangle {x},{y} {width}x{height} overflows u32 bounds"
            )));
        }

        Ok(())
    }

    fn read_u32(data: &[u8], offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        u32::from_ne_bytes(raw)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameVideoHint {
    hint_type: FrameVideoHintType,
    rects: Vec<FrameVideoRect>,
}

impl FrameVideoHint {
    const SIZE_T_LEN: usize = core::mem::size_of::<usize>();
    const NB_RECTS_OFFSET: usize = 0;
    const RECT_OFFSET_OFFSET: usize = Self::NB_RECTS_OFFSET + Self::SIZE_T_LEN;
    const RECT_SIZE_OFFSET: usize = Self::RECT_OFFSET_OFFSET + Self::SIZE_T_LEN;
    const TYPE_OFFSET: usize = Self::RECT_SIZE_OFFSET + Self::SIZE_T_LEN;
    pub const HEADER_LEN: usize =
        Self::align_up(Self::TYPE_OFFSET + 4, core::mem::align_of::<usize>());

    pub fn new(hint_type: FrameVideoHintType, rects: Vec<FrameVideoRect>) -> AvResult<Self> {
        Self::expected_data_len(rects.len()).map_err(|err| {
            AvError::invalid_argument(format!("video hint payload length overflow: {err}"))
        })?;

        Ok(Self { hint_type, rects })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "video hint frame side data requires at least {} header bytes, got {}",
                Self::HEADER_LEN,
                data.len()
            )));
        }

        let nb_rects = Self::read_usize(data, Self::NB_RECTS_OFFSET);
        let rect_offset = Self::read_usize(data, Self::RECT_OFFSET_OFFSET);
        if rect_offset != Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "video hint rectangle offset {rect_offset} does not match native offset {}",
                Self::HEADER_LEN
            )));
        }

        let rect_size = Self::read_usize(data, Self::RECT_SIZE_OFFSET);
        if rect_size != FrameVideoRect::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "video hint rectangle size {rect_size} does not match native size {}",
                FrameVideoRect::DATA_LEN
            )));
        }

        let expected_len = Self::expected_data_len(nb_rects)?;
        if data.len() != expected_len {
            return Err(AvError::invalid_data(format!(
                "video hint payload requires exactly {expected_len} bytes for {nb_rects} rectangles, got {}",
                data.len()
            )));
        }

        let hint_type = FrameVideoHintType::from_raw(Self::read_i32(data, Self::TYPE_OFFSET))?;
        let mut rects = Vec::with_capacity(nb_rects);
        for index in 0..nb_rects {
            let offset = Self::HEADER_LEN + index * FrameVideoRect::DATA_LEN;
            rects.push(FrameVideoRect::parse(
                &data[offset..offset + FrameVideoRect::DATA_LEN],
            )?);
        }

        Ok(Self { hint_type, rects })
    }

    pub const fn hint_type(&self) -> FrameVideoHintType {
        self.hint_type
    }

    pub fn rects(&self) -> &[FrameVideoRect] {
        &self.rects
    }

    pub fn into_rects(self) -> Vec<FrameVideoRect> {
        self.rects
    }

    pub fn nb_rects(&self) -> usize {
        self.rects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    pub fn rect(&self, index: usize) -> Option<FrameVideoRect> {
        self.rects.get(index).copied()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0; Self::HEADER_LEN];
        Self::write_usize(&mut bytes, Self::NB_RECTS_OFFSET, self.rects.len());
        Self::write_usize(&mut bytes, Self::RECT_OFFSET_OFFSET, Self::HEADER_LEN);
        Self::write_usize(&mut bytes, Self::RECT_SIZE_OFFSET, FrameVideoRect::DATA_LEN);
        Self::write_i32(&mut bytes, Self::TYPE_OFFSET, self.hint_type.as_raw());
        for rect in &self.rects {
            bytes.extend_from_slice(&rect.to_bytes());
        }
        bytes
    }

    fn expected_data_len(nb_rects: usize) -> AvResult<usize> {
        let rects_len = nb_rects
            .checked_mul(FrameVideoRect::DATA_LEN)
            .ok_or_else(|| AvError::invalid_data("video hint rectangle data length overflow"))?;
        Self::HEADER_LEN
            .checked_add(rects_len)
            .ok_or_else(|| AvError::invalid_data("video hint payload length overflow"))
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

    fn write_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_usize(data: &mut [u8], offset: usize, value: usize) {
        data[offset..offset + Self::SIZE_T_LEN].copy_from_slice(&value.to_ne_bytes());
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
pub struct FrameLcevc<'a> {
    data: &'a [u8],
}

impl<'a> FrameLcevc<'a> {
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
pub struct FrameViewId {
    value: i32,
}

impl FrameViewId {
    pub const DATA_LEN: usize = 4;

    pub const fn new(value: i32) -> Self {
        Self { value }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "view ID frame side data requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let mut raw = [0; Self::DATA_LEN];
        raw.copy_from_slice(data);
        Ok(Self::new(i32::from_ne_bytes(raw)))
    }

    pub const fn as_raw(self) -> i32 {
        self.value
    }

    pub const fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        self.value.to_ne_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameThreeDReferenceDisplay {
    left_view_id: u16,
    right_view_id: u16,
    exponent_ref_display_width: u8,
    mantissa_ref_display_width: u8,
    exponent_ref_viewing_distance: u8,
    mantissa_ref_viewing_distance: u8,
    additional_shift_present: bool,
    num_sample_shift: i16,
}

impl FrameThreeDReferenceDisplay {
    pub const DATA_LEN: usize = 12;
    const LEFT_VIEW_ID_OFFSET: usize = 0;
    const RIGHT_VIEW_ID_OFFSET: usize = 2;
    const EXPONENT_REF_DISPLAY_WIDTH_OFFSET: usize = 4;
    const MANTISSA_REF_DISPLAY_WIDTH_OFFSET: usize = 5;
    const EXPONENT_REF_VIEWING_DISTANCE_OFFSET: usize = 6;
    const MANTISSA_REF_VIEWING_DISTANCE_OFFSET: usize = 7;
    const ADDITIONAL_SHIFT_PRESENT_OFFSET: usize = 8;
    const NUM_SAMPLE_SHIFT_OFFSET: usize = 10;

    pub const fn new(
        left_view_id: u16,
        right_view_id: u16,
        ref_display_width: (u8, u8),
        ref_viewing_distance: (u8, u8),
        additional_shift_present: bool,
        num_sample_shift: i16,
    ) -> Self {
        Self {
            left_view_id,
            right_view_id,
            exponent_ref_display_width: ref_display_width.0,
            mantissa_ref_display_width: ref_display_width.1,
            exponent_ref_viewing_distance: ref_viewing_distance.0,
            mantissa_ref_viewing_distance: ref_viewing_distance.1,
            additional_shift_present,
            num_sample_shift,
        }
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() != Self::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "3D reference display entry requires exactly {} bytes, got {}",
                Self::DATA_LEN,
                data.len()
            )));
        }

        let additional_shift_present = match data[Self::ADDITIONAL_SHIFT_PRESENT_OFFSET] {
            0 => false,
            1 => true,
            value => {
                return Err(AvError::invalid_data(format!(
                    "3D reference display additional shift flag must be 0 or 1, got {value}"
                )));
            }
        };

        Ok(Self::new(
            Self::read_u16(data, Self::LEFT_VIEW_ID_OFFSET),
            Self::read_u16(data, Self::RIGHT_VIEW_ID_OFFSET),
            (
                data[Self::EXPONENT_REF_DISPLAY_WIDTH_OFFSET],
                data[Self::MANTISSA_REF_DISPLAY_WIDTH_OFFSET],
            ),
            (
                data[Self::EXPONENT_REF_VIEWING_DISTANCE_OFFSET],
                data[Self::MANTISSA_REF_VIEWING_DISTANCE_OFFSET],
            ),
            additional_shift_present,
            Self::read_i16(data, Self::NUM_SAMPLE_SHIFT_OFFSET),
        ))
    }

    pub const fn left_view_id(self) -> u16 {
        self.left_view_id
    }

    pub const fn right_view_id(self) -> u16 {
        self.right_view_id
    }

    pub const fn exponent_ref_display_width(self) -> u8 {
        self.exponent_ref_display_width
    }

    pub const fn mantissa_ref_display_width(self) -> u8 {
        self.mantissa_ref_display_width
    }

    pub const fn exponent_ref_viewing_distance(self) -> u8 {
        self.exponent_ref_viewing_distance
    }

    pub const fn mantissa_ref_viewing_distance(self) -> u8 {
        self.mantissa_ref_viewing_distance
    }

    pub const fn additional_shift_present(self) -> bool {
        self.additional_shift_present
    }

    pub const fn num_sample_shift(self) -> i16 {
        self.num_sample_shift
    }

    pub fn to_bytes(self) -> [u8; Self::DATA_LEN] {
        let mut bytes = [0; Self::DATA_LEN];
        Self::write_u16(&mut bytes, Self::LEFT_VIEW_ID_OFFSET, self.left_view_id);
        Self::write_u16(&mut bytes, Self::RIGHT_VIEW_ID_OFFSET, self.right_view_id);
        bytes[Self::EXPONENT_REF_DISPLAY_WIDTH_OFFSET] = self.exponent_ref_display_width;
        bytes[Self::MANTISSA_REF_DISPLAY_WIDTH_OFFSET] = self.mantissa_ref_display_width;
        bytes[Self::EXPONENT_REF_VIEWING_DISTANCE_OFFSET] = self.exponent_ref_viewing_distance;
        bytes[Self::MANTISSA_REF_VIEWING_DISTANCE_OFFSET] = self.mantissa_ref_viewing_distance;
        bytes[Self::ADDITIONAL_SHIFT_PRESENT_OFFSET] = u8::from(self.additional_shift_present);
        Self::write_i16(
            &mut bytes,
            Self::NUM_SAMPLE_SHIFT_OFFSET,
            self.num_sample_shift,
        );
        bytes
    }

    fn read_u16(data: &[u8], offset: usize) -> u16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&data[offset..offset + 2]);
        u16::from_ne_bytes(raw)
    }

    fn read_i16(data: &[u8], offset: usize) -> i16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&data[offset..offset + 2]);
        i16::from_ne_bytes(raw)
    }

    fn write_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
    }

    fn write_i16(data: &mut [u8], offset: usize, value: i16) {
        data[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameThreeDReferenceDisplays {
    prec_ref_display_width: u8,
    ref_viewing_distance_flag: bool,
    prec_ref_viewing_dist: u8,
    displays: Vec<FrameThreeDReferenceDisplay>,
}

impl FrameThreeDReferenceDisplays {
    pub const MAX_REF_DISPLAYS: usize = 32;
    const SIZE_T_LEN: usize = core::mem::size_of::<usize>();
    const SIZE_T_OFFSET: usize = Self::align_up(4, core::mem::align_of::<usize>());
    pub const ENTRIES_OFFSET_OFFSET: usize = Self::SIZE_T_OFFSET;
    pub const ENTRY_SIZE_OFFSET: usize = Self::ENTRIES_OFFSET_OFFSET + Self::SIZE_T_LEN;
    pub const HEADER_LEN: usize = Self::ENTRY_SIZE_OFFSET + Self::SIZE_T_LEN;
    pub const ENTRIES_OFFSET: usize =
        Self::align_up(Self::HEADER_LEN, core::mem::align_of::<u16>());

    pub fn new(
        prec_ref_display_width: u8,
        ref_viewing_distance_flag: bool,
        prec_ref_viewing_dist: u8,
        displays: Vec<FrameThreeDReferenceDisplay>,
    ) -> AvResult<Self> {
        Self::validate_precision("reference display width", prec_ref_display_width)?;
        Self::validate_precision("reference viewing distance", prec_ref_viewing_dist)?;
        Self::validate_display_count(displays.len())?;
        Self::expected_data_len(displays.len()).map_err(|err| {
            AvError::invalid_argument(format!(
                "3D reference displays payload length overflow: {err}"
            ))
        })?;

        Ok(Self {
            prec_ref_display_width,
            ref_viewing_distance_flag,
            prec_ref_viewing_dist,
            displays,
        })
    }

    pub fn parse(data: &[u8]) -> AvResult<Self> {
        if data.len() < Self::HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "3D reference displays side data requires at least {} header bytes, got {}",
                Self::HEADER_LEN,
                data.len()
            )));
        }

        let prec_ref_display_width = data[0];
        Self::validate_precision("reference display width", prec_ref_display_width)?;
        let ref_viewing_distance_flag = match data[1] {
            0 => false,
            1 => true,
            value => {
                return Err(AvError::invalid_data(format!(
                    "3D reference displays viewing-distance flag must be 0 or 1, got {value}"
                )));
            }
        };
        let prec_ref_viewing_dist = data[2];
        Self::validate_precision("reference viewing distance", prec_ref_viewing_dist)?;

        let num_ref_displays = data[3] as usize;
        Self::validate_display_count(num_ref_displays)?;

        let entries_offset = Self::read_usize(data, Self::ENTRIES_OFFSET_OFFSET);
        if entries_offset != Self::ENTRIES_OFFSET {
            return Err(AvError::invalid_data(format!(
                "3D reference displays entry offset {entries_offset} does not match native offset {}",
                Self::ENTRIES_OFFSET
            )));
        }

        let entry_size = Self::read_usize(data, Self::ENTRY_SIZE_OFFSET);
        if entry_size != FrameThreeDReferenceDisplay::DATA_LEN {
            return Err(AvError::invalid_data(format!(
                "3D reference display entry size {entry_size} does not match native size {}",
                FrameThreeDReferenceDisplay::DATA_LEN
            )));
        }

        let expected_len = Self::expected_data_len(num_ref_displays)?;
        if data.len() != expected_len {
            return Err(AvError::invalid_data(format!(
                "3D reference displays payload requires exactly {expected_len} bytes for {num_ref_displays} displays, got {}",
                data.len()
            )));
        }

        let mut displays = Vec::with_capacity(num_ref_displays);
        for index in 0..num_ref_displays {
            let offset = Self::ENTRIES_OFFSET + index * FrameThreeDReferenceDisplay::DATA_LEN;
            displays.push(FrameThreeDReferenceDisplay::parse(
                &data[offset..offset + FrameThreeDReferenceDisplay::DATA_LEN],
            )?);
        }

        Ok(Self {
            prec_ref_display_width,
            ref_viewing_distance_flag,
            prec_ref_viewing_dist,
            displays,
        })
    }

    pub const fn prec_ref_display_width(&self) -> u8 {
        self.prec_ref_display_width
    }

    pub const fn ref_viewing_distance_flag(&self) -> bool {
        self.ref_viewing_distance_flag
    }

    pub const fn prec_ref_viewing_dist(&self) -> u8 {
        self.prec_ref_viewing_dist
    }

    pub fn displays(&self) -> &[FrameThreeDReferenceDisplay] {
        &self.displays
    }

    pub fn nb_displays(&self) -> usize {
        self.displays.len()
    }

    pub fn display(&self, index: usize) -> Option<FrameThreeDReferenceDisplay> {
        self.displays.get(index).copied()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0; Self::ENTRIES_OFFSET];
        bytes[0] = self.prec_ref_display_width;
        bytes[1] = u8::from(self.ref_viewing_distance_flag);
        bytes[2] = self.prec_ref_viewing_dist;
        bytes[3] = self.displays.len() as u8;
        Self::write_usize(
            &mut bytes,
            Self::ENTRIES_OFFSET_OFFSET,
            Self::ENTRIES_OFFSET,
        );
        Self::write_usize(
            &mut bytes,
            Self::ENTRY_SIZE_OFFSET,
            FrameThreeDReferenceDisplay::DATA_LEN,
        );
        for display in &self.displays {
            bytes.extend_from_slice(&display.to_bytes());
        }
        bytes
    }

    fn expected_data_len(nb_displays: usize) -> AvResult<usize> {
        let displays_len = nb_displays
            .checked_mul(FrameThreeDReferenceDisplay::DATA_LEN)
            .ok_or_else(|| AvError::invalid_data("3D reference display data length overflow"))?;
        Self::ENTRIES_OFFSET
            .checked_add(displays_len)
            .ok_or_else(|| AvError::invalid_data("3D reference displays payload length overflow"))
    }

    fn validate_display_count(nb_displays: usize) -> AvResult<()> {
        if !(1..=Self::MAX_REF_DISPLAYS).contains(&nb_displays) {
            return Err(AvError::invalid_data(format!(
                "3D reference displays count must be 1..={}, got {nb_displays}",
                Self::MAX_REF_DISPLAYS
            )));
        }

        Ok(())
    }

    fn validate_precision(label: &str, value: u8) -> AvResult<()> {
        if value > 31 {
            return Err(AvError::invalid_data(format!(
                "3D reference displays {label} precision must be 0..=31, got {value}"
            )));
        }

        Ok(())
    }

    const fn align_up(value: usize, align: usize) -> usize {
        let remainder = value % align;
        if remainder == 0 {
            value
        } else {
            value + align - remainder
        }
    }

    fn read_usize(data: &[u8], offset: usize) -> usize {
        let mut raw = [0; Self::SIZE_T_LEN];
        raw.copy_from_slice(&data[offset..offset + Self::SIZE_T_LEN]);
        usize::from_ne_bytes(raw)
    }

    fn write_usize(data: &mut [u8], offset: usize, value: usize) {
        data[offset..offset + Self::SIZE_T_LEN].copy_from_slice(&value.to_ne_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifEndian {
    Little,
    Big,
}

impl FrameExifEndian {
    fn read_u16(self, data: &[u8], offset: usize) -> u16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&data[offset..offset + 2]);
        match self {
            Self::Little => u16::from_le_bytes(raw),
            Self::Big => u16::from_be_bytes(raw),
        }
    }

    fn read_i16(self, data: &[u8], offset: usize) -> i16 {
        let mut raw = [0; 2];
        raw.copy_from_slice(&data[offset..offset + 2]);
        match self {
            Self::Little => i16::from_le_bytes(raw),
            Self::Big => i16::from_be_bytes(raw),
        }
    }

    fn read_u32(self, data: &[u8], offset: usize) -> u32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        match self {
            Self::Little => u32::from_le_bytes(raw),
            Self::Big => u32::from_be_bytes(raw),
        }
    }

    fn read_i32(self, data: &[u8], offset: usize) -> i32 {
        let mut raw = [0; 4];
        raw.copy_from_slice(&data[offset..offset + 4]);
        match self {
            Self::Little => i32::from_le_bytes(raw),
            Self::Big => i32::from_be_bytes(raw),
        }
    }

    fn read_u64(self, data: &[u8], offset: usize) -> u64 {
        let mut raw = [0; 8];
        raw.copy_from_slice(&data[offset..offset + 8]);
        match self {
            Self::Little => u64::from_le_bytes(raw),
            Self::Big => u64::from_be_bytes(raw),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifTiffType {
    Byte,
    Ascii,
    Short,
    Long,
    Rational,
    SignedByte,
    Undefined,
    SignedShort,
    SignedLong,
    SignedRational,
    Float,
    Double,
    Ifd,
}

impl FrameExifTiffType {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::Byte),
            2 => Ok(Self::Ascii),
            3 => Ok(Self::Short),
            4 => Ok(Self::Long),
            5 => Ok(Self::Rational),
            6 => Ok(Self::SignedByte),
            7 => Ok(Self::Undefined),
            8 => Ok(Self::SignedShort),
            9 => Ok(Self::SignedLong),
            10 => Ok(Self::SignedRational),
            11 => Ok(Self::Float),
            12 => Ok(Self::Double),
            13 => Ok(Self::Ifd),
            _ => Err(AvError::invalid_data(format!(
                "EXIF TIFF entry type {raw} is not supported"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Byte => 1,
            Self::Ascii => 2,
            Self::Short => 3,
            Self::Long => 4,
            Self::Rational => 5,
            Self::SignedByte => 6,
            Self::Undefined => 7,
            Self::SignedShort => 8,
            Self::SignedLong => 9,
            Self::SignedRational => 10,
            Self::Float => 11,
            Self::Double => 12,
            Self::Ifd => 13,
        }
    }

    pub const fn element_size(self) -> usize {
        match self {
            Self::Byte | Self::Ascii | Self::SignedByte | Self::Undefined => 1,
            Self::Short | Self::SignedShort => 2,
            Self::Long | Self::SignedLong | Self::Float | Self::Ifd => 4,
            Self::Rational | Self::SignedRational | Self::Double => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifIfdPointerKind {
    Exif,
    Gps,
    Interoperability,
}

impl FrameExifIfdPointerKind {
    pub const EXIF_TAG: u16 = 0x8769;
    pub const GPS_TAG: u16 = 0x8825;
    pub const INTEROPERABILITY_TAG: u16 = 0xA005;

    pub const fn from_tag(tag: u16) -> Option<Self> {
        match tag {
            Self::EXIF_TAG => Some(Self::Exif),
            Self::GPS_TAG => Some(Self::Gps),
            Self::INTEROPERABILITY_TAG => Some(Self::Interoperability),
            _ => None,
        }
    }

    pub const fn tag(self) -> u16 {
        match self {
            Self::Exif => Self::EXIF_TAG,
            Self::Gps => Self::GPS_TAG,
            Self::Interoperability => Self::INTEROPERABILITY_TAG,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifRational {
    numerator: u32,
    denominator: u32,
}

impl FrameExifRational {
    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    pub const fn denominator(self) -> u32 {
        self.denominator
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifSignedRational {
    numerator: i32,
    denominator: i32,
}

impl FrameExifSignedRational {
    pub const fn numerator(self) -> i32 {
        self.numerator
    }

    pub const fn denominator(self) -> i32 {
        self.denominator
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifCompression {
    raw: u16,
}

impl FrameExifCompression {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        if raw == 0 {
            return Err(AvError::invalid_data(
                "EXIF compression value 0 is outside the defined TIFF compression set",
            ));
        }
        Ok(Self { raw })
    }

    pub const fn raw(self) -> u16 {
        self.raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifPhotometricInterpretation {
    raw: u16,
}

impl FrameExifPhotometricInterpretation {
    pub const fn from_raw(raw: u16) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u16 {
        self.raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifPredictor {
    raw: u16,
}

impl FrameExifPredictor {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        if raw == 0 {
            return Err(AvError::invalid_data(
                "EXIF predictor value must be non-zero",
            ));
        }
        Ok(Self { raw })
    }

    pub const fn raw(self) -> u16 {
        self.raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifThresholding {
    NoDitheringOrHalftoning,
    OrderedDitherOrHalftone,
    RandomizedProcess,
}

impl FrameExifThresholding {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::NoDitheringOrHalftoning),
            2 => Ok(Self::OrderedDitherOrHalftone),
            3 => Ok(Self::RandomizedProcess),
            _ => Err(AvError::invalid_data(format!(
                "EXIF thresholding value {raw} is outside the defined 1..=3 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::NoDitheringOrHalftoning => 1,
            Self::OrderedDitherOrHalftone => 2,
            Self::RandomizedProcess => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifBitsPerSample<'a> {
    entry: FrameExifEntry<'a>,
}

impl<'a> FrameExifBitsPerSample<'a> {
    pub const fn raw_entry(self) -> FrameExifEntry<'a> {
        self.entry
    }

    pub const fn count(self) -> u32 {
        self.entry.count()
    }

    pub fn values(self) -> AvResult<Vec<u16>> {
        self.entry.short_values()?.ok_or_else(|| {
            AvError::invalid_data("validated EXIF BitsPerSample tag lost SHORT type")
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifFillOrder {
    MostSignificantBitFirst,
    LeastSignificantBitFirst,
}

impl FrameExifFillOrder {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::MostSignificantBitFirst),
            2 => Ok(Self::LeastSignificantBitFirst),
            _ => Err(AvError::invalid_data(format!(
                "EXIF fill order value {raw} is outside the defined 1..=2 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::MostSignificantBitFirst => 1,
            Self::LeastSignificantBitFirst => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifOrientation {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    LeftTop,
    RightTop,
    RightBottom,
    LeftBottom,
}

impl FrameExifOrientation {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::TopLeft),
            2 => Ok(Self::TopRight),
            3 => Ok(Self::BottomRight),
            4 => Ok(Self::BottomLeft),
            5 => Ok(Self::LeftTop),
            6 => Ok(Self::RightTop),
            7 => Ok(Self::RightBottom),
            8 => Ok(Self::LeftBottom),
            _ => Err(AvError::invalid_data(format!(
                "EXIF orientation value {raw} is outside the defined 1..=8 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::TopLeft => 1,
            Self::TopRight => 2,
            Self::BottomRight => 3,
            Self::BottomLeft => 4,
            Self::LeftTop => 5,
            Self::RightTop => 6,
            Self::RightBottom => 7,
            Self::LeftBottom => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifResolutionUnit {
    Unitless,
    Inch,
    Centimeter,
}

impl FrameExifResolutionUnit {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::Unitless),
            2 => Ok(Self::Inch),
            3 => Ok(Self::Centimeter),
            _ => Err(AvError::invalid_data(format!(
                "EXIF resolution unit value {raw} is outside the defined 1..=3 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Unitless => 1,
            Self::Inch => 2,
            Self::Centimeter => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifPlanarConfiguration {
    Chunky,
    Planar,
}

impl FrameExifPlanarConfiguration {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::Chunky),
            2 => Ok(Self::Planar),
            _ => Err(AvError::invalid_data(format!(
                "EXIF planar configuration value {raw} is outside the defined 1..=2 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Chunky => 1,
            Self::Planar => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifYcbCrPositioning {
    Centered,
    CoSited,
}

impl FrameExifYcbCrPositioning {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::Centered),
            2 => Ok(Self::CoSited),
            _ => Err(AvError::invalid_data(format!(
                "EXIF YCbCr positioning value {raw} is outside the defined 1..=2 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Centered => 1,
            Self::CoSited => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifExposureProgram {
    NotDefined,
    Manual,
    NormalProgram,
    AperturePriority,
    ShutterPriority,
    CreativeProgram,
    ActionProgram,
    PortraitMode,
    LandscapeMode,
}

impl FrameExifExposureProgram {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::NotDefined),
            1 => Ok(Self::Manual),
            2 => Ok(Self::NormalProgram),
            3 => Ok(Self::AperturePriority),
            4 => Ok(Self::ShutterPriority),
            5 => Ok(Self::CreativeProgram),
            6 => Ok(Self::ActionProgram),
            7 => Ok(Self::PortraitMode),
            8 => Ok(Self::LandscapeMode),
            _ => Err(AvError::invalid_data(format!(
                "EXIF exposure program value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::NotDefined => 0,
            Self::Manual => 1,
            Self::NormalProgram => 2,
            Self::AperturePriority => 3,
            Self::ShutterPriority => 4,
            Self::CreativeProgram => 5,
            Self::ActionProgram => 6,
            Self::PortraitMode => 7,
            Self::LandscapeMode => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSensitivityType {
    Unknown,
    StandardOutputSensitivity,
    RecommendedExposureIndex,
    IsoSpeed,
    StandardOutputSensitivityAndRecommendedExposureIndex,
    StandardOutputSensitivityAndIsoSpeed,
    RecommendedExposureIndexAndIsoSpeed,
    StandardOutputSensitivityAndRecommendedExposureIndexAndIsoSpeed,
}

impl FrameExifSensitivityType {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::StandardOutputSensitivity),
            2 => Ok(Self::RecommendedExposureIndex),
            3 => Ok(Self::IsoSpeed),
            4 => Ok(Self::StandardOutputSensitivityAndRecommendedExposureIndex),
            5 => Ok(Self::StandardOutputSensitivityAndIsoSpeed),
            6 => Ok(Self::RecommendedExposureIndexAndIsoSpeed),
            7 => Ok(Self::StandardOutputSensitivityAndRecommendedExposureIndexAndIsoSpeed),
            _ => Err(AvError::invalid_data(format!(
                "EXIF sensitivity type value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Unknown => 0,
            Self::StandardOutputSensitivity => 1,
            Self::RecommendedExposureIndex => 2,
            Self::IsoSpeed => 3,
            Self::StandardOutputSensitivityAndRecommendedExposureIndex => 4,
            Self::StandardOutputSensitivityAndIsoSpeed => 5,
            Self::RecommendedExposureIndexAndIsoSpeed => 6,
            Self::StandardOutputSensitivityAndRecommendedExposureIndexAndIsoSpeed => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifMeteringMode {
    Unknown,
    Average,
    CenterWeightedAverage,
    Spot,
    MultiSpot,
    Pattern,
    Partial,
    Other,
}

impl FrameExifMeteringMode {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Average),
            2 => Ok(Self::CenterWeightedAverage),
            3 => Ok(Self::Spot),
            4 => Ok(Self::MultiSpot),
            5 => Ok(Self::Pattern),
            6 => Ok(Self::Partial),
            255 => Ok(Self::Other),
            _ => Err(AvError::invalid_data(format!(
                "EXIF metering mode value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Unknown => 0,
            Self::Average => 1,
            Self::CenterWeightedAverage => 2,
            Self::Spot => 3,
            Self::MultiSpot => 4,
            Self::Pattern => 5,
            Self::Partial => 6,
            Self::Other => 255,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifLightSource {
    Unknown,
    Daylight,
    Fluorescent,
    Tungsten,
    Flash,
    FineWeather,
    CloudyWeather,
    Shade,
    DaylightFluorescent,
    DayWhiteFluorescent,
    CoolWhiteFluorescent,
    WhiteFluorescent,
    StandardLightA,
    StandardLightB,
    StandardLightC,
    D55,
    D65,
    D75,
    D50,
    IsoStudioTungsten,
    Other,
}

impl FrameExifLightSource {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Daylight),
            2 => Ok(Self::Fluorescent),
            3 => Ok(Self::Tungsten),
            4 => Ok(Self::Flash),
            9 => Ok(Self::FineWeather),
            10 => Ok(Self::CloudyWeather),
            11 => Ok(Self::Shade),
            12 => Ok(Self::DaylightFluorescent),
            13 => Ok(Self::DayWhiteFluorescent),
            14 => Ok(Self::CoolWhiteFluorescent),
            15 => Ok(Self::WhiteFluorescent),
            17 => Ok(Self::StandardLightA),
            18 => Ok(Self::StandardLightB),
            19 => Ok(Self::StandardLightC),
            20 => Ok(Self::D55),
            21 => Ok(Self::D65),
            22 => Ok(Self::D75),
            23 => Ok(Self::D50),
            24 => Ok(Self::IsoStudioTungsten),
            255 => Ok(Self::Other),
            _ => Err(AvError::invalid_data(format!(
                "EXIF light source value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Unknown => 0,
            Self::Daylight => 1,
            Self::Fluorescent => 2,
            Self::Tungsten => 3,
            Self::Flash => 4,
            Self::FineWeather => 9,
            Self::CloudyWeather => 10,
            Self::Shade => 11,
            Self::DaylightFluorescent => 12,
            Self::DayWhiteFluorescent => 13,
            Self::CoolWhiteFluorescent => 14,
            Self::WhiteFluorescent => 15,
            Self::StandardLightA => 17,
            Self::StandardLightB => 18,
            Self::StandardLightC => 19,
            Self::D55 => 20,
            Self::D65 => 21,
            Self::D75 => 22,
            Self::D50 => 23,
            Self::IsoStudioTungsten => 24,
            Self::Other => 255,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifFlash {
    raw: u16,
}

impl FrameExifFlash {
    pub const fn from_raw(raw: u16) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u16 {
        self.raw
    }

    pub const fn fired(self) -> bool {
        self.raw & 0x0001 != 0
    }

    pub const fn return_status_bits(self) -> u16 {
        (self.raw >> 1) & 0x0003
    }

    pub const fn mode_bits(self) -> u16 {
        (self.raw >> 3) & 0x0003
    }

    pub const fn has_no_flash_function(self) -> bool {
        self.raw & 0x0020 != 0
    }

    pub const fn red_eye_reduction_supported(self) -> bool {
        self.raw & 0x0040 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifWhiteBalance {
    Auto,
    Manual,
}

impl FrameExifWhiteBalance {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Auto),
            1 => Ok(Self::Manual),
            _ => Err(AvError::invalid_data(format!(
                "EXIF white balance value {raw} is outside the defined 0..=1 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Auto => 0,
            Self::Manual => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifColorSpace {
    Srgb,
    Uncalibrated,
}

impl FrameExifColorSpace {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::Srgb),
            0xFFFF => Ok(Self::Uncalibrated),
            _ => Err(AvError::invalid_data(format!(
                "EXIF color space value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Srgb => 1,
            Self::Uncalibrated => 0xFFFF,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSensingMethod {
    NotDefined,
    OneChipColorArea,
    TwoChipColorArea,
    ThreeChipColorArea,
    ColorSequentialArea,
    Trilinear,
    ColorSequentialLinear,
}

impl FrameExifSensingMethod {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::NotDefined),
            2 => Ok(Self::OneChipColorArea),
            3 => Ok(Self::TwoChipColorArea),
            4 => Ok(Self::ThreeChipColorArea),
            5 => Ok(Self::ColorSequentialArea),
            7 => Ok(Self::Trilinear),
            8 => Ok(Self::ColorSequentialLinear),
            _ => Err(AvError::invalid_data(format!(
                "EXIF sensing method value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::NotDefined => 1,
            Self::OneChipColorArea => 2,
            Self::TwoChipColorArea => 3,
            Self::ThreeChipColorArea => 4,
            Self::ColorSequentialArea => 5,
            Self::Trilinear => 7,
            Self::ColorSequentialLinear => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifFileSource {
    DigitalStillCamera,
}

impl FrameExifFileSource {
    pub fn from_raw(raw: u8) -> AvResult<Self> {
        match raw {
            3 => Ok(Self::DigitalStillCamera),
            _ => Err(AvError::invalid_data(format!(
                "EXIF file source value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u8 {
        match self {
            Self::DigitalStillCamera => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSceneType {
    DirectlyPhotographed,
}

impl FrameExifSceneType {
    pub fn from_raw(raw: u8) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::DirectlyPhotographed),
            _ => Err(AvError::invalid_data(format!(
                "EXIF scene type value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u8 {
        match self {
            Self::DirectlyPhotographed => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifCustomRendered {
    Normal,
    Custom,
}

impl FrameExifCustomRendered {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Custom),
            _ => Err(AvError::invalid_data(format!(
                "EXIF custom rendered value {raw} is outside the defined 0..=1 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Normal => 0,
            Self::Custom => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifExposureMode {
    Auto,
    Manual,
    AutoBracket,
}

impl FrameExifExposureMode {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Auto),
            1 => Ok(Self::Manual),
            2 => Ok(Self::AutoBracket),
            _ => Err(AvError::invalid_data(format!(
                "EXIF exposure mode value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Auto => 0,
            Self::Manual => 1,
            Self::AutoBracket => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSceneCaptureType {
    Standard,
    Landscape,
    Portrait,
    NightScene,
}

impl FrameExifSceneCaptureType {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Standard),
            1 => Ok(Self::Landscape),
            2 => Ok(Self::Portrait),
            3 => Ok(Self::NightScene),
            _ => Err(AvError::invalid_data(format!(
                "EXIF scene capture type value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Standard => 0,
            Self::Landscape => 1,
            Self::Portrait => 2,
            Self::NightScene => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGainControl {
    None,
    LowGainUp,
    HighGainUp,
    LowGainDown,
    HighGainDown,
}

impl FrameExifGainControl {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::None),
            1 => Ok(Self::LowGainUp),
            2 => Ok(Self::HighGainUp),
            3 => Ok(Self::LowGainDown),
            4 => Ok(Self::HighGainDown),
            _ => Err(AvError::invalid_data(format!(
                "EXIF gain control value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::None => 0,
            Self::LowGainUp => 1,
            Self::HighGainUp => 2,
            Self::LowGainDown => 3,
            Self::HighGainDown => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSharpness {
    Normal,
    Soft,
    Hard,
}

impl FrameExifSharpness {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Soft),
            2 => Ok(Self::Hard),
            _ => Err(AvError::invalid_data(format!(
                "EXIF sharpness value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Normal => 0,
            Self::Soft => 1,
            Self::Hard => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSaturation {
    Normal,
    Low,
    High,
}

impl FrameExifSaturation {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Low),
            2 => Ok(Self::High),
            _ => Err(AvError::invalid_data(format!(
                "EXIF saturation value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Normal => 0,
            Self::Low => 1,
            Self::High => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifContrast {
    Normal,
    Soft,
    Hard,
}

impl FrameExifContrast {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Soft),
            2 => Ok(Self::Hard),
            _ => Err(AvError::invalid_data(format!(
                "EXIF contrast value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Normal => 0,
            Self::Soft => 1,
            Self::Hard => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSubjectDistanceRange {
    Unknown,
    Macro,
    CloseView,
    DistantView,
}

impl FrameExifSubjectDistanceRange {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Macro),
            2 => Ok(Self::CloseView),
            3 => Ok(Self::DistantView),
            _ => Err(AvError::invalid_data(format!(
                "EXIF subject distance range value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Unknown => 0,
            Self::Macro => 1,
            Self::CloseView => 2,
            Self::DistantView => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifCompositeImage {
    Unknown,
    NonCompositeImage,
    GeneralCompositeImage,
    CompositeImageCapturedWhenShooting,
}

impl FrameExifCompositeImage {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::NonCompositeImage),
            2 => Ok(Self::GeneralCompositeImage),
            3 => Ok(Self::CompositeImageCapturedWhenShooting),
            _ => Err(AvError::invalid_data(format!(
                "EXIF composite image value {raw} is outside the defined set"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::Unknown => 0,
            Self::NonCompositeImage => 1,
            Self::GeneralCompositeImage => 2,
            Self::CompositeImageCapturedWhenShooting => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSubjectArea {
    Point {
        x: u16,
        y: u16,
    },
    Circle {
        x: u16,
        y: u16,
        diameter: u16,
    },
    Rectangle {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
}

impl FrameExifSubjectArea {
    pub const fn point(x: u16, y: u16) -> Self {
        Self::Point { x, y }
    }

    pub const fn circle(x: u16, y: u16, diameter: u16) -> Self {
        Self::Circle { x, y, diameter }
    }

    pub const fn rectangle(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self::Rectangle {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsLatitudeRef {
    North,
    South,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsLongitudeRef {
    East,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsAltitudeRef {
    AboveSeaLevel,
    BelowSeaLevel,
}

impl FrameExifGpsAltitudeRef {
    pub fn from_raw(raw: u8) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::AboveSeaLevel),
            1 => Ok(Self::BelowSeaLevel),
            _ => Err(AvError::invalid_data(format!(
                "EXIF GPS altitude reference value {raw} is outside the defined 0..=1 range"
            ))),
        }
    }

    pub const fn raw(self) -> u8 {
        match self {
            Self::AboveSeaLevel => 0,
            Self::BelowSeaLevel => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsStatus {
    MeasurementInProgress,
    MeasurementVoid,
}

impl FrameExifGpsStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MeasurementInProgress => "A",
            Self::MeasurementVoid => "V",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsMeasureMode {
    TwoDimensional,
    ThreeDimensional,
}

impl FrameExifGpsMeasureMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TwoDimensional => "2",
            Self::ThreeDimensional => "3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsSpeedRef {
    KilometersPerHour,
    MilesPerHour,
    Knots,
}

impl FrameExifGpsSpeedRef {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KilometersPerHour => "K",
            Self::MilesPerHour => "M",
            Self::Knots => "N",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsDistanceRef {
    Kilometers,
    Miles,
    NauticalMiles,
}

impl FrameExifGpsDistanceRef {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Kilometers => "K",
            Self::Miles => "M",
            Self::NauticalMiles => "N",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsDirectionRef {
    TrueDirection,
    MagneticDirection,
}

impl FrameExifGpsDirectionRef {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TrueDirection => "T",
            Self::MagneticDirection => "M",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifGpsDifferential {
    NoCorrection,
    DifferentialCorrectionApplied,
}

impl FrameExifGpsDifferential {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            0 => Ok(Self::NoCorrection),
            1 => Ok(Self::DifferentialCorrectionApplied),
            _ => Err(AvError::invalid_data(format!(
                "EXIF GPS differential value {raw} is outside the defined 0..=1 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::NoCorrection => 0,
            Self::DifferentialCorrectionApplied => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifNewSubfileType {
    raw: u32,
}

impl FrameExifNewSubfileType {
    pub const REDUCED_RESOLUTION_IMAGE: u32 = 0x1;
    pub const SINGLE_PAGE_OF_MULTI_PAGE_IMAGE: u32 = 0x2;
    pub const TRANSPARENCY_MASK: u32 = 0x4;
    pub const KNOWN_MASK: u32 = Self::REDUCED_RESOLUTION_IMAGE
        | Self::SINGLE_PAGE_OF_MULTI_PAGE_IMAGE
        | Self::TRANSPARENCY_MASK;

    pub fn from_raw(raw: u32) -> AvResult<Self> {
        let unknown = raw & !Self::KNOWN_MASK;
        if unknown != 0 {
            return Err(AvError::invalid_data(format!(
                "EXIF new subfile type flags 0x{raw:08x} contain unknown bits 0x{unknown:08x}"
            )));
        }
        Ok(Self { raw })
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub const fn is_reduced_resolution_image(self) -> bool {
        self.raw & Self::REDUCED_RESOLUTION_IMAGE != 0
    }

    pub const fn is_single_page_of_multi_page_image(self) -> bool {
        self.raw & Self::SINGLE_PAGE_OF_MULTI_PAGE_IMAGE != 0
    }

    pub const fn is_transparency_mask(self) -> bool {
        self.raw & Self::TRANSPARENCY_MASK != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameExifSubfileType {
    FullResolutionImage,
    ReducedResolutionImage,
    SinglePageOfMultiPageImage,
}

impl FrameExifSubfileType {
    pub fn from_raw(raw: u16) -> AvResult<Self> {
        match raw {
            1 => Ok(Self::FullResolutionImage),
            2 => Ok(Self::ReducedResolutionImage),
            3 => Ok(Self::SinglePageOfMultiPageImage),
            _ => Err(AvError::invalid_data(format!(
                "EXIF subfile type value {raw} is outside the defined 1..=3 range"
            ))),
        }
    }

    pub const fn raw(self) -> u16 {
        match self {
            Self::FullResolutionImage => 1,
            Self::ReducedResolutionImage => 2,
            Self::SinglePageOfMultiPageImage => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameExifCommonTags<'a> {
    new_subfile_type: Option<FrameExifNewSubfileType>,
    subfile_type: Option<FrameExifSubfileType>,
    document_name: Option<&'a str>,
    image_description: Option<&'a str>,
    make: Option<&'a str>,
    model: Option<&'a str>,
    image_width: Option<u32>,
    image_length: Option<u32>,
    bits_per_sample: Option<FrameExifBitsPerSample<'a>>,
    compression: Option<FrameExifCompression>,
    photometric_interpretation: Option<FrameExifPhotometricInterpretation>,
    thresholding: Option<FrameExifThresholding>,
    fill_order: Option<FrameExifFillOrder>,
    samples_per_pixel: Option<u16>,
    rows_per_strip: Option<u32>,
    planar_configuration: Option<FrameExifPlanarConfiguration>,
    page_name: Option<&'a str>,
    page_number: Option<[u16; 2]>,
    white_point: Option<[FrameExifRational; 2]>,
    primary_chromaticities: Option<[FrameExifRational; 6]>,
    ycbcr_coefficients: Option<[FrameExifRational; 3]>,
    ycbcr_sub_sampling: Option<[u16; 2]>,
    ycbcr_positioning: Option<FrameExifYcbCrPositioning>,
    reference_black_white: Option<[FrameExifRational; 6]>,
    orientation: Option<FrameExifOrientation>,
    x_resolution: Option<FrameExifRational>,
    y_resolution: Option<FrameExifRational>,
    x_position: Option<FrameExifRational>,
    y_position: Option<FrameExifRational>,
    resolution_unit: Option<FrameExifResolutionUnit>,
    software: Option<&'a str>,
    date_time: Option<&'a str>,
    artist: Option<&'a str>,
    host_computer: Option<&'a str>,
    predictor: Option<FrameExifPredictor>,
    copyright: Option<&'a str>,
    exif_version: Option<[u8; 4]>,
    date_time_original: Option<&'a str>,
    date_time_digitized: Option<&'a str>,
    offset_time: Option<&'a str>,
    offset_time_original: Option<&'a str>,
    offset_time_digitized: Option<&'a str>,
    components_configuration: Option<[u8; 4]>,
    compressed_bits_per_pixel: Option<FrameExifRational>,
    exposure_program: Option<FrameExifExposureProgram>,
    exposure_time: Option<FrameExifRational>,
    f_number: Option<FrameExifRational>,
    spectral_sensitivity: Option<&'a str>,
    photographic_sensitivity: Option<u16>,
    oecf: Option<&'a [u8]>,
    sensitivity_type: Option<FrameExifSensitivityType>,
    standard_output_sensitivity: Option<u32>,
    recommended_exposure_index: Option<u32>,
    iso_speed: Option<u32>,
    iso_speed_latitude_yyy: Option<u32>,
    iso_speed_latitude_zzz: Option<u32>,
    shutter_speed_value: Option<FrameExifSignedRational>,
    aperture_value: Option<FrameExifRational>,
    brightness_value: Option<FrameExifSignedRational>,
    exposure_bias_value: Option<FrameExifSignedRational>,
    max_aperture_value: Option<FrameExifRational>,
    subject_distance: Option<FrameExifRational>,
    metering_mode: Option<FrameExifMeteringMode>,
    light_source: Option<FrameExifLightSource>,
    flash: Option<FrameExifFlash>,
    focal_length: Option<FrameExifRational>,
    subject_area: Option<FrameExifSubjectArea>,
    maker_note: Option<&'a [u8]>,
    user_comment: Option<&'a [u8]>,
    sub_sec_time: Option<&'a str>,
    sub_sec_time_original: Option<&'a str>,
    sub_sec_time_digitized: Option<&'a str>,
    flashpix_version: Option<[u8; 4]>,
    white_balance: Option<FrameExifWhiteBalance>,
    digital_zoom_ratio: Option<FrameExifRational>,
    focal_length_in_35mm_film: Option<u16>,
    color_space: Option<FrameExifColorSpace>,
    flash_energy: Option<FrameExifRational>,
    spatial_frequency_response: Option<&'a [u8]>,
    focal_plane_x_resolution: Option<FrameExifRational>,
    focal_plane_y_resolution: Option<FrameExifRational>,
    focal_plane_resolution_unit: Option<FrameExifResolutionUnit>,
    subject_location: Option<[u16; 2]>,
    exposure_index: Option<FrameExifRational>,
    sensing_method: Option<FrameExifSensingMethod>,
    file_source: Option<FrameExifFileSource>,
    scene_type: Option<FrameExifSceneType>,
    cfa_pattern: Option<&'a [u8]>,
    custom_rendered: Option<FrameExifCustomRendered>,
    exposure_mode: Option<FrameExifExposureMode>,
    scene_capture_type: Option<FrameExifSceneCaptureType>,
    gain_control: Option<FrameExifGainControl>,
    contrast: Option<FrameExifContrast>,
    saturation: Option<FrameExifSaturation>,
    sharpness: Option<FrameExifSharpness>,
    subject_distance_range: Option<FrameExifSubjectDistanceRange>,
    pixel_x_dimension: Option<u32>,
    pixel_y_dimension: Option<u32>,
    related_sound_file: Option<&'a str>,
    temperature: Option<FrameExifSignedRational>,
    humidity: Option<FrameExifRational>,
    pressure: Option<FrameExifRational>,
    water_depth: Option<FrameExifSignedRational>,
    acceleration: Option<FrameExifRational>,
    camera_elevation_angle: Option<FrameExifSignedRational>,
    image_unique_id: Option<&'a str>,
    camera_owner_name: Option<&'a str>,
    body_serial_number: Option<&'a str>,
    lens_specification: Option<[FrameExifRational; 4]>,
    lens_make: Option<&'a str>,
    lens_model: Option<&'a str>,
    lens_serial_number: Option<&'a str>,
    gamma: Option<FrameExifRational>,
    composite_image: Option<FrameExifCompositeImage>,
    source_image_number_of_composite_image: Option<[u16; 2]>,
    source_exposure_times_of_composite_image: Option<&'a [u8]>,
    gps_version_id: Option<[u8; 4]>,
    gps_latitude_ref: Option<FrameExifGpsLatitudeRef>,
    gps_latitude: Option<[FrameExifRational; 3]>,
    gps_longitude_ref: Option<FrameExifGpsLongitudeRef>,
    gps_longitude: Option<[FrameExifRational; 3]>,
    gps_altitude_ref: Option<FrameExifGpsAltitudeRef>,
    gps_altitude: Option<FrameExifRational>,
    gps_time_stamp: Option<[FrameExifRational; 3]>,
    gps_satellites: Option<&'a str>,
    gps_status: Option<FrameExifGpsStatus>,
    gps_measure_mode: Option<FrameExifGpsMeasureMode>,
    gps_dop: Option<FrameExifRational>,
    gps_date_stamp: Option<&'a str>,
    gps_speed_ref: Option<FrameExifGpsSpeedRef>,
    gps_speed: Option<FrameExifRational>,
    gps_track_ref: Option<FrameExifGpsDirectionRef>,
    gps_track: Option<FrameExifRational>,
    gps_img_direction_ref: Option<FrameExifGpsDirectionRef>,
    gps_img_direction: Option<FrameExifRational>,
    gps_map_datum: Option<&'a str>,
    gps_dest_latitude_ref: Option<FrameExifGpsLatitudeRef>,
    gps_dest_latitude: Option<[FrameExifRational; 3]>,
    gps_dest_longitude_ref: Option<FrameExifGpsLongitudeRef>,
    gps_dest_longitude: Option<[FrameExifRational; 3]>,
    gps_dest_bearing_ref: Option<FrameExifGpsDirectionRef>,
    gps_dest_bearing: Option<FrameExifRational>,
    gps_dest_distance_ref: Option<FrameExifGpsDistanceRef>,
    gps_dest_distance: Option<FrameExifRational>,
    gps_processing_method: Option<&'a [u8]>,
    gps_area_information: Option<&'a [u8]>,
    gps_differential: Option<FrameExifGpsDifferential>,
    gps_h_positioning_error: Option<FrameExifRational>,
    interoperability_index: Option<&'a str>,
    interoperability_version: Option<[u8; 4]>,
    related_image_file_format: Option<&'a str>,
    related_image_width: Option<u32>,
    related_image_length: Option<u32>,
}

impl<'a> FrameExifCommonTags<'a> {
    pub const fn new_subfile_type(&self) -> Option<FrameExifNewSubfileType> {
        self.new_subfile_type
    }

    pub const fn subfile_type(&self) -> Option<FrameExifSubfileType> {
        self.subfile_type
    }

    pub const fn document_name(&self) -> Option<&'a str> {
        self.document_name
    }

    pub const fn image_description(&self) -> Option<&'a str> {
        self.image_description
    }

    pub const fn make(&self) -> Option<&'a str> {
        self.make
    }

    pub const fn model(&self) -> Option<&'a str> {
        self.model
    }

    pub const fn image_width(&self) -> Option<u32> {
        self.image_width
    }

    pub const fn image_length(&self) -> Option<u32> {
        self.image_length
    }

    pub const fn bits_per_sample(&self) -> Option<FrameExifBitsPerSample<'a>> {
        self.bits_per_sample
    }

    pub const fn compression(&self) -> Option<FrameExifCompression> {
        self.compression
    }

    pub const fn photometric_interpretation(&self) -> Option<FrameExifPhotometricInterpretation> {
        self.photometric_interpretation
    }

    pub const fn thresholding(&self) -> Option<FrameExifThresholding> {
        self.thresholding
    }

    pub const fn fill_order(&self) -> Option<FrameExifFillOrder> {
        self.fill_order
    }

    pub const fn samples_per_pixel(&self) -> Option<u16> {
        self.samples_per_pixel
    }

    pub const fn rows_per_strip(&self) -> Option<u32> {
        self.rows_per_strip
    }

    pub const fn planar_configuration(&self) -> Option<FrameExifPlanarConfiguration> {
        self.planar_configuration
    }

    pub const fn page_name(&self) -> Option<&'a str> {
        self.page_name
    }

    pub const fn page_number(&self) -> Option<[u16; 2]> {
        self.page_number
    }

    pub const fn white_point(&self) -> Option<[FrameExifRational; 2]> {
        self.white_point
    }

    pub const fn primary_chromaticities(&self) -> Option<[FrameExifRational; 6]> {
        self.primary_chromaticities
    }

    pub const fn ycbcr_coefficients(&self) -> Option<[FrameExifRational; 3]> {
        self.ycbcr_coefficients
    }

    pub const fn ycbcr_sub_sampling(&self) -> Option<[u16; 2]> {
        self.ycbcr_sub_sampling
    }

    pub const fn ycbcr_positioning(&self) -> Option<FrameExifYcbCrPositioning> {
        self.ycbcr_positioning
    }

    pub const fn reference_black_white(&self) -> Option<[FrameExifRational; 6]> {
        self.reference_black_white
    }

    pub const fn orientation(&self) -> Option<FrameExifOrientation> {
        self.orientation
    }

    pub const fn x_resolution(&self) -> Option<FrameExifRational> {
        self.x_resolution
    }

    pub const fn y_resolution(&self) -> Option<FrameExifRational> {
        self.y_resolution
    }

    pub const fn x_position(&self) -> Option<FrameExifRational> {
        self.x_position
    }

    pub const fn y_position(&self) -> Option<FrameExifRational> {
        self.y_position
    }

    pub const fn resolution_unit(&self) -> Option<FrameExifResolutionUnit> {
        self.resolution_unit
    }

    pub const fn software(&self) -> Option<&'a str> {
        self.software
    }

    pub const fn date_time(&self) -> Option<&'a str> {
        self.date_time
    }

    pub const fn artist(&self) -> Option<&'a str> {
        self.artist
    }

    pub const fn host_computer(&self) -> Option<&'a str> {
        self.host_computer
    }

    pub const fn predictor(&self) -> Option<FrameExifPredictor> {
        self.predictor
    }

    pub const fn copyright(&self) -> Option<&'a str> {
        self.copyright
    }

    pub const fn exif_version(&self) -> Option<[u8; 4]> {
        self.exif_version
    }

    pub const fn date_time_original(&self) -> Option<&'a str> {
        self.date_time_original
    }

    pub const fn date_time_digitized(&self) -> Option<&'a str> {
        self.date_time_digitized
    }

    pub const fn offset_time(&self) -> Option<&'a str> {
        self.offset_time
    }

    pub const fn offset_time_original(&self) -> Option<&'a str> {
        self.offset_time_original
    }

    pub const fn offset_time_digitized(&self) -> Option<&'a str> {
        self.offset_time_digitized
    }

    pub const fn components_configuration(&self) -> Option<[u8; 4]> {
        self.components_configuration
    }

    pub const fn compressed_bits_per_pixel(&self) -> Option<FrameExifRational> {
        self.compressed_bits_per_pixel
    }

    pub const fn exposure_program(&self) -> Option<FrameExifExposureProgram> {
        self.exposure_program
    }

    pub const fn exposure_time(&self) -> Option<FrameExifRational> {
        self.exposure_time
    }

    pub const fn f_number(&self) -> Option<FrameExifRational> {
        self.f_number
    }

    pub const fn spectral_sensitivity(&self) -> Option<&'a str> {
        self.spectral_sensitivity
    }

    pub const fn photographic_sensitivity(&self) -> Option<u16> {
        self.photographic_sensitivity
    }

    pub const fn oecf(&self) -> Option<&'a [u8]> {
        self.oecf
    }

    pub const fn sensitivity_type(&self) -> Option<FrameExifSensitivityType> {
        self.sensitivity_type
    }

    pub const fn standard_output_sensitivity(&self) -> Option<u32> {
        self.standard_output_sensitivity
    }

    pub const fn recommended_exposure_index(&self) -> Option<u32> {
        self.recommended_exposure_index
    }

    pub const fn iso_speed(&self) -> Option<u32> {
        self.iso_speed
    }

    pub const fn iso_speed_latitude_yyy(&self) -> Option<u32> {
        self.iso_speed_latitude_yyy
    }

    pub const fn iso_speed_latitude_zzz(&self) -> Option<u32> {
        self.iso_speed_latitude_zzz
    }

    pub const fn shutter_speed_value(&self) -> Option<FrameExifSignedRational> {
        self.shutter_speed_value
    }

    pub const fn aperture_value(&self) -> Option<FrameExifRational> {
        self.aperture_value
    }

    pub const fn brightness_value(&self) -> Option<FrameExifSignedRational> {
        self.brightness_value
    }

    pub const fn exposure_bias_value(&self) -> Option<FrameExifSignedRational> {
        self.exposure_bias_value
    }

    pub const fn max_aperture_value(&self) -> Option<FrameExifRational> {
        self.max_aperture_value
    }

    pub const fn subject_distance(&self) -> Option<FrameExifRational> {
        self.subject_distance
    }

    pub const fn metering_mode(&self) -> Option<FrameExifMeteringMode> {
        self.metering_mode
    }

    pub const fn light_source(&self) -> Option<FrameExifLightSource> {
        self.light_source
    }

    pub const fn flash(&self) -> Option<FrameExifFlash> {
        self.flash
    }

    pub const fn focal_length(&self) -> Option<FrameExifRational> {
        self.focal_length
    }

    pub const fn subject_area(&self) -> Option<FrameExifSubjectArea> {
        self.subject_area
    }

    pub const fn maker_note(&self) -> Option<&'a [u8]> {
        self.maker_note
    }

    pub const fn user_comment(&self) -> Option<&'a [u8]> {
        self.user_comment
    }

    pub const fn sub_sec_time(&self) -> Option<&'a str> {
        self.sub_sec_time
    }

    pub const fn sub_sec_time_original(&self) -> Option<&'a str> {
        self.sub_sec_time_original
    }

    pub const fn sub_sec_time_digitized(&self) -> Option<&'a str> {
        self.sub_sec_time_digitized
    }

    pub const fn flashpix_version(&self) -> Option<[u8; 4]> {
        self.flashpix_version
    }

    pub const fn white_balance(&self) -> Option<FrameExifWhiteBalance> {
        self.white_balance
    }

    pub const fn digital_zoom_ratio(&self) -> Option<FrameExifRational> {
        self.digital_zoom_ratio
    }

    pub const fn focal_length_in_35mm_film(&self) -> Option<u16> {
        self.focal_length_in_35mm_film
    }

    pub const fn color_space(&self) -> Option<FrameExifColorSpace> {
        self.color_space
    }

    pub const fn flash_energy(&self) -> Option<FrameExifRational> {
        self.flash_energy
    }

    pub const fn spatial_frequency_response(&self) -> Option<&'a [u8]> {
        self.spatial_frequency_response
    }

    pub const fn focal_plane_x_resolution(&self) -> Option<FrameExifRational> {
        self.focal_plane_x_resolution
    }

    pub const fn focal_plane_y_resolution(&self) -> Option<FrameExifRational> {
        self.focal_plane_y_resolution
    }

    pub const fn focal_plane_resolution_unit(&self) -> Option<FrameExifResolutionUnit> {
        self.focal_plane_resolution_unit
    }

    pub const fn subject_location(&self) -> Option<[u16; 2]> {
        self.subject_location
    }

    pub const fn exposure_index(&self) -> Option<FrameExifRational> {
        self.exposure_index
    }

    pub const fn sensing_method(&self) -> Option<FrameExifSensingMethod> {
        self.sensing_method
    }

    pub const fn file_source(&self) -> Option<FrameExifFileSource> {
        self.file_source
    }

    pub const fn scene_type(&self) -> Option<FrameExifSceneType> {
        self.scene_type
    }

    pub const fn cfa_pattern(&self) -> Option<&'a [u8]> {
        self.cfa_pattern
    }

    pub const fn custom_rendered(&self) -> Option<FrameExifCustomRendered> {
        self.custom_rendered
    }

    pub const fn exposure_mode(&self) -> Option<FrameExifExposureMode> {
        self.exposure_mode
    }

    pub const fn scene_capture_type(&self) -> Option<FrameExifSceneCaptureType> {
        self.scene_capture_type
    }

    pub const fn gain_control(&self) -> Option<FrameExifGainControl> {
        self.gain_control
    }

    pub const fn contrast(&self) -> Option<FrameExifContrast> {
        self.contrast
    }

    pub const fn saturation(&self) -> Option<FrameExifSaturation> {
        self.saturation
    }

    pub const fn sharpness(&self) -> Option<FrameExifSharpness> {
        self.sharpness
    }

    pub const fn subject_distance_range(&self) -> Option<FrameExifSubjectDistanceRange> {
        self.subject_distance_range
    }

    pub const fn pixel_x_dimension(&self) -> Option<u32> {
        self.pixel_x_dimension
    }

    pub const fn pixel_y_dimension(&self) -> Option<u32> {
        self.pixel_y_dimension
    }

    pub const fn related_sound_file(&self) -> Option<&'a str> {
        self.related_sound_file
    }

    pub const fn temperature(&self) -> Option<FrameExifSignedRational> {
        self.temperature
    }

    pub const fn humidity(&self) -> Option<FrameExifRational> {
        self.humidity
    }

    pub const fn pressure(&self) -> Option<FrameExifRational> {
        self.pressure
    }

    pub const fn water_depth(&self) -> Option<FrameExifSignedRational> {
        self.water_depth
    }

    pub const fn acceleration(&self) -> Option<FrameExifRational> {
        self.acceleration
    }

    pub const fn camera_elevation_angle(&self) -> Option<FrameExifSignedRational> {
        self.camera_elevation_angle
    }

    pub const fn image_unique_id(&self) -> Option<&'a str> {
        self.image_unique_id
    }

    pub const fn camera_owner_name(&self) -> Option<&'a str> {
        self.camera_owner_name
    }

    pub const fn body_serial_number(&self) -> Option<&'a str> {
        self.body_serial_number
    }

    pub const fn lens_specification(&self) -> Option<[FrameExifRational; 4]> {
        self.lens_specification
    }

    pub const fn lens_make(&self) -> Option<&'a str> {
        self.lens_make
    }

    pub const fn lens_model(&self) -> Option<&'a str> {
        self.lens_model
    }

    pub const fn lens_serial_number(&self) -> Option<&'a str> {
        self.lens_serial_number
    }

    pub const fn gamma(&self) -> Option<FrameExifRational> {
        self.gamma
    }

    pub const fn composite_image(&self) -> Option<FrameExifCompositeImage> {
        self.composite_image
    }

    pub const fn source_image_number_of_composite_image(&self) -> Option<[u16; 2]> {
        self.source_image_number_of_composite_image
    }

    pub const fn source_exposure_times_of_composite_image(&self) -> Option<&'a [u8]> {
        self.source_exposure_times_of_composite_image
    }

    pub const fn gps_version_id(&self) -> Option<[u8; 4]> {
        self.gps_version_id
    }

    pub const fn gps_latitude_ref(&self) -> Option<FrameExifGpsLatitudeRef> {
        self.gps_latitude_ref
    }

    pub const fn gps_latitude(&self) -> Option<[FrameExifRational; 3]> {
        self.gps_latitude
    }

    pub const fn gps_longitude_ref(&self) -> Option<FrameExifGpsLongitudeRef> {
        self.gps_longitude_ref
    }

    pub const fn gps_longitude(&self) -> Option<[FrameExifRational; 3]> {
        self.gps_longitude
    }

    pub const fn gps_altitude_ref(&self) -> Option<FrameExifGpsAltitudeRef> {
        self.gps_altitude_ref
    }

    pub const fn gps_altitude(&self) -> Option<FrameExifRational> {
        self.gps_altitude
    }

    pub const fn gps_time_stamp(&self) -> Option<[FrameExifRational; 3]> {
        self.gps_time_stamp
    }

    pub const fn gps_satellites(&self) -> Option<&'a str> {
        self.gps_satellites
    }

    pub const fn gps_status(&self) -> Option<FrameExifGpsStatus> {
        self.gps_status
    }

    pub const fn gps_measure_mode(&self) -> Option<FrameExifGpsMeasureMode> {
        self.gps_measure_mode
    }

    pub const fn gps_dop(&self) -> Option<FrameExifRational> {
        self.gps_dop
    }

    pub const fn gps_date_stamp(&self) -> Option<&'a str> {
        self.gps_date_stamp
    }

    pub const fn gps_speed_ref(&self) -> Option<FrameExifGpsSpeedRef> {
        self.gps_speed_ref
    }

    pub const fn gps_speed(&self) -> Option<FrameExifRational> {
        self.gps_speed
    }

    pub const fn gps_track_ref(&self) -> Option<FrameExifGpsDirectionRef> {
        self.gps_track_ref
    }

    pub const fn gps_track(&self) -> Option<FrameExifRational> {
        self.gps_track
    }

    pub const fn gps_img_direction_ref(&self) -> Option<FrameExifGpsDirectionRef> {
        self.gps_img_direction_ref
    }

    pub const fn gps_img_direction(&self) -> Option<FrameExifRational> {
        self.gps_img_direction
    }

    pub const fn gps_map_datum(&self) -> Option<&'a str> {
        self.gps_map_datum
    }

    pub const fn gps_dest_latitude_ref(&self) -> Option<FrameExifGpsLatitudeRef> {
        self.gps_dest_latitude_ref
    }

    pub const fn gps_dest_latitude(&self) -> Option<[FrameExifRational; 3]> {
        self.gps_dest_latitude
    }

    pub const fn gps_dest_longitude_ref(&self) -> Option<FrameExifGpsLongitudeRef> {
        self.gps_dest_longitude_ref
    }

    pub const fn gps_dest_longitude(&self) -> Option<[FrameExifRational; 3]> {
        self.gps_dest_longitude
    }

    pub const fn gps_dest_bearing_ref(&self) -> Option<FrameExifGpsDirectionRef> {
        self.gps_dest_bearing_ref
    }

    pub const fn gps_dest_bearing(&self) -> Option<FrameExifRational> {
        self.gps_dest_bearing
    }

    pub const fn gps_dest_distance_ref(&self) -> Option<FrameExifGpsDistanceRef> {
        self.gps_dest_distance_ref
    }

    pub const fn gps_dest_distance(&self) -> Option<FrameExifRational> {
        self.gps_dest_distance
    }

    pub const fn gps_processing_method(&self) -> Option<&'a [u8]> {
        self.gps_processing_method
    }

    pub const fn gps_area_information(&self) -> Option<&'a [u8]> {
        self.gps_area_information
    }

    pub const fn gps_differential(&self) -> Option<FrameExifGpsDifferential> {
        self.gps_differential
    }

    pub const fn gps_h_positioning_error(&self) -> Option<FrameExifRational> {
        self.gps_h_positioning_error
    }

    pub const fn interoperability_index(&self) -> Option<&'a str> {
        self.interoperability_index
    }

    pub const fn interoperability_version(&self) -> Option<[u8; 4]> {
        self.interoperability_version
    }

    pub const fn related_image_file_format(&self) -> Option<&'a str> {
        self.related_image_file_format
    }

    pub const fn related_image_width(&self) -> Option<u32> {
        self.related_image_width
    }

    pub const fn related_image_length(&self) -> Option<u32> {
        self.related_image_length
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameExifEntry<'a> {
    tag: u16,
    tiff_type: FrameExifTiffType,
    endian: FrameExifEndian,
    count: u32,
    value_offset: u32,
    data_len: usize,
    value_or_offset_bytes: &'a [u8],
    value_data: &'a [u8],
    data_range: Option<(usize, usize)>,
}

impl<'a> FrameExifEntry<'a> {
    pub const fn tag(self) -> u16 {
        self.tag
    }

    pub const fn tiff_type(self) -> FrameExifTiffType {
        self.tiff_type
    }

    pub const fn endian(self) -> FrameExifEndian {
        self.endian
    }

    pub const fn count(self) -> u32 {
        self.count
    }

    pub const fn value_offset(self) -> u32 {
        self.value_offset
    }

    pub const fn data_len(self) -> usize {
        self.data_len
    }

    pub const fn value_or_offset_bytes(self) -> &'a [u8] {
        self.value_or_offset_bytes
    }

    pub const fn value_data(self) -> &'a [u8] {
        self.value_data
    }

    pub const fn data_range(self) -> Option<(usize, usize)> {
        self.data_range
    }

    pub const fn is_inline(self) -> bool {
        self.data_range.is_none()
    }

    pub fn byte_values(self) -> AvResult<Option<&'a [u8]>> {
        if self.tiff_type != FrameExifTiffType::Byte {
            return Ok(None);
        }

        Ok(Some(self.value_data))
    }

    pub fn undefined_values(self) -> AvResult<Option<&'a [u8]>> {
        if self.tiff_type != FrameExifTiffType::Undefined {
            return Ok(None);
        }

        Ok(Some(self.value_data))
    }

    pub fn ascii_strings(self) -> AvResult<Option<Vec<&'a str>>> {
        if self.tiff_type != FrameExifTiffType::Ascii {
            return Ok(None);
        }
        if self.value_data.is_empty() {
            return Ok(Some(Vec::new()));
        }
        if self.value_data.last() != Some(&0) {
            return Err(AvError::invalid_data(format!(
                "EXIF ASCII tag 0x{:04x} is not NUL-terminated",
                self.tag
            )));
        }

        let mut strings = Vec::new();
        for segment in self.value_data[..self.value_data.len() - 1].split(|byte| *byte == 0) {
            if !segment.is_ascii() {
                return Err(AvError::invalid_data(format!(
                    "EXIF ASCII tag 0x{:04x} contains non-ASCII bytes",
                    self.tag
                )));
            }
            strings.push(core::str::from_utf8(segment).map_err(|_| {
                AvError::invalid_data(format!(
                    "EXIF ASCII tag 0x{:04x} contains invalid UTF-8",
                    self.tag
                ))
            })?);
        }

        Ok(Some(strings))
    }

    pub fn short_values(self) -> AvResult<Option<Vec<u16>>> {
        if self.tiff_type != FrameExifTiffType::Short {
            return Ok(None);
        }
        self.decode_values(2, |data, offset| self.endian.read_u16(data, offset))
            .map(Some)
    }

    pub fn long_values(self) -> AvResult<Option<Vec<u32>>> {
        if !matches!(
            self.tiff_type,
            FrameExifTiffType::Long | FrameExifTiffType::Ifd
        ) {
            return Ok(None);
        }
        self.decode_values(4, |data, offset| self.endian.read_u32(data, offset))
            .map(Some)
    }

    pub fn signed_short_values(self) -> AvResult<Option<Vec<i16>>> {
        if self.tiff_type != FrameExifTiffType::SignedShort {
            return Ok(None);
        }
        self.decode_values(2, |data, offset| self.endian.read_i16(data, offset))
            .map(Some)
    }

    pub fn signed_byte_values(self) -> AvResult<Option<Vec<i8>>> {
        if self.tiff_type != FrameExifTiffType::SignedByte {
            return Ok(None);
        }
        Ok(Some(
            self.value_data
                .iter()
                .map(|value| i8::from_ne_bytes([*value]))
                .collect(),
        ))
    }

    pub fn signed_long_values(self) -> AvResult<Option<Vec<i32>>> {
        if self.tiff_type != FrameExifTiffType::SignedLong {
            return Ok(None);
        }
        self.decode_values(4, |data, offset| self.endian.read_i32(data, offset))
            .map(Some)
    }

    pub fn rational_values(self) -> AvResult<Option<Vec<FrameExifRational>>> {
        if self.tiff_type != FrameExifTiffType::Rational {
            return Ok(None);
        }

        let mut values = Vec::with_capacity(self.count as usize);
        for index in 0..self.count as usize {
            let offset = index * 8;
            let numerator = self.endian.read_u32(self.value_data, offset);
            let denominator = self.endian.read_u32(self.value_data, offset + 4);
            if denominator == 0 {
                return Err(AvError::invalid_data(format!(
                    "EXIF rational tag 0x{:04x} entry {index} has zero denominator",
                    self.tag
                )));
            }
            values.push(FrameExifRational {
                numerator,
                denominator,
            });
        }

        Ok(Some(values))
    }

    pub fn signed_rational_values(self) -> AvResult<Option<Vec<FrameExifSignedRational>>> {
        if self.tiff_type != FrameExifTiffType::SignedRational {
            return Ok(None);
        }

        let mut values = Vec::with_capacity(self.count as usize);
        for index in 0..self.count as usize {
            let offset = index * 8;
            let numerator = self.endian.read_i32(self.value_data, offset);
            let denominator = self.endian.read_i32(self.value_data, offset + 4);
            if denominator == 0 {
                return Err(AvError::invalid_data(format!(
                    "EXIF signed rational tag 0x{:04x} entry {index} has zero denominator",
                    self.tag
                )));
            }
            values.push(FrameExifSignedRational {
                numerator,
                denominator,
            });
        }

        Ok(Some(values))
    }

    pub fn float_values(self) -> AvResult<Option<Vec<f32>>> {
        if self.tiff_type != FrameExifTiffType::Float {
            return Ok(None);
        }
        self.decode_values(4, |data, offset| {
            f32::from_bits(self.endian.read_u32(data, offset))
        })
        .map(Some)
    }

    pub fn double_values(self) -> AvResult<Option<Vec<f64>>> {
        if self.tiff_type != FrameExifTiffType::Double {
            return Ok(None);
        }
        self.decode_values(8, |data, offset| {
            f64::from_bits(self.endian.read_u64(data, offset))
        })
        .map(Some)
    }

    pub const fn ifd_pointer_kind(self) -> Option<FrameExifIfdPointerKind> {
        FrameExifIfdPointerKind::from_tag(self.tag)
    }

    pub fn ifd_pointer_offset(self) -> AvResult<Option<usize>> {
        let Some(kind) = self.ifd_pointer_kind() else {
            return Ok(None);
        };

        if !matches!(
            self.tiff_type,
            FrameExifTiffType::Long | FrameExifTiffType::Ifd
        ) {
            return Err(AvError::invalid_data(format!(
                "EXIF {:?} IFD pointer tag 0x{:04x} must have LONG/IFD type, got type {}",
                kind,
                kind.tag(),
                self.tiff_type.raw()
            )));
        }
        if self.count != 1 {
            return Err(AvError::invalid_data(format!(
                "EXIF {:?} IFD pointer tag 0x{:04x} must contain one offset, got {}",
                kind,
                kind.tag(),
                self.count
            )));
        }

        Ok(Some(self.value_offset as usize))
    }

    fn decode_values<T>(
        self,
        element_len: usize,
        read: impl Fn(&[u8], usize) -> T,
    ) -> AvResult<Vec<T>> {
        let expected_len = element_len
            .checked_mul(self.count as usize)
            .ok_or_else(|| AvError::invalid_data("EXIF typed value length overflow"))?;
        if expected_len != self.value_data.len() {
            return Err(AvError::invalid_data(format!(
                "EXIF tag 0x{:04x} typed value length {} does not match expected length {expected_len}",
                self.tag,
                self.value_data.len()
            )));
        }

        let mut values = Vec::with_capacity(self.count as usize);
        for offset in (0..expected_len).step_by(element_len) {
            values.push(read(self.value_data, offset));
        }
        Ok(values)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameExifIfd<'a> {
    offset: usize,
    entries: Vec<FrameExifEntry<'a>>,
    next_ifd_offset: Option<usize>,
}

impl<'a> FrameExifIfd<'a> {
    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub fn entries(&self) -> &[FrameExifEntry<'a>] {
        &self.entries
    }

    pub fn entry(&self, index: usize) -> Option<FrameExifEntry<'a>> {
        self.entries.get(index).copied()
    }

    pub fn entry_by_tag(&self, tag: u16) -> Option<FrameExifEntry<'a>> {
        self.entries.iter().copied().find(|entry| entry.tag == tag)
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub const fn next_ifd_offset(&self) -> Option<usize> {
        self.next_ifd_offset
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameExifLinkedIfd<'a> {
    kind: FrameExifIfdPointerKind,
    parent_ifd_offset: usize,
    source_tag: u16,
    ifd: FrameExifIfd<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameExifIfdPointer {
    kind: FrameExifIfdPointerKind,
    source_tag: u16,
    offset: usize,
}

impl<'a> FrameExifLinkedIfd<'a> {
    pub const fn kind(&self) -> FrameExifIfdPointerKind {
        self.kind
    }

    pub const fn parent_ifd_offset(&self) -> usize {
        self.parent_ifd_offset
    }

    pub const fn source_tag(&self) -> u16 {
        self.source_tag
    }

    pub const fn offset(&self) -> usize {
        self.ifd.offset
    }

    pub const fn ifd(&self) -> &FrameExifIfd<'a> {
        &self.ifd
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameExif<'a> {
    data: &'a [u8],
    endian: FrameExifEndian,
    first_ifd_offset: usize,
    ifds: Vec<FrameExifIfd<'a>>,
    linked_ifds: Vec<FrameExifLinkedIfd<'a>>,
}

impl<'a> FrameExif<'a> {
    pub const TIFF_HEADER_LEN: usize = 8;
    pub const IFD_ENTRY_LEN: usize = 12;
    pub const IFD_COUNT_LEN: usize = 2;
    pub const NEXT_IFD_OFFSET_LEN: usize = 4;
    pub const MAX_IFDS: usize = 16;
    pub const MAX_LINKED_IFDS: usize = 16;
    pub const MAX_IFD_ENTRIES: usize = 4096;
    pub const TAG_NEW_SUBFILE_TYPE: u16 = 0x00FE;
    pub const TAG_SUBFILE_TYPE: u16 = 0x00FF;
    pub const TAG_IMAGE_WIDTH: u16 = 0x0100;
    pub const TAG_IMAGE_LENGTH: u16 = 0x0101;
    pub const TAG_BITS_PER_SAMPLE: u16 = 0x0102;
    pub const TAG_COMPRESSION: u16 = 0x0103;
    pub const TAG_PHOTOMETRIC_INTERPRETATION: u16 = 0x0106;
    pub const TAG_THRESHOLDING: u16 = 0x0107;
    pub const TAG_FILL_ORDER: u16 = 0x010A;
    pub const TAG_DOCUMENT_NAME: u16 = 0x010D;
    pub const TAG_IMAGE_DESCRIPTION: u16 = 0x010E;
    pub const TAG_MAKE: u16 = 0x010F;
    pub const TAG_MODEL: u16 = 0x0110;
    pub const TAG_ORIENTATION: u16 = 0x0112;
    pub const TAG_SAMPLES_PER_PIXEL: u16 = 0x0115;
    pub const TAG_ROWS_PER_STRIP: u16 = 0x0116;
    pub const TAG_X_RESOLUTION: u16 = 0x011A;
    pub const TAG_Y_RESOLUTION: u16 = 0x011B;
    pub const TAG_PLANAR_CONFIGURATION: u16 = 0x011C;
    pub const TAG_PAGE_NAME: u16 = 0x011D;
    pub const TAG_X_POSITION: u16 = 0x011E;
    pub const TAG_Y_POSITION: u16 = 0x011F;
    pub const TAG_RESOLUTION_UNIT: u16 = 0x0128;
    pub const TAG_PAGE_NUMBER: u16 = 0x0129;
    pub const TAG_SOFTWARE: u16 = 0x0131;
    pub const TAG_DATE_TIME: u16 = 0x0132;
    pub const TAG_ARTIST: u16 = 0x013B;
    pub const TAG_HOST_COMPUTER: u16 = 0x013C;
    pub const TAG_PREDICTOR: u16 = 0x013D;
    pub const TAG_WHITE_POINT: u16 = 0x013E;
    pub const TAG_PRIMARY_CHROMATICITIES: u16 = 0x013F;
    pub const TAG_YCBCR_COEFFICIENTS: u16 = 0x0211;
    pub const TAG_YCBCR_SUB_SAMPLING: u16 = 0x0212;
    pub const TAG_YCBCR_POSITIONING: u16 = 0x0213;
    pub const TAG_REFERENCE_BLACK_WHITE: u16 = 0x0214;
    pub const TAG_COPYRIGHT: u16 = 0x8298;
    pub const TAG_EXPOSURE_PROGRAM: u16 = 0x8822;
    pub const TAG_SPECTRAL_SENSITIVITY: u16 = 0x8824;
    pub const TAG_PHOTOGRAPHIC_SENSITIVITY: u16 = 0x8827;
    pub const TAG_OECF: u16 = 0x8828;
    pub const TAG_SENSITIVITY_TYPE: u16 = 0x8830;
    pub const TAG_STANDARD_OUTPUT_SENSITIVITY: u16 = 0x8831;
    pub const TAG_RECOMMENDED_EXPOSURE_INDEX: u16 = 0x8832;
    pub const TAG_ISO_SPEED: u16 = 0x8833;
    pub const TAG_ISO_SPEED_LATITUDE_YYY: u16 = 0x8834;
    pub const TAG_ISO_SPEED_LATITUDE_ZZZ: u16 = 0x8835;
    pub const TAG_EXPOSURE_TIME: u16 = 0x829A;
    pub const TAG_F_NUMBER: u16 = 0x829D;
    pub const TAG_EXIF_VERSION: u16 = 0x9000;
    pub const TAG_DATE_TIME_ORIGINAL: u16 = 0x9003;
    pub const TAG_DATE_TIME_DIGITIZED: u16 = 0x9004;
    pub const TAG_OFFSET_TIME: u16 = 0x9010;
    pub const TAG_OFFSET_TIME_ORIGINAL: u16 = 0x9011;
    pub const TAG_OFFSET_TIME_DIGITIZED: u16 = 0x9012;
    pub const TAG_COMPONENTS_CONFIGURATION: u16 = 0x9101;
    pub const TAG_COMPRESSED_BITS_PER_PIXEL: u16 = 0x9102;
    pub const TAG_SHUTTER_SPEED_VALUE: u16 = 0x9201;
    pub const TAG_APERTURE_VALUE: u16 = 0x9202;
    pub const TAG_BRIGHTNESS_VALUE: u16 = 0x9203;
    pub const TAG_EXPOSURE_BIAS_VALUE: u16 = 0x9204;
    pub const TAG_MAX_APERTURE_VALUE: u16 = 0x9205;
    pub const TAG_SUBJECT_DISTANCE: u16 = 0x9206;
    pub const TAG_METERING_MODE: u16 = 0x9207;
    pub const TAG_LIGHT_SOURCE: u16 = 0x9208;
    pub const TAG_FLASH: u16 = 0x9209;
    pub const TAG_FOCAL_LENGTH: u16 = 0x920A;
    pub const TAG_SUBJECT_AREA: u16 = 0x9214;
    pub const TAG_MAKER_NOTE: u16 = 0x927C;
    pub const TAG_USER_COMMENT: u16 = 0x9286;
    pub const TAG_SUB_SEC_TIME: u16 = 0x9290;
    pub const TAG_SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;
    pub const TAG_SUB_SEC_TIME_DIGITIZED: u16 = 0x9292;
    pub const TAG_TEMPERATURE: u16 = 0x9400;
    pub const TAG_HUMIDITY: u16 = 0x9401;
    pub const TAG_PRESSURE: u16 = 0x9402;
    pub const TAG_WATER_DEPTH: u16 = 0x9403;
    pub const TAG_ACCELERATION: u16 = 0x9404;
    pub const TAG_CAMERA_ELEVATION_ANGLE: u16 = 0x9405;
    pub const TAG_FLASHPIX_VERSION: u16 = 0xA000;
    pub const TAG_COLOR_SPACE: u16 = 0xA001;
    pub const TAG_PIXEL_X_DIMENSION: u16 = 0xA002;
    pub const TAG_PIXEL_Y_DIMENSION: u16 = 0xA003;
    pub const TAG_RELATED_SOUND_FILE: u16 = 0xA004;
    pub const TAG_FLASH_ENERGY: u16 = 0xA20B;
    pub const TAG_SPATIAL_FREQUENCY_RESPONSE: u16 = 0xA20C;
    pub const TAG_FOCAL_PLANE_X_RESOLUTION: u16 = 0xA20E;
    pub const TAG_FOCAL_PLANE_Y_RESOLUTION: u16 = 0xA20F;
    pub const TAG_FOCAL_PLANE_RESOLUTION_UNIT: u16 = 0xA210;
    pub const TAG_SUBJECT_LOCATION: u16 = 0xA214;
    pub const TAG_EXPOSURE_INDEX: u16 = 0xA215;
    pub const TAG_SENSING_METHOD: u16 = 0xA217;
    pub const TAG_FILE_SOURCE: u16 = 0xA300;
    pub const TAG_SCENE_TYPE: u16 = 0xA301;
    pub const TAG_CFA_PATTERN: u16 = 0xA302;
    pub const TAG_CUSTOM_RENDERED: u16 = 0xA401;
    pub const TAG_EXPOSURE_MODE: u16 = 0xA402;
    pub const TAG_WHITE_BALANCE: u16 = 0xA403;
    pub const TAG_DIGITAL_ZOOM_RATIO: u16 = 0xA404;
    pub const TAG_FOCAL_LENGTH_IN_35MM_FILM: u16 = 0xA405;
    pub const TAG_SCENE_CAPTURE_TYPE: u16 = 0xA406;
    pub const TAG_GAIN_CONTROL: u16 = 0xA407;
    pub const TAG_CONTRAST: u16 = 0xA408;
    pub const TAG_SATURATION: u16 = 0xA409;
    pub const TAG_SHARPNESS: u16 = 0xA40A;
    pub const TAG_SUBJECT_DISTANCE_RANGE: u16 = 0xA40C;
    pub const TAG_IMAGE_UNIQUE_ID: u16 = 0xA420;
    pub const TAG_CAMERA_OWNER_NAME: u16 = 0xA430;
    pub const TAG_BODY_SERIAL_NUMBER: u16 = 0xA431;
    pub const TAG_LENS_SPECIFICATION: u16 = 0xA432;
    pub const TAG_LENS_MAKE: u16 = 0xA433;
    pub const TAG_LENS_MODEL: u16 = 0xA434;
    pub const TAG_LENS_SERIAL_NUMBER: u16 = 0xA435;
    pub const TAG_COMPOSITE_IMAGE: u16 = 0xA460;
    pub const TAG_SOURCE_IMAGE_NUMBER_OF_COMPOSITE_IMAGE: u16 = 0xA461;
    pub const TAG_SOURCE_EXPOSURE_TIMES_OF_COMPOSITE_IMAGE: u16 = 0xA462;
    pub const TAG_GAMMA: u16 = 0xA500;
    pub const TAG_GPS_VERSION_ID: u16 = 0x0000;
    pub const TAG_GPS_LATITUDE_REF: u16 = 0x0001;
    pub const TAG_GPS_LATITUDE: u16 = 0x0002;
    pub const TAG_GPS_LONGITUDE_REF: u16 = 0x0003;
    pub const TAG_GPS_LONGITUDE: u16 = 0x0004;
    pub const TAG_GPS_ALTITUDE_REF: u16 = 0x0005;
    pub const TAG_GPS_ALTITUDE: u16 = 0x0006;
    pub const TAG_GPS_TIME_STAMP: u16 = 0x0007;
    pub const TAG_GPS_SATELLITES: u16 = 0x0008;
    pub const TAG_GPS_STATUS: u16 = 0x0009;
    pub const TAG_GPS_MEASURE_MODE: u16 = 0x000A;
    pub const TAG_GPS_DOP: u16 = 0x000B;
    pub const TAG_GPS_SPEED_REF: u16 = 0x000C;
    pub const TAG_GPS_SPEED: u16 = 0x000D;
    pub const TAG_GPS_TRACK_REF: u16 = 0x000E;
    pub const TAG_GPS_TRACK: u16 = 0x000F;
    pub const TAG_GPS_IMG_DIRECTION_REF: u16 = 0x0010;
    pub const TAG_GPS_IMG_DIRECTION: u16 = 0x0011;
    pub const TAG_GPS_MAP_DATUM: u16 = 0x0012;
    pub const TAG_GPS_DEST_LATITUDE_REF: u16 = 0x0013;
    pub const TAG_GPS_DEST_LATITUDE: u16 = 0x0014;
    pub const TAG_GPS_DEST_LONGITUDE_REF: u16 = 0x0015;
    pub const TAG_GPS_DEST_LONGITUDE: u16 = 0x0016;
    pub const TAG_GPS_DEST_BEARING_REF: u16 = 0x0017;
    pub const TAG_GPS_DEST_BEARING: u16 = 0x0018;
    pub const TAG_GPS_DEST_DISTANCE_REF: u16 = 0x0019;
    pub const TAG_GPS_DEST_DISTANCE: u16 = 0x001A;
    pub const TAG_GPS_PROCESSING_METHOD: u16 = 0x001B;
    pub const TAG_GPS_AREA_INFORMATION: u16 = 0x001C;
    pub const TAG_GPS_DATE_STAMP: u16 = 0x001D;
    pub const TAG_GPS_DIFFERENTIAL: u16 = 0x001E;
    pub const TAG_GPS_H_POSITIONING_ERROR: u16 = 0x001F;
    pub const TAG_INTEROPERABILITY_INDEX: u16 = 0x0001;
    pub const TAG_INTEROPERABILITY_VERSION: u16 = 0x0002;
    pub const TAG_RELATED_IMAGE_FILE_FORMAT: u16 = 0x1000;
    pub const TAG_RELATED_IMAGE_WIDTH: u16 = 0x1001;
    pub const TAG_RELATED_IMAGE_LENGTH: u16 = 0x1002;

    pub fn parse(data: &'a [u8]) -> AvResult<Self> {
        if data.len() < Self::TIFF_HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "EXIF side data requires at least {} TIFF header bytes, got {}",
                Self::TIFF_HEADER_LEN,
                data.len()
            )));
        }

        let endian = if data.starts_with(&[0x49, 0x49, 0x2A, 0x00]) {
            FrameExifEndian::Little
        } else if data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
            FrameExifEndian::Big
        } else {
            return Err(AvError::invalid_data(
                "EXIF side data must start with a TIFF little-endian or big-endian header",
            ));
        };
        let first_ifd_offset = endian.read_u32(data, 4) as usize;
        Self::validate_ifd_offset(data, first_ifd_offset, "first")?;

        let ifds = Self::parse_ifd_chain(data, endian, first_ifd_offset)?;
        let linked_ifds = Self::parse_linked_ifds(data, endian, &ifds)?;
        Ok(Self {
            data,
            endian,
            first_ifd_offset,
            ifds,
            linked_ifds,
        })
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn endian(&self) -> FrameExifEndian {
        self.endian
    }

    pub const fn first_ifd_offset(&self) -> usize {
        self.first_ifd_offset
    }

    pub fn ifds(&self) -> &[FrameExifIfd<'a>] {
        &self.ifds
    }

    pub fn ifd(&self, index: usize) -> Option<&FrameExifIfd<'a>> {
        self.ifds.get(index)
    }

    pub fn ifd_count(&self) -> usize {
        self.ifds.len()
    }

    pub fn linked_ifds(&self) -> &[FrameExifLinkedIfd<'a>] {
        &self.linked_ifds
    }

    pub fn linked_ifd(&self, kind: FrameExifIfdPointerKind) -> Option<&FrameExifLinkedIfd<'a>> {
        self.linked_ifds.iter().find(|ifd| ifd.kind == kind)
    }

    pub fn linked_ifd_count(&self) -> usize {
        self.linked_ifds.len()
    }

    pub fn common_tags(&self) -> AvResult<FrameExifCommonTags<'a>> {
        let mut tags = FrameExifCommonTags::default();

        if let Some(root) = self.ifd(0) {
            tags.new_subfile_type = Self::optional_new_subfile_type_tag(root)?;
            tags.subfile_type = Self::optional_subfile_type_tag(root)?;
            tags.document_name =
                Self::optional_ascii_tag(root, Self::TAG_DOCUMENT_NAME, "DocumentName")?;
            tags.image_description =
                Self::optional_ascii_tag(root, Self::TAG_IMAGE_DESCRIPTION, "ImageDescription")?;
            tags.make = Self::optional_ascii_tag(root, Self::TAG_MAKE, "Make")?;
            tags.model = Self::optional_ascii_tag(root, Self::TAG_MODEL, "Model")?;
            tags.image_width = Self::optional_positive_short_or_long_tag(
                root,
                Self::TAG_IMAGE_WIDTH,
                "ImageWidth",
            )?;
            tags.image_length = Self::optional_positive_short_or_long_tag(
                root,
                Self::TAG_IMAGE_LENGTH,
                "ImageLength",
            )?;
            tags.compression = Self::optional_compression_tag(root)?;
            tags.photometric_interpretation = Self::optional_photometric_interpretation_tag(root)?;
            tags.thresholding = Self::optional_thresholding_tag(root)?;
            tags.fill_order = Self::optional_fill_order_tag(root)?;
            tags.samples_per_pixel = Self::optional_positive_short_tag(
                root,
                Self::TAG_SAMPLES_PER_PIXEL,
                "SamplesPerPixel",
            )?;
            tags.bits_per_sample =
                Self::optional_bits_per_sample_tag(root, tags.samples_per_pixel)?;
            tags.rows_per_strip = Self::optional_positive_short_or_long_tag(
                root,
                Self::TAG_ROWS_PER_STRIP,
                "RowsPerStrip",
            )?;
            tags.planar_configuration = Self::optional_planar_configuration_tag(root)?;
            tags.page_name = Self::optional_ascii_tag(root, Self::TAG_PAGE_NAME, "PageName")?;
            tags.white_point =
                Self::optional_rational_array_tag(root, Self::TAG_WHITE_POINT, "WhitePoint")?;
            tags.primary_chromaticities = Self::optional_rational_array_tag(
                root,
                Self::TAG_PRIMARY_CHROMATICITIES,
                "PrimaryChromaticities",
            )?;
            tags.ycbcr_coefficients = Self::optional_rational_array_tag(
                root,
                Self::TAG_YCBCR_COEFFICIENTS,
                "YCbCrCoefficients",
            )?;
            tags.ycbcr_sub_sampling = Self::optional_ycbcr_sub_sampling_tag(root)?;
            tags.ycbcr_positioning = Self::optional_ycbcr_positioning_tag(root)?;
            tags.reference_black_white = Self::optional_rational_array_tag(
                root,
                Self::TAG_REFERENCE_BLACK_WHITE,
                "ReferenceBlackWhite",
            )?;
            tags.orientation = Self::optional_orientation_tag(root)?;
            tags.x_resolution =
                Self::optional_rational_tag(root, Self::TAG_X_RESOLUTION, "XResolution")?;
            tags.y_resolution =
                Self::optional_rational_tag(root, Self::TAG_Y_RESOLUTION, "YResolution")?;
            tags.x_position = Self::optional_rational_tag(root, Self::TAG_X_POSITION, "XPosition")?;
            tags.y_position = Self::optional_rational_tag(root, Self::TAG_Y_POSITION, "YPosition")?;
            tags.resolution_unit = Self::optional_resolution_unit_tag(root)?;
            tags.page_number =
                Self::optional_short_array_tag::<2>(root, Self::TAG_PAGE_NUMBER, "PageNumber")?;
            tags.software = Self::optional_ascii_tag(root, Self::TAG_SOFTWARE, "Software")?;
            tags.date_time = Self::optional_datetime_tag(root, Self::TAG_DATE_TIME, "DateTime")?;
            tags.artist = Self::optional_ascii_tag(root, Self::TAG_ARTIST, "Artist")?;
            tags.host_computer =
                Self::optional_ascii_tag(root, Self::TAG_HOST_COMPUTER, "HostComputer")?;
            tags.predictor = Self::optional_predictor_tag(root)?;
            tags.copyright = Self::optional_ascii_tag(root, Self::TAG_COPYRIGHT, "Copyright")?;
        }

        if let Some(exif_ifd) = self.linked_ifd(FrameExifIfdPointerKind::Exif) {
            let ifd = exif_ifd.ifd();
            tags.exif_version =
                Self::optional_exif_version_tag(ifd, Self::TAG_EXIF_VERSION, "ExifVersion")?;
            tags.date_time_original =
                Self::optional_datetime_tag(ifd, Self::TAG_DATE_TIME_ORIGINAL, "DateTimeOriginal")?;
            tags.date_time_digitized = Self::optional_datetime_tag(
                ifd,
                Self::TAG_DATE_TIME_DIGITIZED,
                "DateTimeDigitized",
            )?;
            tags.offset_time =
                Self::optional_offset_time_tag(ifd, Self::TAG_OFFSET_TIME, "OffsetTime")?;
            tags.offset_time_original = Self::optional_offset_time_tag(
                ifd,
                Self::TAG_OFFSET_TIME_ORIGINAL,
                "OffsetTimeOriginal",
            )?;
            tags.offset_time_digitized = Self::optional_offset_time_tag(
                ifd,
                Self::TAG_OFFSET_TIME_DIGITIZED,
                "OffsetTimeDigitized",
            )?;
            tags.components_configuration = Self::optional_components_configuration_tag(ifd)?;
            tags.compressed_bits_per_pixel = Self::optional_rational_tag(
                ifd,
                Self::TAG_COMPRESSED_BITS_PER_PIXEL,
                "CompressedBitsPerPixel",
            )?;
            tags.exposure_program = Self::optional_exposure_program_tag(ifd)?;
            tags.exposure_time =
                Self::optional_rational_tag(ifd, Self::TAG_EXPOSURE_TIME, "ExposureTime")?;
            tags.f_number = Self::optional_rational_tag(ifd, Self::TAG_F_NUMBER, "FNumber")?;
            tags.spectral_sensitivity = Self::optional_ascii_tag(
                ifd,
                Self::TAG_SPECTRAL_SENSITIVITY,
                "SpectralSensitivity",
            )?;
            tags.photographic_sensitivity = Self::optional_short_tag(
                ifd,
                Self::TAG_PHOTOGRAPHIC_SENSITIVITY,
                "PhotographicSensitivity",
            )?;
            tags.oecf = Self::optional_undefined_bytes_tag(ifd, Self::TAG_OECF, "OECF")?;
            tags.sensitivity_type = Self::optional_sensitivity_type_tag(ifd)?;
            tags.standard_output_sensitivity = Self::optional_long_tag(
                ifd,
                Self::TAG_STANDARD_OUTPUT_SENSITIVITY,
                "StandardOutputSensitivity",
            )?;
            tags.recommended_exposure_index = Self::optional_long_tag(
                ifd,
                Self::TAG_RECOMMENDED_EXPOSURE_INDEX,
                "RecommendedExposureIndex",
            )?;
            tags.iso_speed = Self::optional_long_tag(ifd, Self::TAG_ISO_SPEED, "ISOSpeed")?;
            tags.iso_speed_latitude_yyy = Self::optional_long_tag(
                ifd,
                Self::TAG_ISO_SPEED_LATITUDE_YYY,
                "ISOSpeedLatitudeyyy",
            )?;
            tags.iso_speed_latitude_zzz = Self::optional_long_tag(
                ifd,
                Self::TAG_ISO_SPEED_LATITUDE_ZZZ,
                "ISOSpeedLatitudezzz",
            )?;
            tags.shutter_speed_value = Self::optional_signed_rational_tag(
                ifd,
                Self::TAG_SHUTTER_SPEED_VALUE,
                "ShutterSpeedValue",
            )?;
            tags.aperture_value =
                Self::optional_rational_tag(ifd, Self::TAG_APERTURE_VALUE, "ApertureValue")?;
            tags.brightness_value = Self::optional_signed_rational_tag(
                ifd,
                Self::TAG_BRIGHTNESS_VALUE,
                "BrightnessValue",
            )?;
            tags.exposure_bias_value = Self::optional_signed_rational_tag(
                ifd,
                Self::TAG_EXPOSURE_BIAS_VALUE,
                "ExposureBiasValue",
            )?;
            tags.max_aperture_value =
                Self::optional_rational_tag(ifd, Self::TAG_MAX_APERTURE_VALUE, "MaxApertureValue")?;
            tags.subject_distance =
                Self::optional_rational_tag(ifd, Self::TAG_SUBJECT_DISTANCE, "SubjectDistance")?;
            tags.metering_mode = Self::optional_metering_mode_tag(ifd)?;
            tags.light_source = Self::optional_light_source_tag(ifd)?;
            tags.flash = Self::optional_flash_tag(ifd)?;
            tags.focal_length =
                Self::optional_rational_tag(ifd, Self::TAG_FOCAL_LENGTH, "FocalLength")?;
            tags.subject_area = Self::optional_subject_area_tag(ifd)?;
            tags.maker_note =
                Self::optional_undefined_bytes_tag(ifd, Self::TAG_MAKER_NOTE, "MakerNote")?;
            tags.user_comment =
                Self::optional_undefined_bytes_tag(ifd, Self::TAG_USER_COMMENT, "UserComment")?;
            tags.sub_sec_time =
                Self::optional_subsecond_time_tag(ifd, Self::TAG_SUB_SEC_TIME, "SubSecTime")?;
            tags.sub_sec_time_original = Self::optional_subsecond_time_tag(
                ifd,
                Self::TAG_SUB_SEC_TIME_ORIGINAL,
                "SubSecTimeOriginal",
            )?;
            tags.sub_sec_time_digitized = Self::optional_subsecond_time_tag(
                ifd,
                Self::TAG_SUB_SEC_TIME_DIGITIZED,
                "SubSecTimeDigitized",
            )?;
            tags.flashpix_version = Self::optional_exif_version_tag(
                ifd,
                Self::TAG_FLASHPIX_VERSION,
                "FlashpixVersion",
            )?;
            tags.color_space = Self::optional_color_space_tag(ifd)?;
            tags.white_balance = Self::optional_white_balance_tag(ifd)?;
            tags.digital_zoom_ratio =
                Self::optional_rational_tag(ifd, Self::TAG_DIGITAL_ZOOM_RATIO, "DigitalZoomRatio")?;
            tags.focal_length_in_35mm_film = Self::optional_short_tag(
                ifd,
                Self::TAG_FOCAL_LENGTH_IN_35MM_FILM,
                "FocalLengthIn35mmFilm",
            )?;
            tags.flash_energy =
                Self::optional_rational_tag(ifd, Self::TAG_FLASH_ENERGY, "FlashEnergy")?;
            tags.spatial_frequency_response = Self::optional_undefined_bytes_tag(
                ifd,
                Self::TAG_SPATIAL_FREQUENCY_RESPONSE,
                "SpatialFrequencyResponse",
            )?;
            tags.focal_plane_x_resolution = Self::optional_rational_tag(
                ifd,
                Self::TAG_FOCAL_PLANE_X_RESOLUTION,
                "FocalPlaneXResolution",
            )?;
            tags.focal_plane_y_resolution = Self::optional_rational_tag(
                ifd,
                Self::TAG_FOCAL_PLANE_Y_RESOLUTION,
                "FocalPlaneYResolution",
            )?;
            tags.focal_plane_resolution_unit = Self::optional_focal_plane_resolution_unit_tag(ifd)?;
            tags.subject_location =
                Self::optional_short_array_tag(ifd, Self::TAG_SUBJECT_LOCATION, "SubjectLocation")?;
            tags.exposure_index =
                Self::optional_rational_tag(ifd, Self::TAG_EXPOSURE_INDEX, "ExposureIndex")?;
            tags.sensing_method = Self::optional_sensing_method_tag(ifd)?;
            tags.file_source = Self::optional_file_source_tag(ifd)?;
            tags.scene_type = Self::optional_scene_type_tag(ifd)?;
            tags.cfa_pattern =
                Self::optional_undefined_bytes_tag(ifd, Self::TAG_CFA_PATTERN, "CFAPattern")?;
            tags.custom_rendered = Self::optional_custom_rendered_tag(ifd)?;
            tags.exposure_mode = Self::optional_exposure_mode_tag(ifd)?;
            tags.scene_capture_type = Self::optional_scene_capture_type_tag(ifd)?;
            tags.gain_control = Self::optional_gain_control_tag(ifd)?;
            tags.contrast = Self::optional_contrast_tag(ifd)?;
            tags.saturation = Self::optional_saturation_tag(ifd)?;
            tags.sharpness = Self::optional_sharpness_tag(ifd)?;
            tags.subject_distance_range = Self::optional_subject_distance_range_tag(ifd)?;
            tags.pixel_x_dimension = Self::optional_positive_short_or_long_tag(
                ifd,
                Self::TAG_PIXEL_X_DIMENSION,
                "PixelXDimension",
            )?;
            tags.pixel_y_dimension = Self::optional_positive_short_or_long_tag(
                ifd,
                Self::TAG_PIXEL_Y_DIMENSION,
                "PixelYDimension",
            )?;
            tags.related_sound_file = Self::optional_ascii_exact_count_tag(
                ifd,
                Self::TAG_RELATED_SOUND_FILE,
                "RelatedSoundFile",
                13,
            )?;
            tags.temperature =
                Self::optional_signed_rational_tag(ifd, Self::TAG_TEMPERATURE, "Temperature")?;
            tags.humidity = Self::optional_rational_tag(ifd, Self::TAG_HUMIDITY, "Humidity")?;
            tags.pressure = Self::optional_rational_tag(ifd, Self::TAG_PRESSURE, "Pressure")?;
            tags.water_depth =
                Self::optional_signed_rational_tag(ifd, Self::TAG_WATER_DEPTH, "WaterDepth")?;
            tags.acceleration =
                Self::optional_rational_tag(ifd, Self::TAG_ACCELERATION, "Acceleration")?;
            tags.camera_elevation_angle = Self::optional_signed_rational_tag(
                ifd,
                Self::TAG_CAMERA_ELEVATION_ANGLE,
                "CameraElevationAngle",
            )?;
            tags.image_unique_id = Self::optional_ascii_exact_count_tag(
                ifd,
                Self::TAG_IMAGE_UNIQUE_ID,
                "ImageUniqueID",
                33,
            )?;
            tags.camera_owner_name =
                Self::optional_ascii_tag(ifd, Self::TAG_CAMERA_OWNER_NAME, "CameraOwnerName")?;
            tags.body_serial_number =
                Self::optional_ascii_tag(ifd, Self::TAG_BODY_SERIAL_NUMBER, "BodySerialNumber")?;
            tags.lens_specification = Self::optional_rational_array_tag(
                ifd,
                Self::TAG_LENS_SPECIFICATION,
                "LensSpecification",
            )?;
            tags.lens_make = Self::optional_ascii_tag(ifd, Self::TAG_LENS_MAKE, "LensMake")?;
            tags.lens_model = Self::optional_ascii_tag(ifd, Self::TAG_LENS_MODEL, "LensModel")?;
            tags.lens_serial_number =
                Self::optional_ascii_tag(ifd, Self::TAG_LENS_SERIAL_NUMBER, "LensSerialNumber")?;
            tags.composite_image = Self::optional_composite_image_tag(ifd)?;
            tags.source_image_number_of_composite_image = Self::optional_short_array_tag(
                ifd,
                Self::TAG_SOURCE_IMAGE_NUMBER_OF_COMPOSITE_IMAGE,
                "SourceImageNumberOfCompositeImage",
            )?;
            tags.source_exposure_times_of_composite_image = Self::optional_undefined_bytes_tag(
                ifd,
                Self::TAG_SOURCE_EXPOSURE_TIMES_OF_COMPOSITE_IMAGE,
                "SourceExposureTimesOfCompositeImage",
            )?;
            tags.gamma = Self::optional_rational_tag(ifd, Self::TAG_GAMMA, "Gamma")?;
        }

        if let Some(gps_ifd) = self.linked_ifd(FrameExifIfdPointerKind::Gps) {
            let ifd = gps_ifd.ifd();
            tags.gps_version_id =
                Self::optional_byte_array_tag(ifd, Self::TAG_GPS_VERSION_ID, "GPSVersionID")?;
            tags.gps_latitude_ref = Self::optional_gps_latitude_ref_tag(
                ifd,
                Self::TAG_GPS_LATITUDE_REF,
                "GPSLatitudeRef",
            )?;
            tags.gps_latitude =
                Self::optional_gps_coordinate_tag(ifd, Self::TAG_GPS_LATITUDE, "GPSLatitude", 90)?;
            tags.gps_longitude_ref = Self::optional_gps_longitude_ref_tag(
                ifd,
                Self::TAG_GPS_LONGITUDE_REF,
                "GPSLongitudeRef",
            )?;
            tags.gps_longitude = Self::optional_gps_coordinate_tag(
                ifd,
                Self::TAG_GPS_LONGITUDE,
                "GPSLongitude",
                180,
            )?;
            tags.gps_altitude_ref = Self::optional_gps_altitude_ref_tag(ifd)?;
            tags.gps_altitude =
                Self::optional_rational_tag(ifd, Self::TAG_GPS_ALTITUDE, "GPSAltitude")?;
            tags.gps_time_stamp = Self::optional_gps_time_stamp_tag(ifd)?;
            tags.gps_satellites =
                Self::optional_ascii_tag(ifd, Self::TAG_GPS_SATELLITES, "GPSSatellites")?;
            tags.gps_status = Self::optional_gps_status_tag(ifd)?;
            tags.gps_measure_mode = Self::optional_gps_measure_mode_tag(ifd)?;
            tags.gps_dop = Self::optional_rational_tag(ifd, Self::TAG_GPS_DOP, "GPSDOP")?;
            tags.gps_speed_ref = Self::optional_gps_speed_ref_tag(ifd)?;
            tags.gps_speed = Self::optional_rational_tag(ifd, Self::TAG_GPS_SPEED, "GPSSpeed")?;
            tags.gps_track_ref =
                Self::optional_gps_direction_ref_tag(ifd, Self::TAG_GPS_TRACK_REF, "GPSTrackRef")?;
            tags.gps_track =
                Self::optional_gps_direction_tag(ifd, Self::TAG_GPS_TRACK, "GPSTrack")?;
            tags.gps_img_direction_ref = Self::optional_gps_direction_ref_tag(
                ifd,
                Self::TAG_GPS_IMG_DIRECTION_REF,
                "GPSImgDirectionRef",
            )?;
            tags.gps_img_direction = Self::optional_gps_direction_tag(
                ifd,
                Self::TAG_GPS_IMG_DIRECTION,
                "GPSImgDirection",
            )?;
            tags.gps_map_datum =
                Self::optional_ascii_tag(ifd, Self::TAG_GPS_MAP_DATUM, "GPSMapDatum")?;
            tags.gps_dest_latitude_ref = Self::optional_gps_latitude_ref_tag(
                ifd,
                Self::TAG_GPS_DEST_LATITUDE_REF,
                "GPSDestLatitudeRef",
            )?;
            tags.gps_dest_latitude = Self::optional_gps_coordinate_tag(
                ifd,
                Self::TAG_GPS_DEST_LATITUDE,
                "GPSDestLatitude",
                90,
            )?;
            tags.gps_dest_longitude_ref = Self::optional_gps_longitude_ref_tag(
                ifd,
                Self::TAG_GPS_DEST_LONGITUDE_REF,
                "GPSDestLongitudeRef",
            )?;
            tags.gps_dest_longitude = Self::optional_gps_coordinate_tag(
                ifd,
                Self::TAG_GPS_DEST_LONGITUDE,
                "GPSDestLongitude",
                180,
            )?;
            tags.gps_dest_bearing_ref = Self::optional_gps_direction_ref_tag(
                ifd,
                Self::TAG_GPS_DEST_BEARING_REF,
                "GPSDestBearingRef",
            )?;
            tags.gps_dest_bearing = Self::optional_gps_direction_tag(
                ifd,
                Self::TAG_GPS_DEST_BEARING,
                "GPSDestBearing",
            )?;
            tags.gps_dest_distance_ref = Self::optional_gps_distance_ref_tag(ifd)?;
            tags.gps_dest_distance =
                Self::optional_rational_tag(ifd, Self::TAG_GPS_DEST_DISTANCE, "GPSDestDistance")?;
            tags.gps_processing_method = Self::optional_undefined_bytes_tag(
                ifd,
                Self::TAG_GPS_PROCESSING_METHOD,
                "GPSProcessingMethod",
            )?;
            tags.gps_area_information = Self::optional_undefined_bytes_tag(
                ifd,
                Self::TAG_GPS_AREA_INFORMATION,
                "GPSAreaInformation",
            )?;
            tags.gps_date_stamp = Self::optional_gps_date_stamp_tag(ifd)?;
            tags.gps_differential = Self::optional_gps_differential_tag(ifd)?;
            tags.gps_h_positioning_error = Self::optional_rational_tag(
                ifd,
                Self::TAG_GPS_H_POSITIONING_ERROR,
                "GPSHPositioningError",
            )?;
        }

        if let Some(interop_ifd) = self.linked_ifd(FrameExifIfdPointerKind::Interoperability) {
            let ifd = interop_ifd.ifd();
            tags.interoperability_index = Self::optional_ascii_tag(
                ifd,
                Self::TAG_INTEROPERABILITY_INDEX,
                "InteroperabilityIndex",
            )?;
            tags.interoperability_version = Self::optional_exif_version_tag(
                ifd,
                Self::TAG_INTEROPERABILITY_VERSION,
                "InteroperabilityVersion",
            )?;
            tags.related_image_file_format = Self::optional_ascii_tag(
                ifd,
                Self::TAG_RELATED_IMAGE_FILE_FORMAT,
                "RelatedImageFileFormat",
            )?;
            tags.related_image_width = Self::optional_positive_short_or_long_tag(
                ifd,
                Self::TAG_RELATED_IMAGE_WIDTH,
                "RelatedImageWidth",
            )?;
            tags.related_image_length = Self::optional_positive_short_or_long_tag(
                ifd,
                Self::TAG_RELATED_IMAGE_LENGTH,
                "RelatedImageLength",
            )?;
        }

        Ok(tags)
    }

    fn optional_ascii_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<&'a str>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        Self::single_ascii_entry(entry, label).map(Some)
    }

    fn optional_ascii_exact_count_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
        expected_count: u32,
    ) -> AvResult<Option<&'a str>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() != expected_count {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly {expected_count} ASCII bytes, got {}",
                    entry.count()
                ),
            ));
        }
        Self::single_ascii_entry(entry, label).map(Some)
    }

    fn single_ascii_entry(entry: FrameExifEntry<'a>, label: &str) -> AvResult<&'a str> {
        let strings = entry.ascii_strings()?.ok_or_else(|| {
            Self::semantic_tag_error(label, entry.tag(), "must have ASCII TIFF type")
        })?;
        if strings.len() != 1 {
            return Err(Self::semantic_tag_error(
                label,
                entry.tag(),
                format!(
                    "must contain exactly one ASCII string, got {}",
                    strings.len()
                ),
            ));
        }
        Ok(strings[0])
    }

    fn optional_offset_time_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<&'a str>> {
        let Some(value) = Self::optional_ascii_exact_count_tag(ifd, tag, label, 7)? else {
            return Ok(None);
        };
        Self::validate_offset_time_ascii(label, tag, value)?;
        Ok(Some(value))
    }

    fn optional_datetime_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<&'a str>> {
        let Some(value) = Self::optional_ascii_exact_count_tag(ifd, tag, label, 20)? else {
            return Ok(None);
        };
        Self::validate_datetime_ascii(label, tag, value)?;
        Ok(Some(value))
    }

    fn optional_subsecond_time_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<&'a str>> {
        let Some(value) = Self::optional_ascii_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        if !value.as_bytes().iter().all(u8::is_ascii_digit) {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "must contain only digits",
            ));
        }
        Ok(Some(value))
    }

    fn optional_gps_date_stamp_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<&'a str>> {
        let Some(value) = Self::optional_ascii_exact_count_tag(
            ifd,
            Self::TAG_GPS_DATE_STAMP,
            "GPSDateStamp",
            11,
        )?
        else {
            return Ok(None);
        };
        Self::validate_gps_date_stamp_ascii(value)?;
        Ok(Some(value))
    }

    fn validate_datetime_ascii(label: &str, tag: u16, value: &str) -> AvResult<()> {
        let bytes = value.as_bytes();
        let valid = bytes.len() == 19
            && bytes.iter().enumerate().all(|(index, byte)| match index {
                4 | 7 | 13 | 16 => *byte == b':',
                10 => *byte == b' ',
                _ => byte.is_ascii_digit(),
            });
        if !valid {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "must match `YYYY:MM:DD HH:MM:SS`",
            ));
        }
        let month = Self::ascii_two_digits(bytes, 5);
        let day = Self::ascii_two_digits(bytes, 8);
        let year = Self::ascii_four_digits(bytes, 0);
        let hour = Self::ascii_two_digits(bytes, 11);
        let minute = Self::ascii_two_digits(bytes, 14);
        let second = Self::ascii_two_digits(bytes, 17);
        Self::validate_calendar_date(label, tag, year, month, day)?;
        Self::validate_clock_time(label, tag, hour, minute, second)?;
        Ok(())
    }

    fn validate_offset_time_ascii(label: &str, tag: u16, value: &str) -> AvResult<()> {
        let bytes = value.as_bytes();
        let valid = bytes.len() == 6
            && matches!(bytes.first().copied(), Some(b'+' | b'-'))
            && bytes[1].is_ascii_digit()
            && bytes[2].is_ascii_digit()
            && bytes[3] == b':'
            && bytes[4].is_ascii_digit()
            && bytes[5].is_ascii_digit();
        if !valid {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "must match `[+-]HH:MM`",
            ));
        }
        let hour = Self::ascii_two_digits(bytes, 1);
        let minute = Self::ascii_two_digits(bytes, 4);
        if hour > 23 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "hour must be in 0..=23",
            ));
        }
        if minute > 59 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "minute must be in 0..=59",
            ));
        }
        Ok(())
    }

    fn validate_gps_date_stamp_ascii(value: &str) -> AvResult<()> {
        let bytes = value.as_bytes();
        let valid = bytes.len() == 10
            && bytes.iter().enumerate().all(|(index, byte)| match index {
                4 | 7 => *byte == b':',
                _ => byte.is_ascii_digit(),
            });
        if !valid {
            return Err(Self::semantic_tag_error(
                "GPSDateStamp",
                Self::TAG_GPS_DATE_STAMP,
                "must match `YYYY:MM:DD`",
            ));
        }
        let year = Self::ascii_four_digits(bytes, 0);
        let month = Self::ascii_two_digits(bytes, 5);
        let day = Self::ascii_two_digits(bytes, 8);
        Self::validate_calendar_date("GPSDateStamp", Self::TAG_GPS_DATE_STAMP, year, month, day)?;
        Ok(())
    }

    fn ascii_four_digits(bytes: &[u8], index: usize) -> u16 {
        ((bytes[index] - b'0') as u16) * 1000
            + ((bytes[index + 1] - b'0') as u16) * 100
            + ((bytes[index + 2] - b'0') as u16) * 10
            + (bytes[index + 3] - b'0') as u16
    }

    fn ascii_two_digits(bytes: &[u8], index: usize) -> u8 {
        (bytes[index] - b'0') * 10 + (bytes[index + 1] - b'0')
    }

    fn validate_calendar_date(
        label: &str,
        tag: u16,
        year: u16,
        month: u8,
        day: u8,
    ) -> AvResult<()> {
        if !(1..=12).contains(&month) {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "month must be in 1..=12",
            ));
        }
        if !(1..=31).contains(&day) {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "day must be in 1..=31",
            ));
        }
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            _ => unreachable!("month range checked above"),
        };
        if day > max_day {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "day is outside the valid range for month",
            ));
        }
        Ok(())
    }

    fn is_leap_year(year: u16) -> bool {
        Self::divisible_by(year, 4)
            && (!Self::divisible_by(year, 100) || Self::divisible_by(year, 400))
    }

    fn divisible_by(value: u16, divisor: u16) -> bool {
        (value / divisor) * divisor == value
    }

    fn validate_clock_time(
        label: &str,
        tag: u16,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> AvResult<()> {
        if hour > 23 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "hour must be in 0..=23",
            ));
        }
        if minute > 59 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "minute must be in 0..=59",
            ));
        }
        if second > 59 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "second must be in 0..=59",
            ));
        }
        Ok(())
    }

    fn optional_short_or_long_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<u32>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() != 1 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!("must contain exactly one value, got {}", entry.count()),
            ));
        }

        match entry.tiff_type() {
            FrameExifTiffType::Short => Ok(Some(
                entry
                    .short_values()?
                    .expect("SHORT type must decode as SHORT values")[0] as u32,
            )),
            FrameExifTiffType::Long => Ok(Some(
                entry
                    .long_values()?
                    .expect("LONG type must decode as LONG values")[0],
            )),
            _ => Err(Self::semantic_tag_error(
                label,
                tag,
                "must have SHORT or LONG TIFF type",
            )),
        }
    }

    fn optional_positive_short_or_long_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<u32>> {
        let Some(value) = Self::optional_short_or_long_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        if value == 0 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "must be greater than zero",
            ));
        }
        Ok(Some(value))
    }

    fn optional_positive_short_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<u16>> {
        let Some(value) = Self::optional_short_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        if value == 0 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "must be greater than zero",
            ));
        }
        Ok(Some(value))
    }

    fn optional_new_subfile_type_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifNewSubfileType>> {
        let Some(raw) = Self::optional_long_tag(ifd, Self::TAG_NEW_SUBFILE_TYPE, "NewSubfileType")?
        else {
            return Ok(None);
        };
        FrameExifNewSubfileType::from_raw(raw).map(Some)
    }

    fn optional_subfile_type_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifSubfileType>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_SUBFILE_TYPE, "SubfileType")?
        else {
            return Ok(None);
        };
        FrameExifSubfileType::from_raw(raw).map(Some)
    }

    fn optional_planar_configuration_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifPlanarConfiguration>> {
        let Some(raw) =
            Self::optional_short_tag(ifd, Self::TAG_PLANAR_CONFIGURATION, "PlanarConfiguration")?
        else {
            return Ok(None);
        };
        FrameExifPlanarConfiguration::from_raw(raw).map(Some)
    }

    fn optional_compression_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifCompression>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_COMPRESSION, "Compression")? else {
            return Ok(None);
        };
        FrameExifCompression::from_raw(raw).map(Some)
    }

    fn optional_predictor_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifPredictor>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_PREDICTOR, "Predictor")? else {
            return Ok(None);
        };
        FrameExifPredictor::from_raw(raw).map(Some)
    }

    fn optional_photometric_interpretation_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifPhotometricInterpretation>> {
        let Some(raw) = Self::optional_short_tag(
            ifd,
            Self::TAG_PHOTOMETRIC_INTERPRETATION,
            "PhotometricInterpretation",
        )?
        else {
            return Ok(None);
        };
        Ok(Some(FrameExifPhotometricInterpretation::from_raw(raw)))
    }

    fn optional_thresholding_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifThresholding>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_THRESHOLDING, "Thresholding")?
        else {
            return Ok(None);
        };
        FrameExifThresholding::from_raw(raw).map(Some)
    }

    fn optional_fill_order_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifFillOrder>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_FILL_ORDER, "FillOrder")? else {
            return Ok(None);
        };
        FrameExifFillOrder::from_raw(raw).map(Some)
    }

    fn optional_bits_per_sample_tag(
        ifd: &FrameExifIfd<'a>,
        samples_per_pixel: Option<u16>,
    ) -> AvResult<Option<FrameExifBitsPerSample<'a>>> {
        let Some(entry) = ifd.entry_by_tag(Self::TAG_BITS_PER_SAMPLE) else {
            return Ok(None);
        };
        if entry.count() == 0 {
            return Err(Self::semantic_tag_error(
                "BitsPerSample",
                Self::TAG_BITS_PER_SAMPLE,
                "must contain at least one SHORT value",
            ));
        }
        if let Some(expected) = samples_per_pixel {
            if entry.count() != u32::from(expected) {
                return Err(Self::semantic_tag_error(
                    "BitsPerSample",
                    Self::TAG_BITS_PER_SAMPLE,
                    format!(
                        "count {} must match SamplesPerPixel {expected}",
                        entry.count()
                    ),
                ));
            }
        }
        let values = entry.short_values()?.ok_or_else(|| {
            Self::semantic_tag_error(
                "BitsPerSample",
                Self::TAG_BITS_PER_SAMPLE,
                "must have SHORT TIFF type",
            )
        })?;
        if values.contains(&0) {
            return Err(Self::semantic_tag_error(
                "BitsPerSample",
                Self::TAG_BITS_PER_SAMPLE,
                "bit depths must be greater than zero",
            ));
        }
        Ok(Some(FrameExifBitsPerSample { entry }))
    }

    fn optional_ycbcr_sub_sampling_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<[u16; 2]>> {
        let Some(values) =
            Self::optional_short_array_tag(ifd, Self::TAG_YCBCR_SUB_SAMPLING, "YCbCrSubSampling")?
        else {
            return Ok(None);
        };
        if values[0] == 0 || values[1] == 0 {
            return Err(Self::semantic_tag_error(
                "YCbCrSubSampling",
                Self::TAG_YCBCR_SUB_SAMPLING,
                "horizontal and vertical sampling factors must be non-zero",
            ));
        }
        Ok(Some(values))
    }

    fn optional_ycbcr_positioning_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifYcbCrPositioning>> {
        let Some(raw) =
            Self::optional_short_tag(ifd, Self::TAG_YCBCR_POSITIONING, "YCbCrPositioning")?
        else {
            return Ok(None);
        };
        FrameExifYcbCrPositioning::from_raw(raw).map(Some)
    }

    fn optional_orientation_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifOrientation>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_ORIENTATION, "Orientation")? else {
            return Ok(None);
        };
        FrameExifOrientation::from_raw(raw).map(Some)
    }

    fn optional_resolution_unit_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifResolutionUnit>> {
        Self::optional_resolution_unit_by_tag(ifd, Self::TAG_RESOLUTION_UNIT, "ResolutionUnit")
    }

    fn optional_focal_plane_resolution_unit_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifResolutionUnit>> {
        Self::optional_resolution_unit_by_tag(
            ifd,
            Self::TAG_FOCAL_PLANE_RESOLUTION_UNIT,
            "FocalPlaneResolutionUnit",
        )
    }

    fn optional_resolution_unit_by_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<FrameExifResolutionUnit>> {
        let Some(raw) = Self::optional_short_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        FrameExifResolutionUnit::from_raw(raw).map(Some)
    }

    fn optional_exposure_program_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifExposureProgram>> {
        let Some(raw) =
            Self::optional_short_tag(ifd, Self::TAG_EXPOSURE_PROGRAM, "ExposureProgram")?
        else {
            return Ok(None);
        };
        FrameExifExposureProgram::from_raw(raw).map(Some)
    }

    fn optional_sensitivity_type_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifSensitivityType>> {
        let Some(raw) =
            Self::optional_short_tag(ifd, Self::TAG_SENSITIVITY_TYPE, "SensitivityType")?
        else {
            return Ok(None);
        };
        FrameExifSensitivityType::from_raw(raw).map(Some)
    }

    fn optional_metering_mode_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifMeteringMode>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_METERING_MODE, "MeteringMode")?
        else {
            return Ok(None);
        };
        FrameExifMeteringMode::from_raw(raw).map(Some)
    }

    fn optional_light_source_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifLightSource>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_LIGHT_SOURCE, "LightSource")?
        else {
            return Ok(None);
        };
        FrameExifLightSource::from_raw(raw).map(Some)
    }

    fn optional_flash_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifFlash>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_FLASH, "Flash")? else {
            return Ok(None);
        };
        Ok(Some(FrameExifFlash::from_raw(raw)))
    }

    fn optional_subject_area_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifSubjectArea>> {
        let Some(entry) = ifd.entry_by_tag(Self::TAG_SUBJECT_AREA) else {
            return Ok(None);
        };
        if !(2..=4).contains(&entry.count()) {
            return Err(Self::semantic_tag_error(
                "SubjectArea",
                Self::TAG_SUBJECT_AREA,
                format!(
                    "must contain 2, 3, or 4 SHORT values, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry.short_values()?.ok_or_else(|| {
            Self::semantic_tag_error(
                "SubjectArea",
                Self::TAG_SUBJECT_AREA,
                "must have SHORT TIFF type",
            )
        })?;
        let subject_area = match values.as_slice() {
            [x, y] => FrameExifSubjectArea::point(*x, *y),
            [x, y, diameter] => {
                if *diameter == 0 {
                    return Err(Self::semantic_tag_error(
                        "SubjectArea",
                        Self::TAG_SUBJECT_AREA,
                        "circle diameter must be non-zero",
                    ));
                }
                FrameExifSubjectArea::circle(*x, *y, *diameter)
            }
            [x, y, width, height] => {
                if *width == 0 || *height == 0 {
                    return Err(Self::semantic_tag_error(
                        "SubjectArea",
                        Self::TAG_SUBJECT_AREA,
                        "rectangle width and height must be non-zero",
                    ));
                }
                FrameExifSubjectArea::rectangle(*x, *y, *width, *height)
            }
            _ => unreachable!("SubjectArea count was validated before value extraction"),
        };
        Ok(Some(subject_area))
    }

    fn optional_color_space_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifColorSpace>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_COLOR_SPACE, "ColorSpace")? else {
            return Ok(None);
        };
        FrameExifColorSpace::from_raw(raw).map(Some)
    }

    fn optional_white_balance_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifWhiteBalance>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_WHITE_BALANCE, "WhiteBalance")?
        else {
            return Ok(None);
        };
        FrameExifWhiteBalance::from_raw(raw).map(Some)
    }

    fn optional_sensing_method_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifSensingMethod>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_SENSING_METHOD, "SensingMethod")?
        else {
            return Ok(None);
        };
        FrameExifSensingMethod::from_raw(raw).map(Some)
    }

    fn optional_file_source_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifFileSource>> {
        let Some(raw) =
            Self::optional_undefined_array_tag::<1>(ifd, Self::TAG_FILE_SOURCE, "FileSource")?
        else {
            return Ok(None);
        };
        FrameExifFileSource::from_raw(raw[0]).map(Some)
    }

    fn optional_scene_type_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifSceneType>> {
        let Some(raw) =
            Self::optional_undefined_array_tag::<1>(ifd, Self::TAG_SCENE_TYPE, "SceneType")?
        else {
            return Ok(None);
        };
        FrameExifSceneType::from_raw(raw[0]).map(Some)
    }

    fn optional_custom_rendered_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifCustomRendered>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_CUSTOM_RENDERED, "CustomRendered")?
        else {
            return Ok(None);
        };
        FrameExifCustomRendered::from_raw(raw).map(Some)
    }

    fn optional_exposure_mode_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifExposureMode>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_EXPOSURE_MODE, "ExposureMode")?
        else {
            return Ok(None);
        };
        FrameExifExposureMode::from_raw(raw).map(Some)
    }

    fn optional_scene_capture_type_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifSceneCaptureType>> {
        let Some(raw) =
            Self::optional_short_tag(ifd, Self::TAG_SCENE_CAPTURE_TYPE, "SceneCaptureType")?
        else {
            return Ok(None);
        };
        FrameExifSceneCaptureType::from_raw(raw).map(Some)
    }

    fn optional_gain_control_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifGainControl>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_GAIN_CONTROL, "GainControl")?
        else {
            return Ok(None);
        };
        FrameExifGainControl::from_raw(raw).map(Some)
    }

    fn optional_contrast_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifContrast>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_CONTRAST, "Contrast")? else {
            return Ok(None);
        };
        FrameExifContrast::from_raw(raw).map(Some)
    }

    fn optional_saturation_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifSaturation>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_SATURATION, "Saturation")? else {
            return Ok(None);
        };
        FrameExifSaturation::from_raw(raw).map(Some)
    }

    fn optional_sharpness_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifSharpness>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_SHARPNESS, "Sharpness")? else {
            return Ok(None);
        };
        FrameExifSharpness::from_raw(raw).map(Some)
    }

    fn optional_subject_distance_range_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifSubjectDistanceRange>> {
        let Some(raw) = Self::optional_short_tag(
            ifd,
            Self::TAG_SUBJECT_DISTANCE_RANGE,
            "SubjectDistanceRange",
        )?
        else {
            return Ok(None);
        };
        FrameExifSubjectDistanceRange::from_raw(raw).map(Some)
    }

    fn optional_composite_image_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifCompositeImage>> {
        let Some(raw) = Self::optional_short_tag(ifd, Self::TAG_COMPOSITE_IMAGE, "CompositeImage")?
        else {
            return Ok(None);
        };
        FrameExifCompositeImage::from_raw(raw).map(Some)
    }

    fn optional_short_tag(ifd: &FrameExifIfd<'a>, tag: u16, label: &str) -> AvResult<Option<u16>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() != 1 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly one SHORT value, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry
            .short_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have SHORT TIFF type"))?;
        Ok(Some(values[0]))
    }

    fn optional_long_tag(ifd: &FrameExifIfd<'a>, tag: u16, label: &str) -> AvResult<Option<u32>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() != 1 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!("must contain exactly one LONG value, got {}", entry.count()),
            ));
        }
        let values = entry
            .long_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have LONG TIFF type"))?;
        Ok(Some(values[0]))
    }

    fn optional_short_array_tag<const N: usize>(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<[u16; N]>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() as usize != N {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly {N} SHORT values, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry
            .short_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have SHORT TIFF type"))?;
        let mut array = [0; N];
        array.copy_from_slice(&values);
        Ok(Some(array))
    }

    fn optional_rational_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<FrameExifRational>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() != 1 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly one unsigned rational value, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry
            .rational_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have RATIONAL TIFF type"))?;
        Ok(Some(values[0]))
    }

    fn optional_signed_rational_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<FrameExifSignedRational>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() != 1 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly one signed rational value, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry
            .signed_rational_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have SRATIONAL TIFF type"))?;
        Ok(Some(values[0]))
    }

    fn optional_rational_array3_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<[FrameExifRational; 3]>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() != 3 {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly three unsigned rational values, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry
            .rational_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have RATIONAL TIFF type"))?;
        Ok(Some([values[0], values[1], values[2]]))
    }

    fn optional_gps_coordinate_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
        max_degrees: u32,
    ) -> AvResult<Option<[FrameExifRational; 3]>> {
        let Some(values) = Self::optional_rational_array3_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        if !Self::rational_less_or_equal(values[0], max_degrees)
            || !Self::rational_less_than(values[1], 60)
            || !Self::rational_less_than(values[2], 60)
            || !Self::dms_less_or_equal(values, max_degrees)
        {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must be a valid DMS coordinate in 0..={max_degrees} degrees with minutes and seconds below 60"
                ),
            ));
        }
        Ok(Some(values))
    }

    fn optional_gps_time_stamp_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<[FrameExifRational; 3]>> {
        let Some(values) =
            Self::optional_rational_array3_tag(ifd, Self::TAG_GPS_TIME_STAMP, "GPSTimeStamp")?
        else {
            return Ok(None);
        };
        if !Self::rational_less_than(values[0], 24)
            || !Self::rational_less_than(values[1], 60)
            || !Self::rational_less_than(values[2], 60)
            || !Self::dms_less_than(values, 24)
        {
            return Err(Self::semantic_tag_error(
                "GPSTimeStamp",
                Self::TAG_GPS_TIME_STAMP,
                "must be a valid UTC time with hours below 24 and minutes/seconds below 60",
            ));
        }
        Ok(Some(values))
    }

    fn optional_gps_direction_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<FrameExifRational>> {
        let Some(value) = Self::optional_rational_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        if !Self::rational_less_than(value, 360) {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "must be a compass direction below 360 degrees",
            ));
        }
        Ok(Some(value))
    }

    fn rational_less_than(value: FrameExifRational, upper: u32) -> bool {
        (value.numerator as u128) < (upper as u128) * (value.denominator as u128)
    }

    fn rational_less_or_equal(value: FrameExifRational, upper: u32) -> bool {
        (value.numerator as u128) <= (upper as u128) * (value.denominator as u128)
    }

    fn dms_less_than(values: [FrameExifRational; 3], upper_degrees: u32) -> bool {
        Self::dms_scaled_numerator(values)
            < (upper_degrees as u128) * 3600 * Self::dms_scaled_denominator(values)
    }

    fn dms_less_or_equal(values: [FrameExifRational; 3], upper_degrees: u32) -> bool {
        Self::dms_scaled_numerator(values)
            <= (upper_degrees as u128) * 3600 * Self::dms_scaled_denominator(values)
    }

    fn dms_scaled_numerator(values: [FrameExifRational; 3]) -> u128 {
        let [degrees, minutes, seconds] = values;
        (degrees.numerator as u128)
            * 3600
            * (minutes.denominator as u128)
            * (seconds.denominator as u128)
            + (minutes.numerator as u128)
                * 60
                * (degrees.denominator as u128)
                * (seconds.denominator as u128)
            + (seconds.numerator as u128)
                * (degrees.denominator as u128)
                * (minutes.denominator as u128)
    }

    fn dms_scaled_denominator(values: [FrameExifRational; 3]) -> u128 {
        values.iter().fold(1u128, |product, value| {
            product * (value.denominator as u128)
        })
    }

    fn optional_rational_array_tag<const N: usize>(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<[FrameExifRational; N]>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() as usize != N {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly {N} unsigned rational values, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry
            .rational_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have RATIONAL TIFF type"))?;
        let mut array = [FrameExifRational {
            numerator: 0,
            denominator: 1,
        }; N];
        array.copy_from_slice(&values);
        Ok(Some(array))
    }

    fn optional_byte_array_tag<const N: usize>(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<[u8; N]>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() as usize != N {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly {N} BYTE values, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry
            .byte_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have BYTE TIFF type"))?;
        let mut array = [0; N];
        array.copy_from_slice(values);
        Ok(Some(array))
    }

    fn optional_undefined_bytes_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<&'a [u8]>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        let values = entry
            .undefined_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have UNDEFINED TIFF type"))?;
        Ok(Some(values))
    }

    fn optional_undefined_array_tag<const N: usize>(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<[u8; N]>> {
        let Some(entry) = ifd.entry_by_tag(tag) else {
            return Ok(None);
        };
        if entry.count() as usize != N {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                format!(
                    "must contain exactly {N} UNDEFINED bytes, got {}",
                    entry.count()
                ),
            ));
        }
        let values = entry
            .undefined_values()?
            .ok_or_else(|| Self::semantic_tag_error(label, tag, "must have UNDEFINED TIFF type"))?;
        let mut array = [0; N];
        array.copy_from_slice(values);
        Ok(Some(array))
    }

    fn optional_exif_version_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<[u8; 4]>> {
        let Some(value) = Self::optional_undefined_array_tag::<4>(ifd, tag, label)? else {
            return Ok(None);
        };
        if !value.iter().all(u8::is_ascii_digit) {
            return Err(Self::semantic_tag_error(
                label,
                tag,
                "must contain four ASCII digit bytes",
            ));
        }
        Ok(Some(value))
    }

    fn optional_components_configuration_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<[u8; 4]>> {
        let Some(value) = Self::optional_undefined_array_tag::<4>(
            ifd,
            Self::TAG_COMPONENTS_CONFIGURATION,
            "ComponentsConfiguration",
        )?
        else {
            return Ok(None);
        };
        if let Some(component) = value.iter().find(|&&component| component > 6) {
            return Err(Self::semantic_tag_error(
                "ComponentsConfiguration",
                Self::TAG_COMPONENTS_CONFIGURATION,
                format!("component identifiers must be in 0..=6, got {component}"),
            ));
        }
        Ok(Some(value))
    }

    fn optional_gps_latitude_ref_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<FrameExifGpsLatitudeRef>> {
        let Some(value) = Self::optional_gps_ascii_ref_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        match value {
            "N" => Ok(Some(FrameExifGpsLatitudeRef::North)),
            "S" => Ok(Some(FrameExifGpsLatitudeRef::South)),
            _ => Err(Self::semantic_tag_error(
                label,
                tag,
                format!("must be `N` or `S`, got `{value}`"),
            )),
        }
    }

    fn optional_gps_longitude_ref_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<FrameExifGpsLongitudeRef>> {
        let Some(value) = Self::optional_gps_ascii_ref_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        match value {
            "E" => Ok(Some(FrameExifGpsLongitudeRef::East)),
            "W" => Ok(Some(FrameExifGpsLongitudeRef::West)),
            _ => Err(Self::semantic_tag_error(
                label,
                tag,
                format!("must be `E` or `W`, got `{value}`"),
            )),
        }
    }

    fn optional_gps_altitude_ref_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifGpsAltitudeRef>> {
        let Some(value) =
            Self::optional_byte_array_tag::<1>(ifd, Self::TAG_GPS_ALTITUDE_REF, "GPSAltitudeRef")?
        else {
            return Ok(None);
        };
        FrameExifGpsAltitudeRef::from_raw(value[0]).map(Some)
    }

    fn optional_gps_status_tag(ifd: &FrameExifIfd<'a>) -> AvResult<Option<FrameExifGpsStatus>> {
        let Some(value) = Self::optional_gps_ascii_ref_tag(ifd, Self::TAG_GPS_STATUS, "GPSStatus")?
        else {
            return Ok(None);
        };
        match value {
            "A" => Ok(Some(FrameExifGpsStatus::MeasurementInProgress)),
            "V" => Ok(Some(FrameExifGpsStatus::MeasurementVoid)),
            _ => Err(Self::semantic_tag_error(
                "GPSStatus",
                Self::TAG_GPS_STATUS,
                format!("must be `A` or `V`, got `{value}`"),
            )),
        }
    }

    fn optional_gps_measure_mode_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifGpsMeasureMode>> {
        let Some(value) =
            Self::optional_gps_ascii_ref_tag(ifd, Self::TAG_GPS_MEASURE_MODE, "GPSMeasureMode")?
        else {
            return Ok(None);
        };
        match value {
            "2" => Ok(Some(FrameExifGpsMeasureMode::TwoDimensional)),
            "3" => Ok(Some(FrameExifGpsMeasureMode::ThreeDimensional)),
            _ => Err(Self::semantic_tag_error(
                "GPSMeasureMode",
                Self::TAG_GPS_MEASURE_MODE,
                format!("must be `2` or `3`, got `{value}`"),
            )),
        }
    }

    fn optional_gps_speed_ref_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifGpsSpeedRef>> {
        let Some(value) =
            Self::optional_gps_ascii_ref_tag(ifd, Self::TAG_GPS_SPEED_REF, "GPSSpeedRef")?
        else {
            return Ok(None);
        };
        match value {
            "K" => Ok(Some(FrameExifGpsSpeedRef::KilometersPerHour)),
            "M" => Ok(Some(FrameExifGpsSpeedRef::MilesPerHour)),
            "N" => Ok(Some(FrameExifGpsSpeedRef::Knots)),
            _ => Err(Self::semantic_tag_error(
                "GPSSpeedRef",
                Self::TAG_GPS_SPEED_REF,
                format!("must be `K`, `M`, or `N`, got `{value}`"),
            )),
        }
    }

    fn optional_gps_distance_ref_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifGpsDistanceRef>> {
        let Some(value) = Self::optional_gps_ascii_ref_tag(
            ifd,
            Self::TAG_GPS_DEST_DISTANCE_REF,
            "GPSDestDistanceRef",
        )?
        else {
            return Ok(None);
        };
        match value {
            "K" => Ok(Some(FrameExifGpsDistanceRef::Kilometers)),
            "M" => Ok(Some(FrameExifGpsDistanceRef::Miles)),
            "N" => Ok(Some(FrameExifGpsDistanceRef::NauticalMiles)),
            _ => Err(Self::semantic_tag_error(
                "GPSDestDistanceRef",
                Self::TAG_GPS_DEST_DISTANCE_REF,
                format!("must be `K`, `M`, or `N`, got `{value}`"),
            )),
        }
    }

    fn optional_gps_direction_ref_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<FrameExifGpsDirectionRef>> {
        let Some(value) = Self::optional_gps_ascii_ref_tag(ifd, tag, label)? else {
            return Ok(None);
        };
        match value {
            "T" => Ok(Some(FrameExifGpsDirectionRef::TrueDirection)),
            "M" => Ok(Some(FrameExifGpsDirectionRef::MagneticDirection)),
            _ => Err(Self::semantic_tag_error(
                label,
                tag,
                format!("must be `T` or `M`, got `{value}`"),
            )),
        }
    }

    fn optional_gps_differential_tag(
        ifd: &FrameExifIfd<'a>,
    ) -> AvResult<Option<FrameExifGpsDifferential>> {
        let Some(raw) =
            Self::optional_short_tag(ifd, Self::TAG_GPS_DIFFERENTIAL, "GPSDifferential")?
        else {
            return Ok(None);
        };
        FrameExifGpsDifferential::from_raw(raw).map(Some)
    }

    fn optional_gps_ascii_ref_tag(
        ifd: &FrameExifIfd<'a>,
        tag: u16,
        label: &str,
    ) -> AvResult<Option<&'a str>> {
        Self::optional_ascii_exact_count_tag(ifd, tag, label, 2)
    }

    fn semantic_tag_error(label: &str, tag: u16, message: impl core::fmt::Display) -> AvError {
        AvError::invalid_data(format!("EXIF {label} tag 0x{tag:04x} {message}"))
    }

    fn parse_linked_ifds(
        data: &'a [u8],
        endian: FrameExifEndian,
        root_ifds: &[FrameExifIfd<'a>],
    ) -> AvResult<Vec<FrameExifLinkedIfd<'a>>> {
        let mut linked_ifds = Vec::new();
        let mut seen_offsets = root_ifds
            .iter()
            .map(FrameExifIfd::offset)
            .collect::<Vec<_>>();

        for ifd in root_ifds {
            for pointer in Self::ifd_pointers(ifd)? {
                Self::append_linked_ifd_chain(
                    data,
                    endian,
                    &mut seen_offsets,
                    &mut linked_ifds,
                    ifd.offset,
                    pointer,
                )?;
            }
        }

        let mut index = 0;
        while index < linked_ifds.len() {
            let parent_offset = linked_ifds[index].offset();
            let pointers = Self::ifd_pointers(linked_ifds[index].ifd())?;
            for pointer in pointers {
                Self::append_linked_ifd_chain(
                    data,
                    endian,
                    &mut seen_offsets,
                    &mut linked_ifds,
                    parent_offset,
                    pointer,
                )?;
            }
            index += 1;
        }

        Ok(linked_ifds)
    }

    fn ifd_pointers(ifd: &FrameExifIfd<'a>) -> AvResult<Vec<FrameExifIfdPointer>> {
        let mut pointers = Vec::new();
        for entry in ifd.entries() {
            if let Some(kind) = entry.ifd_pointer_kind() {
                let offset = entry
                    .ifd_pointer_offset()?
                    .expect("pointer kind requires pointer offset");
                if offset < Self::TIFF_HEADER_LEN {
                    return Err(AvError::invalid_data(format!(
                        "EXIF {:?} IFD pointer tag 0x{:04x} offset {offset} is before the TIFF header end {}",
                        kind,
                        kind.tag(),
                        Self::TIFF_HEADER_LEN
                    )));
                }
                pointers.push(FrameExifIfdPointer {
                    kind,
                    source_tag: entry.tag(),
                    offset,
                });
            }
        }

        Ok(pointers)
    }

    fn append_linked_ifd_chain(
        data: &'a [u8],
        endian: FrameExifEndian,
        seen_offsets: &mut Vec<usize>,
        linked_ifds: &mut Vec<FrameExifLinkedIfd<'a>>,
        parent_ifd_offset: usize,
        pointer: FrameExifIfdPointer,
    ) -> AvResult<()> {
        let FrameExifIfdPointer {
            kind,
            source_tag,
            offset,
        } = pointer;
        let chain = Self::parse_ifd_chain(data, endian, offset)?;
        for ifd in chain {
            if seen_offsets.contains(&ifd.offset) {
                return Err(AvError::invalid_data(format!(
                    "EXIF {:?} IFD pointer tag 0x{source_tag:04x} loops to already parsed IFD offset {}",
                    kind,
                    ifd.offset
                )));
            }
            if linked_ifds.len() >= Self::MAX_LINKED_IFDS {
                return Err(AvError::invalid_data(format!(
                    "EXIF linked IFD count exceeds {}",
                    Self::MAX_LINKED_IFDS
                )));
            }
            seen_offsets.push(ifd.offset);
            linked_ifds.push(FrameExifLinkedIfd {
                kind,
                parent_ifd_offset,
                source_tag,
                ifd,
            });
        }

        Ok(())
    }

    fn parse_ifd_chain(
        data: &'a [u8],
        endian: FrameExifEndian,
        first_offset: usize,
    ) -> AvResult<Vec<FrameExifIfd<'a>>> {
        let mut ifds = Vec::new();
        let mut seen_offsets = Vec::new();
        let mut offset = first_offset;

        for _ in 0..Self::MAX_IFDS {
            if seen_offsets.contains(&offset) {
                return Err(AvError::invalid_data(format!(
                    "EXIF IFD chain loops back to offset {offset}"
                )));
            }
            seen_offsets.push(offset);
            let ifd = Self::parse_ifd(data, endian, offset)?;
            let next_ifd_offset = ifd.next_ifd_offset;
            ifds.push(ifd);
            match next_ifd_offset {
                Some(next) => {
                    Self::validate_ifd_offset(data, next, "next")?;
                    offset = next;
                }
                None => return Ok(ifds),
            }
        }

        Err(AvError::invalid_data(format!(
            "EXIF IFD chain exceeds {} directories",
            Self::MAX_IFDS
        )))
    }

    fn parse_ifd(
        data: &'a [u8],
        endian: FrameExifEndian,
        offset: usize,
    ) -> AvResult<FrameExifIfd<'a>> {
        Self::validate_ifd_offset(data, offset, "current")?;
        let count_end = offset
            .checked_add(Self::IFD_COUNT_LEN)
            .ok_or_else(|| AvError::invalid_data("EXIF IFD entry-count offset overflows usize"))?;
        let entry_count = endian.read_u16(data, offset) as usize;
        if entry_count > Self::MAX_IFD_ENTRIES {
            return Err(AvError::invalid_data(format!(
                "EXIF IFD entry count {entry_count} exceeds {}",
                Self::MAX_IFD_ENTRIES
            )));
        }

        let entries_len = entry_count
            .checked_mul(Self::IFD_ENTRY_LEN)
            .ok_or_else(|| AvError::invalid_data("EXIF IFD entry-table length overflow"))?;
        let next_offset_position = count_end
            .checked_add(entries_len)
            .ok_or_else(|| AvError::invalid_data("EXIF IFD next-offset position overflow"))?;
        let table_end = next_offset_position
            .checked_add(Self::NEXT_IFD_OFFSET_LEN)
            .ok_or_else(|| AvError::invalid_data("EXIF IFD table end overflow"))?;
        if table_end > data.len() {
            return Err(AvError::invalid_data(format!(
                "EXIF IFD at offset {offset} requires {table_end} bytes, got {}",
                data.len()
            )));
        }

        let mut entries = Vec::with_capacity(entry_count);
        for index in 0..entry_count {
            let entry_offset = count_end + index * Self::IFD_ENTRY_LEN;
            entries.push(Self::parse_ifd_entry(data, endian, entry_offset)?);
        }

        let next = endian.read_u32(data, next_offset_position);
        Ok(FrameExifIfd {
            offset,
            entries,
            next_ifd_offset: (next != 0).then_some(next as usize),
        })
    }

    fn parse_ifd_entry(
        data: &'a [u8],
        endian: FrameExifEndian,
        entry_offset: usize,
    ) -> AvResult<FrameExifEntry<'a>> {
        let tag = endian.read_u16(data, entry_offset);
        let tiff_type = FrameExifTiffType::from_raw(endian.read_u16(data, entry_offset + 2))?;
        let count = endian.read_u32(data, entry_offset + 4);
        let value_offset = endian.read_u32(data, entry_offset + 8);
        let data_len = tiff_type
            .element_size()
            .checked_mul(count as usize)
            .ok_or_else(|| AvError::invalid_data("EXIF TIFF entry value length overflow"))?;
        let value_field_offset = entry_offset + 8;
        let value_or_offset_bytes = &data[value_field_offset..value_field_offset + 4];

        let (value_data, data_range) = if data_len <= 4 {
            (
                &data[value_field_offset..value_field_offset + data_len],
                None,
            )
        } else {
            let start = value_offset as usize;
            let end = start
                .checked_add(data_len)
                .ok_or_else(|| AvError::invalid_data("EXIF TIFF entry data range end overflow"))?;
            if end > data.len() {
                return Err(AvError::invalid_data(format!(
                    "EXIF TIFF entry tag 0x{tag:04x} data range {start}..{end} exceeds payload length {}",
                    data.len()
                )));
            }
            (&data[start..end], Some((start, end)))
        };

        Ok(FrameExifEntry {
            tag,
            tiff_type,
            endian,
            count,
            value_offset,
            data_len,
            value_or_offset_bytes,
            value_data,
            data_range,
        })
    }

    fn validate_ifd_offset(data: &[u8], offset: usize, label: &str) -> AvResult<()> {
        if offset < Self::TIFF_HEADER_LEN {
            return Err(AvError::invalid_data(format!(
                "EXIF {label} IFD offset {offset} is before the TIFF header end {}",
                Self::TIFF_HEADER_LEN
            )));
        }
        let count_end = offset.checked_add(Self::IFD_COUNT_LEN).ok_or_else(|| {
            AvError::invalid_data(format!("EXIF {label} IFD offset overflows usize"))
        })?;
        if count_end > data.len() {
            return Err(AvError::invalid_data(format!(
                "EXIF {label} IFD offset {offset} does not leave room for an entry count in {} bytes",
                data.len()
            )));
        }

        Ok(())
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

    pub fn new_a53_closed_captions(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(FrameSideDataKind::A53ClosedCaptions, data)?;
        FrameA53ClosedCaptions::parse(side_data.data())?;
        Ok(side_data)
    }

    pub fn new_stereo3d(value: FrameStereo3d) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::Stereo3d, value.to_bytes().to_vec())
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

    pub fn new_video_hint(value: FrameVideoHint) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::VideoHint, value.to_bytes())
    }

    pub fn new_lcevc(data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::Lcevc, data)
    }

    pub fn new_view_id(value: FrameViewId) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::ViewId, value.to_bytes().to_vec())
    }

    pub fn new_three_d_reference_displays(value: FrameThreeDReferenceDisplays) -> AvResult<Self> {
        Self::new_with_kind(FrameSideDataKind::ThreeDReferenceDisplays, value.to_bytes())
    }

    pub fn new_exif(data: Vec<u8>) -> AvResult<Self> {
        let side_data = Self::new_with_kind(FrameSideDataKind::Exif, data)?;
        FrameExif::parse(side_data.data())?;
        Ok(side_data)
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

    pub fn new_ambient_viewing_environment(
        value: FrameAmbientViewingEnvironment,
    ) -> AvResult<Self> {
        Self::new_with_kind(
            FrameSideDataKind::AmbientViewingEnvironment,
            value.to_bytes().to_vec(),
        )
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

    pub fn a53_closed_captions(&self) -> AvResult<Option<FrameA53ClosedCaptions<'_>>> {
        if self.kind != FrameSideDataKind::A53ClosedCaptions {
            return Ok(None);
        }

        FrameA53ClosedCaptions::parse(self.data()).map(Some)
    }

    pub fn stereo3d(&self) -> AvResult<Option<FrameStereo3d>> {
        if self.kind != FrameSideDataKind::Stereo3d {
            return Ok(None);
        }

        FrameStereo3d::parse(self.data()).map(Some)
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

    pub fn video_hint(&self) -> AvResult<Option<FrameVideoHint>> {
        if self.kind != FrameSideDataKind::VideoHint {
            return Ok(None);
        }

        FrameVideoHint::parse(self.data()).map(Some)
    }

    pub fn lcevc(&self) -> Option<FrameLcevc<'_>> {
        if self.kind != FrameSideDataKind::Lcevc {
            return None;
        }

        Some(FrameLcevc::parse(self.data()))
    }

    pub fn view_id(&self) -> AvResult<Option<FrameViewId>> {
        if self.kind != FrameSideDataKind::ViewId {
            return Ok(None);
        }

        FrameViewId::parse(self.data()).map(Some)
    }

    pub fn three_d_reference_displays(&self) -> AvResult<Option<FrameThreeDReferenceDisplays>> {
        if self.kind != FrameSideDataKind::ThreeDReferenceDisplays {
            return Ok(None);
        }

        FrameThreeDReferenceDisplays::parse(self.data()).map(Some)
    }

    pub fn exif(&self) -> AvResult<Option<FrameExif<'_>>> {
        if self.kind != FrameSideDataKind::Exif {
            return Ok(None);
        }

        FrameExif::parse(self.data()).map(Some)
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

    pub fn ambient_viewing_environment(&self) -> AvResult<Option<FrameAmbientViewingEnvironment>> {
        if self.kind != FrameSideDataKind::AmbientViewingEnvironment {
            return Ok(None);
        }

        FrameAmbientViewingEnvironment::parse(self.data()).map(Some)
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
        PixelFormat::MonoWhite | PixelFormat::MonoBlack => Ok(vec![VideoPlaneShape {
            row_bytes: one_bit_line_size(width),
            rows: height,
        }]),
        PixelFormat::Ya8 => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 2, "8-bit gray-alpha video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Ya16Le | PixelFormat::Ya16Be | PixelFormat::Yaf16Le | PixelFormat::Yaf16Be => {
            Ok(vec![VideoPlaneShape {
                row_bytes: checked_mul(width, 4, "16-bit gray-alpha video frame line size")?,
                rows: height,
            }])
        }
        PixelFormat::Gray9Le
        | PixelFormat::Gray9Be
        | PixelFormat::Gray10Le
        | PixelFormat::Gray10Be
        | PixelFormat::Gray12Le
        | PixelFormat::Gray12Be
        | PixelFormat::Gray14Le
        | PixelFormat::Gray14Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 2, "high bit-depth gray video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Gray16Le | PixelFormat::Gray16Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 2, "16-bit gray video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Gray32Le | PixelFormat::Gray32Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 4, "32-bit gray video frame line size")?,
            rows: height,
        }]),
        PixelFormat::GrayF16Le | PixelFormat::GrayF16Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 2, "16-bit floating gray video frame line size")?,
            rows: height,
        }]),
        PixelFormat::GrayF32Le | PixelFormat::GrayF32Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 4, "32-bit floating gray video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Yaf32Le | PixelFormat::Yaf32Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 8, "32-bit floating gray-alpha video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Rgb24 | PixelFormat::Bgr24 | PixelFormat::Vyu444 => {
            Ok(vec![VideoPlaneShape {
                row_bytes: checked_mul(width, 3, "24-bit packed video frame line size")?,
                rows: height,
            }])
        }
        PixelFormat::Pal8
        | PixelFormat::Rgb8
        | PixelFormat::Bgr8
        | PixelFormat::Rgb4Byte
        | PixelFormat::Bgr4Byte => Ok(vec![VideoPlaneShape {
            row_bytes: width,
            rows: height,
        }]),
        PixelFormat::Rgb4 | PixelFormat::Bgr4 => Ok(vec![VideoPlaneShape {
            row_bytes: nibble_line_size(width),
            rows: height,
        }]),
        PixelFormat::Rgb565Be
        | PixelFormat::Rgb565Le
        | PixelFormat::Rgb555Be
        | PixelFormat::Rgb555Le
        | PixelFormat::Bgr565Be
        | PixelFormat::Bgr565Le
        | PixelFormat::Bgr555Be
        | PixelFormat::Bgr555Le
        | PixelFormat::Rgb444Le
        | PixelFormat::Rgb444Be
        | PixelFormat::Bgr444Le
        | PixelFormat::Bgr444Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 2, "16-bit packed RGB video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Yuyv422 | PixelFormat::Uyvy422 | PixelFormat::Yvyu422 => {
            Ok(vec![VideoPlaneShape {
                row_bytes: checked_mul(width, 2, "packed YUV 4:2:2 video frame line size")?,
                rows: height,
            }])
        }
        PixelFormat::Y210Le
        | PixelFormat::Y210Be
        | PixelFormat::Y212Le
        | PixelFormat::Y212Be
        | PixelFormat::Y216Le
        | PixelFormat::Y216Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(
                width,
                4,
                "high bit-depth packed YUV 4:2:2 video frame line size",
            )?,
            rows: height,
        }]),
        PixelFormat::Rgb48Le
        | PixelFormat::Rgb48Be
        | PixelFormat::Bgr48Le
        | PixelFormat::Bgr48Be
        | PixelFormat::Xyz12Le
        | PixelFormat::Xyz12Be
        | PixelFormat::Xv36Le
        | PixelFormat::Xv36Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 6, "six-byte packed video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Rgba64Le
        | PixelFormat::Rgba64Be
        | PixelFormat::Bgra64Le
        | PixelFormat::Bgra64Be
        | PixelFormat::Ayuv64Le
        | PixelFormat::Ayuv64Be
        | PixelFormat::Xv48Le
        | PixelFormat::Xv48Be => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 8, "64-bit packed video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Rgba
        | PixelFormat::Bgra
        | PixelFormat::Argb
        | PixelFormat::Abgr
        | PixelFormat::ZeroRgb
        | PixelFormat::Rgb0
        | PixelFormat::ZeroBgr
        | PixelFormat::Bgr0
        | PixelFormat::X2Rgb10Le
        | PixelFormat::X2Rgb10Be
        | PixelFormat::X2Bgr10Le
        | PixelFormat::X2Bgr10Be
        | PixelFormat::Vuya
        | PixelFormat::Vuyx
        | PixelFormat::Xv30Le
        | PixelFormat::Xv30Be
        | PixelFormat::V30xLe
        | PixelFormat::V30xBe
        | PixelFormat::Ayuv
        | PixelFormat::Uyva => Ok(vec![VideoPlaneShape {
            row_bytes: checked_mul(width, 4, "32-bit packed video frame line size")?,
            rows: height,
        }]),
        PixelFormat::Gbrp => Ok(vec![
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
        ]),
        PixelFormat::Gbrap => Ok(vec![
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
        ]),
        PixelFormat::Gbrp9Le
        | PixelFormat::Gbrp9Be
        | PixelFormat::Gbrp10Le
        | PixelFormat::Gbrp10Be
        | PixelFormat::Gbrp12Le
        | PixelFormat::Gbrp12Be
        | PixelFormat::Gbrp14Le
        | PixelFormat::Gbrp14Be
        | PixelFormat::Gbrp16Le
        | PixelFormat::Gbrp16Be => {
            let row_bytes =
                checked_mul(width, 2, "high bit-depth planar GBR video frame line size")?;
            Ok(vec![
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
            ])
        }
        PixelFormat::Gbrap10Le
        | PixelFormat::Gbrap10Be
        | PixelFormat::Gbrap12Le
        | PixelFormat::Gbrap12Be
        | PixelFormat::Gbrap14Le
        | PixelFormat::Gbrap14Be
        | PixelFormat::Gbrap16Le
        | PixelFormat::Gbrap16Be
        | PixelFormat::GbrapF16Le
        | PixelFormat::GbrapF16Be => {
            let row_bytes =
                checked_mul(width, 2, "high bit-depth planar GBRA video frame line size")?;
            Ok(vec![
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
            ])
        }
        PixelFormat::Gbrap32Le
        | PixelFormat::Gbrap32Be
        | PixelFormat::GbrapF32Le
        | PixelFormat::GbrapF32Be => {
            let row_bytes = checked_mul(width, 4, "32-bit planar GBRA video frame line size")?;
            Ok(vec![
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
            ])
        }
        PixelFormat::Yuva420p
        | PixelFormat::Yuva422p
        | PixelFormat::Yuva444p
        | PixelFormat::Yuva420p9Le
        | PixelFormat::Yuva420p9Be
        | PixelFormat::Yuva422p9Le
        | PixelFormat::Yuva422p9Be
        | PixelFormat::Yuva444p9Le
        | PixelFormat::Yuva444p9Be
        | PixelFormat::Yuva420p10Le
        | PixelFormat::Yuva420p10Be
        | PixelFormat::Yuva422p10Le
        | PixelFormat::Yuva422p10Be
        | PixelFormat::Yuva444p10Le
        | PixelFormat::Yuva444p10Be
        | PixelFormat::Yuva422p12Le
        | PixelFormat::Yuva422p12Be
        | PixelFormat::Yuva444p12Le
        | PixelFormat::Yuva444p12Be
        | PixelFormat::Yuva420p16Le
        | PixelFormat::Yuva420p16Be
        | PixelFormat::Yuva422p16Le
        | PixelFormat::Yuva422p16Be
        | PixelFormat::Yuva444p16Le
        | PixelFormat::Yuva444p16Be => {
            let (log2_chroma_w, log2_chroma_h) = pixel_format.log2_chroma();
            let bytes_per_sample = if pixel_format.bits_per_component() > 8 {
                2
            } else {
                1
            };
            let luma_row_bytes =
                checked_mul(width, bytes_per_sample, "planar YUVA video frame line size")?;
            let chroma_row_bytes = checked_mul(
                width >> log2_chroma_w,
                bytes_per_sample,
                "planar YUVA chroma video frame line size",
            )?;
            Ok(vec![
                VideoPlaneShape {
                    row_bytes: luma_row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes: chroma_row_bytes,
                    rows: height >> log2_chroma_h,
                },
                VideoPlaneShape {
                    row_bytes: chroma_row_bytes,
                    rows: height >> log2_chroma_h,
                },
                VideoPlaneShape {
                    row_bytes: luma_row_bytes,
                    rows: height,
                },
            ])
        }
        PixelFormat::Yuv420p
        | PixelFormat::YuvJ420p
        | PixelFormat::Yuv422p
        | PixelFormat::YuvJ422p
        | PixelFormat::Yuv410p
        | PixelFormat::Yuv411p
        | PixelFormat::YuvJ411p
        | PixelFormat::Yuv440p
        | PixelFormat::YuvJ440p
        | PixelFormat::Yuv444p
        | PixelFormat::YuvJ444p
        | PixelFormat::Yuv440p10Le
        | PixelFormat::Yuv440p10Be
        | PixelFormat::Yuv440p12Le
        | PixelFormat::Yuv440p12Be
        | PixelFormat::Yuv420p9Le
        | PixelFormat::Yuv420p9Be
        | PixelFormat::Yuv422p9Le
        | PixelFormat::Yuv422p9Be
        | PixelFormat::Yuv444p9Le
        | PixelFormat::Yuv444p9Be
        | PixelFormat::Yuv420p10Le
        | PixelFormat::Yuv420p10Be
        | PixelFormat::Yuv422p10Le
        | PixelFormat::Yuv422p10Be
        | PixelFormat::Yuv444p10Le
        | PixelFormat::Yuv444p10Be
        | PixelFormat::Yuv420p12Le
        | PixelFormat::Yuv420p12Be
        | PixelFormat::Yuv422p12Le
        | PixelFormat::Yuv422p12Be
        | PixelFormat::Yuv444p12Le
        | PixelFormat::Yuv444p12Be
        | PixelFormat::Yuv420p14Le
        | PixelFormat::Yuv420p14Be
        | PixelFormat::Yuv422p14Le
        | PixelFormat::Yuv422p14Be
        | PixelFormat::Yuv444p14Le
        | PixelFormat::Yuv444p14Be
        | PixelFormat::Yuv420p16Le
        | PixelFormat::Yuv420p16Be
        | PixelFormat::Yuv422p16Le
        | PixelFormat::Yuv422p16Be
        | PixelFormat::Yuv444p16Le
        | PixelFormat::Yuv444p16Be => {
            let (log2_chroma_w, log2_chroma_h) = pixel_format.log2_chroma();
            let bytes_per_sample = if matches!(
                pixel_format,
                PixelFormat::Yuv420p9Le
                    | PixelFormat::Yuv420p9Be
                    | PixelFormat::Yuv422p9Le
                    | PixelFormat::Yuv422p9Be
                    | PixelFormat::Yuv444p9Le
                    | PixelFormat::Yuv444p9Be
                    | PixelFormat::Yuv420p10Le
                    | PixelFormat::Yuv420p10Be
                    | PixelFormat::Yuv422p10Le
                    | PixelFormat::Yuv422p10Be
                    | PixelFormat::Yuv440p10Le
                    | PixelFormat::Yuv440p10Be
                    | PixelFormat::Yuv444p10Le
                    | PixelFormat::Yuv444p10Be
                    | PixelFormat::Yuv420p12Le
                    | PixelFormat::Yuv420p12Be
                    | PixelFormat::Yuv422p12Le
                    | PixelFormat::Yuv422p12Be
                    | PixelFormat::Yuv440p12Le
                    | PixelFormat::Yuv440p12Be
                    | PixelFormat::Yuv444p12Le
                    | PixelFormat::Yuv444p12Be
                    | PixelFormat::Yuv420p14Le
                    | PixelFormat::Yuv420p14Be
                    | PixelFormat::Yuv422p14Le
                    | PixelFormat::Yuv422p14Be
                    | PixelFormat::Yuv444p14Le
                    | PixelFormat::Yuv444p14Be
                    | PixelFormat::Yuv420p16Le
                    | PixelFormat::Yuv420p16Be
                    | PixelFormat::Yuv422p16Le
                    | PixelFormat::Yuv422p16Be
                    | PixelFormat::Yuv444p16Le
                    | PixelFormat::Yuv444p16Be
            ) {
                2
            } else {
                1
            };
            let luma_row_bytes = checked_mul(
                width,
                bytes_per_sample,
                "planar YUV video frame luma line size",
            )?;
            let chroma_row_bytes = checked_mul(
                width >> log2_chroma_w,
                bytes_per_sample,
                "planar YUV video frame chroma line size",
            )?;
            Ok(vec![
                VideoPlaneShape {
                    row_bytes: luma_row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes: chroma_row_bytes,
                    rows: height >> log2_chroma_h,
                },
                VideoPlaneShape {
                    row_bytes: chroma_row_bytes,
                    rows: height >> log2_chroma_h,
                },
            ])
        }
        PixelFormat::Nv12 | PixelFormat::Nv21 => Ok(vec![
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
            VideoPlaneShape {
                row_bytes: width,
                rows: height >> 1,
            },
        ]),
        PixelFormat::Nv16 => Ok(vec![
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
            VideoPlaneShape {
                row_bytes: width,
                rows: height,
            },
        ]),
        PixelFormat::Nv20Le | PixelFormat::Nv20Be => {
            let row_bytes = checked_mul(
                width,
                2,
                "semi-planar 10-bit 4:2:2 YUV video frame line size",
            )?;
            Ok(vec![
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes,
                    rows: height,
                },
            ])
        }
        PixelFormat::Nv24 | PixelFormat::Nv42 => {
            let chroma_row_bytes = checked_mul(
                width,
                2,
                "semi-planar 4:4:4 YUV video frame chroma line size",
            )?;
            Ok(vec![
                VideoPlaneShape {
                    row_bytes: width,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes: chroma_row_bytes,
                    rows: height,
                },
            ])
        }
        PixelFormat::P010Le
        | PixelFormat::P010Be
        | PixelFormat::P012Le
        | PixelFormat::P012Be
        | PixelFormat::P016Le
        | PixelFormat::P016Be
        | PixelFormat::P210Le
        | PixelFormat::P210Be
        | PixelFormat::P212Le
        | PixelFormat::P212Be
        | PixelFormat::P216Le
        | PixelFormat::P216Be
        | PixelFormat::P410Le
        | PixelFormat::P410Be
        | PixelFormat::P412Le
        | PixelFormat::P412Be
        | PixelFormat::P416Le
        | PixelFormat::P416Be => {
            let (log2_chroma_w, log2_chroma_h) = pixel_format.log2_chroma();
            let luma_row_bytes = checked_mul(
                width,
                2,
                "semi-planar high-bit YUV video frame luma line size",
            )?;
            let chroma_samples_per_row = width >> log2_chroma_w;
            let chroma_row_bytes = checked_mul(
                chroma_samples_per_row,
                4,
                "semi-planar high-bit YUV video frame chroma line size",
            )?;
            Ok(vec![
                VideoPlaneShape {
                    row_bytes: luma_row_bytes,
                    rows: height,
                },
                VideoPlaneShape {
                    row_bytes: chroma_row_bytes,
                    rows: height >> log2_chroma_h,
                },
            ])
        }
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

fn nibble_line_size(width: usize) -> usize {
    (width / 2) + (width % 2)
}

fn one_bit_line_size(width: usize) -> usize {
    width.div_ceil(8)
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

    fn exif_with_linked_ifds_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&3u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            0x010F,
            FrameExifTiffType::Ascii,
            6,
            50u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            56u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::GPS_TAG,
            FrameExifTiffType::Long,
            1,
            74u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"Rusty\0");

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::INTEROPERABILITY_TAG,
            FrameExifTiffType::Long,
            1,
            92u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(&mut data, 0x0000, FrameExifTiffType::Byte, 4, [2, 3, 0, 0]);
        data.extend_from_slice(&0u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(&mut data, 0x0001, FrameExifTiffType::Ascii, 4, *b"R98\0");
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 110);
        data
    }

    fn exif_value_semantics_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&11u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            0x010F,
            FrameExifTiffType::Ascii,
            6,
            146u32.to_le_bytes(),
        );
        push_exif_entry(&mut data, 0x0112, FrameExifTiffType::Short, 1, [6, 0, 0, 0]);
        push_exif_entry(
            &mut data,
            0x0100,
            FrameExifTiffType::Long,
            1,
            640u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            0x011A,
            FrameExifTiffType::Rational,
            1,
            152u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            0xC001,
            FrameExifTiffType::SignedShort,
            2,
            [0xFF, 0xFF, 0x02, 0x00],
        );
        push_exif_entry(
            &mut data,
            0xC002,
            FrameExifTiffType::SignedLong,
            1,
            (-42i32).to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            0xC003,
            FrameExifTiffType::SignedRational,
            1,
            160u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            0xC004,
            FrameExifTiffType::SignedByte,
            3,
            [0xFF, 0x00, 0x02, 0x00],
        );
        push_exif_entry(
            &mut data,
            0xC005,
            FrameExifTiffType::Float,
            1,
            1.25f32.to_bits().to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            0xC006,
            FrameExifTiffType::Double,
            1,
            168u32.to_le_bytes(),
        );
        push_exif_entry(&mut data, 0x0000, FrameExifTiffType::Byte, 4, [2, 3, 0, 0]);
        data.extend_from_slice(&0u32.to_le_bytes());

        data.extend_from_slice(b"Rusty\0");
        data.extend_from_slice(&300u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&2i32.to_le_bytes());
        data.extend_from_slice(&(-2.5f64).to_bits().to_le_bytes());

        assert_eq!(data.len(), 176);
        data
    }

    fn exif_common_tags_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&9u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_MAKE,
            FrameExifTiffType::Ascii,
            6,
            122u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_MODEL,
            FrameExifTiffType::Ascii,
            7,
            128u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_IMAGE_WIDTH,
            FrameExifTiffType::Long,
            1,
            640u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_IMAGE_LENGTH,
            FrameExifTiffType::Short,
            1,
            [0xE0, 0x01, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_ORIENTATION,
            FrameExifTiffType::Short,
            1,
            [6, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_X_RESOLUTION,
            FrameExifTiffType::Rational,
            1,
            136u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_RESOLUTION_UNIT,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            144u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::GPS_TAG,
            FrameExifTiffType::Long,
            1,
            224u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 122);

        data.extend_from_slice(b"Rusty\0");
        data.extend_from_slice(b"Camera\0");
        data.push(0);
        data.extend_from_slice(&300u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        assert_eq!(data.len(), 144);

        data.extend_from_slice(&3u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_EXIF_VERSION,
            FrameExifTiffType::Undefined,
            4,
            *b"0231",
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_DATE_TIME_ORIGINAL,
            FrameExifTiffType::Ascii,
            20,
            186u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::INTEROPERABILITY_TAG,
            FrameExifTiffType::Long,
            1,
            206u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"2026:05:04 12:34:56\0");
        assert_eq!(data.len(), 206);

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_INTEROPERABILITY_INDEX,
            FrameExifTiffType::Ascii,
            4,
            *b"R98\0",
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 224);

        data.extend_from_slice(&5u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_VERSION_ID,
            FrameExifTiffType::Byte,
            4,
            [2, 3, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_LATITUDE_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'N', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_LATITUDE,
            FrameExifTiffType::Rational,
            3,
            290u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_LONGITUDE_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'W', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_LONGITUDE,
            FrameExifTiffType::Rational,
            3,
            314u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 290);

        for value in [37u32, 48, 30, 122, 24, 15] {
            data.extend_from_slice(&value.to_le_bytes());
            data.extend_from_slice(&1u32.to_le_bytes());
        }

        assert_eq!(data.len(), 338);
        data
    }

    fn exif_root_image_layout_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&4u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SAMPLES_PER_PIXEL,
            FrameExifTiffType::Short,
            1,
            [3, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PLANAR_CONFIGURATION,
            FrameExifTiffType::Short,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_YCBCR_SUB_SAMPLING,
            FrameExifTiffType::Short,
            2,
            [2, 0, 2, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_YCBCR_POSITIONING,
            FrameExifTiffType::Short,
            1,
            [1, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 62);
        data
    }

    fn exif_root_colorimetry_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&4u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_WHITE_POINT,
            FrameExifTiffType::Rational,
            2,
            62u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PRIMARY_CHROMATICITIES,
            FrameExifTiffType::Rational,
            6,
            78u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_YCBCR_COEFFICIENTS,
            FrameExifTiffType::Rational,
            3,
            126u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_REFERENCE_BLACK_WHITE,
            FrameExifTiffType::Rational,
            6,
            150u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        for (numerator, denominator) in [(1u32, 3u32), (1, 4)] {
            data.extend_from_slice(&numerator.to_le_bytes());
            data.extend_from_slice(&denominator.to_le_bytes());
        }
        for (numerator, denominator) in [
            (640u32, 1000u32),
            (330, 1000),
            (300, 1000),
            (600, 1000),
            (150, 1000),
            (60, 1000),
        ] {
            data.extend_from_slice(&numerator.to_le_bytes());
            data.extend_from_slice(&denominator.to_le_bytes());
        }
        for (numerator, denominator) in [(299u32, 1000u32), (587, 1000), (114, 1000)] {
            data.extend_from_slice(&numerator.to_le_bytes());
            data.extend_from_slice(&denominator.to_le_bytes());
        }
        for (numerator, denominator) in [
            (0u32, 1u32),
            (255, 1),
            (128, 1),
            (255, 1),
            (128, 1),
            (255, 1),
        ] {
            data.extend_from_slice(&numerator.to_le_bytes());
            data.extend_from_slice(&denominator.to_le_bytes());
        }

        assert_eq!(data.len(), 198);
        data
    }

    fn exif_root_subfile_type_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&2u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_NEW_SUBFILE_TYPE,
            FrameExifTiffType::Long,
            1,
            (FrameExifNewSubfileType::REDUCED_RESOLUTION_IMAGE
                | FrameExifNewSubfileType::SINGLE_PAGE_OF_MULTI_PAGE_IMAGE)
                .to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SUBFILE_TYPE,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 38);
        data
    }

    fn exif_root_camera_identity_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&2u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_MAKE,
            FrameExifTiffType::Ascii,
            3,
            [b'M', b'K', 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_MODEL,
            FrameExifTiffType::Ascii,
            3,
            [b'M', b'2', 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 38);
        data
    }

    fn exif_root_orientation_resolution_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&2u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_ORIENTATION,
            FrameExifTiffType::Short,
            1,
            [6, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_RESOLUTION_UNIT,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 38);
        data
    }

    fn exif_root_resolution_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&2u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_X_RESOLUTION,
            FrameExifTiffType::Rational,
            1,
            38u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_Y_RESOLUTION,
            FrameExifTiffType::Rational,
            1,
            46u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        data.extend_from_slice(&300u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&72u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());

        assert_eq!(data.len(), 54);
        data
    }

    fn exif_root_document_page_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&3u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_DOCUMENT_NAME,
            FrameExifTiffType::Ascii,
            4,
            [b'D', b'o', b'c', 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PAGE_NAME,
            FrameExifTiffType::Ascii,
            7,
            50u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PAGE_NUMBER,
            FrameExifTiffType::Short,
            2,
            [1, 0, 10, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"Page A\0");

        assert_eq!(data.len(), 57);
        data
    }

    fn exif_root_host_computer_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_HOST_COMPUTER,
            FrameExifTiffType::Ascii,
            3,
            [b'P', b'C', 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 26);
        data
    }

    fn exif_root_predictor_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PREDICTOR,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 26);
        data
    }

    fn exif_root_copyright_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_COPYRIGHT,
            FrameExifTiffType::Ascii,
            3,
            [b'C', b'C', 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 26);
        data
    }

    fn exif_root_coding_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&2u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_COMPRESSION,
            FrameExifTiffType::Short,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PHOTOMETRIC_INTERPRETATION,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 38);
        data
    }

    fn exif_root_bits_per_sample_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&2u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_BITS_PER_SAMPLE,
            FrameExifTiffType::Short,
            3,
            38u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SAMPLES_PER_PIXEL,
            FrameExifTiffType::Short,
            1,
            [3, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&[8, 0, 8, 0, 8, 0]);

        assert_eq!(data.len(), 44);
        data
    }

    fn exif_root_thresholding_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_THRESHOLDING,
            FrameExifTiffType::Short,
            1,
            [3, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 26);
        data
    }

    fn exif_root_fill_order_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FILL_ORDER,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 26);
        data
    }

    fn exif_root_strip_position_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&3u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_ROWS_PER_STRIP,
            FrameExifTiffType::Long,
            1,
            8u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_X_POSITION,
            FrameExifTiffType::Rational,
            1,
            50u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_Y_POSITION,
            FrameExifTiffType::Rational,
            1,
            58u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        for (numerator, denominator) in [(1u32, 2u32), (3, 4)] {
            data.extend_from_slice(&numerator.to_le_bytes());
            data.extend_from_slice(&denominator.to_le_bytes());
        }

        assert_eq!(data.len(), 66);
        data
    }

    fn exif_gps_altitude_time_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::GPS_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&4u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_ALTITUDE_REF,
            FrameExifTiffType::Byte,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_ALTITUDE,
            FrameExifTiffType::Rational,
            1,
            80u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_TIME_STAMP,
            FrameExifTiffType::Rational,
            3,
            88u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DATE_STAMP,
            FrameExifTiffType::Ascii,
            11,
            112u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 80);

        data.extend_from_slice(&15u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        for value in [12u32, 34, 56] {
            data.extend_from_slice(&value.to_le_bytes());
            data.extend_from_slice(&1u32.to_le_bytes());
        }
        data.extend_from_slice(b"2026:05:06\0");

        assert_eq!(data.len(), 123);
        data
    }

    fn exif_gps_acquisition_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::GPS_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&4u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_SATELLITES,
            FrameExifTiffType::Ascii,
            8,
            80u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_STATUS,
            FrameExifTiffType::Ascii,
            2,
            [b'A', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_MEASURE_MODE,
            FrameExifTiffType::Ascii,
            2,
            [b'3', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DOP,
            FrameExifTiffType::Rational,
            1,
            88u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 80);

        data.extend_from_slice(b"12 used\0");
        data.extend_from_slice(&7u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());

        assert_eq!(data.len(), 96);
        data
    }

    fn exif_gps_motion_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::GPS_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&7u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_SPEED_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'K', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_SPEED,
            FrameExifTiffType::Rational,
            1,
            116u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_TRACK_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'T', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_TRACK,
            FrameExifTiffType::Rational,
            1,
            124u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_IMG_DIRECTION_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'M', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_IMG_DIRECTION,
            FrameExifTiffType::Rational,
            1,
            132u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_MAP_DATUM,
            FrameExifTiffType::Ascii,
            7,
            140u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 116);

        data.extend_from_slice(&88u32.to_le_bytes());
        data.extend_from_slice(&5u32.to_le_bytes());
        data.extend_from_slice(&270u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&135u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(b"WGS-84\0");

        assert_eq!(data.len(), 147);
        data
    }

    fn exif_gps_destination_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::GPS_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&8u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DEST_LATITUDE_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'S', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DEST_LATITUDE,
            FrameExifTiffType::Rational,
            3,
            128u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DEST_LONGITUDE_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'E', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DEST_LONGITUDE,
            FrameExifTiffType::Rational,
            3,
            152u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DEST_BEARING_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'T', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DEST_BEARING,
            FrameExifTiffType::Rational,
            1,
            176u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DEST_DISTANCE_REF,
            FrameExifTiffType::Ascii,
            2,
            [b'N', 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DEST_DISTANCE,
            FrameExifTiffType::Rational,
            1,
            184u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 128);

        for value in [33u32, 52, 7] {
            data.extend_from_slice(&value.to_le_bytes());
            data.extend_from_slice(&1u32.to_le_bytes());
        }
        for value in [151u32, 12, 9] {
            data.extend_from_slice(&value.to_le_bytes());
            data.extend_from_slice(&1u32.to_le_bytes());
        }
        data.extend_from_slice(&91u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&42u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());

        assert_eq!(data.len(), 192);
        data
    }

    fn exif_gps_processing_error_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::GPS_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&4u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_PROCESSING_METHOD,
            FrameExifTiffType::Undefined,
            12,
            80u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_AREA_INFORMATION,
            FrameExifTiffType::Undefined,
            12,
            92u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_DIFFERENTIAL,
            FrameExifTiffType::Short,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GPS_H_POSITIONING_ERROR,
            FrameExifTiffType::Rational,
            1,
            104u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 80);

        data.extend_from_slice(b"ASCII\0\0\0GPS\0");
        data.extend_from_slice(b"ASCII\0\0\0AREA");
        data.extend_from_slice(&5u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());

        assert_eq!(data.len(), 112);
        data
    }

    fn exif_interoperability_related_image_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::INTEROPERABILITY_TAG,
            FrameExifTiffType::Long,
            1,
            44u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 44);

        data.extend_from_slice(&5u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_INTEROPERABILITY_INDEX,
            FrameExifTiffType::Ascii,
            4,
            *b"R98\0",
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_INTEROPERABILITY_VERSION,
            FrameExifTiffType::Undefined,
            4,
            *b"0100",
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_RELATED_IMAGE_FILE_FORMAT,
            FrameExifTiffType::Ascii,
            5,
            110u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_RELATED_IMAGE_WIDTH,
            FrameExifTiffType::Short,
            1,
            [64, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_RELATED_IMAGE_LENGTH,
            FrameExifTiffType::Long,
            1,
            48u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 110);

        data.extend_from_slice(b"JPEG\0");
        assert_eq!(data.len(), 115);
        data
    }

    fn exif_exposure_tags_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&7u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_EXPOSURE_TIME,
            FrameExifTiffType::Rational,
            1,
            116u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_F_NUMBER,
            FrameExifTiffType::Rational,
            1,
            124u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_EXPOSURE_BIAS_VALUE,
            FrameExifTiffType::SignedRational,
            1,
            132u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FOCAL_LENGTH,
            FrameExifTiffType::Rational,
            1,
            140u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PIXEL_X_DIMENSION,
            FrameExifTiffType::Long,
            1,
            1920u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PIXEL_Y_DIMENSION,
            FrameExifTiffType::Short,
            1,
            [0x38, 0x04, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_DATE_TIME_DIGITIZED,
            FrameExifTiffType::Ascii,
            20,
            148u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 116);

        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&125u32.to_le_bytes());
        data.extend_from_slice(&28u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&3i32.to_le_bytes());
        data.extend_from_slice(&50u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(b"2026:05:04 12:35:00\0");

        assert_eq!(data.len(), 168);
        data
    }

    fn exif_apex_exposure_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&3u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SHUTTER_SPEED_VALUE,
            FrameExifTiffType::SignedRational,
            1,
            68u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_APERTURE_VALUE,
            FrameExifTiffType::Rational,
            1,
            76u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_BRIGHTNESS_VALUE,
            FrameExifTiffType::SignedRational,
            1,
            84u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 68);

        data.extend_from_slice(&(-7i32).to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&56u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&(-3i32).to_le_bytes());
        data.extend_from_slice(&2i32.to_le_bytes());

        assert_eq!(data.len(), 92);
        data
    }

    fn exif_sensitivity_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&7u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PHOTOGRAPHIC_SENSITIVITY,
            FrameExifTiffType::Short,
            1,
            [200, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SENSITIVITY_TYPE,
            FrameExifTiffType::Short,
            1,
            [3, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_STANDARD_OUTPUT_SENSITIVITY,
            FrameExifTiffType::Long,
            1,
            160u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_RECOMMENDED_EXPOSURE_INDEX,
            FrameExifTiffType::Long,
            1,
            180u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_ISO_SPEED,
            FrameExifTiffType::Long,
            1,
            200u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_ISO_SPEED_LATITUDE_YYY,
            FrameExifTiffType::Long,
            1,
            125u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_ISO_SPEED_LATITUDE_ZZZ,
            FrameExifTiffType::Long,
            1,
            400u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 116);
        data
    }

    fn exif_camera_characterization_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&8u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SPECTRAL_SENSITIVITY,
            FrameExifTiffType::Ascii,
            8,
            128u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_OECF,
            FrameExifTiffType::Undefined,
            8,
            136u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FLASH_ENERGY,
            FrameExifTiffType::Rational,
            1,
            144u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SPATIAL_FREQUENCY_RESPONSE,
            FrameExifTiffType::Undefined,
            7,
            152u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FOCAL_PLANE_X_RESOLUTION,
            FrameExifTiffType::Rational,
            1,
            160u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FOCAL_PLANE_Y_RESOLUTION,
            FrameExifTiffType::Rational,
            1,
            168u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FOCAL_PLANE_RESOLUTION_UNIT,
            FrameExifTiffType::Short,
            1,
            [3, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_CFA_PATTERN,
            FrameExifTiffType::Undefined,
            8,
            176u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 128);

        data.extend_from_slice(b"RGB 550\0");
        data.extend_from_slice(b"oecf0001");
        data.extend_from_slice(&25u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(b"sfr0001");
        data.push(0);
        data.extend_from_slice(&3000u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&2000u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&[2, 0, 2, 0, 1, 0, 2, 1]);
        assert_eq!(data.len(), 184);
        data
    }

    fn exif_offset_time_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&3u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_OFFSET_TIME,
            FrameExifTiffType::Ascii,
            7,
            68u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_OFFSET_TIME_ORIGINAL,
            FrameExifTiffType::Ascii,
            7,
            75u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_OFFSET_TIME_DIGITIZED,
            FrameExifTiffType::Ascii,
            7,
            82u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 68);

        data.extend_from_slice(b"+09:00\0");
        data.extend_from_slice(b"-07:30\0");
        data.extend_from_slice(b"+00:00\0");
        assert_eq!(data.len(), 89);
        data
    }

    fn exif_capture_settings_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&7u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_EXPOSURE_PROGRAM,
            FrameExifTiffType::Short,
            1,
            [3, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_METERING_MODE,
            FrameExifTiffType::Short,
            1,
            [5, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_LIGHT_SOURCE,
            FrameExifTiffType::Short,
            1,
            [21, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FLASH,
            FrameExifTiffType::Short,
            1,
            [0x41, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_WHITE_BALANCE,
            FrameExifTiffType::Short,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_DIGITAL_ZOOM_RATIO,
            FrameExifTiffType::Rational,
            1,
            116u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FOCAL_LENGTH_IN_35MM_FILM,
            FrameExifTiffType::Short,
            1,
            [75, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 116);

        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());

        assert_eq!(data.len(), 124);
        data
    }

    fn exif_rendering_scene_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&12u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_COLOR_SPACE,
            FrameExifTiffType::Short,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SENSING_METHOD,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FILE_SOURCE,
            FrameExifTiffType::Undefined,
            1,
            [3, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SCENE_TYPE,
            FrameExifTiffType::Undefined,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_CUSTOM_RENDERED,
            FrameExifTiffType::Short,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_EXPOSURE_MODE,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SCENE_CAPTURE_TYPE,
            FrameExifTiffType::Short,
            1,
            [3, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GAIN_CONTROL,
            FrameExifTiffType::Short,
            1,
            [4, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_CONTRAST,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SATURATION,
            FrameExifTiffType::Short,
            1,
            [1, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SHARPNESS,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SUBJECT_DISTANCE_RANGE,
            FrameExifTiffType::Short,
            1,
            [3, 0, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());

        assert_eq!(data.len(), 176);
        data
    }

    fn exif_optics_subject_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&6u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_COMPRESSED_BITS_PER_PIXEL,
            FrameExifTiffType::Rational,
            1,
            104u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_MAX_APERTURE_VALUE,
            FrameExifTiffType::Rational,
            1,
            112u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SUBJECT_DISTANCE,
            FrameExifTiffType::Rational,
            1,
            120u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SUBJECT_AREA,
            FrameExifTiffType::Short,
            4,
            128u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SUBJECT_LOCATION,
            FrameExifTiffType::Short,
            2,
            [0x40, 0x01, 0xF0, 0x00],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_EXPOSURE_INDEX,
            FrameExifTiffType::Rational,
            1,
            136u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 104);

        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&14u32.to_le_bytes());
        data.extend_from_slice(&5u32.to_le_bytes());
        data.extend_from_slice(&125u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&100u16.to_le_bytes());
        data.extend_from_slice(&150u16.to_le_bytes());
        data.extend_from_slice(&80u16.to_le_bytes());
        data.extend_from_slice(&60u16.to_le_bytes());
        data.extend_from_slice(&200u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());

        assert_eq!(data.len(), 144);
        data
    }

    fn exif_version_timing_comment_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&9u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_COMPONENTS_CONFIGURATION,
            FrameExifTiffType::Undefined,
            4,
            [1, 2, 3, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_MAKER_NOTE,
            FrameExifTiffType::Undefined,
            6,
            140u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_USER_COMMENT,
            FrameExifTiffType::Undefined,
            16,
            146u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SUB_SEC_TIME,
            FrameExifTiffType::Ascii,
            4,
            *b"123\0",
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SUB_SEC_TIME_ORIGINAL,
            FrameExifTiffType::Ascii,
            5,
            162u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SUB_SEC_TIME_DIGITIZED,
            FrameExifTiffType::Ascii,
            3,
            [b'8', b'9', 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_FLASHPIX_VERSION,
            FrameExifTiffType::Undefined,
            4,
            *b"0100",
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_RELATED_SOUND_FILE,
            FrameExifTiffType::Ascii,
            13,
            167u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PIXEL_X_DIMENSION,
            FrameExifTiffType::Short,
            1,
            [0x80, 0x02, 0, 0],
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 140);

        data.extend_from_slice(b"maker!");
        data.extend_from_slice(b"ASCII\0\0\0hello\0\0\0");
        data.extend_from_slice(b"4567\0");
        data.extend_from_slice(b"SOUND001.WAV\0");

        assert_eq!(data.len(), 180);
        data
    }

    fn exif_camera_lens_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&7u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_IMAGE_UNIQUE_ID,
            FrameExifTiffType::Ascii,
            33,
            116u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_CAMERA_OWNER_NAME,
            FrameExifTiffType::Ascii,
            9,
            149u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_BODY_SERIAL_NUMBER,
            FrameExifTiffType::Ascii,
            9,
            158u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_LENS_SPECIFICATION,
            FrameExifTiffType::Rational,
            4,
            167u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_LENS_MAKE,
            FrameExifTiffType::Ascii,
            7,
            199u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_LENS_MODEL,
            FrameExifTiffType::Ascii,
            8,
            206u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_LENS_SERIAL_NUMBER,
            FrameExifTiffType::Ascii,
            9,
            214u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 116);

        data.extend_from_slice(b"0123456789abcdef0123456789abcdef\0");
        data.extend_from_slice(b"A Camera\0");
        data.extend_from_slice(b"BODY1234\0");
        data.extend_from_slice(&24u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&70u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&28u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&40u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(b"LensCo\0");
        data.extend_from_slice(b"Prime50\0");
        data.extend_from_slice(b"LENS5678\0");

        assert_eq!(data.len(), 223);
        data
    }

    fn exif_gamma_composite_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&4u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_GAMMA,
            FrameExifTiffType::Rational,
            1,
            80u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_COMPOSITE_IMAGE,
            FrameExifTiffType::Short,
            1,
            [2, 0, 0, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SOURCE_IMAGE_NUMBER_OF_COMPOSITE_IMAGE,
            FrameExifTiffType::Short,
            2,
            [5, 0, 3, 0],
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SOURCE_EXPOSURE_TIMES_OF_COMPOSITE_IMAGE,
            FrameExifTiffType::Undefined,
            12,
            88u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 80);

        data.extend_from_slice(&22u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(b"exp-times-01");

        assert_eq!(data.len(), 100);
        data
    }

    fn exif_environment_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&1u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExifIfdPointerKind::EXIF_TAG,
            FrameExifTiffType::Long,
            1,
            26u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 26);

        data.extend_from_slice(&6u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_TEMPERATURE,
            FrameExifTiffType::SignedRational,
            1,
            104u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_HUMIDITY,
            FrameExifTiffType::Rational,
            1,
            112u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_PRESSURE,
            FrameExifTiffType::Rational,
            1,
            120u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_WATER_DEPTH,
            FrameExifTiffType::SignedRational,
            1,
            128u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_ACCELERATION,
            FrameExifTiffType::Rational,
            1,
            136u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_CAMERA_ELEVATION_ANGLE,
            FrameExifTiffType::SignedRational,
            1,
            144u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 104);

        data.extend_from_slice(&(-5i32).to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&55u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1013u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&(-3i32).to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&98u32.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&(-12i32).to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());

        assert_eq!(data.len(), 152);
        data
    }

    fn exif_descriptive_tags_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&5u16.to_le_bytes());
        push_exif_entry(
            &mut data,
            FrameExif::TAG_IMAGE_DESCRIPTION,
            FrameExifTiffType::Ascii,
            13,
            74u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_SOFTWARE,
            FrameExifTiffType::Ascii,
            11,
            87u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_DATE_TIME,
            FrameExifTiffType::Ascii,
            20,
            98u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_ARTIST,
            FrameExifTiffType::Ascii,
            7,
            118u32.to_le_bytes(),
        );
        push_exif_entry(
            &mut data,
            FrameExif::TAG_COPYRIGHT,
            FrameExifTiffType::Ascii,
            13,
            125u32.to_le_bytes(),
        );
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 74);

        data.extend_from_slice(b"Frame sample\0");
        data.extend_from_slice(b"ffmpegrust\0");
        data.extend_from_slice(b"2026:05:05 01:02:03\0");
        data.extend_from_slice(b"OpenAI\0");
        data.extend_from_slice(b"2026 Example\0");

        assert_eq!(data.len(), 138);
        data
    }

    fn big_exif_value_semantics_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x4D, 0x4D, 0x00, 0x2A]);
        data.extend_from_slice(&8u32.to_be_bytes());

        data.extend_from_slice(&3u16.to_be_bytes());
        push_exif_entry_be(
            &mut data,
            0x0112,
            FrameExifTiffType::Short,
            1,
            [0x12, 0x34, 0, 0],
        );
        push_exif_entry_be(
            &mut data,
            0x0100,
            FrameExifTiffType::Long,
            1,
            0x0102_0304u32.to_be_bytes(),
        );
        push_exif_entry_be(
            &mut data,
            0x011A,
            FrameExifTiffType::Rational,
            1,
            50u32.to_be_bytes(),
        );
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&3u32.to_be_bytes());
        data.extend_from_slice(&2u32.to_be_bytes());

        assert_eq!(data.len(), 58);
        data
    }

    fn push_exif_entry(
        data: &mut Vec<u8>,
        tag: u16,
        tiff_type: FrameExifTiffType,
        count: u32,
        value_or_offset: [u8; 4],
    ) {
        data.extend_from_slice(&tag.to_le_bytes());
        data.extend_from_slice(&tiff_type.raw().to_le_bytes());
        data.extend_from_slice(&count.to_le_bytes());
        data.extend_from_slice(&value_or_offset);
    }

    fn push_exif_entry_be(
        data: &mut Vec<u8>,
        tag: u16,
        tiff_type: FrameExifTiffType,
        count: u32,
        value_or_offset: [u8; 4],
    ) {
        data.extend_from_slice(&tag.to_be_bytes());
        data.extend_from_slice(&tiff_type.raw().to_be_bytes());
        data.extend_from_slice(&count.to_be_bytes());
        data.extend_from_slice(&value_or_offset);
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

        let rgb8 = VideoFrame::new(3, 2, PixelFormat::Rgb8, vec![vec![0; 6]]).unwrap();
        assert_eq!(rgb8.line_sizes(), &[3]);

        let pal8 = VideoFrame::new(3, 2, PixelFormat::Pal8, vec![vec![0; 6]]).unwrap();
        assert_eq!(pal8.line_sizes(), &[3]);

        let bgr4_byte = VideoFrame::new(3, 2, PixelFormat::Bgr4Byte, vec![vec![0; 6]]).unwrap();
        assert_eq!(bgr4_byte.line_sizes(), &[3]);

        let rgb4 = VideoFrame::new(3, 2, PixelFormat::Rgb4, vec![vec![0; 4]]).unwrap();
        assert_eq!(rgb4.line_sizes(), &[2]);

        let bgr4 = VideoFrame::new(4, 1, PixelFormat::Bgr4, vec![vec![0; 2]]).unwrap();
        assert_eq!(bgr4.line_sizes(), &[2]);

        let monow = VideoFrame::new(9, 2, PixelFormat::MonoWhite, vec![vec![0; 4]]).unwrap();
        assert_eq!(monow.line_sizes(), &[2]);

        let monob = VideoFrame::new(16, 1, PixelFormat::MonoBlack, vec![vec![0; 2]]).unwrap();
        assert_eq!(monob.line_sizes(), &[2]);

        let rgb565 = VideoFrame::new(3, 2, PixelFormat::Rgb565Le, vec![vec![0; 12]]).unwrap();
        assert_eq!(rgb565.line_sizes(), &[6]);

        let bgr444 = VideoFrame::new(3, 2, PixelFormat::Bgr444Be, vec![vec![0; 12]]).unwrap();
        assert_eq!(bgr444.line_sizes(), &[6]);

        let yuyv422 = VideoFrame::new(2, 2, PixelFormat::Yuyv422, vec![vec![0; 8]]).unwrap();
        assert_eq!(yuyv422.line_sizes(), &[4]);

        let uyvy422 = VideoFrame::new(4, 1, PixelFormat::Uyvy422, vec![vec![0; 8]]).unwrap();
        assert_eq!(uyvy422.line_sizes(), &[8]);

        let y210 = VideoFrame::new(2, 2, PixelFormat::Y210Le, vec![vec![0; 16]]).unwrap();
        assert_eq!(y210.line_sizes(), &[8]);

        let y212 = VideoFrame::new(4, 1, PixelFormat::Y212Be, vec![vec![0; 16]]).unwrap();
        assert_eq!(y212.line_sizes(), &[16]);

        let y216 = VideoFrame::new(2, 2, PixelFormat::Y216Le, vec![vec![0; 16]]).unwrap();
        assert_eq!(y216.line_sizes(), &[8]);

        let nv12 = VideoFrame::new(4, 2, PixelFormat::Nv12, vec![vec![0; 8], vec![1; 4]]).unwrap();
        assert_eq!(nv12.line_sizes(), &[4, 4]);

        let nv21 = VideoFrame::new(2, 4, PixelFormat::Nv21, vec![vec![0; 8], vec![1; 4]]).unwrap();
        assert_eq!(nv21.line_sizes(), &[2, 2]);

        let nv16 =
            VideoFrame::new(4, 3, PixelFormat::Nv16, vec![vec![0; 12], vec![1; 12]]).unwrap();
        assert_eq!(nv16.line_sizes(), &[4, 4]);

        let nv20 =
            VideoFrame::new(4, 3, PixelFormat::Nv20Le, vec![vec![0; 24], vec![1; 24]]).unwrap();
        assert_eq!(nv20.line_sizes(), &[8, 8]);

        let nv42 = VideoFrame::new(3, 2, PixelFormat::Nv42, vec![vec![0; 6], vec![1; 12]]).unwrap();
        assert_eq!(nv42.line_sizes(), &[3, 6]);

        let p010 =
            VideoFrame::new(4, 2, PixelFormat::P010Le, vec![vec![0; 16], vec![1; 8]]).unwrap();
        assert_eq!(p010.line_sizes(), &[8, 8]);

        let p212 =
            VideoFrame::new(4, 3, PixelFormat::P212Be, vec![vec![0; 24], vec![1; 24]]).unwrap();
        assert_eq!(p212.line_sizes(), &[8, 8]);

        let p416 =
            VideoFrame::new(3, 2, PixelFormat::P416Le, vec![vec![0; 12], vec![1; 24]]).unwrap();
        assert_eq!(p416.line_sizes(), &[6, 12]);

        let rgba = VideoFrame::new(3, 2, PixelFormat::Rgba, vec![vec![0; 24]]).unwrap();
        assert_eq!(rgba.line_sizes(), &[12]);

        let x2rgb10 = VideoFrame::new(3, 2, PixelFormat::X2Rgb10Le, vec![vec![0; 24]]).unwrap();
        assert_eq!(x2rgb10.line_sizes(), &[12]);

        let vuya = VideoFrame::new(3, 2, PixelFormat::Vuya, vec![vec![0; 24]]).unwrap();
        assert_eq!(vuya.line_sizes(), &[12]);

        let xv30 = VideoFrame::new(3, 2, PixelFormat::Xv30Le, vec![vec![0; 24]]).unwrap();
        assert_eq!(xv30.line_sizes(), &[12]);

        let v30x = VideoFrame::new(3, 2, PixelFormat::V30xBe, vec![vec![0; 24]]).unwrap();
        assert_eq!(v30x.line_sizes(), &[12]);

        let vyu444 = VideoFrame::new(3, 2, PixelFormat::Vyu444, vec![vec![0; 18]]).unwrap();
        assert_eq!(vyu444.line_sizes(), &[9]);

        let gray16 = VideoFrame::new(3, 2, PixelFormat::Gray16Le, vec![vec![0; 12]]).unwrap();
        assert_eq!(gray16.line_sizes(), &[6]);

        let gray32 = VideoFrame::new(3, 2, PixelFormat::Gray32Le, vec![vec![0; 24]]).unwrap();
        assert_eq!(gray32.line_sizes(), &[12]);

        let grayf16 = VideoFrame::new(3, 2, PixelFormat::GrayF16Le, vec![vec![0; 12]]).unwrap();
        assert_eq!(grayf16.line_sizes(), &[6]);

        let grayf32 = VideoFrame::new(3, 2, PixelFormat::GrayF32Le, vec![vec![0; 24]]).unwrap();
        assert_eq!(grayf32.line_sizes(), &[12]);

        let yaf16 = VideoFrame::new(3, 2, PixelFormat::Yaf16Le, vec![vec![0; 24]]).unwrap();
        assert_eq!(yaf16.line_sizes(), &[12]);

        let yaf32 = VideoFrame::new(3, 2, PixelFormat::Yaf32Be, vec![vec![0; 48]]).unwrap();
        assert_eq!(yaf32.line_sizes(), &[24]);

        let gbrp = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrp,
            vec![vec![0; 6], vec![0; 6], vec![0; 6]],
        )
        .unwrap();
        assert_eq!(gbrp.line_sizes(), &[3, 3, 3]);

        let gbrp9 = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrp9Le,
            vec![vec![0; 12], vec![0; 12], vec![0; 12]],
        )
        .unwrap();
        assert_eq!(gbrp9.line_sizes(), &[6, 6, 6]);

        let gbrp10 = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrp10Be,
            vec![vec![0; 12], vec![0; 12], vec![0; 12]],
        )
        .unwrap();
        assert_eq!(gbrp10.line_sizes(), &[6, 6, 6]);

        let gbrp12 = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrp12Be,
            vec![vec![0; 12], vec![0; 12], vec![0; 12]],
        )
        .unwrap();
        assert_eq!(gbrp12.line_sizes(), &[6, 6, 6]);

        let gbrp14 = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrp14Be,
            vec![vec![0; 12], vec![0; 12], vec![0; 12]],
        )
        .unwrap();
        assert_eq!(gbrp14.line_sizes(), &[6, 6, 6]);

        let gbrp16 = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrp16Be,
            vec![vec![0; 12], vec![0; 12], vec![0; 12]],
        )
        .unwrap();
        assert_eq!(gbrp16.line_sizes(), &[6, 6, 6]);

        let gbrap = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrap,
            vec![vec![0; 6], vec![0; 6], vec![0; 6], vec![0; 6]],
        )
        .unwrap();
        assert_eq!(gbrap.line_sizes(), &[3, 3, 3, 3]);

        let gbrap16 = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrap16Be,
            vec![vec![0; 12], vec![0; 12], vec![0; 12], vec![0; 12]],
        )
        .unwrap();
        assert_eq!(gbrap16.line_sizes(), &[6, 6, 6, 6]);

        let gbrap32 = VideoFrame::new(
            3,
            2,
            PixelFormat::Gbrap32Be,
            vec![vec![0; 24], vec![0; 24], vec![0; 24], vec![0; 24]],
        )
        .unwrap();
        assert_eq!(gbrap32.line_sizes(), &[12, 12, 12, 12]);

        let gbrapf16 = VideoFrame::new(
            3,
            2,
            PixelFormat::GbrapF16Be,
            vec![vec![0; 12], vec![0; 12], vec![0; 12], vec![0; 12]],
        )
        .unwrap();
        assert_eq!(gbrapf16.line_sizes(), &[6, 6, 6, 6]);

        let gbrapf32 = VideoFrame::new(
            3,
            2,
            PixelFormat::GbrapF32Be,
            vec![vec![0; 24], vec![0; 24], vec![0; 24], vec![0; 24]],
        )
        .unwrap();
        assert_eq!(gbrapf32.line_sizes(), &[12, 12, 12, 12]);

        let ya8 = VideoFrame::new(3, 2, PixelFormat::Ya8, vec![vec![0; 12]]).unwrap();
        assert_eq!(ya8.line_sizes(), &[6]);

        let ya16 = VideoFrame::new(3, 2, PixelFormat::Ya16Be, vec![vec![0; 24]]).unwrap();
        assert_eq!(ya16.line_sizes(), &[12]);

        let gray10 = VideoFrame::new(3, 2, PixelFormat::Gray10Le, vec![vec![0; 12]]).unwrap();
        assert_eq!(gray10.line_sizes(), &[6]);

        let rgb48 = VideoFrame::new(3, 2, PixelFormat::Rgb48Le, vec![vec![0; 36]]).unwrap();
        assert_eq!(rgb48.line_sizes(), &[18]);

        let rgba64 = VideoFrame::new(3, 2, PixelFormat::Rgba64Le, vec![vec![0; 48]]).unwrap();
        assert_eq!(rgba64.line_sizes(), &[24]);

        let ayuv64 = VideoFrame::new(2, 2, PixelFormat::Ayuv64Le, vec![vec![0; 32]]).unwrap();
        assert_eq!(ayuv64.line_sizes(), &[16]);

        let xyz12 = VideoFrame::new(3, 2, PixelFormat::Xyz12Le, vec![vec![0; 36]]).unwrap();
        assert_eq!(xyz12.line_sizes(), &[18]);

        let xv36 = VideoFrame::new(3, 2, PixelFormat::Xv36Le, vec![vec![0; 36]]).unwrap();
        assert_eq!(xv36.line_sizes(), &[18]);

        let xv48 = VideoFrame::new(3, 2, PixelFormat::Xv48Be, vec![vec![0; 48]]).unwrap();
        assert_eq!(xv48.line_sizes(), &[24]);

        let rgb0 = VideoFrame::new(3, 2, PixelFormat::Rgb0, vec![vec![0; 24]]).unwrap();
        assert_eq!(rgb0.line_sizes(), &[12]);

        let yuv = VideoFrame::new(
            4,
            2,
            PixelFormat::Yuv420p,
            vec![vec![0; 8], vec![1; 2], vec![2; 2]],
        )
        .unwrap();
        assert_eq!(yuv.line_sizes(), &[4, 2, 2]);

        let yuvj = VideoFrame::new(
            4,
            2,
            PixelFormat::YuvJ420p,
            vec![vec![0; 8], vec![1; 2], vec![2; 2]],
        )
        .unwrap();
        assert_eq!(yuvj.line_sizes(), &[4, 2, 2]);

        let yuv420p9 = VideoFrame::new(
            4,
            2,
            PixelFormat::Yuv420p9Le,
            vec![vec![0; 16], vec![1; 4], vec![2; 4]],
        )
        .unwrap();
        assert_eq!(yuv420p9.line_sizes(), &[8, 4, 4]);

        let yuv420p16 = VideoFrame::new(
            4,
            2,
            PixelFormat::Yuv420p16Le,
            vec![vec![0; 16], vec![1; 4], vec![2; 4]],
        )
        .unwrap();
        assert_eq!(yuv420p16.line_sizes(), &[8, 4, 4]);

        let yuv422 = VideoFrame::new(
            4,
            3,
            PixelFormat::Yuv422p,
            vec![vec![0; 12], vec![1; 6], vec![2; 6]],
        )
        .unwrap();
        assert_eq!(yuv422.line_sizes(), &[4, 2, 2]);

        let yuv420p10 = VideoFrame::new(
            4,
            2,
            PixelFormat::Yuv420p10Le,
            vec![vec![0; 16], vec![1; 4], vec![2; 4]],
        )
        .unwrap();
        assert_eq!(yuv420p10.line_sizes(), &[8, 4, 4]);

        let yuv444p12 = VideoFrame::new(
            3,
            2,
            PixelFormat::Yuv444p12Be,
            vec![vec![0; 12], vec![1; 12], vec![2; 12]],
        )
        .unwrap();
        assert_eq!(yuv444p12.line_sizes(), &[6, 6, 6]);

        let yuvj440 = VideoFrame::new(
            3,
            2,
            PixelFormat::YuvJ440p,
            vec![vec![0; 6], vec![1; 3], vec![2; 3]],
        )
        .unwrap();
        assert_eq!(yuvj440.line_sizes(), &[3, 3, 3]);

        let yuv411 = VideoFrame::new(
            4,
            3,
            PixelFormat::Yuv411p,
            vec![vec![0; 12], vec![1; 3], vec![2; 3]],
        )
        .unwrap();
        assert_eq!(yuv411.line_sizes(), &[4, 1, 1]);

        let yuv410 = VideoFrame::new(
            4,
            4,
            PixelFormat::Yuv410p,
            vec![vec![0; 16], vec![1], vec![2]],
        )
        .unwrap();
        assert_eq!(yuv410.line_sizes(), &[4, 1, 1]);

        let yuv440 = VideoFrame::new(
            3,
            2,
            PixelFormat::Yuv440p,
            vec![vec![0; 6], vec![1; 3], vec![2; 3]],
        )
        .unwrap();
        assert_eq!(yuv440.line_sizes(), &[3, 3, 3]);

        let yuv440p10 = VideoFrame::new(
            3,
            2,
            PixelFormat::Yuv440p10Le,
            vec![vec![0; 12], vec![1; 6], vec![2; 6]],
        )
        .unwrap();
        assert_eq!(yuv440p10.line_sizes(), &[6, 6, 6]);

        let yuv444 = VideoFrame::new(
            3,
            2,
            PixelFormat::Yuv444p,
            vec![vec![0; 6], vec![1; 6], vec![2; 6]],
        )
        .unwrap();
        assert_eq!(yuv444.line_sizes(), &[3, 3, 3]);

        let yuva420 = VideoFrame::new(
            4,
            2,
            PixelFormat::Yuva420p,
            vec![vec![0; 8], vec![1; 2], vec![2; 2], vec![3; 8]],
        )
        .unwrap();
        assert_eq!(yuva420.line_sizes(), &[4, 2, 2, 4]);

        let yuva420p9 = VideoFrame::new(
            4,
            2,
            PixelFormat::Yuva420p9Le,
            vec![vec![0; 16], vec![1; 4], vec![2; 4], vec![3; 16]],
        )
        .unwrap();
        assert_eq!(yuva420p9.line_sizes(), &[8, 4, 4, 8]);

        let yuva444p16 = VideoFrame::new(
            3,
            2,
            PixelFormat::Yuva444p16Be,
            vec![vec![0; 12], vec![1; 12], vec![2; 12], vec![3; 12]],
        )
        .unwrap();
        assert_eq!(yuva444p16.line_sizes(), &[6, 6, 6, 6]);

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
    fn frame_side_data_data_mut_detaches_shared_readonly_payload() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let release_capture = std::sync::Arc::clone(&released);
        let payload = BufferRef::from_external_slice_with_len_and_opaque_readonly(
            std::sync::Arc::<[u8]>::from(vec![1, 2, 3, 0xEE]),
            3,
            String::from("side-data"),
            move |opaque| {
                release_capture.lock().unwrap().push(opaque);
            },
        )
        .unwrap();
        let mut side_data =
            FrameSideData::new_with_buffer_ref("displaymatrix", payload.clone()).unwrap();
        side_data.metadata_mut().set("rotation", "90").unwrap();
        let cloned = side_data.clone();

        assert!(!side_data.is_writable());
        assert!(side_data.buffer().shares_storage(&payload));
        assert!(cloned.buffer().shares_storage(side_data.buffer()));

        side_data.data_mut()[1] = 0x99;

        assert!(side_data.is_writable());
        assert_eq!(side_data.data(), &[1, 0x99, 3]);
        assert_eq!(side_data.metadata().get("rotation"), Some("90"));
        assert!(!side_data.buffer().shares_storage(&payload));
        assert!(!side_data.buffer().shares_storage(cloned.buffer()));
        assert_eq!(cloned.data(), &[1, 2, 3]);
        assert_eq!(payload.as_slice(), &[1, 2, 3]);
        assert_eq!(payload.padding_slice(), &[0xEE]);
        assert!(released.lock().unwrap().is_empty());

        drop(payload);
        assert!(released.lock().unwrap().is_empty());
        drop(side_data);
        assert!(released.lock().unwrap().is_empty());
        drop(cloned);
        assert_eq!(*released.lock().unwrap(), vec![String::from("side-data")]);
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
    fn frame_side_data_parses_a53_closed_captions_payload() {
        let payload = vec![0x04, 0xF8, 0x2A, 0x05, 0x43, 0x21];
        let side_data = FrameSideData::new_a53_closed_captions(payload.clone()).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::A53ClosedCaptions);
        assert_eq!(side_data.data(), payload.as_slice());

        let captions = side_data.a53_closed_captions().unwrap().unwrap();
        assert_eq!(captions.data(), payload.as_slice());
        assert!(!captions.is_empty());
        assert_eq!(captions.entry_count(), 2);
        assert_eq!(captions.entry(0), Some([0x04, 0xF8, 0x2A]));
        assert_eq!(captions.entry(1), Some([0x05, 0x43, 0x21]));
        assert_eq!(captions.entry(2), None);
        assert_eq!(
            captions.entries().collect::<Vec<_>>(),
            vec![[0x04, 0xF8, 0x2A], [0x05, 0x43, 0x21]]
        );

        let empty = FrameSideData::new_a53_closed_captions(Vec::new()).unwrap();
        let empty_captions = empty.a53_closed_captions().unwrap().unwrap();
        assert!(empty_captions.is_empty());
        assert_eq!(empty_captions.entry_count(), 0);

        let display_matrix =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, payload).unwrap();
        assert_eq!(display_matrix.a53_closed_captions().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_a53_closed_captions_payload() {
        for data in [vec![0; 1], vec![0; 2], vec![0; 4], vec![0; 5]] {
            assert_eq!(
                FrameA53ClosedCaptions::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            assert_eq!(
                FrameSideData::new_a53_closed_captions(data.clone())
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::A53ClosedCaptions, data).unwrap();
            assert_eq!(
                side_data.a53_closed_captions().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_a53 =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 3]).unwrap();
        assert_eq!(non_a53.a53_closed_captions().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_stereo3d_payload() {
        let expected = FrameStereo3d::new(
            FrameStereo3dType::SideBySide,
            FrameStereo3dFlags::INVERT,
            FrameStereo3dView::Right,
            FrameStereo3dPrimaryEye::Left,
            63_500,
            Rational::from_raw(-1, 2),
            Rational::from_raw(90, 1),
        )
        .unwrap();

        assert_eq!(FrameStereo3d::DATA_LEN, 36);
        assert_eq!(FrameStereo3d::TYPE_OFFSET, 0);
        assert_eq!(FrameStereo3d::FLAGS_OFFSET, 4);
        assert_eq!(FrameStereo3d::VIEW_OFFSET, 8);
        assert_eq!(FrameStereo3d::PRIMARY_EYE_OFFSET, 12);
        assert_eq!(FrameStereo3d::BASELINE_OFFSET, 16);
        assert_eq!(FrameStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET, 20);
        assert_eq!(FrameStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET, 28);
        assert_eq!(expected.stereo_type(), FrameStereo3dType::SideBySide);
        assert_eq!(expected.flags(), FrameStereo3dFlags::INVERT);
        assert!(expected.has_inverted_views());
        assert!(expected.flags().contains(FrameStereo3dFlags::INVERT));
        assert_eq!(expected.view(), FrameStereo3dView::Right);
        assert_eq!(expected.primary_eye(), FrameStereo3dPrimaryEye::Left);
        assert_eq!(expected.baseline(), 63_500);
        assert_eq!(
            expected.horizontal_disparity_adjustment(),
            Rational::from_raw(-1, 2)
        );
        assert_eq!(
            expected.horizontal_field_of_view(),
            Rational::from_raw(90, 1)
        );
        assert_eq!(
            FrameStereo3d::parse(&expected.to_bytes()).unwrap(),
            expected
        );

        let side_data = FrameSideData::new_stereo3d(expected).unwrap();
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::Stereo3d);
        assert_eq!(side_data.data(), &expected.to_bytes()[..]);
        assert_eq!(side_data.stereo3d().unwrap(), Some(expected));

        let unset_rationals = FrameStereo3d::new(
            FrameStereo3dType::TwoDimensional,
            FrameStereo3dFlags::EMPTY,
            FrameStereo3dView::Packed,
            FrameStereo3dPrimaryEye::None,
            0,
            Rational::from_raw(0, 0),
            Rational::from_raw(0, 0),
        )
        .unwrap();
        assert_eq!(
            FrameStereo3d::parse(&unset_rationals.to_bytes()).unwrap(),
            unset_rationals
        );
        assert!(unset_rationals.flags().is_empty());
        assert!(!unset_rationals.has_inverted_views());

        assert_eq!(
            FrameStereo3dFlags::from_bits(FrameStereo3dFlags::INVERT.bits()).unwrap(),
            FrameStereo3dFlags::INVERT
        );
        assert_eq!(
            FrameStereo3dType::SideBySideQuincunx.ffmpeg_constant(),
            "AV_STEREO3D_SIDEBYSIDE_QUINCUNX"
        );
        assert_eq!(
            FrameStereo3dView::Unspecified.ffmpeg_constant(),
            "AV_STEREO3D_VIEW_UNSPEC"
        );
        assert_eq!(
            FrameStereo3dPrimaryEye::Right.ffmpeg_constant(),
            "AV_PRIMARY_EYE_RIGHT"
        );

        let display_matrix = FrameSideData::new_with_kind(
            FrameSideDataKind::DisplayMatrix,
            expected.to_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(display_matrix.stereo3d().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_stereo3d_payload() {
        for data in [
            Vec::new(),
            vec![0; FrameStereo3d::DATA_LEN - 1],
            vec![0; FrameStereo3d::DATA_LEN + 1],
        ] {
            assert_eq!(
                FrameStereo3d::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::Stereo3d, data).unwrap();
            assert_eq!(
                side_data.stereo3d().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let valid_payload = FrameStereo3d::new(
            FrameStereo3dType::SideBySide,
            FrameStereo3dFlags::INVERT,
            FrameStereo3dView::Packed,
            FrameStereo3dPrimaryEye::Right,
            1,
            Rational::from_raw(0, 1),
            Rational::from_raw(45, 1),
        )
        .unwrap()
        .to_bytes();

        let mut invalid_payloads = Vec::new();
        for (offset, value) in [
            (FrameStereo3d::TYPE_OFFSET, 9),
            (FrameStereo3d::TYPE_OFFSET, -1),
            (FrameStereo3d::FLAGS_OFFSET, 2),
            (FrameStereo3d::FLAGS_OFFSET, -1),
            (FrameStereo3d::VIEW_OFFSET, 4),
            (FrameStereo3d::PRIMARY_EYE_OFFSET, 3),
        ] {
            let mut data = valid_payload.to_vec();
            write_ne_i32(&mut data, offset, value);
            invalid_payloads.push(data);
        }

        for value in [
            Rational::from_raw(2, 1),
            Rational::from_raw(-2, 1),
            Rational::from_raw(1, 0),
        ] {
            let mut data = valid_payload.to_vec();
            write_ne_rational(
                &mut data,
                FrameStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET,
                value,
            );
            invalid_payloads.push(data);
        }

        for value in [Rational::from_raw(-1, 1), Rational::from_raw(1, 0)] {
            let mut data = valid_payload.to_vec();
            write_ne_rational(
                &mut data,
                FrameStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET,
                value,
            );
            invalid_payloads.push(data);
        }

        for data in invalid_payloads {
            assert_eq!(
                FrameStereo3d::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::Stereo3d, data).unwrap();
            assert_eq!(
                side_data.stereo3d().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            FrameStereo3dType::from_raw(9).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameStereo3dFlags::from_bits(2).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameStereo3dFlags::from_raw(-1).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameStereo3d::new(
                FrameStereo3dType::SideBySide,
                FrameStereo3dFlags::EMPTY,
                FrameStereo3dView::Packed,
                FrameStereo3dPrimaryEye::None,
                0,
                Rational::from_raw(2, 1),
                Rational::from_raw(0, 1),
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );

        let non_stereo =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 36]).unwrap();
        assert_eq!(non_stereo.stereo3d().unwrap(), None);
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
    fn frame_side_data_parses_ambient_viewing_environment_payload() {
        let value = FrameAmbientViewingEnvironment::new(
            Rational::from_raw(203, 10),
            Rational::from_raw(15_635, 50_000),
            Rational::from_raw(16_450, 50_000),
        )
        .unwrap();
        let side_data = FrameSideData::new_ambient_viewing_environment(value).unwrap();

        assert_eq!(FrameAmbientViewingEnvironment::DATA_LEN, 24);
        assert_eq!(
            side_data.kind_id(),
            &FrameSideDataKind::AmbientViewingEnvironment
        );
        assert_eq!(side_data.data(), value.to_bytes().as_slice());
        let parsed = side_data.ambient_viewing_environment().unwrap().unwrap();
        assert_eq!(parsed, value);
        assert_eq!(parsed.ambient_illuminance(), Rational::from_raw(203, 10));
        assert_eq!(parsed.ambient_light_x(), Rational::from_raw(15_635, 50_000));
        assert_eq!(parsed.ambient_light_y(), Rational::from_raw(16_450, 50_000));
        assert_eq!(
            FrameAmbientViewingEnvironment::parse(&value.to_bytes()).unwrap(),
            value
        );

        let default_value = FrameAmbientViewingEnvironment::new(
            Rational::from_raw(0, 1),
            Rational::from_raw(0, 1),
            Rational::from_raw(0, 1),
        )
        .unwrap();
        assert_eq!(
            FrameAmbientViewingEnvironment::parse(&default_value.to_bytes()).unwrap(),
            default_value
        );

        let non_ambient =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 36]).unwrap();
        assert_eq!(non_ambient.ambient_viewing_environment().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_ambient_viewing_environment_payload() {
        let value = FrameAmbientViewingEnvironment::new(
            Rational::from_raw(203, 10),
            Rational::from_raw(15_635, 50_000),
            Rational::from_raw(16_450, 50_000),
        )
        .unwrap();

        for data in [
            Vec::new(),
            vec![0; FrameAmbientViewingEnvironment::DATA_LEN - 1],
            vec![0; FrameAmbientViewingEnvironment::DATA_LEN + 1],
        ] {
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::AmbientViewingEnvironment, data)
                    .unwrap();
            assert_eq!(
                side_data.ambient_viewing_environment().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        for (offset, bad_value) in [
            (
                FrameAmbientViewingEnvironment::AMBIENT_ILLUMINANCE_OFFSET,
                Rational::from_raw(-1, 1),
            ),
            (
                FrameAmbientViewingEnvironment::AMBIENT_ILLUMINANCE_OFFSET,
                Rational::from_raw(1, 0),
            ),
            (
                FrameAmbientViewingEnvironment::AMBIENT_LIGHT_X_OFFSET,
                Rational::from_raw(2, 1),
            ),
            (
                FrameAmbientViewingEnvironment::AMBIENT_LIGHT_Y_OFFSET,
                Rational::from_raw(-1, 1),
            ),
            (
                FrameAmbientViewingEnvironment::AMBIENT_LIGHT_Y_OFFSET,
                Rational::from_raw(0, 0),
            ),
        ] {
            let mut bad = value.to_bytes();
            write_ne_rational(&mut bad, offset, bad_value);
            let side_data = FrameSideData::new_with_kind(
                FrameSideDataKind::AmbientViewingEnvironment,
                bad.to_vec(),
            )
            .unwrap();
            assert_eq!(
                side_data.ambient_viewing_environment().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        assert_eq!(
            FrameAmbientViewingEnvironment::new(
                Rational::from_raw(1, 1),
                Rational::from_raw(3, 2),
                Rational::from_raw(0, 1)
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );

        let non_ambient =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_ambient.ambient_viewing_environment().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_video_hint_payload() {
        let first = FrameVideoRect::new(0, 16, 32, 48).unwrap();
        let second = FrameVideoRect::new(64, 0, 16, 16).unwrap();
        let value = FrameVideoHint::new(FrameVideoHintType::Changed, vec![first, second]).unwrap();
        let side_data = FrameSideData::new_video_hint(value.clone()).unwrap();

        assert_eq!(FrameVideoRect::DATA_LEN, 16);
        assert_eq!(
            FrameVideoHint::HEADER_LEN,
            if core::mem::size_of::<usize>() == 8 {
                32
            } else {
                16
            }
        );
        assert_eq!(side_data.kind_id(), &FrameSideDataKind::VideoHint);
        assert_eq!(side_data.data(), value.to_bytes());
        let parsed = side_data.video_hint().unwrap().unwrap();
        assert_eq!(parsed, value);
        assert_eq!(parsed.hint_type(), FrameVideoHintType::Changed);
        assert_eq!(
            parsed.hint_type().ffmpeg_constant(),
            "AV_VIDEO_HINT_TYPE_CHANGED"
        );
        assert_eq!(parsed.nb_rects(), 2);
        assert!(!parsed.is_empty());
        assert_eq!(parsed.rect(0), Some(first));
        assert_eq!(parsed.rect(1), Some(second));
        assert_eq!(parsed.rect(2), None);
        assert_eq!(parsed.rects(), &[first, second]);
        assert_eq!(
            parsed.to_bytes().len(),
            FrameVideoHint::HEADER_LEN + 2 * FrameVideoRect::DATA_LEN
        );
        assert_eq!(FrameVideoHint::parse(&parsed.to_bytes()).unwrap(), parsed);
        assert_eq!(first.to_bytes()[0..4], 0u32.to_ne_bytes());
        assert_eq!(first.x(), 0);
        assert_eq!(first.y(), 16);
        assert_eq!(first.width(), 32);
        assert_eq!(first.height(), 48);

        let empty = FrameVideoHint::new(FrameVideoHintType::Constant, Vec::new()).unwrap();
        let empty_parsed = FrameVideoHint::parse(&empty.to_bytes()).unwrap();
        assert_eq!(empty_parsed.hint_type(), FrameVideoHintType::Constant);
        assert_eq!(
            empty_parsed.hint_type().ffmpeg_constant(),
            "AV_VIDEO_HINT_TYPE_CONSTANT"
        );
        assert_eq!(empty_parsed.nb_rects(), 0);
        assert!(empty_parsed.is_empty());
        assert_eq!(empty_parsed.to_bytes().len(), FrameVideoHint::HEADER_LEN);

        let non_hint =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_hint.video_hint().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_video_hint_payload() {
        let rect = FrameVideoRect::new(0, 0, 16, 16).unwrap();
        let value = FrameVideoHint::new(FrameVideoHintType::Changed, vec![rect]).unwrap();

        assert_eq!(
            FrameVideoRect::parse(&[0; FrameVideoRect::DATA_LEN - 1])
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameVideoHint::parse(&[0; FrameVideoHint::HEADER_LEN - 1])
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        for bad in [
            {
                let mut bad = value.to_bytes();
                write_ne_usize(
                    &mut bad,
                    FrameVideoHint::RECT_OFFSET_OFFSET,
                    FrameVideoHint::HEADER_LEN - 4,
                );
                bad
            },
            {
                let mut bad = value.to_bytes();
                write_ne_usize(
                    &mut bad,
                    FrameVideoHint::RECT_SIZE_OFFSET,
                    FrameVideoRect::DATA_LEN + 4,
                );
                bad
            },
            {
                let mut bad = value.to_bytes();
                write_ne_i32(&mut bad, FrameVideoHint::TYPE_OFFSET, 2);
                bad
            },
            {
                let mut bad = value.to_bytes();
                write_ne_usize(&mut bad, FrameVideoHint::NB_RECTS_OFFSET, 2);
                bad
            },
            {
                let mut bad = value.to_bytes();
                bad.push(0);
                bad
            },
        ] {
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::VideoHint, bad).unwrap();
            assert_eq!(
                side_data.video_hint().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut zero_width = value.to_bytes();
        write_ne_u32(&mut zero_width, FrameVideoHint::HEADER_LEN + 8, 0);
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::VideoHint, zero_width).unwrap();
        assert_eq!(
            side_data.video_hint().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let mut overflow = value.to_bytes();
        write_ne_u32(&mut overflow, FrameVideoHint::HEADER_LEN, u32::MAX);
        write_ne_u32(&mut overflow, FrameVideoHint::HEADER_LEN + 8, 1);
        let side_data =
            FrameSideData::new_with_kind(FrameSideDataKind::VideoHint, overflow).unwrap();
        assert_eq!(
            side_data.video_hint().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        assert_eq!(
            FrameVideoRect::new(0, 0, 0, 16).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            FrameVideoRect::new(u32::MAX, 0, 1, 16).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        let non_hint =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_hint.video_hint().unwrap(), None);
    }

    #[test]
    fn frame_side_data_preserves_lcevc_payload() {
        let lcevc_bytes = vec![0x00, 0x00, 0x01, 0x7E, 0xAB, 0x00, 0x00, 0x03, 0x01];
        let side_data = FrameSideData::new_lcevc(lcevc_bytes.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &FrameSideDataKind::Lcevc);
        assert_eq!(side_data.data(), lcevc_bytes.as_slice());
        let parsed = side_data.lcevc().unwrap();
        assert_eq!(FrameLcevc::parse(&lcevc_bytes), parsed);
        assert_eq!(parsed.data(), lcevc_bytes.as_slice());
        assert!(!parsed.is_empty());

        let empty = FrameSideData::new_lcevc(Vec::new()).unwrap();
        assert!(empty.lcevc().unwrap().is_empty());

        let non_lcevc =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
        assert_eq!(non_lcevc.lcevc(), None);
    }

    #[test]
    fn frame_side_data_parses_view_id_payload() {
        for raw in [42, -1, i32::MIN, i32::MAX] {
            let value = FrameViewId::new(raw);
            let side_data = FrameSideData::new_view_id(value).unwrap();

            assert_eq!(side_data.kind_id(), &FrameSideDataKind::ViewId);
            assert_eq!(side_data.data(), &raw.to_ne_bytes());
            assert_eq!(value.as_raw(), raw);
            assert_eq!(value.to_bytes(), raw.to_ne_bytes());
            assert_eq!(FrameViewId::parse(&value.to_bytes()).unwrap(), value);
            assert_eq!(side_data.view_id().unwrap(), Some(value));
        }

        let non_view =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 4]).unwrap();
        assert_eq!(non_view.view_id().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_view_id_payload() {
        for data in [Vec::new(), vec![0; 3], vec![0; 5]] {
            assert_eq!(
                FrameViewId::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );

            let side_data = FrameSideData::new_with_kind(FrameSideDataKind::ViewId, data).unwrap();
            assert_eq!(
                side_data.view_id().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_view =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_view.view_id().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_three_d_reference_displays_payload() {
        let first = FrameThreeDReferenceDisplay::new(0, 1, (12, 34), (5, 67), true, -11);
        let second = FrameThreeDReferenceDisplay::new(2, 3, (10, 20), (4, 40), false, 0);
        let value = FrameThreeDReferenceDisplays::new(31, true, 7, vec![first, second]).unwrap();
        let side_data = FrameSideData::new_three_d_reference_displays(value.clone()).unwrap();

        assert_eq!(FrameThreeDReferenceDisplay::DATA_LEN, 12);
        assert_eq!(
            FrameThreeDReferenceDisplays::HEADER_LEN,
            if core::mem::size_of::<usize>() == 8 {
                24
            } else {
                12
            }
        );
        assert_eq!(
            FrameThreeDReferenceDisplays::ENTRIES_OFFSET,
            FrameThreeDReferenceDisplays::HEADER_LEN
        );
        assert_eq!(
            side_data.kind_id(),
            &FrameSideDataKind::ThreeDReferenceDisplays
        );
        assert_eq!(side_data.data(), value.to_bytes());
        let parsed = side_data.three_d_reference_displays().unwrap().unwrap();
        assert_eq!(parsed, value);
        assert_eq!(parsed.prec_ref_display_width(), 31);
        assert!(parsed.ref_viewing_distance_flag());
        assert_eq!(parsed.prec_ref_viewing_dist(), 7);
        assert_eq!(parsed.nb_displays(), 2);
        assert_eq!(parsed.displays(), &[first, second]);
        assert_eq!(parsed.display(0), Some(first));
        assert_eq!(parsed.display(1), Some(second));
        assert_eq!(parsed.display(2), None);
        assert_eq!(first.left_view_id(), 0);
        assert_eq!(first.right_view_id(), 1);
        assert_eq!(first.exponent_ref_display_width(), 12);
        assert_eq!(first.mantissa_ref_display_width(), 34);
        assert_eq!(first.exponent_ref_viewing_distance(), 5);
        assert_eq!(first.mantissa_ref_viewing_distance(), 67);
        assert!(first.additional_shift_present());
        assert_eq!(first.num_sample_shift(), -11);
        assert_eq!(
            FrameThreeDReferenceDisplay::parse(&first.to_bytes()).unwrap(),
            first
        );
        assert_eq!(
            FrameThreeDReferenceDisplays::parse(&parsed.to_bytes()).unwrap(),
            parsed
        );

        let non_tdrdi =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
        assert_eq!(non_tdrdi.three_d_reference_displays().unwrap(), None);
    }

    #[test]
    fn frame_side_data_rejects_malformed_three_d_reference_displays_payload() {
        let display = FrameThreeDReferenceDisplay::new(0, 1, (12, 34), (5, 67), true, -11);
        let value = FrameThreeDReferenceDisplays::new(31, true, 7, vec![display]).unwrap();

        assert_eq!(
            FrameThreeDReferenceDisplay::parse(&[0; FrameThreeDReferenceDisplay::DATA_LEN - 1])
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameThreeDReferenceDisplays::parse(&[0; FrameThreeDReferenceDisplays::HEADER_LEN - 1])
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameThreeDReferenceDisplays::new(31, true, 7, Vec::new())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            FrameThreeDReferenceDisplays::new(
                31,
                true,
                7,
                vec![display; FrameThreeDReferenceDisplays::MAX_REF_DISPLAYS + 1],
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );

        for bad in [
            {
                let mut bad = value.to_bytes();
                bad[0] = 32;
                bad
            },
            {
                let mut bad = value.to_bytes();
                bad[1] = 2;
                bad
            },
            {
                let mut bad = value.to_bytes();
                bad[2] = 32;
                bad
            },
            {
                let mut bad = value.to_bytes();
                bad[3] = 0;
                bad
            },
            {
                let mut bad = value.to_bytes();
                bad[3] = (FrameThreeDReferenceDisplays::MAX_REF_DISPLAYS + 1) as u8;
                bad
            },
            {
                let mut bad = value.to_bytes();
                write_ne_usize(
                    &mut bad,
                    FrameThreeDReferenceDisplays::ENTRIES_OFFSET_OFFSET,
                    FrameThreeDReferenceDisplays::ENTRIES_OFFSET - 2,
                );
                bad
            },
            {
                let mut bad = value.to_bytes();
                write_ne_usize(
                    &mut bad,
                    FrameThreeDReferenceDisplays::ENTRY_SIZE_OFFSET,
                    FrameThreeDReferenceDisplay::DATA_LEN + 2,
                );
                bad
            },
            {
                let mut bad = value.to_bytes();
                bad[FrameThreeDReferenceDisplays::ENTRIES_OFFSET
                    + FrameThreeDReferenceDisplay::ADDITIONAL_SHIFT_PRESENT_OFFSET] = 2;
                bad
            },
            {
                let mut bad = value.to_bytes();
                bad.push(0);
                bad
            },
            {
                let mut bad = value.to_bytes();
                bad.pop();
                bad
            },
        ] {
            let side_data =
                FrameSideData::new_with_kind(FrameSideDataKind::ThreeDReferenceDisplays, bad)
                    .unwrap();
            assert_eq!(
                side_data.three_d_reference_displays().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        let non_tdrdi =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_tdrdi.three_d_reference_displays().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_exif_payload() {
        let exif_bytes = minimal_little_exif_fixture();
        let side_data = FrameSideData::new_exif(exif_bytes.clone()).unwrap();

        assert_eq!(side_data.kind_id(), &FrameSideDataKind::Exif);
        assert_eq!(side_data.data(), exif_bytes.as_slice());
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
        assert_eq!(entry.tiff_type().raw(), 2);
        assert_eq!(entry.tiff_type().element_size(), 1);
        assert_eq!(entry.count(), 6);
        assert_eq!(entry.value_offset(), 26);
        assert_eq!(entry.data_len(), 6);
        assert!(!entry.is_inline());
        assert_eq!(entry.value_or_offset_bytes(), &26u32.to_le_bytes());
        assert_eq!(entry.data_range(), Some((26, 32)));
        assert_eq!(entry.value_data(), b"Rusty\0");
        assert_eq!(entry.ascii_strings().unwrap().unwrap(), ["Rusty"]);
        assert_eq!(ifd.entries(), &[entry]);
        assert_eq!(FrameExif::parse(parsed.data()).unwrap(), parsed);

        let big = FrameSideData::new_exif(minimal_big_exif_fixture()).unwrap();
        let parsed_big = big.exif().unwrap().unwrap();
        assert_eq!(parsed_big.endian(), FrameExifEndian::Big);
        assert_eq!(parsed_big.first_ifd_offset(), 8);
        assert_eq!(parsed_big.ifd_count(), 1);
        assert!(parsed_big.ifd(0).unwrap().is_empty());
        assert_eq!(parsed_big.ifd(0).unwrap().next_ifd_offset(), None);

        let non_exif =
            FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
        assert_eq!(non_exif.exif().unwrap(), None);
    }

    #[test]
    fn frame_side_data_parses_exif_linked_ifds() {
        let side_data = FrameSideData::new_exif(exif_with_linked_ifds_fixture()).unwrap();
        let parsed = side_data.exif().unwrap().unwrap();

        assert_eq!(parsed.ifd_count(), 1);
        assert_eq!(parsed.ifd(0).unwrap().entry_count(), 3);
        assert_eq!(parsed.linked_ifd_count(), 3);
        assert_eq!(
            parsed.linked_ifds()[0].kind(),
            FrameExifIfdPointerKind::Exif
        );
        assert_eq!(parsed.linked_ifds()[1].kind(), FrameExifIfdPointerKind::Gps);
        assert_eq!(
            parsed.linked_ifds()[2].kind(),
            FrameExifIfdPointerKind::Interoperability
        );

        let exif_ifd = parsed.linked_ifd(FrameExifIfdPointerKind::Exif).unwrap();
        assert_eq!(exif_ifd.parent_ifd_offset(), 8);
        assert_eq!(exif_ifd.source_tag(), FrameExifIfdPointerKind::EXIF_TAG);
        assert_eq!(exif_ifd.offset(), 56);
        assert_eq!(exif_ifd.ifd().entry_count(), 1);
        assert_eq!(
            exif_ifd
                .ifd()
                .entry_by_tag(FrameExifIfdPointerKind::INTEROPERABILITY_TAG)
                .unwrap()
                .ifd_pointer_offset()
                .unwrap(),
            Some(92)
        );

        let gps_ifd = parsed.linked_ifd(FrameExifIfdPointerKind::Gps).unwrap();
        assert_eq!(gps_ifd.parent_ifd_offset(), 8);
        assert_eq!(gps_ifd.source_tag(), FrameExifIfdPointerKind::GPS_TAG);
        assert_eq!(gps_ifd.offset(), 74);
        let gps_version = gps_ifd.ifd().entry_by_tag(0x0000).unwrap();
        assert_eq!(gps_version.tiff_type(), FrameExifTiffType::Byte);
        assert_eq!(gps_version.count(), 4);
        assert_eq!(gps_version.value_data(), &[2, 3, 0, 0]);

        let interop = parsed
            .linked_ifd(FrameExifIfdPointerKind::Interoperability)
            .unwrap();
        assert_eq!(interop.parent_ifd_offset(), 56);
        assert_eq!(
            interop.source_tag(),
            FrameExifIfdPointerKind::INTEROPERABILITY_TAG
        );
        assert_eq!(interop.offset(), 92);
        let interop_index = interop.ifd().entry_by_tag(0x0001).unwrap();
        assert_eq!(interop_index.tiff_type(), FrameExifTiffType::Ascii);
        assert_eq!(interop_index.value_data(), b"R98\0");

        assert_eq!(parsed.ifd(0).unwrap().entry_by_tag(0xDEAD), None);
        assert_eq!(FrameExifIfdPointerKind::from_tag(0x010F), None);
    }

    #[test]
    fn frame_side_data_decodes_exif_entry_values() {
        let side_data = FrameSideData::new_exif(exif_value_semantics_fixture()).unwrap();
        let parsed = side_data.exif().unwrap().unwrap();
        let ifd = parsed.ifd(0).unwrap();

        let make = ifd.entry_by_tag(0x010F).unwrap();
        assert_eq!(make.ascii_strings().unwrap().unwrap(), ["Rusty"]);
        assert_eq!(make.short_values().unwrap(), None);

        let orientation = ifd.entry_by_tag(0x0112).unwrap();
        assert_eq!(orientation.short_values().unwrap().unwrap(), [6]);
        assert_eq!(orientation.long_values().unwrap(), None);

        let width = ifd.entry_by_tag(0x0100).unwrap();
        assert_eq!(width.long_values().unwrap().unwrap(), [640]);

        let x_resolution = ifd.entry_by_tag(0x011A).unwrap();
        assert_eq!(
            x_resolution.rational_values().unwrap().unwrap(),
            [FrameExifRational {
                numerator: 300,
                denominator: 1,
            }]
        );

        let signed_short = ifd.entry_by_tag(0xC001).unwrap();
        assert_eq!(
            signed_short.signed_short_values().unwrap().unwrap(),
            [-1, 2]
        );

        let signed_long = ifd.entry_by_tag(0xC002).unwrap();
        assert_eq!(signed_long.signed_long_values().unwrap().unwrap(), [-42]);

        let signed_rational = ifd.entry_by_tag(0xC003).unwrap();
        assert_eq!(
            signed_rational.signed_rational_values().unwrap().unwrap(),
            [FrameExifSignedRational {
                numerator: -1,
                denominator: 2,
            }]
        );

        let signed_byte = ifd.entry_by_tag(0xC004).unwrap();
        assert_eq!(
            signed_byte.signed_byte_values().unwrap().unwrap(),
            [-1, 0, 2]
        );
        assert_eq!(signed_byte.byte_values().unwrap(), None);

        let float = ifd.entry_by_tag(0xC005).unwrap();
        assert_eq!(
            float.float_values().unwrap().unwrap()[0].to_bits(),
            1.25f32.to_bits()
        );
        assert_eq!(float.long_values().unwrap(), None);

        let double = ifd.entry_by_tag(0xC006).unwrap();
        assert_eq!(
            double.double_values().unwrap().unwrap()[0].to_bits(),
            (-2.5f64).to_bits()
        );
        assert_eq!(double.rational_values().unwrap(), None);

        let gps_version = ifd.entry_by_tag(0x0000).unwrap();
        assert_eq!(gps_version.byte_values().unwrap().unwrap(), &[2, 3, 0, 0]);

        let big_bytes = big_exif_value_semantics_fixture();
        let big = FrameExif::parse(&big_bytes).unwrap();
        let big_ifd = big.ifd(0).unwrap();
        assert_eq!(
            big_ifd
                .entry_by_tag(0x0112)
                .unwrap()
                .short_values()
                .unwrap()
                .unwrap(),
            [0x1234]
        );
        assert_eq!(
            big_ifd
                .entry_by_tag(0x0100)
                .unwrap()
                .long_values()
                .unwrap()
                .unwrap(),
            [0x0102_0304]
        );
        assert_eq!(
            big_ifd
                .entry_by_tag(0x011A)
                .unwrap()
                .rational_values()
                .unwrap()
                .unwrap(),
            [FrameExifRational {
                numerator: 3,
                denominator: 2,
            }]
        );

        let mut bad_ascii = exif_value_semantics_fixture();
        bad_ascii[151] = b'!';
        let bad_ascii = FrameExif::parse(&bad_ascii).unwrap();
        assert_eq!(
            bad_ascii
                .ifd(0)
                .unwrap()
                .entry_by_tag(0x010F)
                .unwrap()
                .ascii_strings()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_rational = exif_value_semantics_fixture();
        bad_rational[156..160].copy_from_slice(&0u32.to_le_bytes());
        let bad_rational = FrameExif::parse(&bad_rational).unwrap();
        assert_eq!(
            bad_rational
                .ifd(0)
                .unwrap()
                .entry_by_tag(0x011A)
                .unwrap()
                .rational_values()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_common_exif_tags() {
        let side_data = FrameSideData::new_exif(exif_common_tags_fixture()).unwrap();
        let parsed = side_data.exif().unwrap().unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.make(), Some("Rusty"));
        assert_eq!(common.model(), Some("Camera"));
        assert_eq!(common.image_width(), Some(640));
        assert_eq!(common.image_length(), Some(480));
        assert_eq!(common.orientation(), Some(FrameExifOrientation::RightTop));
        assert_eq!(common.orientation().unwrap().raw(), 6);
        assert_eq!(
            common.x_resolution(),
            Some(FrameExifRational {
                numerator: 300,
                denominator: 1,
            })
        );
        assert_eq!(common.y_resolution(), None);
        assert_eq!(
            common.resolution_unit(),
            Some(FrameExifResolutionUnit::Inch)
        );
        assert_eq!(common.resolution_unit().unwrap().raw(), 2);
        assert_eq!(common.exif_version(), Some(*b"0231"));
        assert_eq!(common.date_time_original(), Some("2026:05:04 12:34:56"));
        assert_eq!(common.gps_version_id(), Some([2, 3, 0, 0]));
        assert_eq!(
            common.gps_latitude_ref(),
            Some(FrameExifGpsLatitudeRef::North)
        );
        assert_eq!(
            common.gps_latitude(),
            Some([
                FrameExifRational {
                    numerator: 37,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 48,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 30,
                    denominator: 1,
                },
            ])
        );
        assert_eq!(
            common.gps_longitude_ref(),
            Some(FrameExifGpsLongitudeRef::West)
        );
        assert_eq!(
            common.gps_longitude(),
            Some([
                FrameExifRational {
                    numerator: 122,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 24,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 15,
                    denominator: 1,
                },
            ])
        );
        assert_eq!(common.interoperability_index(), Some("R98"));

        let mut bad_image_width_zero = exif_common_tags_fixture();
        bad_image_width_zero[42..46].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_image_width_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_image_length_zero = exif_common_tags_fixture();
        bad_image_length_zero[54..58].copy_from_slice(&[0; 4]);
        assert_eq!(
            FrameExif::parse(&bad_image_length_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_latitude_degrees = exif_common_tags_fixture();
        bad_latitude_degrees[290..294].copy_from_slice(&91u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_latitude_degrees)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut latitude_boundary = exif_common_tags_fixture();
        latitude_boundary[290..294].copy_from_slice(&90u32.to_le_bytes());
        latitude_boundary[298..302].copy_from_slice(&0u32.to_le_bytes());
        latitude_boundary[306..310].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&latitude_boundary)
                .unwrap()
                .common_tags()
                .unwrap()
                .gps_latitude(),
            Some([
                FrameExifRational {
                    numerator: 90,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 0,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 0,
                    denominator: 1,
                },
            ])
        );

        let mut bad_latitude_composite = exif_common_tags_fixture();
        bad_latitude_composite[290..294].copy_from_slice(&90u32.to_le_bytes());
        bad_latitude_composite[298..302].copy_from_slice(&1u32.to_le_bytes());
        bad_latitude_composite[306..310].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_latitude_composite)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_longitude_seconds = exif_common_tags_fixture();
        bad_longitude_seconds[330..334].copy_from_slice(&60u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_longitude_seconds)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut longitude_boundary = exif_common_tags_fixture();
        longitude_boundary[314..318].copy_from_slice(&180u32.to_le_bytes());
        longitude_boundary[322..326].copy_from_slice(&0u32.to_le_bytes());
        longitude_boundary[330..334].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&longitude_boundary)
                .unwrap()
                .common_tags()
                .unwrap()
                .gps_longitude(),
            Some([
                FrameExifRational {
                    numerator: 180,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 0,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 0,
                    denominator: 1,
                },
            ])
        );

        let mut bad_longitude_composite = exif_common_tags_fixture();
        bad_longitude_composite[314..318].copy_from_slice(&180u32.to_le_bytes());
        bad_longitude_composite[322..326].copy_from_slice(&0u32.to_le_bytes());
        bad_longitude_composite[330..334].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_longitude_composite)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_orientation_count = exif_common_tags_fixture();
        bad_orientation_count[62..66].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_orientation_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_exif_version_count = exif_common_tags_fixture();
        bad_exif_version_count[150..154].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_exif_version_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_exif_version_digit = exif_common_tags_fixture();
        bad_exif_version_digit[154] = b'v';
        assert_eq!(
            FrameExif::parse(&bad_exif_version_digit)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_original_count = exif_common_tags_fixture();
        bad_original_count[162..166].copy_from_slice(&19u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_original_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_original_shape = exif_common_tags_fixture();
        bad_original_shape[196] = b'T';
        assert_eq!(
            FrameExif::parse(&bad_original_shape)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_original_month = exif_common_tags_fixture();
        bad_original_month[191..193].copy_from_slice(b"13");
        assert_eq!(
            FrameExif::parse(&bad_original_month)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_original_calendar_day = exif_common_tags_fixture();
        bad_original_calendar_day[191..193].copy_from_slice(b"04");
        bad_original_calendar_day[194..196].copy_from_slice(b"31");
        assert_eq!(
            FrameExif::parse(&bad_original_calendar_day)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_gps_version_count = exif_common_tags_fixture();
        bad_gps_version_count[230..234].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_gps_version_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_latitude_ref_count = exif_common_tags_fixture();
        bad_latitude_ref_count[242..246].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_latitude_ref_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_gps_ref = exif_common_tags_fixture();
        bad_gps_ref[246] = b'X';
        assert_eq!(
            FrameExif::parse(&bad_gps_ref)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_gps_altitude_time_tags() {
        let exif_bytes = exif_gps_altitude_time_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.gps_altitude_ref(),
            Some(FrameExifGpsAltitudeRef::BelowSeaLevel)
        );
        assert_eq!(common.gps_altitude_ref().unwrap().raw(), 1);
        assert_eq!(
            common.gps_altitude(),
            Some(FrameExifRational {
                numerator: 15,
                denominator: 2,
            })
        );
        assert_eq!(
            common.gps_time_stamp(),
            Some([
                FrameExifRational {
                    numerator: 12,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 34,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 56,
                    denominator: 1,
                },
            ])
        );
        assert_eq!(common.gps_date_stamp(), Some("2026:05:06"));

        let mut bad_altitude_ref = exif_gps_altitude_time_fixture();
        bad_altitude_ref[36] = 2;
        assert_eq!(
            FrameExif::parse(&bad_altitude_ref)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_time_stamp_count = exif_gps_altitude_time_fixture();
        bad_time_stamp_count[56..60].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_time_stamp_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_time_stamp_hour = exif_gps_altitude_time_fixture();
        bad_time_stamp_hour[88..92].copy_from_slice(&24u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_time_stamp_hour)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_time_stamp_seconds = exif_gps_altitude_time_fixture();
        bad_time_stamp_seconds[104..108].copy_from_slice(&60u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_time_stamp_seconds)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_time_stamp_composite = exif_gps_altitude_time_fixture();
        bad_time_stamp_composite[88..92].copy_from_slice(&47u32.to_le_bytes());
        bad_time_stamp_composite[92..96].copy_from_slice(&2u32.to_le_bytes());
        bad_time_stamp_composite[96..100].copy_from_slice(&30u32.to_le_bytes());
        bad_time_stamp_composite[104..108].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_time_stamp_composite)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_date_stamp_type = exif_gps_altitude_time_fixture();
        bad_date_stamp_type[66..68].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_date_stamp_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_date_stamp_day = exif_gps_altitude_time_fixture();
        bad_date_stamp_day[120..122].copy_from_slice(b"32");
        assert_eq!(
            FrameExif::parse(&bad_date_stamp_day)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_date_stamp_calendar_day = exif_gps_altitude_time_fixture();
        bad_date_stamp_calendar_day[117..119].copy_from_slice(b"02");
        bad_date_stamp_calendar_day[120..122].copy_from_slice(b"30");
        assert_eq!(
            FrameExif::parse(&bad_date_stamp_calendar_day)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut leap_date_stamp = exif_gps_altitude_time_fixture();
        leap_date_stamp[112..122].copy_from_slice(b"2024:02:29");
        assert_eq!(
            FrameExif::parse(&leap_date_stamp)
                .unwrap()
                .common_tags()
                .unwrap()
                .gps_date_stamp(),
            Some("2024:02:29")
        );

        let mut bad_date_stamp_count = exif_gps_altitude_time_fixture();
        bad_date_stamp_count[68..72].copy_from_slice(&10u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_date_stamp_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_date_stamp_shape = exif_gps_altitude_time_fixture();
        bad_date_stamp_shape[116] = b'-';
        assert_eq!(
            FrameExif::parse(&bad_date_stamp_shape)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_gps_acquisition_tags() {
        let exif_bytes = exif_gps_acquisition_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.gps_satellites(), Some("12 used"));
        assert_eq!(
            common.gps_status(),
            Some(FrameExifGpsStatus::MeasurementInProgress)
        );
        assert_eq!(common.gps_status().unwrap().as_str(), "A");
        assert_eq!(
            common.gps_measure_mode(),
            Some(FrameExifGpsMeasureMode::ThreeDimensional)
        );
        assert_eq!(common.gps_measure_mode().unwrap().as_str(), "3");
        assert_eq!(
            common.gps_dop(),
            Some(FrameExifRational {
                numerator: 7,
                denominator: 2,
            })
        );

        let mut bad_status = exif_gps_acquisition_fixture();
        bad_status[48] = b'X';
        assert_eq!(
            FrameExif::parse(&bad_status)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_status_count = exif_gps_acquisition_fixture();
        bad_status_count[44..48].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_status_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_measure_mode_count = exif_gps_acquisition_fixture();
        bad_measure_mode_count[56..60].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_measure_mode_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_dop_type = exif_gps_acquisition_fixture();
        bad_dop_type[66..68].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_dop_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_gps_motion_tags() {
        let exif_bytes = exif_gps_motion_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.gps_speed_ref(),
            Some(FrameExifGpsSpeedRef::KilometersPerHour)
        );
        assert_eq!(common.gps_speed_ref().unwrap().as_str(), "K");
        assert_eq!(
            common.gps_speed(),
            Some(FrameExifRational {
                numerator: 88,
                denominator: 5,
            })
        );
        assert_eq!(
            common.gps_track_ref(),
            Some(FrameExifGpsDirectionRef::TrueDirection)
        );
        assert_eq!(common.gps_track_ref().unwrap().as_str(), "T");
        assert_eq!(
            common.gps_track(),
            Some(FrameExifRational {
                numerator: 270,
                denominator: 1,
            })
        );
        let mut bad_track_direction = exif_gps_motion_fixture();
        bad_track_direction[124..128].copy_from_slice(&360u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_track_direction)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            common.gps_img_direction_ref(),
            Some(FrameExifGpsDirectionRef::MagneticDirection)
        );
        assert_eq!(common.gps_img_direction_ref().unwrap().as_str(), "M");
        assert_eq!(
            common.gps_img_direction(),
            Some(FrameExifRational {
                numerator: 135,
                denominator: 1,
            })
        );
        let mut bad_img_direction = exif_gps_motion_fixture();
        bad_img_direction[132..136].copy_from_slice(&360u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_img_direction)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(common.gps_map_datum(), Some("WGS-84"));

        let mut bad_speed_ref = exif_gps_motion_fixture();
        bad_speed_ref[36] = b'X';
        assert_eq!(
            FrameExif::parse(&bad_speed_ref)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_track_ref_count = exif_gps_motion_fixture();
        bad_track_ref_count[56..60].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_track_ref_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_img_direction_count = exif_gps_motion_fixture();
        bad_img_direction_count[92..96].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_img_direction_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_map_datum_type = exif_gps_motion_fixture();
        bad_map_datum_type[102..104].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_map_datum_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_gps_destination_tags() {
        let exif_bytes = exif_gps_destination_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.gps_dest_latitude_ref(),
            Some(FrameExifGpsLatitudeRef::South)
        );
        assert_eq!(
            common.gps_dest_latitude(),
            Some([
                FrameExifRational {
                    numerator: 33,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 52,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 7,
                    denominator: 1,
                },
            ])
        );
        assert_eq!(
            common.gps_dest_longitude_ref(),
            Some(FrameExifGpsLongitudeRef::East)
        );
        assert_eq!(
            common.gps_dest_longitude(),
            Some([
                FrameExifRational {
                    numerator: 151,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 12,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 9,
                    denominator: 1,
                },
            ])
        );
        assert_eq!(
            common.gps_dest_bearing_ref(),
            Some(FrameExifGpsDirectionRef::TrueDirection)
        );
        assert_eq!(common.gps_dest_bearing_ref().unwrap().as_str(), "T");
        assert_eq!(
            common.gps_dest_bearing(),
            Some(FrameExifRational {
                numerator: 91,
                denominator: 2,
            })
        );
        let mut bad_dest_bearing = exif_gps_destination_fixture();
        bad_dest_bearing[176..180].copy_from_slice(&720u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_dest_bearing)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            common.gps_dest_distance_ref(),
            Some(FrameExifGpsDistanceRef::NauticalMiles)
        );
        assert_eq!(common.gps_dest_distance_ref().unwrap().as_str(), "N");
        assert_eq!(
            common.gps_dest_distance(),
            Some(FrameExifRational {
                numerator: 42,
                denominator: 1,
            })
        );

        let mut bad_dest_latitude_ref = exif_gps_destination_fixture();
        bad_dest_latitude_ref[36] = b'X';
        assert_eq!(
            FrameExif::parse(&bad_dest_latitude_ref)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_dest_latitude_degrees = exif_gps_destination_fixture();
        bad_dest_latitude_degrees[128..132].copy_from_slice(&91u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_dest_latitude_degrees)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_dest_longitude_count = exif_gps_destination_fixture();
        bad_dest_longitude_count[68..72].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_dest_longitude_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_dest_longitude_minutes = exif_gps_destination_fixture();
        bad_dest_longitude_minutes[160..164].copy_from_slice(&60u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_dest_longitude_minutes)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_dest_distance_ref_count = exif_gps_destination_fixture();
        bad_dest_distance_ref_count[104..108].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_dest_distance_ref_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_dest_distance_ref_type = exif_gps_destination_fixture();
        bad_dest_distance_ref_type[102..104]
            .copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_dest_distance_ref_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_gps_processing_error_tags() {
        let exif_bytes = exif_gps_processing_error_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.gps_processing_method(),
            Some(&b"ASCII\0\0\0GPS\0"[..])
        );
        assert_eq!(common.gps_area_information(), Some(&b"ASCII\0\0\0AREA"[..]));
        assert_eq!(
            common.gps_differential(),
            Some(FrameExifGpsDifferential::DifferentialCorrectionApplied)
        );
        assert_eq!(common.gps_differential().unwrap().raw(), 1);
        assert_eq!(
            common.gps_h_positioning_error(),
            Some(FrameExifRational {
                numerator: 5,
                denominator: 2,
            })
        );

        let mut bad_processing_type = exif_gps_processing_error_fixture();
        bad_processing_type[30..32].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_processing_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_area_type = exif_gps_processing_error_fixture();
        bad_area_type[42..44].copy_from_slice(&FrameExifTiffType::Ascii.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_area_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_differential_count = exif_gps_processing_error_fixture();
        bad_differential_count[56..60].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_differential_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_differential_value = exif_gps_processing_error_fixture();
        bad_differential_value[60..62].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_differential_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_h_positioning_error_type = exif_gps_processing_error_fixture();
        bad_h_positioning_error_type[66..68]
            .copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_h_positioning_error_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_h_positioning_error_denominator = exif_gps_processing_error_fixture();
        bad_h_positioning_error_denominator[108..112].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_h_positioning_error_denominator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_image_layout_tags() {
        let exif_bytes = exif_root_image_layout_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.samples_per_pixel(), Some(3));
        assert_eq!(
            common.planar_configuration(),
            Some(FrameExifPlanarConfiguration::Chunky)
        );
        assert_eq!(common.planar_configuration().unwrap().raw(), 1);
        assert_eq!(common.ycbcr_sub_sampling(), Some([2, 2]));
        assert_eq!(
            common.ycbcr_positioning(),
            Some(FrameExifYcbCrPositioning::Centered)
        );
        assert_eq!(common.ycbcr_positioning().unwrap().raw(), 1);

        let mut bad_samples_type = exif_root_image_layout_fixture();
        bad_samples_type[12..14].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_samples_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_samples_count = exif_root_image_layout_fixture();
        bad_samples_count[14..18].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_samples_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_samples_zero = exif_root_image_layout_fixture();
        bad_samples_zero[18..20].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_samples_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_planar_count = exif_root_image_layout_fixture();
        bad_planar_count[26..30].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_planar_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_planar_value = exif_root_image_layout_fixture();
        bad_planar_value[30..32].copy_from_slice(&3u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_planar_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subsampling_type = exif_root_image_layout_fixture();
        bad_subsampling_type[36..38].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subsampling_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subsampling_count = exif_root_image_layout_fixture();
        bad_subsampling_count[38..42].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subsampling_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subsampling_zero = exif_root_image_layout_fixture();
        bad_subsampling_zero[42..44].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subsampling_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_ycbcr_positioning_type = exif_root_image_layout_fixture();
        bad_ycbcr_positioning_type[48..50]
            .copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_ycbcr_positioning_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_ycbcr_positioning_value = exif_root_image_layout_fixture();
        bad_ycbcr_positioning_value[54..56].copy_from_slice(&3u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_ycbcr_positioning_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_colorimetry_tags() {
        let exif_bytes = exif_root_colorimetry_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.white_point(),
            Some([
                FrameExifRational {
                    numerator: 1,
                    denominator: 3,
                },
                FrameExifRational {
                    numerator: 1,
                    denominator: 4,
                },
            ])
        );
        assert_eq!(
            common.primary_chromaticities(),
            Some([
                FrameExifRational {
                    numerator: 640,
                    denominator: 1000,
                },
                FrameExifRational {
                    numerator: 330,
                    denominator: 1000,
                },
                FrameExifRational {
                    numerator: 300,
                    denominator: 1000,
                },
                FrameExifRational {
                    numerator: 600,
                    denominator: 1000,
                },
                FrameExifRational {
                    numerator: 150,
                    denominator: 1000,
                },
                FrameExifRational {
                    numerator: 60,
                    denominator: 1000,
                },
            ])
        );
        assert_eq!(
            common.ycbcr_coefficients(),
            Some([
                FrameExifRational {
                    numerator: 299,
                    denominator: 1000,
                },
                FrameExifRational {
                    numerator: 587,
                    denominator: 1000,
                },
                FrameExifRational {
                    numerator: 114,
                    denominator: 1000,
                },
            ])
        );
        assert_eq!(
            common.reference_black_white(),
            Some([
                FrameExifRational {
                    numerator: 0,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 255,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 128,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 255,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 128,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 255,
                    denominator: 1,
                },
            ])
        );

        let mut bad_white_count = exif_root_colorimetry_fixture();
        bad_white_count[14..18].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_white_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_primary_type = exif_root_colorimetry_fixture();
        bad_primary_type[24..26].copy_from_slice(&FrameExifTiffType::Short.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_primary_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_ycbcr_denominator = exif_root_colorimetry_fixture();
        bad_ycbcr_denominator[130..134].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_ycbcr_denominator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_reference_count = exif_root_colorimetry_fixture();
        bad_reference_count[50..54].copy_from_slice(&5u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_reference_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_subfile_type_tags() {
        let exif_bytes = exif_root_subfile_type_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();
        let new_subfile_type = common.new_subfile_type().unwrap();

        assert_eq!(new_subfile_type.raw(), 0x3);
        assert!(new_subfile_type.is_reduced_resolution_image());
        assert!(new_subfile_type.is_single_page_of_multi_page_image());
        assert!(!new_subfile_type.is_transparency_mask());
        assert_eq!(
            common.subfile_type(),
            Some(FrameExifSubfileType::ReducedResolutionImage)
        );
        assert_eq!(
            common.subfile_type().map(FrameExifSubfileType::raw),
            Some(2)
        );

        let mut bad_new_subfile_type_type = exif_root_subfile_type_fixture();
        bad_new_subfile_type_type[12..14]
            .copy_from_slice(&FrameExifTiffType::Short.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_new_subfile_type_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_new_subfile_type_flags = exif_root_subfile_type_fixture();
        bad_new_subfile_type_flags[18..22].copy_from_slice(&0x8u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_new_subfile_type_flags)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subfile_type_count = exif_root_subfile_type_fixture();
        bad_subfile_type_count[26..30].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subfile_type_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subfile_type_value = exif_root_subfile_type_fixture();
        bad_subfile_type_value[30..32].copy_from_slice(&4u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subfile_type_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_camera_identity_tags() {
        let exif_bytes = exif_root_camera_identity_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.make(), Some("MK"));
        assert_eq!(common.model(), Some("M2"));

        let mut bad_make_type = exif_root_camera_identity_fixture();
        bad_make_type[12..14].copy_from_slice(&FrameExifTiffType::Undefined.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_make_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_model_terminator = exif_root_camera_identity_fixture();
        bad_model_terminator[32] = b'!';
        assert_eq!(
            FrameExif::parse(&bad_model_terminator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_model_multiple_strings = exif_root_camera_identity_fixture();
        bad_model_multiple_strings[26..30].copy_from_slice(&4u32.to_le_bytes());
        bad_model_multiple_strings[30..34].copy_from_slice(&[b'M', 0, b'2', 0]);
        assert_eq!(
            FrameExif::parse(&bad_model_multiple_strings)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_orientation_resolution_tags() {
        let exif_bytes = exif_root_orientation_resolution_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.orientation(), Some(FrameExifOrientation::RightTop));
        assert_eq!(
            common.resolution_unit(),
            Some(FrameExifResolutionUnit::Inch)
        );

        let mut bad_orientation_type = exif_root_orientation_resolution_fixture();
        bad_orientation_type[12..14]
            .copy_from_slice(&FrameExifTiffType::Undefined.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_orientation_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_orientation_value = exif_root_orientation_resolution_fixture();
        bad_orientation_value[18..20].copy_from_slice(&9u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_orientation_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_resolution_count = exif_root_orientation_resolution_fixture();
        bad_resolution_count[26..30].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_resolution_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_resolution_value = exif_root_orientation_resolution_fixture();
        bad_resolution_value[30..32].copy_from_slice(&4u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_resolution_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_resolution_tags() {
        let exif_bytes = exif_root_resolution_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.x_resolution(),
            Some(FrameExifRational {
                numerator: 300,
                denominator: 1,
            })
        );
        assert_eq!(
            common.y_resolution(),
            Some(FrameExifRational {
                numerator: 72,
                denominator: 1,
            })
        );

        let mut bad_x_type = exif_root_resolution_fixture();
        bad_x_type[12..14].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_x_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_x_count = exif_root_resolution_fixture();
        bad_x_count[14..18].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_x_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_x_denominator = exif_root_resolution_fixture();
        bad_x_denominator[42..46].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_x_denominator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_y_count = exif_root_resolution_fixture();
        bad_y_count[26..30].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_y_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_y_denominator = exif_root_resolution_fixture();
        bad_y_denominator[50..54].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_y_denominator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_document_page_tags() {
        let exif_bytes = exif_root_document_page_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.document_name(), Some("Doc"));
        assert_eq!(common.page_name(), Some("Page A"));
        assert_eq!(common.page_number(), Some([1, 10]));

        let mut bad_document_type = exif_root_document_page_fixture();
        bad_document_type[12..14]
            .copy_from_slice(&FrameExifTiffType::Undefined.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_document_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_page_name_terminator = exif_root_document_page_fixture();
        bad_page_name_terminator[56] = b'!';
        assert_eq!(
            FrameExif::parse(&bad_page_name_terminator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_page_number_type = exif_root_document_page_fixture();
        bad_page_number_type[36..38].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_page_number_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_page_number_count = exif_root_document_page_fixture();
        bad_page_number_count[38..42].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_page_number_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_host_computer_tag() {
        let exif_bytes = exif_root_host_computer_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.host_computer(), Some("PC"));

        let mut bad_type = exif_root_host_computer_fixture();
        bad_type[12..14].copy_from_slice(&FrameExifTiffType::Undefined.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_terminator = exif_root_host_computer_fixture();
        bad_terminator[20] = b'!';
        assert_eq!(
            FrameExif::parse(&bad_terminator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_multiple_strings = exif_root_host_computer_fixture();
        bad_multiple_strings[14..18].copy_from_slice(&4u32.to_le_bytes());
        bad_multiple_strings[18..22].copy_from_slice(&[b'P', 0, b'C', 0]);
        assert_eq!(
            FrameExif::parse(&bad_multiple_strings)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_predictor_tag() {
        let exif_bytes = exif_root_predictor_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.predictor().map(FrameExifPredictor::raw), Some(2));

        let mut bad_type = exif_root_predictor_fixture();
        bad_type[12..14].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_count = exif_root_predictor_fixture();
        bad_count[14..18].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_zero = exif_root_predictor_fixture();
        bad_zero[18..20].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_copyright_tag() {
        let exif_bytes = exif_root_copyright_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.copyright(), Some("CC"));

        let mut bad_type = exif_root_copyright_fixture();
        bad_type[12..14].copy_from_slice(&FrameExifTiffType::Undefined.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_terminator = exif_root_copyright_fixture();
        bad_terminator[20] = b'!';
        assert_eq!(
            FrameExif::parse(&bad_terminator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_multiple_strings = exif_root_copyright_fixture();
        bad_multiple_strings[14..18].copy_from_slice(&4u32.to_le_bytes());
        bad_multiple_strings[18..22].copy_from_slice(&[b'C', 0, b'C', 0]);
        assert_eq!(
            FrameExif::parse(&bad_multiple_strings)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_coding_tags() {
        let exif_bytes = exif_root_coding_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.compression().map(FrameExifCompression::raw), Some(1));
        assert_eq!(
            common
                .photometric_interpretation()
                .map(FrameExifPhotometricInterpretation::raw),
            Some(2)
        );

        let mut bad_compression_type = exif_root_coding_fixture();
        bad_compression_type[12..14].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_compression_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_compression_count = exif_root_coding_fixture();
        bad_compression_count[14..18].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_compression_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_compression_zero = exif_root_coding_fixture();
        bad_compression_zero[18..20].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_compression_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_photometric_type = exif_root_coding_fixture();
        bad_photometric_type[24..26].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_photometric_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_photometric_count = exif_root_coding_fixture();
        bad_photometric_count[26..30].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_photometric_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_bits_per_sample_tags() {
        let exif_bytes = exif_root_bits_per_sample_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();
        let bits_per_sample = common.bits_per_sample().unwrap();

        assert_eq!(common.samples_per_pixel(), Some(3));
        assert_eq!(bits_per_sample.count(), 3);
        assert_eq!(
            bits_per_sample.raw_entry().tag(),
            FrameExif::TAG_BITS_PER_SAMPLE
        );
        assert_eq!(bits_per_sample.values().unwrap(), [8, 8, 8]);

        let mut bad_type = exif_root_bits_per_sample_fixture();
        bad_type[12..14].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        bad_type[14..18].copy_from_slice(&1u32.to_le_bytes());
        bad_type[30..32].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_count = exif_root_bits_per_sample_fixture();
        bad_count[14..18].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_depth = exif_root_bits_per_sample_fixture();
        bad_depth[38..40].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_depth)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_samples_per_pixel_match = exif_root_bits_per_sample_fixture();
        bad_samples_per_pixel_match[30..32].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_samples_per_pixel_match)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_thresholding_tags() {
        let exif_bytes = exif_root_thresholding_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.thresholding(),
            Some(FrameExifThresholding::RandomizedProcess)
        );
        assert_eq!(
            common.thresholding().map(FrameExifThresholding::raw),
            Some(3)
        );

        let mut bad_type = exif_root_thresholding_fixture();
        bad_type[12..14].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_count = exif_root_thresholding_fixture();
        bad_count[14..18].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_value = exif_root_thresholding_fixture();
        bad_value[18..20].copy_from_slice(&4u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_fill_order_tags() {
        let exif_bytes = exif_root_fill_order_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.fill_order(),
            Some(FrameExifFillOrder::LeastSignificantBitFirst)
        );
        assert_eq!(common.fill_order().map(FrameExifFillOrder::raw), Some(2));

        let mut bad_type = exif_root_fill_order_fixture();
        bad_type[12..14].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_count = exif_root_fill_order_fixture();
        bad_count[14..18].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_value = exif_root_fill_order_fixture();
        bad_value[18..20].copy_from_slice(&3u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_root_strip_position_tags() {
        let exif_bytes = exif_root_strip_position_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.rows_per_strip(), Some(8));
        assert_eq!(
            common.x_position(),
            Some(FrameExifRational {
                numerator: 1,
                denominator: 2,
            })
        );
        assert_eq!(
            common.y_position(),
            Some(FrameExifRational {
                numerator: 3,
                denominator: 4,
            })
        );

        let mut bad_rows_type = exif_root_strip_position_fixture();
        bad_rows_type[12..14].copy_from_slice(&FrameExifTiffType::Rational.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_rows_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_rows_zero = exif_root_strip_position_fixture();
        bad_rows_zero[18..22].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_rows_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_x_count = exif_root_strip_position_fixture();
        bad_x_count[26..30].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_x_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_y_denominator = exif_root_strip_position_fixture();
        bad_y_denominator[62..66].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_y_denominator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_interoperability_related_image_tags() {
        let exif_bytes = exif_interoperability_related_image_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.interoperability_index(), Some("R98"));
        assert_eq!(common.interoperability_version(), Some(*b"0100"));
        assert_eq!(common.related_image_file_format(), Some("JPEG"));
        assert_eq!(common.related_image_width(), Some(64));
        assert_eq!(common.related_image_length(), Some(48));

        let mut bad_version_type = exif_interoperability_related_image_fixture();
        bad_version_type[60..62].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_version_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_version_count = exif_interoperability_related_image_fixture();
        bad_version_count[62..66].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_version_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_version_digit = exif_interoperability_related_image_fixture();
        bad_version_digit[66] = b'v';
        assert_eq!(
            FrameExif::parse(&bad_version_digit)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_format_type = exif_interoperability_related_image_fixture();
        bad_format_type[72..74].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_format_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_format_nul = exif_interoperability_related_image_fixture();
        bad_format_nul[114] = b'X';
        assert_eq!(
            FrameExif::parse(&bad_format_nul)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_width_count = exif_interoperability_related_image_fixture();
        bad_width_count[86..90].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_width_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_width_type = exif_interoperability_related_image_fixture();
        bad_width_type[84..86].copy_from_slice(&FrameExifTiffType::Rational.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_width_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_width_zero = exif_interoperability_related_image_fixture();
        bad_width_zero[90..92].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_width_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_length_zero = exif_interoperability_related_image_fixture();
        bad_length_zero[102..106].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_length_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_exposure_tags() {
        let exif_bytes = exif_exposure_tags_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.exposure_time(),
            Some(FrameExifRational {
                numerator: 1,
                denominator: 125,
            })
        );
        assert_eq!(
            common.f_number(),
            Some(FrameExifRational {
                numerator: 28,
                denominator: 10,
            })
        );
        assert_eq!(
            common.exposure_bias_value(),
            Some(FrameExifSignedRational {
                numerator: -1,
                denominator: 3,
            })
        );
        assert_eq!(
            common.focal_length(),
            Some(FrameExifRational {
                numerator: 50,
                denominator: 1,
            })
        );
        assert_eq!(common.pixel_x_dimension(), Some(1920));
        assert_eq!(common.pixel_y_dimension(), Some(1080));
        assert_eq!(common.date_time_digitized(), Some("2026:05:04 12:35:00"));

        let mut bad_exposure_time_type = exif_exposure_tags_fixture();
        bad_exposure_time_type[30..32]
            .copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_exposure_time_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_digitized_count = exif_exposure_tags_fixture();
        bad_digitized_count[104..108].copy_from_slice(&19u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_digitized_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_digitized_shape = exif_exposure_tags_fixture();
        bad_digitized_shape[151] = b'X';
        assert_eq!(
            FrameExif::parse(&bad_digitized_shape)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_digitized_minute = exif_exposure_tags_fixture();
        bad_digitized_minute[162..164].copy_from_slice(b"60");
        assert_eq!(
            FrameExif::parse(&bad_digitized_minute)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_digitized_calendar_day = exif_exposure_tags_fixture();
        bad_digitized_calendar_day[153..155].copy_from_slice(b"02");
        bad_digitized_calendar_day[156..158].copy_from_slice(b"30");
        assert_eq!(
            FrameExif::parse(&bad_digitized_calendar_day)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut leap_digitized = exif_exposure_tags_fixture();
        leap_digitized[148..158].copy_from_slice(b"2024:02:29");
        assert_eq!(
            FrameExif::parse(&leap_digitized)
                .unwrap()
                .common_tags()
                .unwrap()
                .date_time_digitized(),
            Some("2024:02:29 12:35:00")
        );

        let mut bad_pixel_count = exif_exposure_tags_fixture();
        bad_pixel_count[92..96].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_pixel_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_pixel_x_zero = exif_exposure_tags_fixture();
        bad_pixel_x_zero[84..88].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_pixel_x_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_pixel_y_zero = exif_exposure_tags_fixture();
        bad_pixel_y_zero[96..100].copy_from_slice(&[0; 4]);
        assert_eq!(
            FrameExif::parse(&bad_pixel_y_zero)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_apex_exposure_tags() {
        let exif_bytes = exif_apex_exposure_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.shutter_speed_value(),
            Some(FrameExifSignedRational {
                numerator: -7,
                denominator: 1,
            })
        );
        assert_eq!(
            common.aperture_value(),
            Some(FrameExifRational {
                numerator: 56,
                denominator: 10,
            })
        );
        assert_eq!(
            common.brightness_value(),
            Some(FrameExifSignedRational {
                numerator: -3,
                denominator: 2,
            })
        );

        let mut bad_shutter_count = exif_apex_exposure_fixture();
        bad_shutter_count[32..36].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_shutter_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_aperture_type = exif_apex_exposure_fixture();
        bad_aperture_type[42..44].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_aperture_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_brightness_denominator = exif_apex_exposure_fixture();
        bad_brightness_denominator[88..92].copy_from_slice(&0i32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_brightness_denominator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_sensitivity_tags() {
        let exif_bytes = exif_sensitivity_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.photographic_sensitivity(), Some(200));
        assert_eq!(
            common.sensitivity_type(),
            Some(FrameExifSensitivityType::IsoSpeed)
        );
        assert_eq!(common.sensitivity_type().unwrap().raw(), 3);
        assert_eq!(common.standard_output_sensitivity(), Some(160));
        assert_eq!(common.recommended_exposure_index(), Some(180));
        assert_eq!(common.iso_speed(), Some(200));
        assert_eq!(common.iso_speed_latitude_yyy(), Some(125));
        assert_eq!(common.iso_speed_latitude_zzz(), Some(400));

        let mut bad_photographic_count = exif_sensitivity_fixture();
        bad_photographic_count[32..36].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_photographic_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_sensitivity_type_value = exif_sensitivity_fixture();
        bad_sensitivity_type_value[48..50].copy_from_slice(&8u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_sensitivity_type_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_standard_output_type = exif_sensitivity_fixture();
        bad_standard_output_type[54..56]
            .copy_from_slice(&FrameExifTiffType::Short.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_standard_output_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_camera_characterization_tags() {
        let exif_bytes = exif_camera_characterization_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.spectral_sensitivity(), Some("RGB 550"));
        assert_eq!(common.oecf(), Some(&b"oecf0001"[..]));
        assert_eq!(
            common.flash_energy(),
            Some(FrameExifRational {
                numerator: 25,
                denominator: 10,
            })
        );
        assert_eq!(common.spatial_frequency_response(), Some(&b"sfr0001"[..]));
        assert_eq!(
            common.focal_plane_x_resolution(),
            Some(FrameExifRational {
                numerator: 3000,
                denominator: 1,
            })
        );
        assert_eq!(
            common.focal_plane_y_resolution(),
            Some(FrameExifRational {
                numerator: 2000,
                denominator: 1,
            })
        );
        assert_eq!(
            common.focal_plane_resolution_unit(),
            Some(FrameExifResolutionUnit::Centimeter)
        );
        assert_eq!(common.focal_plane_resolution_unit().unwrap().raw(), 3);
        assert_eq!(common.cfa_pattern(), Some(&[2, 0, 2, 0, 1, 0, 2, 1][..]));

        let mut bad_spectral_type = exif_camera_characterization_fixture();
        bad_spectral_type[30..32]
            .copy_from_slice(&FrameExifTiffType::Undefined.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_spectral_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_oecf_type = exif_camera_characterization_fixture();
        bad_oecf_type[42..44].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_oecf_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_flash_energy_count = exif_camera_characterization_fixture();
        bad_flash_energy_count[56..60].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_flash_energy_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_spatial_type = exif_camera_characterization_fixture();
        bad_spatial_type[66..68].copy_from_slice(&FrameExifTiffType::Ascii.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_spatial_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_focal_plane_unit_value = exif_camera_characterization_fixture();
        bad_focal_plane_unit_value[108..110].copy_from_slice(&4u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_focal_plane_unit_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_cfa_type = exif_camera_characterization_fixture();
        bad_cfa_type[114..116].copy_from_slice(&FrameExifTiffType::Short.raw().to_le_bytes());
        bad_cfa_type[116..120].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_cfa_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_offset_time_tags() {
        let exif_bytes = exif_offset_time_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.offset_time(), Some("+09:00"));
        assert_eq!(common.offset_time_original(), Some("-07:30"));
        assert_eq!(common.offset_time_digitized(), Some("+00:00"));

        let mut bad_offset_count = exif_offset_time_fixture();
        bad_offset_count[32..36].copy_from_slice(&6u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_offset_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_offset_shape = exif_offset_time_fixture();
        bad_offset_shape[68] = b'Z';
        assert_eq!(
            FrameExif::parse(&bad_offset_shape)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_offset_hour = exif_offset_time_fixture();
        bad_offset_hour[69..71].copy_from_slice(b"24");
        assert_eq!(
            FrameExif::parse(&bad_offset_hour)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_original_minute = exif_offset_time_fixture();
        bad_original_minute[79..81].copy_from_slice(b"60");
        assert_eq!(
            FrameExif::parse(&bad_original_minute)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_original_type = exif_offset_time_fixture();
        bad_original_type[42..44].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_original_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_digitized_ascii = exif_offset_time_fixture();
        bad_digitized_ascii[88] = b'!';
        assert_eq!(
            FrameExif::parse(&bad_digitized_ascii)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_capture_setting_tags() {
        let exif_bytes = exif_capture_settings_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.exposure_program(),
            Some(FrameExifExposureProgram::AperturePriority)
        );
        assert_eq!(common.exposure_program().unwrap().raw(), 3);
        assert_eq!(common.metering_mode(), Some(FrameExifMeteringMode::Pattern));
        assert_eq!(common.metering_mode().unwrap().raw(), 5);
        assert_eq!(common.light_source(), Some(FrameExifLightSource::D65));
        assert_eq!(common.light_source().unwrap().raw(), 21);
        assert_eq!(common.flash(), Some(FrameExifFlash::from_raw(0x0041)));
        assert!(common.flash().unwrap().fired());
        assert!(common.flash().unwrap().red_eye_reduction_supported());
        assert_eq!(common.flash().unwrap().return_status_bits(), 0);
        assert_eq!(common.flash().unwrap().mode_bits(), 0);
        assert!(!common.flash().unwrap().has_no_flash_function());
        assert_eq!(common.white_balance(), Some(FrameExifWhiteBalance::Manual));
        assert_eq!(common.white_balance().unwrap().raw(), 1);
        assert_eq!(
            common.digital_zoom_ratio(),
            Some(FrameExifRational {
                numerator: 3,
                denominator: 2,
            })
        );
        assert_eq!(common.focal_length_in_35mm_film(), Some(75));

        let mut bad_exposure_program = exif_capture_settings_fixture();
        bad_exposure_program[36..38].copy_from_slice(&9u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_exposure_program)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_light_source = exif_capture_settings_fixture();
        bad_light_source[60..62].copy_from_slice(&7u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_light_source)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_digital_zoom_type = exif_capture_settings_fixture();
        bad_digital_zoom_type[90..92].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_digital_zoom_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_focal_length_count = exif_capture_settings_fixture();
        bad_focal_length_count[104..108].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_focal_length_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_rendering_scene_tags() {
        let exif_bytes = exif_rendering_scene_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.color_space(), Some(FrameExifColorSpace::Srgb));
        assert_eq!(common.color_space().unwrap().raw(), 1);
        assert_eq!(
            common.sensing_method(),
            Some(FrameExifSensingMethod::OneChipColorArea)
        );
        assert_eq!(common.sensing_method().unwrap().raw(), 2);
        assert_eq!(
            common.file_source(),
            Some(FrameExifFileSource::DigitalStillCamera)
        );
        assert_eq!(common.file_source().unwrap().raw(), 3);
        assert_eq!(
            common.scene_type(),
            Some(FrameExifSceneType::DirectlyPhotographed)
        );
        assert_eq!(common.scene_type().unwrap().raw(), 1);
        assert_eq!(
            common.custom_rendered(),
            Some(FrameExifCustomRendered::Custom)
        );
        assert_eq!(common.custom_rendered().unwrap().raw(), 1);
        assert_eq!(
            common.exposure_mode(),
            Some(FrameExifExposureMode::AutoBracket)
        );
        assert_eq!(common.exposure_mode().unwrap().raw(), 2);
        assert_eq!(
            common.scene_capture_type(),
            Some(FrameExifSceneCaptureType::NightScene)
        );
        assert_eq!(common.scene_capture_type().unwrap().raw(), 3);
        assert_eq!(
            common.gain_control(),
            Some(FrameExifGainControl::HighGainDown)
        );
        assert_eq!(common.gain_control().unwrap().raw(), 4);
        assert_eq!(common.contrast(), Some(FrameExifContrast::Hard));
        assert_eq!(common.contrast().unwrap().raw(), 2);
        assert_eq!(common.saturation(), Some(FrameExifSaturation::Low));
        assert_eq!(common.saturation().unwrap().raw(), 1);
        assert_eq!(common.sharpness(), Some(FrameExifSharpness::Hard));
        assert_eq!(common.sharpness().unwrap().raw(), 2);
        assert_eq!(
            common.subject_distance_range(),
            Some(FrameExifSubjectDistanceRange::DistantView)
        );
        assert_eq!(common.subject_distance_range().unwrap().raw(), 3);

        let mut bad_color_space = exif_rendering_scene_fixture();
        bad_color_space[36..38].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_color_space)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_file_source_type = exif_rendering_scene_fixture();
        bad_file_source_type[54..56].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_file_source_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_scene_capture_count = exif_rendering_scene_fixture();
        bad_scene_capture_count[104..108].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_scene_capture_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subject_distance_raw = exif_rendering_scene_fixture();
        bad_subject_distance_raw[168..170].copy_from_slice(&4u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subject_distance_raw)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_optics_subject_tags() {
        let exif_bytes = exif_optics_subject_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.compressed_bits_per_pixel(),
            Some(FrameExifRational {
                numerator: 3,
                denominator: 2,
            })
        );
        assert_eq!(
            common.max_aperture_value(),
            Some(FrameExifRational {
                numerator: 14,
                denominator: 5,
            })
        );
        assert_eq!(
            common.subject_distance(),
            Some(FrameExifRational {
                numerator: 125,
                denominator: 10,
            })
        );
        assert_eq!(
            common.subject_area(),
            Some(FrameExifSubjectArea::rectangle(100, 150, 80, 60))
        );
        assert_eq!(common.subject_location(), Some([320, 240]));
        assert_eq!(
            common.exposure_index(),
            Some(FrameExifRational {
                numerator: 200,
                denominator: 1,
            })
        );

        let mut point_area = exif_optics_subject_fixture();
        point_area[68..72].copy_from_slice(&2u32.to_le_bytes());
        point_area[72..76].copy_from_slice([0x40, 0x01, 0xF0, 0x00].as_slice());
        assert_eq!(
            FrameExif::parse(&point_area)
                .unwrap()
                .common_tags()
                .unwrap()
                .subject_area(),
            Some(FrameExifSubjectArea::point(320, 240))
        );

        let mut circle_area = exif_optics_subject_fixture();
        circle_area[68..72].copy_from_slice(&3u32.to_le_bytes());
        circle_area[72..76].copy_from_slice(&144u32.to_le_bytes());
        circle_area.extend_from_slice(&320u16.to_le_bytes());
        circle_area.extend_from_slice(&240u16.to_le_bytes());
        circle_area.extend_from_slice(&50u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&circle_area)
                .unwrap()
                .common_tags()
                .unwrap()
                .subject_area(),
            Some(FrameExifSubjectArea::circle(320, 240, 50))
        );

        let mut bad_circle_diameter = exif_optics_subject_fixture();
        bad_circle_diameter[68..72].copy_from_slice(&3u32.to_le_bytes());
        bad_circle_diameter[72..76].copy_from_slice(&144u32.to_le_bytes());
        bad_circle_diameter.extend_from_slice(&320u16.to_le_bytes());
        bad_circle_diameter.extend_from_slice(&240u16.to_le_bytes());
        bad_circle_diameter.extend_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_circle_diameter)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_rectangle_width = exif_optics_subject_fixture();
        bad_rectangle_width[132..134].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_rectangle_width)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_rectangle_height = exif_optics_subject_fixture();
        bad_rectangle_height[134..136].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_rectangle_height)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subject_area_count = exif_optics_subject_fixture();
        bad_subject_area_count[68..72].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subject_area_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subject_area_type = exif_optics_subject_fixture();
        bad_subject_area_type[66..68].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subject_area_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_subject_location_count = exif_optics_subject_fixture();
        bad_subject_location_count[80..84].copy_from_slice(&3u32.to_le_bytes());
        bad_subject_location_count[84..88].copy_from_slice(&144u32.to_le_bytes());
        bad_subject_location_count.extend_from_slice(&320u16.to_le_bytes());
        bad_subject_location_count.extend_from_slice(&240u16.to_le_bytes());
        bad_subject_location_count.extend_from_slice(&50u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_subject_location_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_exposure_index_denominator = exif_optics_subject_fixture();
        bad_exposure_index_denominator[140..144].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_exposure_index_denominator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_version_timing_comment_tags() {
        let exif_bytes = exif_version_timing_comment_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.components_configuration(), Some([1, 2, 3, 0]));
        assert_eq!(common.maker_note(), Some(&b"maker!"[..]));
        assert_eq!(common.user_comment(), Some(&b"ASCII\0\0\0hello\0\0\0"[..]));
        assert_eq!(common.sub_sec_time(), Some("123"));
        assert_eq!(common.sub_sec_time_original(), Some("4567"));
        assert_eq!(common.sub_sec_time_digitized(), Some("89"));
        assert_eq!(common.flashpix_version(), Some(*b"0100"));
        assert_eq!(common.related_sound_file(), Some("SOUND001.WAV"));
        assert_eq!(common.pixel_x_dimension(), Some(640));

        let mut bad_components_count = exif_version_timing_comment_fixture();
        bad_components_count[32..36].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_components_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_components_value = exif_version_timing_comment_fixture();
        bad_components_value[36] = 7;
        assert_eq!(
            FrameExif::parse(&bad_components_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_maker_note_type = exif_version_timing_comment_fixture();
        bad_maker_note_type[42..44].copy_from_slice(&FrameExifTiffType::Byte.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_maker_note_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_sub_sec_time = exif_version_timing_comment_fixture();
        bad_sub_sec_time[75] = b'!';
        assert_eq!(
            FrameExif::parse(&bad_sub_sec_time)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_sub_sec_digit = exif_version_timing_comment_fixture();
        bad_sub_sec_digit[72] = b'a';
        assert_eq!(
            FrameExif::parse(&bad_sub_sec_digit)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_original_sub_sec_digit = exif_version_timing_comment_fixture();
        bad_original_sub_sec_digit[164] = b'x';
        assert_eq!(
            FrameExif::parse(&bad_original_sub_sec_digit)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_digitized_sub_sec_digit = exif_version_timing_comment_fixture();
        bad_digitized_sub_sec_digit[97] = b'z';
        assert_eq!(
            FrameExif::parse(&bad_digitized_sub_sec_digit)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_flashpix_count = exif_version_timing_comment_fixture();
        bad_flashpix_count[104..108].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_flashpix_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_flashpix_digit = exif_version_timing_comment_fixture();
        bad_flashpix_digit[108] = b'v';
        assert_eq!(
            FrameExif::parse(&bad_flashpix_digit)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_related_sound_count = exif_version_timing_comment_fixture();
        bad_related_sound_count[116..120].copy_from_slice(&12u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_related_sound_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_camera_lens_tags() {
        let exif_bytes = exif_camera_lens_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.image_unique_id(),
            Some("0123456789abcdef0123456789abcdef")
        );
        assert_eq!(common.camera_owner_name(), Some("A Camera"));
        assert_eq!(common.body_serial_number(), Some("BODY1234"));
        assert_eq!(
            common.lens_specification(),
            Some([
                FrameExifRational {
                    numerator: 24,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 70,
                    denominator: 1,
                },
                FrameExifRational {
                    numerator: 28,
                    denominator: 10,
                },
                FrameExifRational {
                    numerator: 40,
                    denominator: 10,
                },
            ])
        );
        assert_eq!(common.lens_make(), Some("LensCo"));
        assert_eq!(common.lens_model(), Some("Prime50"));
        assert_eq!(common.lens_serial_number(), Some("LENS5678"));

        let mut bad_unique_id_count = exif_camera_lens_fixture();
        bad_unique_id_count[32..36].copy_from_slice(&32u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_unique_id_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_lens_spec_count = exif_camera_lens_fixture();
        bad_lens_spec_count[68..72].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_lens_spec_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_lens_spec_type = exif_camera_lens_fixture();
        bad_lens_spec_type[66..68].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_lens_spec_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_lens_model_ascii = exif_camera_lens_fixture();
        bad_lens_model_ascii[206] = 0xff;
        assert_eq!(
            FrameExif::parse(&bad_lens_model_ascii)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_gamma_composite_tags() {
        let exif_bytes = exif_gamma_composite_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.gamma(),
            Some(FrameExifRational {
                numerator: 22,
                denominator: 10,
            })
        );
        assert_eq!(
            common.composite_image(),
            Some(FrameExifCompositeImage::GeneralCompositeImage)
        );
        assert_eq!(
            common.source_image_number_of_composite_image(),
            Some([5, 3])
        );
        assert_eq!(
            common.source_exposure_times_of_composite_image(),
            Some(&b"exp-times-01"[..])
        );

        let mut bad_gamma_count = exif_gamma_composite_fixture();
        bad_gamma_count[32..36].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_gamma_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_composite_value = exif_gamma_composite_fixture();
        bad_composite_value[48..50].copy_from_slice(&9u16.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_composite_value)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_source_count = exif_gamma_composite_fixture();
        bad_source_count[56..60].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_source_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_exposure_type = exif_gamma_composite_fixture();
        bad_exposure_type[66..68].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        bad_exposure_type[68..72].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_exposure_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_environment_tags() {
        let exif_bytes = exif_environment_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(
            common.temperature(),
            Some(FrameExifSignedRational {
                numerator: -5,
                denominator: 1,
            })
        );
        assert_eq!(
            common.humidity(),
            Some(FrameExifRational {
                numerator: 55,
                denominator: 1,
            })
        );
        assert_eq!(
            common.pressure(),
            Some(FrameExifRational {
                numerator: 1013,
                denominator: 10,
            })
        );
        assert_eq!(
            common.water_depth(),
            Some(FrameExifSignedRational {
                numerator: -3,
                denominator: 1,
            })
        );
        assert_eq!(
            common.acceleration(),
            Some(FrameExifRational {
                numerator: 98,
                denominator: 10,
            })
        );
        assert_eq!(
            common.camera_elevation_angle(),
            Some(FrameExifSignedRational {
                numerator: -12,
                denominator: 1,
            })
        );

        let mut bad_temperature_count = exif_environment_fixture();
        bad_temperature_count[32..36].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_temperature_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_humidity_type = exif_environment_fixture();
        bad_humidity_type[42..44].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_humidity_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_water_depth_denominator = exif_environment_fixture();
        bad_water_depth_denominator[132..136].copy_from_slice(&0i32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_water_depth_denominator)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_interprets_exif_descriptive_tags() {
        let exif_bytes = exif_descriptive_tags_fixture();
        let parsed = FrameExif::parse(&exif_bytes).unwrap();
        let common = parsed.common_tags().unwrap();

        assert_eq!(common.image_description(), Some("Frame sample"));
        assert_eq!(common.software(), Some("ffmpegrust"));
        assert_eq!(common.date_time(), Some("2026:05:05 01:02:03"));
        assert_eq!(common.artist(), Some("OpenAI"));
        assert_eq!(common.copyright(), Some("2026 Example"));

        let mut bad_software_type = exif_descriptive_tags_fixture();
        bad_software_type[24..26].copy_from_slice(&FrameExifTiffType::Long.raw().to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_software_type)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_date_time_count = exif_descriptive_tags_fixture();
        bad_date_time_count[38..42].copy_from_slice(&19u32.to_le_bytes());
        assert_eq!(
            FrameExif::parse(&bad_date_time_count)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_date_time_shape = exif_descriptive_tags_fixture();
        bad_date_time_shape[102] = b'-';
        assert_eq!(
            FrameExif::parse(&bad_date_time_shape)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );

        let mut bad_date_time_hour = exif_descriptive_tags_fixture();
        bad_date_time_hour[109..111].copy_from_slice(b"24");
        assert_eq!(
            FrameExif::parse(&bad_date_time_hour)
                .unwrap()
                .common_tags()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn frame_side_data_rejects_exif_descriptive_ascii_shapes() {
        fn assert_invalid_common_tags(data: Vec<u8>) {
            assert_eq!(
                FrameExif::parse(&data)
                    .unwrap()
                    .common_tags()
                    .unwrap_err()
                    .kind(),
                AvErrorKind::InvalidData
            );
        }

        let mut bad_image_description_terminator = exif_descriptive_tags_fixture();
        bad_image_description_terminator[86] = b'!';
        assert_invalid_common_tags(bad_image_description_terminator);

        let mut bad_software_non_ascii = exif_descriptive_tags_fixture();
        bad_software_non_ascii[87] = 0xFF;
        assert_invalid_common_tags(bad_software_non_ascii);

        let mut bad_artist_multiple_strings = exif_descriptive_tags_fixture();
        bad_artist_multiple_strings[122] = 0;
        assert_invalid_common_tags(bad_artist_multiple_strings);
    }

    #[test]
    fn frame_side_data_rejects_malformed_exif_payload() {
        fn assert_invalid(data: Vec<u8>) {
            assert_eq!(
                FrameExif::parse(&data).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            assert_eq!(
                FrameSideData::new_exif(data.clone()).unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
            let side_data = FrameSideData::new_with_kind(FrameSideDataKind::Exif, data).unwrap();
            assert_eq!(
                side_data.exif().unwrap_err().kind(),
                AvErrorKind::InvalidData
            );
        }

        assert_invalid(Vec::new());
        assert_invalid(vec![0; FrameExif::TIFF_HEADER_LEN - 1]);
        assert_invalid(vec![0x45, 0x78, 0x69, 0x66, 8, 0, 0, 0]);

        let mut bad_first_offset = minimal_little_exif_fixture();
        bad_first_offset[4..8].copy_from_slice(&6u32.to_le_bytes());
        assert_invalid(bad_first_offset);

        let mut bad_missing_count = minimal_little_exif_fixture();
        bad_missing_count[4..8].copy_from_slice(&31u32.to_le_bytes());
        assert_invalid(bad_missing_count);

        let mut too_many_entries = Vec::new();
        too_many_entries.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        too_many_entries.extend_from_slice(&8u32.to_le_bytes());
        too_many_entries.extend_from_slice(&(FrameExif::MAX_IFD_ENTRIES as u16 + 1).to_le_bytes());
        assert_invalid(too_many_entries);

        let mut truncated_table = Vec::new();
        truncated_table.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        truncated_table.extend_from_slice(&8u32.to_le_bytes());
        truncated_table.extend_from_slice(&1u16.to_le_bytes());
        truncated_table.extend_from_slice(&[0; 4]);
        assert_invalid(truncated_table);

        let mut bad_type = minimal_little_exif_fixture();
        bad_type[12..14].copy_from_slice(&0u16.to_le_bytes());
        assert_invalid(bad_type);

        let mut bad_range = minimal_little_exif_fixture();
        bad_range[18..22].copy_from_slice(&250u32.to_le_bytes());
        assert_invalid(bad_range);

        let mut bad_pointer_type = exif_with_linked_ifds_fixture();
        bad_pointer_type[24..26].copy_from_slice(&FrameExifTiffType::Short.raw().to_le_bytes());
        assert_invalid(bad_pointer_type);

        let mut bad_pointer_count = exif_with_linked_ifds_fixture();
        bad_pointer_count[26..30].copy_from_slice(&2u32.to_le_bytes());
        assert_invalid(bad_pointer_count);

        let mut bad_pointer_offset = exif_with_linked_ifds_fixture();
        bad_pointer_offset[30..34].copy_from_slice(&7u32.to_le_bytes());
        assert_invalid(bad_pointer_offset);

        let mut pointer_loop = exif_with_linked_ifds_fixture();
        pointer_loop[30..34].copy_from_slice(&8u32.to_le_bytes());
        assert_invalid(pointer_loop);

        let mut looped_ifd = Vec::new();
        looped_ifd.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        looped_ifd.extend_from_slice(&8u32.to_le_bytes());
        looped_ifd.extend_from_slice(&0u16.to_le_bytes());
        looped_ifd.extend_from_slice(&8u32.to_le_bytes());
        assert_invalid(looped_ifd);

        let mut bad_next_offset = Vec::new();
        bad_next_offset.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        bad_next_offset.extend_from_slice(&8u32.to_le_bytes());
        bad_next_offset.extend_from_slice(&0u16.to_le_bytes());
        bad_next_offset.extend_from_slice(&7u32.to_le_bytes());
        assert_invalid(bad_next_offset);

        assert_eq!(
            FrameExifTiffType::from_raw(14).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let non_exif =
            FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0]).unwrap();
        assert_eq!(non_exif.exif().unwrap(), None);
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
