use crate::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Gray8,
    Rgb24,
    Bgr24,
    Rgba,
    Bgra,
    Argb,
    Abgr,
    Yuv420p,
}

impl PixelFormat {
    pub const ALL: &'static [Self] = &[
        Self::Gray8,
        Self::Rgb24,
        Self::Bgr24,
        Self::Rgba,
        Self::Bgra,
        Self::Argb,
        Self::Abgr,
        Self::Yuv420p,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Gray8 => "gray",
            Self::Rgb24 => "rgb24",
            Self::Bgr24 => "bgr24",
            Self::Rgba => "rgba",
            Self::Bgra => "bgra",
            Self::Argb => "argb",
            Self::Abgr => "abgr",
            Self::Yuv420p => "yuv420p",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "gray" | "gray8" => Some(Self::Gray8),
            "rgb24" => Some(Self::Rgb24),
            "bgr24" => Some(Self::Bgr24),
            "rgba" => Some(Self::Rgba),
            "bgra" => Some(Self::Bgra),
            "argb" => Some(Self::Argb),
            "abgr" => Some(Self::Abgr),
            "yuv420p" => Some(Self::Yuv420p),
            _ => None,
        }
    }

    pub fn plane_count(self) -> usize {
        match self {
            Self::Gray8
            | Self::Rgb24
            | Self::Bgr24
            | Self::Rgba
            | Self::Bgra
            | Self::Argb
            | Self::Abgr => 1,
            Self::Yuv420p => 3,
        }
    }

    pub fn is_planar(self) -> bool {
        self.plane_count() > 1
    }

    pub fn is_packed(self) -> bool {
        !self.is_planar()
    }

    pub fn has_alpha(self) -> bool {
        matches!(self, Self::Rgba | Self::Bgra | Self::Argb | Self::Abgr)
    }

    pub fn packed_bytes_per_pixel(self) -> Option<usize> {
        match self {
            Self::Gray8 => Some(1),
            Self::Rgb24 | Self::Bgr24 => Some(3),
            Self::Rgba | Self::Bgra | Self::Argb | Self::Abgr => Some(4),
            Self::Yuv420p => None,
        }
    }

    pub fn plane_sizes(self, width: usize, height: usize) -> AvResult<Vec<usize>> {
        validate_dimensions(width, height, "pixel format")?;
        let pixels = checked_area(width, height, "pixel format frame area")?;

        match self {
            Self::Gray8 => Ok(vec![pixels]),
            Self::Rgb24 | Self::Bgr24 => Ok(vec![checked_mul(
                pixels,
                3,
                "24-bit packed pixel format frame size",
            )?]),
            Self::Rgba | Self::Bgra | Self::Argb | Self::Abgr => Ok(vec![checked_mul(
                pixels,
                4,
                "32-bit packed pixel format frame size",
            )?]),
            Self::Yuv420p => {
                if width % 2 != 0 || height % 2 != 0 {
                    return Err(AvError::invalid_argument(
                        "yuv420p pixel format dimensions must be even",
                    ));
                }
                let chroma = checked_area(width / 2, height / 2, "yuv420p chroma plane area")?;
                Ok(vec![pixels, chroma, chroma])
            }
        }
    }

    pub fn frame_size(self, width: usize, height: usize) -> AvResult<usize> {
        sum_checked(
            self.plane_sizes(width, height)?,
            "pixel format frame size overflow",
        )
    }

    pub fn split_planes(self, data: &[u8], width: usize, height: usize) -> AvResult<Vec<Vec<u8>>> {
        let plane_sizes = self.plane_sizes(width, height)?;
        let expected = sum_checked(
            plane_sizes.iter().copied(),
            "pixel format frame size overflow",
        )?;
        if data.len() != expected {
            return Err(AvError::invalid_data(format!(
                "{} frame has {} bytes, expected {expected}",
                self.name(),
                data.len()
            )));
        }

        let mut start = 0;
        let mut planes = Vec::with_capacity(plane_sizes.len());
        for size in plane_sizes {
            let end = start + size;
            planes.push(data[start..end].to_vec());
            start = end;
        }
        Ok(planes)
    }
}

