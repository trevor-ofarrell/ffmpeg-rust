use crate::{AvError, AvResult, BufferRef, ChannelLayout, Dictionary, PixelFormat, SampleFormat};

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

    pub fn take_side_data(&mut self) -> Vec<FrameSideData> {
        std::mem::take(&mut self.side_data)
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::empty()
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
    ContentLightLevel,
    IccProfile,
    Spherical,
    RegionsOfInterest,
    VideoEncParams,
    SeiUnregistered,
    FilmGrainParams,
    DetectionBboxes,
    DynamicHdrPlus,
    DynamicHdrVivid,
    AmbientViewingEnvironment,
    DolbyVisionRpuBuffer,
    DolbyVisionMetadata,
    Lcevc,
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
        Self::ContentLightLevel,
        Self::IccProfile,
        Self::Spherical,
        Self::RegionsOfInterest,
        Self::VideoEncParams,
        Self::SeiUnregistered,
        Self::FilmGrainParams,
        Self::DetectionBboxes,
        Self::DynamicHdrPlus,
        Self::DynamicHdrVivid,
        Self::AmbientViewingEnvironment,
        Self::DolbyVisionRpuBuffer,
        Self::DolbyVisionMetadata,
        Self::Lcevc,
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
            Self::ContentLightLevel => "content_light_level",
            Self::IccProfile => "icc_profile",
            Self::Spherical => "spherical",
            Self::RegionsOfInterest => "regions_of_interest",
            Self::VideoEncParams => "video_enc_params",
            Self::SeiUnregistered => "sei_unregistered",
            Self::FilmGrainParams => "film_grain_params",
            Self::DetectionBboxes => "detection_bboxes",
            Self::DynamicHdrPlus => "dynamic_hdr_plus",
            Self::DynamicHdrVivid => "dynamic_hdr_vivid",
            Self::AmbientViewingEnvironment => "ambient_viewing_environment",
            Self::DolbyVisionRpuBuffer => "dolby_vision_rpu_buffer",
            Self::DolbyVisionMetadata => "dolby_vision_metadata",
            Self::Lcevc => "lcevc",
            Self::Unknown(name) => name.as_str(),
        }
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    fn known_from_name(name: &str) -> Option<Self> {
        let normalized = normalize_frame_side_data_name(name);
        match normalized.as_str() {
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
            "contentlightlevel" | "content_light_level" => Some(Self::ContentLightLevel),
            "iccprofile" | "icc_profile" => Some(Self::IccProfile),
            "spherical" => Some(Self::Spherical),
            "roi" | "region_of_interest" | "regions_of_interest" => Some(Self::RegionsOfInterest),
            "videoencparams" | "video_enc_params" => Some(Self::VideoEncParams),
            "seiunregistered" | "sei_unregistered" => Some(Self::SeiUnregistered),
            "filmgrainparams" | "film_grain_params" => Some(Self::FilmGrainParams),
            "detectionbboxes" | "detection_bboxes" => Some(Self::DetectionBboxes),
            "dynamichdrplus" | "dynamic_hdr_plus" | "hdr_plus" => Some(Self::DynamicHdrPlus),
            "dynamichdrvivid" | "dynamic_hdr_vivid" | "hdr_vivid" => Some(Self::DynamicHdrVivid),
            "ambientviewingenvironment" | "ambient_viewing_environment" => {
                Some(Self::AmbientViewingEnvironment)
            }
            "dolbyvisionrpubuffer" | "dolby_vision_rpu_buffer" | "dovi_rpu_buffer" => {
                Some(Self::DolbyVisionRpuBuffer)
            }
            "dolbyvisionmetadata" | "dolby_vision_metadata" | "dovi_metadata" => {
                Some(Self::DolbyVisionMetadata)
            }
            "lcevc" => Some(Self::Lcevc),
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

fn checked_mul(value: usize, factor: usize, context: &'static str) -> AvResult<usize> {
    value
        .checked_mul(factor)
        .ok_or_else(|| AvError::invalid_argument(format!("{context} overflow")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

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
        assert_eq!(FrameSideDataKind::DisplayMatrix.name(), "displaymatrix");
        assert!(FrameSideDataKind::DisplayMatrix.is_known());
        assert!(FrameSideDataKind::KNOWN.contains(&FrameSideDataKind::DisplayMatrix));

        let unknown = FrameSideDataKind::from_name("vendor.private.side-data").unwrap();
        assert_eq!(
            unknown,
            FrameSideDataKind::Unknown(String::from("vendor.private.side-data"))
        );
        assert_eq!(unknown.name(), "vendor.private.side-data");
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
