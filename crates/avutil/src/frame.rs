use crate::{AvError, AvResult, BufferRef, ChannelLayout, Dictionary, PixelFormat, SampleFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameData {
    Video(VideoFrame),
    Audio(AudioFrame),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pts: Option<i64>,
    data: FrameData,
    hw_frames_context: Option<BufferRef>,
    side_data: Vec<FrameSideData>,
}

impl Frame {
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

    pub fn pts(&self) -> Option<i64> {
        self.pts
    }

    pub fn set_pts(&mut self, pts: Option<i64>) {
        self.pts = pts;
    }

    pub fn data(&self) -> &FrameData {
        &self.data
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

    pub fn remove_side_data(&mut self, kind: &str) -> Option<FrameSideData> {
        self.side_data
            .iter()
            .position(|side_data| side_data.kind() == kind)
            .map(|index| self.side_data.remove(index))
    }

    pub fn take_side_data(&mut self) -> Vec<FrameSideData> {
        std::mem::take(&mut self.side_data)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSideData {
    kind: String,
    buffer: BufferRef,
    metadata: Dictionary,
}

impl FrameSideData {
    pub fn new(kind: impl Into<String>, data: Vec<u8>) -> AvResult<Self> {
        Self::new_with_buffer_ref(kind, BufferRef::from_vec(data))
    }

    pub fn new_with_buffer_ref(kind: impl Into<String>, buffer: BufferRef) -> AvResult<Self> {
        Ok(Self {
            kind: validate_frame_side_data_kind(kind.into())?,
            buffer,
            metadata: Dictionary::new(),
        })
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn data(&self) -> &[u8] {
        self.buffer.as_slice()
    }

    pub fn buffer(&self) -> &BufferRef {
        &self.buffer
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
        if width == 0 || height == 0 {
            return Err(AvError::invalid_argument(
                "video frame dimensions must be non-zero",
            ));
        }

        let expected_plane_sizes = pixel_format.plane_sizes(width, height)?;
        let planes = snapshot_plane_buffers(
            &plane_buffers,
            &expected_plane_sizes,
            pixel_format.name(),
            "video",
        )?;
        let line_sizes = video_line_sizes(pixel_format, width, height)?;

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
        Self::new_inner(
            sample_rate,
            channels,
            ChannelLayout::default_for_count(channels),
            sample_format,
            samples_per_channel,
            plane_buffers,
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
        Self::new_inner(
            sample_rate,
            channels,
            Some(channel_layout),
            sample_format,
            samples_per_channel,
            plane_buffers,
        )
    }

    fn new_inner(
        sample_rate: u32,
        channels: u16,
        channel_layout: Option<ChannelLayout>,
        sample_format: SampleFormat,
        samples_per_channel: usize,
        plane_buffers: Vec<BufferRef>,
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

        let expected_plane_sizes = sample_format.plane_sizes(samples_per_channel, channels)?;
        let planes = snapshot_plane_buffers(
            &plane_buffers,
            &expected_plane_sizes,
            sample_format.name(),
            "audio",
        )?;

        Ok(Self {
            sample_rate,
            channels,
            channel_layout,
            sample_format,
            samples_per_channel,
            line_sizes: expected_plane_sizes,
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
}

fn snapshot_plane_buffers(
    plane_buffers: &[BufferRef],
    expected_plane_sizes: &[usize],
    format_name: &str,
    media_kind: &str,
) -> AvResult<Vec<Vec<u8>>> {
    if plane_buffers.len() != expected_plane_sizes.len() {
        return Err(AvError::invalid_argument(format!(
            "{format_name} {media_kind} frame expects {} planes, got {}",
            expected_plane_sizes.len(),
            plane_buffers.len()
        )));
    }

    let mut planes = Vec::with_capacity(plane_buffers.len());
    for (index, (plane, expected_size)) in
        plane_buffers.iter().zip(expected_plane_sizes).enumerate()
    {
        if plane.len() != *expected_size {
            return Err(AvError::invalid_data(format!(
                "{format_name} {media_kind} frame plane {index} has {} bytes, expected {expected_size}",
                plane.len()
            )));
        }
        planes.push(plane.as_slice().to_vec());
    }
    Ok(planes)
}

fn video_line_sizes(
    pixel_format: PixelFormat,
    width: usize,
    height: usize,
) -> AvResult<Vec<usize>> {
    pixel_format.plane_sizes(width, height)?;

    match pixel_format {
        PixelFormat::Gray8 => Ok(vec![width]),
        PixelFormat::Rgb24 => Ok(vec![checked_mul(width, 3, "rgb24 video frame line size")?]),
        PixelFormat::Rgba => Ok(vec![checked_mul(width, 4, "rgba video frame line size")?]),
        PixelFormat::Yuv420p => Ok(vec![width, width / 2, width / 2]),
    }
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
            .add_side_data_buffer("replaygain", BufferRef::copy_from_slice(&[9, 8, 7]))
            .unwrap();
        side_data.metadata_mut().set("gain", "-3.0 dB").unwrap();

        assert_eq!(frame.side_data().len(), 2);
        assert_eq!(frame.side_data()[0].data(), &[1, 2]);
        assert_eq!(frame.side_data()[1].metadata().get("gain"), Some("-3.0 dB"));

        let removed = frame.remove_side_data("alpha_info").unwrap();
        assert_eq!(removed.kind(), "alpha_info");
        assert_eq!(removed.data(), &[1, 2]);
        assert!(frame.remove_side_data("missing").is_none());
        assert_eq!(frame.side_data().len(), 1);

        let taken = frame.take_side_data();
        assert!(frame.side_data().is_empty());
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].kind(), "replaygain");
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
