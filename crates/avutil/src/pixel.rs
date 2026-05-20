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
    Yuv422p,
    Yuv444p,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormatClass {
    Gray,
    Rgb,
    Yuv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelFormatDescriptor {
    pub format: PixelFormat,
    pub name: &'static str,
    pub class: PixelFormatClass,
    pub component_count: usize,
    pub bits_per_component: u8,
    pub bits_per_pixel: u8,
    pub plane_count: usize,
    pub is_planar: bool,
    pub has_alpha: bool,
    pub packed_bytes_per_pixel: Option<usize>,
    pub log2_chroma_w: u8,
    pub log2_chroma_h: u8,
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
        Self::Yuv422p,
        Self::Yuv444p,
    ];

    pub fn name(self) -> &'static str {
        self.descriptor().name
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
            "yuv422p" => Some(Self::Yuv422p),
            "yuv444p" => Some(Self::Yuv444p),
            _ => None,
        }
    }

    pub fn descriptor(self) -> PixelFormatDescriptor {
        let (
            name,
            class,
            component_count,
            bits_per_pixel,
            plane_count,
            is_planar,
            has_alpha,
            packed_bytes_per_pixel,
            log2_chroma_w,
            log2_chroma_h,
        ) = match self {
            Self::Gray8 => (
                "gray",
                PixelFormatClass::Gray,
                1,
                8,
                1,
                false,
                false,
                Some(1),
                0,
                0,
            ),
            Self::Rgb24 => (
                "rgb24",
                PixelFormatClass::Rgb,
                3,
                24,
                1,
                false,
                false,
                Some(3),
                0,
                0,
            ),
            Self::Bgr24 => (
                "bgr24",
                PixelFormatClass::Rgb,
                3,
                24,
                1,
                false,
                false,
                Some(3),
                0,
                0,
            ),
            Self::Rgba => (
                "rgba",
                PixelFormatClass::Rgb,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Bgra => (
                "bgra",
                PixelFormatClass::Rgb,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Argb => (
                "argb",
                PixelFormatClass::Rgb,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Abgr => (
                "abgr",
                PixelFormatClass::Rgb,
                4,
                32,
                1,
                false,
                true,
                Some(4),
                0,
                0,
            ),
            Self::Yuv420p => (
                "yuv420p",
                PixelFormatClass::Yuv,
                3,
                12,
                3,
                true,
                false,
                None,
                1,
                1,
            ),
            Self::Yuv422p => (
                "yuv422p",
                PixelFormatClass::Yuv,
                3,
                16,
                3,
                true,
                false,
                None,
                1,
                0,
            ),
            Self::Yuv444p => (
                "yuv444p",
                PixelFormatClass::Yuv,
                3,
                24,
                3,
                true,
                false,
                None,
                0,
                0,
            ),
        };

        PixelFormatDescriptor {
            format: self,
            name,
            class,
            component_count,
            bits_per_component: 8,
            bits_per_pixel,
            plane_count,
            is_planar,
            has_alpha,
            packed_bytes_per_pixel,
            log2_chroma_w,
            log2_chroma_h,
        }
    }

    pub fn class(self) -> PixelFormatClass {
        self.descriptor().class
    }

    pub fn is_gray(self) -> bool {
        self.class() == PixelFormatClass::Gray
    }

    pub fn is_rgb(self) -> bool {
        self.class() == PixelFormatClass::Rgb
    }

    pub fn is_yuv(self) -> bool {
        self.class() == PixelFormatClass::Yuv
    }

    pub fn component_count(self) -> usize {
        self.descriptor().component_count
    }

    pub fn bits_per_component(self) -> u8 {
        self.descriptor().bits_per_component
    }

    pub fn bits_per_pixel(self) -> u8 {
        self.descriptor().bits_per_pixel
    }

    pub fn log2_chroma(self) -> (u8, u8) {
        let descriptor = self.descriptor();
        (descriptor.log2_chroma_w, descriptor.log2_chroma_h)
    }

    pub fn has_chroma_subsampling(self) -> bool {
        let (log2_chroma_w, log2_chroma_h) = self.log2_chroma();
        log2_chroma_w != 0 || log2_chroma_h != 0
    }

    pub fn plane_count(self) -> usize {
        self.descriptor().plane_count
    }

    pub fn is_planar(self) -> bool {
        self.descriptor().is_planar
    }

    pub fn is_packed(self) -> bool {
        !self.is_planar()
    }

    pub fn has_alpha(self) -> bool {
        self.descriptor().has_alpha
    }

    pub fn packed_bytes_per_pixel(self) -> Option<usize> {
        self.descriptor().packed_bytes_per_pixel
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
            Self::Yuv420p | Self::Yuv422p | Self::Yuv444p => {
                let descriptor = self.descriptor();
                let chroma_w = 1_usize << descriptor.log2_chroma_w;
                let chroma_h = 1_usize << descriptor.log2_chroma_h;
                if descriptor.log2_chroma_w != 0 && width % chroma_w != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format width must be divisible by {}",
                        self.name(),
                        chroma_w
                    )));
                }
                if descriptor.log2_chroma_h != 0 && height % chroma_h != 0 {
                    return Err(AvError::invalid_argument(format!(
                        "{} pixel format height must be divisible by {}",
                        self.name(),
                        chroma_h
                    )));
                }
                let chroma = checked_area(
                    width >> descriptor.log2_chroma_w,
                    height >> descriptor.log2_chroma_h,
                    "planar YUV chroma plane area",
                )?;
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
        assert_eq!(
            PixelFormat::from_name("yuv422p"),
            Some(PixelFormat::Yuv422p)
        );
        assert_eq!(
            PixelFormat::from_name("yuv444p"),
            Some(PixelFormat::Yuv444p)
        );
        assert_eq!(PixelFormat::ALL.len(), 10);
        assert_eq!(PixelFormat::Rgba.plane_count(), 1);
        assert_eq!(PixelFormat::Yuv420p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv422p.plane_count(), 3);
        assert_eq!(PixelFormat::Yuv444p.plane_count(), 3);
        assert!(!PixelFormat::Rgb24.is_planar());
        assert!(PixelFormat::Rgb24.is_packed());
        assert!(PixelFormat::Yuv420p.is_planar());
        assert!(PixelFormat::Yuv422p.is_planar());
        assert!(PixelFormat::Yuv444p.is_planar());
        assert!(!PixelFormat::Yuv420p.is_packed());
        assert!(!PixelFormat::Rgb24.has_alpha());
        assert!(PixelFormat::Bgra.has_alpha());
        assert_eq!(PixelFormat::Bgr24.packed_bytes_per_pixel(), Some(3));
        assert_eq!(PixelFormat::Argb.packed_bytes_per_pixel(), Some(4));
        assert_eq!(PixelFormat::Yuv420p.packed_bytes_per_pixel(), None);
    }

    #[test]
    fn pixel_formats_report_descriptor_metadata() {
        let gray = PixelFormat::Gray8.descriptor();
        assert_eq!(gray.format, PixelFormat::Gray8);
        assert_eq!(gray.name, "gray");
        assert_eq!(gray.class, PixelFormatClass::Gray);
        assert_eq!(gray.component_count, 1);
        assert_eq!(gray.bits_per_component, 8);
        assert_eq!(gray.bits_per_pixel, 8);
        assert_eq!(gray.plane_count, 1);
        assert!(!gray.is_planar);
        assert!(!gray.has_alpha);
        assert_eq!(gray.packed_bytes_per_pixel, Some(1));
        assert_eq!((gray.log2_chroma_w, gray.log2_chroma_h), (0, 0));

        for format in [
            PixelFormat::Rgb24,
            PixelFormat::Bgr24,
            PixelFormat::Rgba,
            PixelFormat::Bgra,
            PixelFormat::Argb,
            PixelFormat::Abgr,
        ] {
            let descriptor = format.descriptor();
            assert_eq!(descriptor.format, format);
            assert_eq!(PixelFormat::from_name(descriptor.name), Some(format));
            assert_eq!(descriptor.class, PixelFormatClass::Rgb);
            assert!(format.is_rgb());
            assert!(!format.is_gray());
            assert!(!format.is_yuv());
            assert_eq!(descriptor.bits_per_component, 8);
            assert_eq!(descriptor.plane_count, 1);
            assert!(!descriptor.is_planar);
            assert_eq!(descriptor.has_alpha, format.has_alpha());
            assert_eq!(
                descriptor.packed_bytes_per_pixel,
                format.packed_bytes_per_pixel()
            );
            assert_eq!(format.log2_chroma(), (0, 0));
            assert!(!format.has_chroma_subsampling());
        }

        assert_eq!(PixelFormat::Rgb24.component_count(), 3);
        assert_eq!(PixelFormat::Rgb24.bits_per_pixel(), 24);
        assert_eq!(PixelFormat::Rgba.component_count(), 4);
        assert_eq!(PixelFormat::Rgba.bits_per_pixel(), 32);

        for (format, expected_name, expected_bits_per_pixel, expected_log2_chroma) in [
            (PixelFormat::Yuv420p, "yuv420p", 12, (1, 1)),
            (PixelFormat::Yuv422p, "yuv422p", 16, (1, 0)),
            (PixelFormat::Yuv444p, "yuv444p", 24, (0, 0)),
        ] {
            let yuv = format.descriptor();
            assert_eq!(yuv.format, format);
            assert_eq!(yuv.name, expected_name);
            assert_eq!(yuv.class, PixelFormatClass::Yuv);
            assert!(format.is_yuv());
            assert!(!format.is_rgb());
            assert_eq!(yuv.component_count, 3);
            assert_eq!(yuv.bits_per_component, 8);
            assert_eq!(yuv.bits_per_pixel, expected_bits_per_pixel);
            assert_eq!(yuv.plane_count, 3);
            assert!(yuv.is_planar);
            assert!(!yuv.has_alpha);
            assert_eq!(yuv.packed_bytes_per_pixel, None);
            assert_eq!(format.log2_chroma(), expected_log2_chroma);
            assert_eq!(
                format.has_chroma_subsampling(),
                expected_log2_chroma != (0, 0)
            );
        }
        assert_eq!(PixelFormat::Yuv420p.log2_chroma(), (1, 1));
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
        assert_eq!(
            PixelFormat::Yuv422p.plane_sizes(4, 3).unwrap(),
            vec![12, 6, 6]
        );
        assert_eq!(PixelFormat::Yuv422p.frame_size(4, 3).unwrap(), 24);
        assert_eq!(
            PixelFormat::Yuv444p.plane_sizes(3, 2).unwrap(),
            vec![6, 6, 6]
        );
        assert_eq!(PixelFormat::Yuv444p.frame_size(3, 2).unwrap(), 18);
    }

    #[test]
    fn pixel_formats_split_frame_payloads_by_plane() {
        let planes = PixelFormat::Yuv420p
            .split_planes(&[0, 1, 2, 3, 4, 5], 2, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3], vec![4], vec![5]]);

        let planes = PixelFormat::Yuv422p
            .split_planes(&[0, 1, 2, 3, 4, 5, 6, 7], 2, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1, 2, 3], vec![4, 5], vec![6, 7]]);

        let planes = PixelFormat::Yuv444p
            .split_planes(&[0, 1, 2, 3, 4, 5], 1, 2)
            .unwrap();

        assert_eq!(planes, vec![vec![0, 1], vec![2, 3], vec![4, 5]]);
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
            PixelFormat::Yuv422p.frame_size(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(PixelFormat::Yuv422p.frame_size(2, 3).unwrap(), 12);
        assert_eq!(PixelFormat::Yuv444p.frame_size(3, 2).unwrap(), 18);
        assert_eq!(
            PixelFormat::Rgb24
                .split_planes(&[0; 5], 1, 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }
}
