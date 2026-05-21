use crate::{AvError, AvResult};

const FFMPEG_INT_MAX: usize = i32::MAX as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormat {
    U8,
    S16,
    S32,
    Flt,
    Dbl,
    U8P,
    S16P,
    S32P,
    FltP,
    DblP,
    S64,
    S64P,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormatFamily {
    U8,
    S16,
    S32,
    Flt,
    Dbl,
    S64,
}

impl SampleFormatFamily {
    pub fn packed(self) -> SampleFormat {
        match self {
            Self::U8 => SampleFormat::U8,
            Self::S16 => SampleFormat::S16,
            Self::S32 => SampleFormat::S32,
            Self::Flt => SampleFormat::Flt,
            Self::Dbl => SampleFormat::Dbl,
            Self::S64 => SampleFormat::S64,
        }
    }

    pub fn planar(self) -> SampleFormat {
        match self {
            Self::U8 => SampleFormat::U8P,
            Self::S16 => SampleFormat::S16P,
            Self::S32 => SampleFormat::S32P,
            Self::Flt => SampleFormat::FltP,
            Self::Dbl => SampleFormat::DblP,
            Self::S64 => SampleFormat::S64P,
        }
    }

    pub fn numeric_kind(self) -> SampleFormatNumericKind {
        match self {
            Self::U8 => SampleFormatNumericKind::UnsignedInteger,
            Self::S16 | Self::S32 | Self::S64 => SampleFormatNumericKind::SignedInteger,
            Self::Flt | Self::Dbl => SampleFormatNumericKind::Float,
        }
    }

    pub fn bytes_per_sample(self) -> usize {
        self.packed().bytes_per_sample()
    }

    pub fn sample_bits(self) -> usize {
        self.bytes_per_sample() * 8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormatNumericKind {
    UnsignedInteger,
    SignedInteger,
    Float,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleBufferLayout {
    line_size: usize,
    buffer_size: usize,
    plane_count: usize,
    samples_per_channel: usize,
    alignment: usize,
}

impl SampleBufferLayout {
    pub fn line_size(self) -> usize {
        self.line_size
    }

    pub fn buffer_size(self) -> usize {
        self.buffer_size
    }

    pub fn plane_count(self) -> usize {
        self.plane_count
    }

    pub fn samples_per_channel(self) -> usize {
        self.samples_per_channel
    }

    pub fn alignment(self) -> usize {
        self.alignment
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleSilenceRange {
    byte_offset: usize,
    byte_len: usize,
    plane_count: usize,
    fill_byte: u8,
}

impl SampleSilenceRange {
    pub fn byte_offset(self) -> usize {
        self.byte_offset
    }

    pub fn byte_len(self) -> usize {
        self.byte_len
    }

    pub fn plane_count(self) -> usize {
        self.plane_count
    }

    pub fn fill_byte(self) -> u8 {
        self.fill_byte
    }

    pub fn is_empty(self) -> bool {
        self.byte_len == 0
    }
}

impl SampleFormat {
    pub const ALL: [Self; 12] = [
        Self::U8,
        Self::S16,
        Self::S32,
        Self::Flt,
        Self::Dbl,
        Self::U8P,
        Self::S16P,
        Self::S32P,
        Self::FltP,
        Self::DblP,
        Self::S64,
        Self::S64P,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::S16 => "s16",
            Self::S32 => "s32",
            Self::Flt => "flt",
            Self::Dbl => "dbl",
            Self::U8P => "u8p",
            Self::S16P => "s16p",
            Self::S32P => "s32p",
            Self::FltP => "fltp",
            Self::DblP => "dblp",
            Self::S64 => "s64",
            Self::S64P => "s64p",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "u8" => Some(Self::U8),
            "s16" => Some(Self::S16),
            "s32" => Some(Self::S32),
            "flt" => Some(Self::Flt),
            "dbl" => Some(Self::Dbl),
            "u8p" => Some(Self::U8P),
            "s16p" => Some(Self::S16P),
            "s32p" => Some(Self::S32P),
            "fltp" => Some(Self::FltP),
            "dblp" => Some(Self::DblP),
            "s64" => Some(Self::S64),
            "s64p" => Some(Self::S64P),
            _ => None,
        }
    }

    pub fn bytes_per_sample(self) -> usize {
        match self {
            Self::U8 | Self::U8P => 1,
            Self::S16 | Self::S16P => 2,
            Self::S32 | Self::S32P | Self::Flt | Self::FltP => 4,
            Self::Dbl | Self::DblP | Self::S64 | Self::S64P => 8,
        }
    }

    pub fn sample_bits(self) -> usize {
        self.bytes_per_sample() * 8
    }

    pub fn is_planar(self) -> bool {
        matches!(
            self,
            Self::U8P | Self::S16P | Self::S32P | Self::FltP | Self::DblP | Self::S64P
        )
    }

    pub fn family(self) -> SampleFormatFamily {
        match self {
            Self::U8 | Self::U8P => SampleFormatFamily::U8,
            Self::S16 | Self::S16P => SampleFormatFamily::S16,
            Self::S32 | Self::S32P => SampleFormatFamily::S32,
            Self::Flt | Self::FltP => SampleFormatFamily::Flt,
            Self::Dbl | Self::DblP => SampleFormatFamily::Dbl,
            Self::S64 | Self::S64P => SampleFormatFamily::S64,
        }
    }

    pub fn numeric_kind(self) -> SampleFormatNumericKind {
        self.family().numeric_kind()
    }

    pub fn is_integer(self) -> bool {
        !self.is_float()
    }

    pub fn is_float(self) -> bool {
        self.numeric_kind() == SampleFormatNumericKind::Float
    }

    pub fn is_signed_integer(self) -> bool {
        self.numeric_kind() == SampleFormatNumericKind::SignedInteger
    }

    pub fn is_unsigned_integer(self) -> bool {
        self.numeric_kind() == SampleFormatNumericKind::UnsignedInteger
    }

    pub fn packed(self) -> Self {
        self.family().packed()
    }

    pub fn planar(self) -> Self {
        self.family().planar()
    }

    pub fn with_planar(self, planar: bool) -> Self {
        if planar {
            self.planar()
        } else {
            self.packed()
        }
    }

    pub fn plane_count(self, channels: u16) -> AvResult<usize> {
        validate_channels(channels)?;
        Ok(if self.is_planar() {
            usize::from(channels)
        } else {
            1
        })
    }

    pub fn bytes_per_sample_frame(self, channels: u16) -> AvResult<usize> {
        validate_channels(channels)?;
        self.bytes_per_sample()
            .checked_mul(usize::from(channels))
            .ok_or_else(|| AvError::invalid_argument("sample format frame size overflow"))
    }

    pub fn plane_sizes(self, samples_per_channel: usize, channels: u16) -> AvResult<Vec<usize>> {
        let plane_count = self.plane_count(channels)?;
        if self.is_planar() {
            let plane_size = samples_per_channel
                .checked_mul(self.bytes_per_sample())
                .ok_or_else(|| AvError::invalid_argument("sample format plane size overflow"))?;
            Ok(vec![plane_size; plane_count])
        } else {
            let frame_size = self.bytes_per_sample_frame(channels)?;
            let payload_size = samples_per_channel
                .checked_mul(frame_size)
                .ok_or_else(|| AvError::invalid_argument("sample format payload size overflow"))?;
            Ok(vec![payload_size])
        }
    }

    pub fn buffer_layout(
        self,
        samples_per_channel: usize,
        channels: u16,
        alignment: usize,
    ) -> AvResult<SampleBufferLayout> {
        validate_channels(channels)?;
        if samples_per_channel == 0 {
            return Err(AvError::invalid_argument(
                "sample buffer sample count must be non-zero",
            ));
        }

        let (effective_samples, effective_alignment) = if alignment == 0 {
            if samples_per_channel > FFMPEG_INT_MAX - 31 {
                return Err(AvError::invalid_argument(
                    "sample buffer auto-alignment sample count overflow",
                ));
            }
            (
                align_size(samples_per_channel, 32, "sample buffer samples")?,
                1,
            )
        } else {
            if alignment > FFMPEG_INT_MAX {
                return Err(AvError::invalid_argument(
                    "sample buffer alignment is out of FFmpeg int range",
                ));
            }
            (samples_per_channel, alignment)
        };

        let channels = usize::from(channels);
        let sample_bytes = self.bytes_per_sample();
        let unaligned_line_size = if self.is_planar() {
            effective_samples
                .checked_mul(sample_bytes)
                .ok_or_else(|| AvError::invalid_argument("sample buffer line size overflow"))?
        } else {
            effective_samples
                .checked_mul(sample_bytes)
                .and_then(|bytes| bytes.checked_mul(channels))
                .ok_or_else(|| AvError::invalid_argument("sample buffer line size overflow"))?
        };
        let line_size = align_size(
            unaligned_line_size,
            effective_alignment,
            "sample buffer line size",
        )?;
        if line_size > FFMPEG_INT_MAX {
            return Err(AvError::invalid_argument(
                "sample buffer line size exceeds FFmpeg int range",
            ));
        }

        let plane_count = if self.is_planar() { channels } else { 1 };
        let buffer_size = line_size
            .checked_mul(plane_count)
            .ok_or_else(|| AvError::invalid_argument("sample buffer size overflow"))?;
        if buffer_size > FFMPEG_INT_MAX {
            return Err(AvError::invalid_argument(
                "sample buffer size exceeds FFmpeg int range",
            ));
        }

        Ok(SampleBufferLayout {
            line_size,
            buffer_size,
            plane_count,
            samples_per_channel: effective_samples,
            alignment: effective_alignment,
        })
    }

    pub fn aligned_plane_sizes(
        self,
        samples_per_channel: usize,
        channels: u16,
        alignment: usize,
    ) -> AvResult<Vec<usize>> {
        let layout = self.buffer_layout(samples_per_channel, channels, alignment)?;
        Ok(vec![layout.line_size(); layout.plane_count()])
    }

    pub fn silence_byte(self) -> u8 {
        match self {
            Self::U8 | Self::U8P => 0x80,
            Self::S16
            | Self::S16P
            | Self::S32
            | Self::S32P
            | Self::Flt
            | Self::FltP
            | Self::Dbl
            | Self::DblP
            | Self::S64
            | Self::S64P => 0x00,
        }
    }

    pub fn silence_range(
        self,
        offset_samples: usize,
        samples_per_channel: usize,
        channels: u16,
    ) -> AvResult<SampleSilenceRange> {
        validate_channels(channels)?;
        let channel_count = usize::from(channels);
        let block_channels = if self.is_planar() { 1 } else { channel_count };
        let block_align = self
            .bytes_per_sample()
            .checked_mul(block_channels)
            .ok_or_else(|| AvError::invalid_argument("sample silence block alignment overflow"))?;
        let byte_offset = offset_samples
            .checked_mul(block_align)
            .ok_or_else(|| AvError::invalid_argument("sample silence offset overflow"))?;
        let byte_len = samples_per_channel
            .checked_mul(block_align)
            .ok_or_else(|| AvError::invalid_argument("sample silence length overflow"))?;
        byte_offset
            .checked_add(byte_len)
            .ok_or_else(|| AvError::invalid_argument("sample silence range overflow"))?;

        Ok(SampleSilenceRange {
            byte_offset,
            byte_len,
            plane_count: if self.is_planar() { channel_count } else { 1 },
            fill_byte: self.silence_byte(),
        })
    }

    pub fn fill_silence<P: AsMut<[u8]>>(
        self,
        audio_data: &mut [P],
        offset_samples: usize,
        samples_per_channel: usize,
        channels: u16,
    ) -> AvResult<()> {
        let range = self.silence_range(offset_samples, samples_per_channel, channels)?;
        if audio_data.len() != range.plane_count() {
            return Err(AvError::invalid_argument(
                "sample silence plane count does not match format",
            ));
        }
        let byte_end = range
            .byte_offset()
            .checked_add(range.byte_len())
            .ok_or_else(|| AvError::invalid_argument("sample silence range overflow"))?;

        for plane in audio_data.iter_mut() {
            if byte_end > plane.as_mut().len() {
                return Err(AvError::invalid_argument(
                    "sample silence range exceeds plane length",
                ));
            }
        }

        for plane in audio_data {
            plane.as_mut()[range.byte_offset()..byte_end].fill(range.fill_byte());
        }

        Ok(())
    }
}

fn validate_channels(channels: u16) -> AvResult<()> {
    if channels == 0 {
        return Err(AvError::invalid_argument(
            "sample format channel count must be non-zero",
        ));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn sample_formats_report_ffmpeg_names_and_layout() {
        let cases = [
            (SampleFormat::U8, "u8", 1, false),
            (SampleFormat::S16, "s16", 2, false),
            (SampleFormat::S32, "s32", 4, false),
            (SampleFormat::Flt, "flt", 4, false),
            (SampleFormat::Dbl, "dbl", 8, false),
            (SampleFormat::U8P, "u8p", 1, true),
            (SampleFormat::S16P, "s16p", 2, true),
            (SampleFormat::S32P, "s32p", 4, true),
            (SampleFormat::FltP, "fltp", 4, true),
            (SampleFormat::DblP, "dblp", 8, true),
            (SampleFormat::S64, "s64", 8, false),
            (SampleFormat::S64P, "s64p", 8, true),
        ];

        assert_eq!(SampleFormat::ALL.len(), cases.len());
        for (format, name, bytes_per_sample, is_planar) in cases {
            assert_eq!(SampleFormat::from_name(name), Some(format));
            assert_eq!(format.name(), name);
            assert_eq!(format.bytes_per_sample(), bytes_per_sample);
            assert_eq!(format.is_planar(), is_planar);
            assert_eq!(
                format.plane_count(2).unwrap(),
                if is_planar { 2 } else { 1 }
            );
            assert!(SampleFormat::ALL.contains(&format));
        }

        assert_eq!(SampleFormat::from_name("s24"), None);
    }

    #[test]
    fn sample_formats_compute_packed_payload_sizes() {
        for (format, bytes_per_sample) in [
            (SampleFormat::U8, 1),
            (SampleFormat::S16, 2),
            (SampleFormat::S32, 4),
            (SampleFormat::Flt, 4),
            (SampleFormat::Dbl, 8),
            (SampleFormat::S64, 8),
        ] {
            assert_eq!(format.bytes_per_sample(), bytes_per_sample);
            assert_eq!(format.bytes_per_sample_frame(1).unwrap(), bytes_per_sample);
            assert_eq!(
                format.bytes_per_sample_frame(6).unwrap(),
                bytes_per_sample * 6
            );
            assert_eq!(
                format.plane_sizes(1024, 2).unwrap(),
                vec![1024 * bytes_per_sample * 2]
            );
            assert_eq!(format.plane_sizes(0, 2).unwrap(), vec![0]);
        }
    }

    #[test]
    fn sample_formats_compute_planar_payload_sizes() {
        for (format, bytes_per_sample) in [
            (SampleFormat::U8P, 1),
            (SampleFormat::S16P, 2),
            (SampleFormat::S32P, 4),
            (SampleFormat::FltP, 4),
            (SampleFormat::DblP, 8),
            (SampleFormat::S64P, 8),
        ] {
            assert_eq!(format.bytes_per_sample(), bytes_per_sample);
            assert_eq!(format.bytes_per_sample_frame(1).unwrap(), bytes_per_sample);
            assert_eq!(
                format.bytes_per_sample_frame(6).unwrap(),
                bytes_per_sample * 6
            );
            assert_eq!(
                format.plane_sizes(1024, 2).unwrap(),
                vec![1024 * bytes_per_sample, 1024 * bytes_per_sample]
            );
            assert_eq!(format.plane_sizes(0, 3).unwrap(), vec![0, 0, 0]);
        }
    }

    #[test]
    fn sample_formats_compute_ffmpeg_buffer_layouts() {
        let packed = SampleFormat::S16.buffer_layout(1024, 2, 1).unwrap();
        assert_eq!(packed.line_size(), 4096);
        assert_eq!(packed.buffer_size(), 4096);
        assert_eq!(packed.plane_count(), 1);
        assert_eq!(packed.samples_per_channel(), 1024);
        assert_eq!(packed.alignment(), 1);
        assert_eq!(
            SampleFormat::S16.aligned_plane_sizes(1024, 2, 1).unwrap(),
            vec![4096]
        );

        let planar = SampleFormat::S16P.buffer_layout(1024, 2, 1).unwrap();
        assert_eq!(planar.line_size(), 2048);
        assert_eq!(planar.buffer_size(), 4096);
        assert_eq!(planar.plane_count(), 2);
        assert_eq!(
            SampleFormat::S16P.aligned_plane_sizes(1024, 2, 1).unwrap(),
            vec![2048, 2048]
        );

        let aligned_planar = SampleFormat::FltP.buffer_layout(3, 2, 16).unwrap();
        assert_eq!(aligned_planar.line_size(), 16);
        assert_eq!(aligned_planar.buffer_size(), 32);
        assert_eq!(aligned_planar.plane_count(), 2);
        assert_eq!(aligned_planar.samples_per_channel(), 3);
        assert_eq!(aligned_planar.alignment(), 16);

        let aligned_packed = SampleFormat::S16.buffer_layout(3, 2, 8).unwrap();
        assert_eq!(aligned_packed.line_size(), 16);
        assert_eq!(aligned_packed.buffer_size(), 16);
    }

    #[test]
    fn sample_buffer_layout_zero_alignment_matches_ffmpeg_auto_sample_padding() {
        let packed = SampleFormat::S16.buffer_layout(1, 1, 0).unwrap();
        assert_eq!(packed.samples_per_channel(), 32);
        assert_eq!(packed.alignment(), 1);
        assert_eq!(packed.line_size(), 64);
        assert_eq!(packed.buffer_size(), 64);

        let planar = SampleFormat::U8P.buffer_layout(33, 2, 0).unwrap();
        assert_eq!(planar.samples_per_channel(), 64);
        assert_eq!(planar.alignment(), 1);
        assert_eq!(planar.line_size(), 64);
        assert_eq!(planar.buffer_size(), 128);
        assert_eq!(planar.plane_count(), 2);
    }

    #[test]
    fn sample_buffer_layout_rejects_invalid_and_overflowing_inputs() {
        assert_eq!(
            SampleFormat::S16.buffer_layout(0, 2, 1).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            SampleFormat::S16.buffer_layout(1, 0, 1).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            SampleFormat::S16
                .buffer_layout(usize::MAX, 2, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            SampleFormat::S16
                .buffer_layout(1, 2, usize::MAX)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            SampleFormat::S64
                .buffer_layout(FFMPEG_INT_MAX, 2, 1)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            SampleFormat::S16
                .buffer_layout(FFMPEG_INT_MAX, 1, 0)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn sample_formats_report_ffmpeg_silence_bytes() {
        for format in SampleFormat::ALL {
            let expected = match format {
                SampleFormat::U8 | SampleFormat::U8P => 0x80,
                _ => 0x00,
            };
            assert_eq!(format.silence_byte(), expected);
        }
    }

    #[test]
    fn sample_formats_compute_silence_ranges() {
        let packed = SampleFormat::S16.silence_range(3, 2, 2).unwrap();
        assert_eq!(packed.byte_offset(), 12);
        assert_eq!(packed.byte_len(), 8);
        assert_eq!(packed.plane_count(), 1);
        assert_eq!(packed.fill_byte(), 0);
        assert!(!packed.is_empty());

        let planar = SampleFormat::U8P.silence_range(2, 3, 2).unwrap();
        assert_eq!(planar.byte_offset(), 2);
        assert_eq!(planar.byte_len(), 3);
        assert_eq!(planar.plane_count(), 2);
        assert_eq!(planar.fill_byte(), 0x80);

        let empty = SampleFormat::FltP.silence_range(4, 0, 6).unwrap();
        assert_eq!(empty.byte_offset(), 16);
        assert_eq!(empty.byte_len(), 0);
        assert_eq!(empty.plane_count(), 6);
        assert!(empty.is_empty());
    }

    #[test]
    fn sample_formats_fill_packed_silence_like_ffmpeg() {
        let mut planes = vec![vec![0x7F; 24]];
        SampleFormat::S16
            .fill_silence(&mut planes, 1, 2, 2)
            .unwrap();
        assert_eq!(&planes[0][..4], &[0x7F; 4]);
        assert_eq!(&planes[0][4..12], &[0; 8]);
        assert_eq!(&planes[0][12..], &[0x7F; 12]);

        SampleFormat::U8.fill_silence(&mut planes, 0, 3, 2).unwrap();
        assert_eq!(&planes[0][..6], &[0x80; 6]);
        assert_eq!(&planes[0][6..12], &[0; 6]);
    }

    #[test]
    fn sample_formats_fill_planar_u8_silence_like_ffmpeg() {
        let mut planes = vec![vec![0x11; 8], vec![0x22; 8]];
        SampleFormat::U8P
            .fill_silence(&mut planes, 2, 3, 2)
            .unwrap();

        assert_eq!(
            planes[0],
            vec![0x11, 0x11, 0x80, 0x80, 0x80, 0x11, 0x11, 0x11]
        );
        assert_eq!(
            planes[1],
            vec![0x22, 0x22, 0x80, 0x80, 0x80, 0x22, 0x22, 0x22]
        );
    }

    #[test]
    fn sample_silence_rejects_invalid_inputs_without_mutation() {
        assert_eq!(
            SampleFormat::S16.silence_range(0, 1, 0).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            SampleFormat::S64
                .silence_range(usize::MAX, 1, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );

        let mut wrong_plane_count = vec![vec![0x55; 16], vec![0x66; 16]];
        let before = wrong_plane_count.clone();
        assert_eq!(
            SampleFormat::S16
                .fill_silence(&mut wrong_plane_count, 0, 1, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(wrong_plane_count, before);

        let mut short_planar = vec![vec![0x55; 8], vec![0x66; 3]];
        let before = short_planar.clone();
        assert_eq!(
            SampleFormat::S16P
                .fill_silence(&mut short_planar, 1, 2, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(short_planar, before);
    }

    #[test]
    fn sample_formats_report_descriptor_metadata_and_counterparts() {
        let cases = [
            (
                SampleFormat::U8,
                SampleFormatFamily::U8,
                SampleFormatNumericKind::UnsignedInteger,
                SampleFormat::U8,
                SampleFormat::U8P,
            ),
            (
                SampleFormat::S16,
                SampleFormatFamily::S16,
                SampleFormatNumericKind::SignedInteger,
                SampleFormat::S16,
                SampleFormat::S16P,
            ),
            (
                SampleFormat::S32,
                SampleFormatFamily::S32,
                SampleFormatNumericKind::SignedInteger,
                SampleFormat::S32,
                SampleFormat::S32P,
            ),
            (
                SampleFormat::Flt,
                SampleFormatFamily::Flt,
                SampleFormatNumericKind::Float,
                SampleFormat::Flt,
                SampleFormat::FltP,
            ),
            (
                SampleFormat::Dbl,
                SampleFormatFamily::Dbl,
                SampleFormatNumericKind::Float,
                SampleFormat::Dbl,
                SampleFormat::DblP,
            ),
            (
                SampleFormat::U8P,
                SampleFormatFamily::U8,
                SampleFormatNumericKind::UnsignedInteger,
                SampleFormat::U8,
                SampleFormat::U8P,
            ),
            (
                SampleFormat::S16P,
                SampleFormatFamily::S16,
                SampleFormatNumericKind::SignedInteger,
                SampleFormat::S16,
                SampleFormat::S16P,
            ),
            (
                SampleFormat::S32P,
                SampleFormatFamily::S32,
                SampleFormatNumericKind::SignedInteger,
                SampleFormat::S32,
                SampleFormat::S32P,
            ),
            (
                SampleFormat::FltP,
                SampleFormatFamily::Flt,
                SampleFormatNumericKind::Float,
                SampleFormat::Flt,
                SampleFormat::FltP,
            ),
            (
                SampleFormat::DblP,
                SampleFormatFamily::Dbl,
                SampleFormatNumericKind::Float,
                SampleFormat::Dbl,
                SampleFormat::DblP,
            ),
            (
                SampleFormat::S64,
                SampleFormatFamily::S64,
                SampleFormatNumericKind::SignedInteger,
                SampleFormat::S64,
                SampleFormat::S64P,
            ),
            (
                SampleFormat::S64P,
                SampleFormatFamily::S64,
                SampleFormatNumericKind::SignedInteger,
                SampleFormat::S64,
                SampleFormat::S64P,
            ),
        ];

        for (format, family, numeric_kind, packed, planar) in cases {
            assert_eq!(format.family(), family);
            assert_eq!(format.numeric_kind(), numeric_kind);
            assert_eq!(format.sample_bits(), format.bytes_per_sample() * 8);
            assert_eq!(family.bytes_per_sample(), format.bytes_per_sample());
            assert_eq!(family.sample_bits(), format.sample_bits());
            assert_eq!(family.numeric_kind(), numeric_kind);
            assert_eq!(format.packed(), packed);
            assert_eq!(format.planar(), planar);
            assert_eq!(format.with_planar(false), packed);
            assert_eq!(format.with_planar(true), planar);
            assert_eq!(family.packed(), packed);
            assert_eq!(family.planar(), planar);
            assert!(!packed.is_planar());
            assert!(planar.is_planar());
            assert_eq!(
                format.is_float(),
                numeric_kind == SampleFormatNumericKind::Float
            );
            assert_eq!(
                format.is_integer(),
                numeric_kind != SampleFormatNumericKind::Float
            );
            assert_eq!(
                format.is_signed_integer(),
                numeric_kind == SampleFormatNumericKind::SignedInteger
            );
            assert_eq!(
                format.is_unsigned_integer(),
                numeric_kind == SampleFormatNumericKind::UnsignedInteger
            );
        }
    }

    #[test]
    fn sample_formats_reject_invalid_channel_counts() {
        assert_eq!(
            SampleFormat::S16
                .bytes_per_sample_frame(0)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            SampleFormat::S16P.plane_count(0).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
    }
}
