#![no_main]

use avcodec::{PcmS16leDecoder, PixelFormat, RawVideoDecoder};
use avutil::{AvErrorKind, FrameData, Packet, SampleFormat};
use libfuzzer_sys::fuzz_target;

const MAX_RAWVIDEO_PAYLOAD: usize = 320;
const MAX_PCM_PAYLOAD: usize = 192;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);

    exercise_rawvideo(&mut cursor);
    exercise_pcm(&mut cursor);
    exercise_fixtures();
});

fn exercise_rawvideo(cursor: &mut Cursor<'_>) {
    let pixel_format = pixel_format_from(cursor.next());
    let width = dimension_from(cursor.next(), pixel_format, true);
    let height = dimension_from(cursor.next(), pixel_format, false);
    let Ok(decoder) = RawVideoDecoder::new(width, height, pixel_format) else {
        return;
    };

    assert_eq!(decoder.width(), width);
    assert_eq!(decoder.height(), height);
    assert_eq!(decoder.pixel_format(), pixel_format);

    let expected_size = decoder.frame_size();
    let payload = payload_for_mode(cursor, expected_size, MAX_RAWVIDEO_PAYLOAD);
    let pts = timestamp_from(cursor.next(), cursor.next());
    let mut packet = Packet::new(payload, usize::from(cursor.next().unwrap_or_default() % 3));
    packet.set_pts(pts);

    match decoder.decode_packet(&packet) {
        Ok(frame) => {
            assert_eq!(packet.data().len(), expected_size);
            assert_eq!(frame.pts(), pts);
            let FrameData::Video(video) = frame.data() else {
                panic!("rawvideo decoder returned non-video frame");
            };
            assert_eq!(video.width(), width);
            assert_eq!(video.height(), height);
            assert_eq!(video.pixel_format(), pixel_format);
            assert_eq!(video.pixel_format_name(), pixel_format.name());
            assert_eq!(video.planes().len(), pixel_format.plane_count());
            let plane_sizes = pixel_format.plane_sizes(width, height).unwrap();
            for (plane, expected_plane_size) in video.planes().iter().zip(plane_sizes) {
                assert_eq!(plane.len(), expected_plane_size);
            }
            let reconstructed: Vec<u8> = video
                .planes()
                .iter()
                .flat_map(|plane| plane.iter().copied())
                .collect();
            assert_eq!(reconstructed, packet.data());
        }
        Err(error) => {
            assert_ne!(packet.data().len(), expected_size);
            assert_eq!(error.kind(), AvErrorKind::InvalidData);
        }
    }
}

fn exercise_pcm(cursor: &mut Cursor<'_>) {
    let sample_rate = sample_rate_from(cursor.next());
    let channels = channel_count_from(cursor.next());
    let Ok(decoder) = PcmS16leDecoder::new(sample_rate, channels) else {
        return;
    };

    assert_eq!(decoder.sample_rate(), sample_rate);
    assert_eq!(decoder.channels(), channels);
    assert_eq!(
        decoder.bytes_per_sample_frame(),
        usize::from(channels) * PcmS16leDecoder::BYTES_PER_SAMPLE
    );

    let aligned_samples = usize::from(cursor.next().unwrap_or_default() % 16);
    let expected_aligned_len = aligned_samples * decoder.bytes_per_sample_frame();
    let payload = payload_for_mode(cursor, expected_aligned_len, MAX_PCM_PAYLOAD);
    let pts = timestamp_from(cursor.next(), cursor.next());
    let mut packet = Packet::new(payload, 0);
    packet.set_pts(pts);

    match decoder.decode_packet(&packet) {
        Ok(frame) => {
            assert_eq!(frame.pts(), pts);
            assert_eq!(
                decoder.samples_per_channel(packet.data().len()).unwrap(),
                packet.data().len() / decoder.bytes_per_sample_frame()
            );
            let FrameData::Audio(audio) = frame.data() else {
                panic!("pcm_s16le decoder returned non-audio frame");
            };
            assert_eq!(audio.sample_rate(), sample_rate);
            assert_eq!(audio.channels(), channels);
            assert_eq!(audio.sample_format(), SampleFormat::S16);
            assert_eq!(audio.sample_format_name(), "s16");
            assert_eq!(
                audio.samples_per_channel(),
                packet.data().len() / decoder.bytes_per_sample_frame()
            );
            assert_eq!(audio.planes(), &[packet.data().to_vec()]);
        }
        Err(error) => {
            assert_ne!(packet.data().len() % decoder.bytes_per_sample_frame(), 0);
            assert_eq!(error.kind(), AvErrorKind::InvalidData);
        }
    }
}

