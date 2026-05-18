use crate::{AvError, AvResult};

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
}

fn validate_channels(channels: u16) -> AvResult<()> {
    if channels == 0 {
        return Err(AvError::invalid_argument(
            "sample format channel count must be non-zero",
        ));
    }
    Ok(())
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