fn validate_dimensions(width: usize, height: usize, context: &str) -> AvResult<()> {
    if width == 0 || height == 0 {
        return Err(AvError::invalid_argument(format!(
            "{context} dimensions must be non-zero"
        )));
    }
    Ok(())
}

fn checked_area(width: usize, height: usize, context: &str) -> AvResult<usize> {
    width
        .checked_mul(height)
        .ok_or_else(|| AvError::invalid_argument(format!("{context} overflow")))
}

fn checked_mul(value: usize, factor: usize, context: &str) -> AvResult<usize> {
    value
        .checked_mul(factor)
        .ok_or_else(|| AvError::invalid_argument(format!("{context} overflow")))
}

fn sum_checked(
    values: impl IntoIterator<Item = usize>,
    overflow_message: &'static str,
) -> AvResult<usize> {
    values.into_iter().try_fold(0_usize, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| AvError::invalid_argument(overflow_message))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn pixel_formats_report_ffmpeg_names_and_layout() {
        assert_eq!(PixelFormat::from_name("gray"), Some(PixelFormat::Gray8));
        assert_eq!(PixelFormat::from_name("gray8"), Some(PixelFormat::Gray8));
        assert_eq!(PixelFormat::Rgb24.name(), "rgb24");
        assert_eq!(PixelFormat::from_name("bgr24"), Some(PixelFormat::Bgr24));
        assert_eq!(PixelFormat::from_name("bgra"), Some(PixelFormat::Bgra));
        assert_eq!(PixelFormat::from_name("argb"), Some(PixelFormat::Argb));
        assert_eq!(PixelFormat::from_name("abgr"), Some(PixelFormat::Abgr));
        assert_eq!(PixelFormat::ALL.len(), 8);
        assert_eq!(PixelFormat::Rgba.plane_count(), 1);
        assert_eq!(PixelFormat::Yuv420p.plane_count(), 3);
        assert!(!PixelFormat::Rgb24.is_planar());
        assert!(PixelFormat::Rgb24.is_packed());
        assert!(PixelFormat::Yuv420p.is_planar());
        assert!(!PixelFormat::Yuv420p.is_packed());
        assert!(!PixelFormat::Rgb24.has_alpha());
        assert!(PixelFormat::Bgra.has_alpha());
        assert_eq!(PixelFormat::Bgr24.packed_bytes_per_pixel(), Some(3));
        assert_eq!(PixelFormat::Argb.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Yuv420p.packed_bytes_per_pixel(), None);
    }

    #[test]
    fn pixel_formats_compute_plane_and_frame_sizes() {
        assert_eq!(PixelFormat::Gray8.plane_sizes(2, 2).unwrap(), vec![4]);
        assert_eq!(PixelFormat::Rgb24.frame_size(2, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Bgr24.frame_size(2, 2).unwrap(), 12);
        assert_eq!(PixelFormat::Rgba.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Bgra.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Argb.frame_size(2, 2).unwrap(), 16);
        assert_eq!(PixelFormat::Abgr.frame_size(2, 2).unwrap(), 16);
        assert_eq!(
            PixelFormat::Yuv420p.plane_sizes(4, 2).unwrap(),
            vec![8, 2, 2]
        );
        assert_eq!(PixelFormat::Yuv420p.frame_size(4, 2).unwrap(), 12);
    }

    #[test]
    fn pixel_formats_split_frame_payloads_by_plane() {
        let planes = PixelFormat::Yuv420p
            .split_planes(&[0, 1, 2, 3, 4, 5], 2, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3], vec![4], vec![5]]);
    }

    #[test]
    fn pixel_formats_reject_invalid_dimensions_and_payload_sizes() {
        assert_eq!(
            PixelFormat::Gray8.frame_size(0, 1).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Yuv420p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PixelFormat::Rgb24
                .split_planes(&[0; 5], 1, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }
}