fn exercise_fixtures() {
    let gray = RawVideoDecoder::new(2, 2, PixelFormat::Gray8).unwrap();
    let mut gray_packet = Packet::new(vec![0, 1, 2, 3], 0);
    gray_packet.set_pts(Some(7));
    let gray_frame = gray.decode_packet(&gray_packet).unwrap();
    assert_eq!(gray_frame.pts(), Some(7));

    let gray16 = RawVideoDecoder::new(2, 1, PixelFormat::Gray16Le).unwrap();
    assert!(gray16
        .decode_packet(&Packet::new(vec![0, 1, 2, 3], 0))
        .is_ok());
    assert_eq!(
        gray16
            .decode_packet(&Packet::new(vec![0, 1, 2], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gray10 = RawVideoDecoder::new(2, 1, PixelFormat::Gray10Le).unwrap();
    assert!(gray10
        .decode_packet(&Packet::new(vec![0, 1, 2, 3], 0))
        .is_ok());
    assert_eq!(
        gray10
            .decode_packet(&Packet::new(vec![0, 1, 2], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gray32 = RawVideoDecoder::new(2, 1, PixelFormat::Gray32Be).unwrap();
    assert!(gray32.decode_packet(&Packet::new(vec![0; 8], 0)).is_ok());
    assert_eq!(
        gray32
            .decode_packet(&Packet::new(vec![0; 7], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let grayf16 = RawVideoDecoder::new(2, 1, PixelFormat::GrayF16Le).unwrap();
    assert!(grayf16.decode_packet(&Packet::new(vec![0; 4], 0)).is_ok());
    assert_eq!(
        grayf16
            .decode_packet(&Packet::new(vec![0; 3], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let grayf32 = RawVideoDecoder::new(2, 1, PixelFormat::GrayF32Be).unwrap();
    assert!(grayf32.decode_packet(&Packet::new(vec![0; 8], 0)).is_ok());
    assert_eq!(
        grayf32
            .decode_packet(&Packet::new(vec![0; 7], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrp = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp).unwrap();
    assert!(gbrp.decode_packet(&Packet::new(vec![0; 6], 0)).is_ok());
    assert_eq!(
        gbrp.decode_packet(&Packet::new(vec![0; 5], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrp9 = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp9Le).unwrap();
    assert!(gbrp9
        .decode_packet(&Packet::new((0..12).collect(), 0))
        .is_ok());
    assert_eq!(
        gbrp9
            .decode_packet(&Packet::new((0..11).collect(), 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrp10 = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp10Le).unwrap();
    assert!(gbrp10
        .decode_packet(&Packet::new((0..12).collect(), 0))
        .is_ok());
    assert_eq!(
        gbrp10
            .decode_packet(&Packet::new((0..11).collect(), 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrp12 = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp12Le).unwrap();
    assert!(gbrp12
        .decode_packet(&Packet::new((0..12).collect(), 0))
        .is_ok());
    assert_eq!(
        gbrp12
            .decode_packet(&Packet::new((0..11).collect(), 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrp14 = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp14Le).unwrap();
    assert!(gbrp14
        .decode_packet(&Packet::new((0..12).collect(), 0))
        .is_ok());
    assert_eq!(
        gbrp14
            .decode_packet(&Packet::new((0..11).collect(), 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrp16 = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp16Le).unwrap();
    assert!(gbrp16
        .decode_packet(&Packet::new((0..12).collect(), 0))
        .is_ok());
    assert_eq!(
        gbrp16
            .decode_packet(&Packet::new((0..11).collect(), 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrap = RawVideoDecoder::new(2, 1, PixelFormat::Gbrap).unwrap();
    assert!(gbrap
        .decode_packet(&Packet::new((0..8).collect(), 0))
        .is_ok());
    assert_eq!(
        gbrap
            .decode_packet(&Packet::new((0..7).collect(), 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrap16 = RawVideoDecoder::new(2, 1, PixelFormat::Gbrap16Le).unwrap();
    assert!(gbrap16
        .decode_packet(&Packet::new((0..16).collect(), 0))
        .is_ok());
    assert_eq!(
        gbrap16
            .decode_packet(&Packet::new((0..15).collect(), 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let gbrapf32 = RawVideoDecoder::new(1, 1, PixelFormat::GbrapF32Le).unwrap();
    assert!(gbrapf32
        .decode_packet(&Packet::new((0..16).collect(), 0))
        .is_ok());
    assert_eq!(
        gbrapf32
            .decode_packet(&Packet::new((0..15).collect(), 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let ya8 = RawVideoDecoder::new(2, 1, PixelFormat::Ya8).unwrap();
    assert!(ya8
        .decode_packet(&Packet::new(vec![0x10, 0xff, 0x80, 0x40], 0))
        .is_ok());
    assert_eq!(
        ya8.decode_packet(&Packet::new(vec![0x10, 0xff, 0x80], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let ya16 = RawVideoDecoder::new(2, 1, PixelFormat::Ya16Be).unwrap();
    assert!(ya16.decode_packet(&Packet::new(vec![0; 8], 0)).is_ok());
    assert_eq!(
        ya16.decode_packet(&Packet::new(vec![0; 7], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let rgb0 = RawVideoDecoder::new(2, 1, PixelFormat::Rgb0).unwrap();
    assert!(rgb0.decode_packet(&Packet::new(vec![0; 8], 0)).is_ok());
    assert_eq!(
        rgb0.decode_packet(&Packet::new(vec![0; 7], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    for format in [
        PixelFormat::Rgb8,
        PixelFormat::Bgr8,
        PixelFormat::Rgb4Byte,
        PixelFormat::Bgr4Byte,
    ] {
        let decoder = RawVideoDecoder::new(4, 1, format).unwrap();
        assert!(decoder.decode_packet(&Packet::new(vec![0; 4], 0)).is_ok());
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 3], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    let pal8 = RawVideoDecoder::new(2, 2, PixelFormat::Pal8).unwrap();
    assert!(pal8
        .decode_packet(&Packet::new(vec![0, 1, 2, 3], 0))
        .is_ok());
    assert_eq!(
        pal8.decode_packet(&Packet::new(vec![0, 1, 2], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    for format in [PixelFormat::Rgb4, PixelFormat::Bgr4] {
        let decoder = RawVideoDecoder::new(3, 2, format).unwrap();
        assert!(decoder.decode_packet(&Packet::new(vec![0; 4], 0)).is_ok());
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 3], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    for (format, width, height, valid_len, invalid_len) in [
        (PixelFormat::MonoWhite, 9, 2, 4, 3),
        (PixelFormat::MonoBlack, 16, 1, 2, 1),
    ] {
        let decoder = RawVideoDecoder::new(width, height, format).unwrap();
        assert!(decoder
            .decode_packet(&Packet::new(vec![0; valid_len], 0))
            .is_ok());
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; invalid_len], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    for format in [
        PixelFormat::Rgb565Le,
        PixelFormat::Rgb555Be,
        PixelFormat::Bgr565Le,
        PixelFormat::Bgr555Be,
        PixelFormat::Rgb444Le,
        PixelFormat::Bgr444Be,
    ] {
        let decoder = RawVideoDecoder::new(2, 1, format).unwrap();
        assert!(decoder.decode_packet(&Packet::new(vec![0; 4], 0)).is_ok());
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 3], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    for format in [
        PixelFormat::Yuyv422,
        PixelFormat::Uyvy422,
        PixelFormat::Yvyu422,
    ] {
        let decoder = RawVideoDecoder::new(2, 1, format).unwrap();
        assert!(decoder.decode_packet(&Packet::new(vec![0; 4], 0)).is_ok());
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 3], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert!(RawVideoDecoder::new(3, 1, format).is_err());
    }

    for format in [PixelFormat::Nv12, PixelFormat::Nv21] {
        let decoder = RawVideoDecoder::new(4, 2, format).unwrap();
        assert!(decoder.decode_packet(&Packet::new(vec![0; 12], 0)).is_ok());
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 11], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert!(RawVideoDecoder::new(3, 2, format).is_err());
        assert!(RawVideoDecoder::new(4, 3, format).is_err());
    }

    let rgb48 = RawVideoDecoder::new(1, 1, PixelFormat::Rgb48Le).unwrap();
    assert!(rgb48.decode_packet(&Packet::new(vec![0; 6], 0)).is_ok());
    assert_eq!(
        rgb48
            .decode_packet(&Packet::new(vec![0; 5], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let rgba64 = RawVideoDecoder::new(1, 1, PixelFormat::Rgba64Le).unwrap();
    assert!(rgba64.decode_packet(&Packet::new(vec![0; 8], 0)).is_ok());
    assert_eq!(
        rgba64
            .decode_packet(&Packet::new(vec![0; 7], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv = RawVideoDecoder::new(4, 2, PixelFormat::Yuv420p).unwrap();
    assert!(yuv.decode_packet(&Packet::new(vec![0; 12], 0)).is_ok());
    assert_eq!(
        yuv.decode_packet(&Packet::new(vec![0; 11], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuva420 = RawVideoDecoder::new(4, 2, PixelFormat::Yuva420p).unwrap();
    assert!(yuva420
        .decode_packet(&Packet::new(vec![0; 20], 0))
        .is_ok());
    assert_eq!(
        yuva420
            .decode_packet(&Packet::new(vec![0; 19], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuva420p).is_err());
    assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuva420p).is_err());

    let yuv422 = RawVideoDecoder::new(4, 3, PixelFormat::Yuv422p).unwrap();
    assert!(yuv422.decode_packet(&Packet::new(vec![0; 24], 0)).is_ok());
    assert_eq!(
        yuv422
            .decode_packet(&Packet::new(vec![0; 23], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv420p9 = RawVideoDecoder::new(4, 2, PixelFormat::Yuv420p9Le).unwrap();
    assert!(yuv420p9
        .decode_packet(&Packet::new(vec![0; 24], 0))
        .is_ok());
    assert_eq!(
        yuv420p9
            .decode_packet(&Packet::new(vec![0; 23], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv411 = RawVideoDecoder::new(4, 3, PixelFormat::Yuv411p).unwrap();
    assert!(yuv411.decode_packet(&Packet::new(vec![0; 18], 0)).is_ok());
    assert_eq!(
        yuv411
            .decode_packet(&Packet::new(vec![0; 17], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv410 = RawVideoDecoder::new(4, 4, PixelFormat::Yuv410p).unwrap();
    assert!(yuv410.decode_packet(&Packet::new(vec![0; 18], 0)).is_ok());
    assert_eq!(
        yuv410
            .decode_packet(&Packet::new(vec![0; 17], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv440 = RawVideoDecoder::new(3, 2, PixelFormat::Yuv440p).unwrap();
    assert!(yuv440.decode_packet(&Packet::new(vec![0; 12], 0)).is_ok());
    assert_eq!(
        yuv440
            .decode_packet(&Packet::new(vec![0; 11], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv440p10 = RawVideoDecoder::new(3, 2, PixelFormat::Yuv440p10Le).unwrap();
    assert!(yuv440p10
        .decode_packet(&Packet::new(vec![0; 24], 0))
        .is_ok());
    assert_eq!(
        yuv440p10
            .decode_packet(&Packet::new(vec![0; 23], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv440p12 = RawVideoDecoder::new(3, 2, PixelFormat::Yuv440p12Be).unwrap();
    assert!(yuv440p12
        .decode_packet(&Packet::new(vec![0; 24], 0))
        .is_ok());
    assert_eq!(
        yuv440p12
            .decode_packet(&Packet::new(vec![0; 23], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv420p10 = RawVideoDecoder::new(4, 2, PixelFormat::Yuv420p10Le).unwrap();
    assert!(yuv420p10
        .decode_packet(&Packet::new(vec![0; 24], 0))
        .is_ok());
    assert_eq!(
        yuv420p10
            .decode_packet(&Packet::new(vec![0; 23], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv420p14 = RawVideoDecoder::new(4, 2, PixelFormat::Yuv420p14Le).unwrap();
    assert!(yuv420p14
        .decode_packet(&Packet::new(vec![0; 24], 0))
        .is_ok());
    assert_eq!(
        yuv420p14
            .decode_packet(&Packet::new(vec![0; 23], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let yuv444 = RawVideoDecoder::new(3, 2, PixelFormat::Yuv444p).unwrap();
    assert!(yuv444.decode_packet(&Packet::new(vec![0; 18], 0)).is_ok());
    assert_eq!(
        yuv444
            .decode_packet(&Packet::new(vec![0; 17], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    for (format, width, height, valid_len, invalid_len) in [
        (PixelFormat::YuvJ420p, 4, 2, 12, 11),
        (PixelFormat::YuvJ422p, 4, 3, 24, 23),
        (PixelFormat::YuvJ411p, 4, 3, 18, 17),
        (PixelFormat::YuvJ440p, 3, 2, 12, 11),
        (PixelFormat::YuvJ444p, 3, 2, 18, 17),
    ] {
        let decoder = RawVideoDecoder::new(width, height, format).unwrap();
        assert!(decoder
            .decode_packet(&Packet::new(vec![0; valid_len], 0))
            .is_ok());
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; invalid_len], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    let pcm = PcmS16leDecoder::new(48_000, 2).unwrap();
    let mut packet = Packet::new(vec![0, 0, 1, 0, 2, 0, 3, 0], 0);
    packet.set_pts(Some(1024));
    let frame = pcm.decode_packet(&packet).unwrap();
    assert_eq!(frame.pts(), Some(1024));
    assert_eq!(
        pcm.decode_packet(&Packet::new(vec![0, 1, 2], 0))
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
}

fn payload_for_mode(cursor: &mut Cursor<'_>, expected_len: usize, max_len: usize) -> Vec<u8> {
    let len = match cursor.next().unwrap_or_default() % 5 {
        0 => expected_len,
        1 => expected_len.saturating_sub(1),
        2 => expected_len.saturating_add(1).min(max_len),
        3 => usize::from(cursor.next().unwrap_or_default()) % (max_len + 1),
        _ => 0,
    };
    payload_from(cursor, len.min(max_len))
}

fn payload_from(cursor: &mut Cursor<'_>, len: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(len);
    for _ in 0..len {
        payload.push(cursor.next().unwrap_or_default());
    }
    payload
}

fn pixel_format_from(byte: Option<u8>) -> PixelFormat {
    let formats = PixelFormat::ALL;
    formats[usize::from(byte.unwrap_or_default()) % formats.len()]
}

fn dimension_from(byte: Option<u8>, pixel_format: PixelFormat, is_width: bool) -> usize {
    let mut dimension = usize::from(byte.unwrap_or_default() % 8) + 1;
    let (log2_chroma_w, log2_chroma_h) = pixel_format.log2_chroma();
    let divisor = if is_width {
        1_usize << log2_chroma_w
    } else {
        1_usize << log2_chroma_h
    };
    if divisor > 1 && dimension % divisor != 0 {
        dimension += divisor - (dimension % divisor);
    }
    if !is_width && byte.unwrap_or_default() == 0 {
        return usize::from(log2_chroma_h == 0);
    }
    dimension
}

fn sample_rate_from(byte: Option<u8>) -> u32 {
    match byte.unwrap_or_default() % 7 {
        0 => 0,
        1 => 8_000,
        2 => 44_100,
        3 => 48_000,
        4 => 96_000,
        5 => 192_000,
        _ => u32::from(byte.unwrap_or_default()),
    }
}

fn channel_count_from(byte: Option<u8>) -> u16 {
    match byte.unwrap_or_default() % 8 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 6,
        4 => 8,
        _ => u16::from(byte.unwrap_or_default() % 12),
    }
}

fn timestamp_from(mode: Option<u8>, magnitude: Option<u8>) -> Option<i64> {
    let magnitude = i64::from(magnitude.unwrap_or_default());
    match mode.unwrap_or_default() % 5 {
        0 => None,
        1 => Some(0),
        2 => Some(magnitude),
        3 => Some(-magnitude),
        _ => Some(i64::MAX - magnitude),
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.data.get(self.offset).copied();
        self.offset = self.offset.saturating_add(usize::from(byte.is_some()));
        byte
    }
}
