#![no_main]

use avutil::{
    adler32, crc32_ieee, rescale_q, rescale_q_rnd, rescale_q_rnd_pass_minmax, Adler32, AudioFrame,
    AvError, AvErrorKind, Channel, ChannelLayout, Crc32, Frame, FrameData, Packet, PacketFlags,
    PixelFormat, Rational, Rounding, SampleFormat, SideData, VideoFrame,
};
use libfuzzer_sys::fuzz_target;
use std::io;

const MAX_PAYLOAD: usize = 128;
const MAX_SAMPLES: usize = 64;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);

    exercise_errors(&mut cursor);
    exercise_rational_and_timebase(&mut cursor);
    exercise_pixel_and_video_frame(&mut cursor);
    exercise_sample_channel_and_audio_frame(&mut cursor);
    exercise_packet_and_hashes(&mut cursor);
    exercise_fixtures();
});

fn exercise_errors(cursor: &mut Cursor<'_>) {
    let io_kind = io_error_kind_from(cursor.next());
    let err = AvError::from_io_error("fuzz io", io::Error::new(io_kind, "source failure"));
    assert_eq!(err.kind(), expected_av_error_kind_for_io(io_kind));
    assert_eq!(err.io_kind(), Some(io_kind));
    assert_eq!(err.is_eof(), err.kind() == AvErrorKind::EndOfFile);
    assert!(err.message().contains("fuzz io"));
    assert!(err.message().contains("source failure"));

    for (err, kind) in [
        (
            AvError::invalid_argument("invalid argument"),
            AvErrorKind::InvalidArgument,
        ),
        (
            AvError::invalid_data("invalid data"),
            AvErrorKind::InvalidData,
        ),
        (AvError::not_found("not found"), AvErrorKind::NotFound),
        (AvError::end_of_file("end of file"), AvErrorKind::EndOfFile),
        (
            AvError::unsupported("unsupported"),
            AvErrorKind::Unsupported,
        ),
        (AvError::external("external"), AvErrorKind::External),
        (AvError::bug("bug"), AvErrorKind::Bug),
    ] {
        assert_eq!(err.kind(), kind);
        assert_eq!(err.io_kind(), None);
        assert_eq!(err.is_eof(), kind == AvErrorKind::EndOfFile);
        assert!(!err.message().is_empty());
    }
}

fn exercise_rational_and_timebase(cursor: &mut Cursor<'_>) {
    let numerator = small_i32_from(cursor.next());
    let denominator = small_i32_from(cursor.next());
    let Ok(rational) = Rational::new(numerator, denominator) else {
        assert_eq!(denominator, 0);
        return;
    };

    assert!(rational.den() > 0);
    if numerator == 0 {
        assert_eq!(rational, Rational::ZERO);
        assert!(rational.reciprocal().is_err());
    } else {
        let reciprocal = rational.reciprocal().unwrap();
        assert_eq!(rational.checked_mul(reciprocal).unwrap(), Rational::ONE);
    }

    let other = positive_rational_from(cursor.next(), cursor.next());
    assert_eq!(
        rational
            .checked_add(other)
            .unwrap()
            .checked_sub(other)
            .unwrap(),
        rational
    );
    assert_eq!(
        rational
            .checked_sub(other)
            .unwrap()
            .checked_add(other)
            .unwrap(),
        rational
    );
    assert_eq!(
        rational.checked_neg().unwrap().checked_neg().unwrap(),
        rational
    );
    assert_eq!(
        rational
            .checked_div(other)
            .unwrap()
            .checked_mul(other)
            .unwrap(),
        rational
    );

    let src = positive_rational_from(cursor.next(), cursor.next());
    let dst = positive_rational_from(cursor.next(), cursor.next());
    let value = small_i64_from(cursor.next(), cursor.next());
    assert_eq!(
        rescale_q(value, src, dst).unwrap(),
        expected_rescale(value, src, dst, Rounding::NearInf).unwrap()
    );
    for rounding in [
        Rounding::Zero,
        Rounding::Inf,
        Rounding::Down,
        Rounding::Up,
        Rounding::NearInf,
    ] {
        assert_eq!(
            rescale_q_rnd(value, src, dst, rounding).unwrap(),
            expected_rescale(value, src, dst, rounding).unwrap()
        );
    }
    for sentinel in [i64::MIN, i64::MAX] {
        assert_eq!(
            rescale_q_rnd_pass_minmax(sentinel, src, dst, Rounding::NearInf).unwrap(),
            sentinel
        );
    }
    assert_eq!(
        rescale_q_rnd_pass_minmax(value, src, dst, Rounding::NearInf).unwrap(),
        expected_rescale(value, src, dst, Rounding::NearInf).unwrap()
    );
}

fn exercise_pixel_and_video_frame(cursor: &mut Cursor<'_>) {
    let pixel_format = pixel_format_from(cursor.next());
    let width = dimension_from(cursor.next());
    let height = dimension_from(cursor.next());

    let Ok(plane_sizes) = pixel_format.plane_sizes(width, height) else {
        assert!(width == 0 || height == 0 || pixel_format == PixelFormat::Yuv420p);
        return;
    };
    let frame_size = pixel_format.frame_size(width, height).unwrap();
    assert_eq!(frame_size, plane_sizes.iter().sum::<usize>());
    assert_eq!(pixel_format.plane_count(), plane_sizes.len());
    assert_eq!(pixel_format.is_planar(), pixel_format.plane_count() > 1);

    let payload = payload_from(cursor, frame_size);
    let planes = pixel_format.split_planes(&payload, width, height).unwrap();
    assert_eq!(planes.len(), plane_sizes.len());
    assert_eq!(planes.iter().map(Vec::len).collect::<Vec<_>>(), plane_sizes);
    assert_eq!(planes.concat(), payload);

    let video = VideoFrame::new(width, height, pixel_format, planes.clone()).unwrap();
    assert_eq!(video.width(), width);
    assert_eq!(video.height(), height);
    assert_eq!(video.pixel_format(), pixel_format);
    assert_eq!(video.pixel_format_name(), pixel_format.name());
    assert_eq!(video.planes(), planes.as_slice());

    let mut frame = Frame::video(video);
    let pts = timestamp_from(cursor.next());
    frame.set_pts(pts);
    assert_eq!(frame.pts(), pts);
    assert!(matches!(frame.data(), FrameData::Video(_)));

    if frame_size > 0 {
        let err = pixel_format
            .split_planes(&payload[..frame_size - 1], width, height)
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
    }
}

fn exercise_sample_channel_and_audio_frame(cursor: &mut Cursor<'_>) {
    let sample_rate = sample_rate_from(cursor.next());
    let channels = channel_count_from(cursor.next());
    let samples_per_channel = usize::from(cursor.next().unwrap_or_default()) % (MAX_SAMPLES + 1);
    let sample_format = SampleFormat::S16;

    assert_eq!(
        SampleFormat::from_name(sample_format.name()),
        Some(sample_format)
    );
    assert!(!sample_format.is_planar());

    let Ok(plane_sizes) = sample_format.plane_sizes(samples_per_channel, channels) else {
        assert_eq!(channels, 0);
        assert!(AudioFrame::new(sample_rate, channels, sample_format, 1, vec![vec![0]]).is_err());
        return;
    };
    assert_eq!(
        sample_format.plane_count(channels).unwrap(),
        plane_sizes.len()
    );
    assert_eq!(sample_format.bytes_per_sample(), 2);
    assert_eq!(
        sample_format.bytes_per_sample_frame(channels).unwrap(),
        usize::from(channels) * sample_format.bytes_per_sample()
    );

    if sample_rate == 0 {
        assert!(AudioFrame::new(sample_rate, channels, sample_format, 1, vec![vec![0]]).is_err());
        return;
    }

    let planes = plane_sizes
        .iter()
        .map(|size| payload_from(cursor, *size))
        .collect::<Vec<_>>();
    let audio = AudioFrame::new(
        sample_rate,
        channels,
        sample_format,
        samples_per_channel,
        planes.clone(),
    )
    .unwrap();
    assert_eq!(audio.sample_rate(), sample_rate);
    assert_eq!(audio.channels(), channels);
    assert_eq!(audio.sample_format(), sample_format);
    assert_eq!(audio.sample_format_name(), sample_format.name());
    assert_eq!(audio.samples_per_channel(), samples_per_channel);
    assert_eq!(audio.planes(), planes.as_slice());
    assert_eq!(
        audio.channel_layout(),
        ChannelLayout::default_for_count(channels)
    );

    let mut frame = Frame::audio(audio);
    frame.set_pts(timestamp_from(cursor.next()));
    assert!(matches!(frame.data(), FrameData::Audio(_)));

    let layout = channel_layout_from(cursor.next());
    if layout.channel_count() == channels {
        let frame = AudioFrame::new_with_channel_layout(
            sample_rate,
            layout,
            sample_format,
            samples_per_channel,
            planes,
        )
        .unwrap();
        assert_eq!(frame.channel_layout(), Some(layout));
    } else {
        assert_eq!(
            layout.validate_channel_count(channels).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
    }
}

fn exercise_packet_and_hashes(cursor: &mut Cursor<'_>) {
    let payload_len = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD + 1);
    let payload = payload_from(cursor, payload_len);
    let stream_index = usize::from(cursor.next().unwrap_or_default() % 4);
    let mut packet = Packet::new(payload.clone(), stream_index);
    assert_eq!(packet.data(), payload.as_slice());
    assert_eq!(packet.stream_index(), stream_index);
    assert_eq!(packet.pts(), None);
    assert_eq!(packet.dts(), None);
    assert_eq!(packet.duration(), 0);

    let pts = timestamp_from(cursor.next());
    let dts = timestamp_from(cursor.next());
    packet.set_pts(pts);
    packet.set_dts(dts);
    assert_eq!(packet.pts(), pts);
    assert_eq!(packet.dts(), dts);

    let duration = i64::from(cursor.next().unwrap_or_default());
    packet.set_duration(duration).unwrap();
    assert_eq!(packet.duration(), duration);
    assert_eq!(
        packet.set_duration(-1).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(packet.duration(), duration);

    packet.set_key(cursor.next().unwrap_or_default().is_multiple_of(2));
    if packet.flags().contains(PacketFlags::KEY) {
        assert_ne!(packet.flags().bits() & PacketFlags::KEY.bits(), 0);
    }

    let side_data_len = usize::from(cursor.next().unwrap_or_default() % 16);
    let side_data_payload = payload_from(cursor, side_data_len);
    let side_data = SideData::new("fuzz_side_data", side_data_payload.clone()).unwrap();
    packet.push_side_data(side_data);
    assert_eq!(packet.side_data()[0].kind(), "fuzz_side_data");
    assert_eq!(packet.side_data()[0].data(), side_data_payload.as_slice());
    assert_eq!(
        SideData::new(" \t", Vec::new()).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );

    let split = usize::from(cursor.next().unwrap_or_default()) % (payload.len() + 1);
    let mut adler = Adler32::new();
    adler.update(&payload[..split]);
    adler.update(&payload[split..]);
    assert_eq!(adler.finalize(), adler32(&payload));

    let mut crc = Crc32::new();
    crc.update(&payload[..split]);
    crc.update(&payload[split..]);
    assert_eq!(crc.finalize(), crc32_ieee(&payload));
}

fn exercise_fixtures() {
    assert_eq!(PixelFormat::from_name("gray8"), Some(PixelFormat::Gray8));
    assert_eq!(
        PixelFormat::Yuv420p.plane_sizes(2, 2).unwrap(),
        vec![4, 1, 1]
    );
    assert_eq!(SampleFormat::S16.plane_sizes(2, 2).unwrap(), vec![8]);
    assert!(ChannelLayout::stereo().contains(Channel::FrontLeft));
    assert!(!ChannelLayout::stereo().contains(Channel::LowFrequency));

    let video = VideoFrame::new(1, 1, PixelFormat::Rgb24, vec![vec![1, 2, 3]]).unwrap();
    assert_eq!(Frame::video(video).pts(), None);

    let audio = AudioFrame::new_with_channel_layout(
        48_000,
        ChannelLayout::stereo(),
        SampleFormat::S16,
        1,
        vec![vec![0, 0, 1, 0]],
    )
    .unwrap();
    assert_eq!(audio.channel_layout(), Some(ChannelLayout::stereo()));
}

fn expected_rescale(
    value: i64,
    src: Rational,
    dst: Rational,
    rounding: Rounding,
) -> Result<i64, ()> {
    let numerator = i128::from(value) * i128::from(src.num()) * i128::from(dst.den());
    let denominator = i128::from(src.den()) * i128::from(dst.num());
    i64::try_from(div_round(numerator, denominator, rounding)).map_err(|_| ())
}

fn div_round(numerator: i128, denominator: i128, rounding: Rounding) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return quotient;
    }

    let same_sign = (numerator >= 0) == (denominator >= 0);
    match rounding {
        Rounding::Zero => quotient,
        Rounding::Inf => {
            if same_sign {
                quotient + 1
            } else {
                quotient - 1
            }
        }
        Rounding::Down => {
            if same_sign {
                quotient
            } else {
                quotient - 1
            }
        }
        Rounding::Up => {
            if same_sign {
                quotient + 1
            } else {
                quotient
            }
        }
        Rounding::NearInf => {
            if remainder.abs() * 2 >= denominator.abs() {
                if same_sign {
                    quotient + 1
                } else {
                    quotient - 1
                }
            } else {
                quotient
            }
        }
    }
}

fn pixel_format_from(byte: Option<u8>) -> PixelFormat {
    match byte.unwrap_or_default() % 4 {
        0 => PixelFormat::Gray8,
        1 => PixelFormat::Rgb24,
        2 => PixelFormat::Rgba,
        _ => PixelFormat::Yuv420p,
    }
}

fn channel_layout_from(byte: Option<u8>) -> ChannelLayout {
    match byte.unwrap_or_default() % 6 {
        0 => ChannelLayout::mono(),
        1 => ChannelLayout::stereo(),
        2 => ChannelLayout::quad(),
        3 => ChannelLayout::five_one(),
        4 => ChannelLayout::five_one_side(),
        _ => ChannelLayout::seven_one(),
    }
}

fn dimension_from(byte: Option<u8>) -> usize {
    usize::from(byte.unwrap_or_default() % 9)
}

fn sample_rate_from(byte: Option<u8>) -> u32 {
    match byte.unwrap_or_default() % 5 {
        0 => 0,
        1 => 8_000,
        2 => 44_100,
        3 => 48_000,
        _ => 96_000,
    }
}

fn channel_count_from(byte: Option<u8>) -> u16 {
    match byte.unwrap_or_default() % 7 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 4,
        4 => 6,
        5 => 8,
        _ => u16::from(byte.unwrap_or_default() % 12),
    }
}

fn timestamp_from(byte: Option<u8>) -> Option<i64> {
    match byte.unwrap_or_default() % 4 {
        0 => None,
        1 => Some(0),
        2 => Some(i64::from(byte.unwrap_or_default())),
        _ => Some(-i64::from(byte.unwrap_or_default())),
    }
}

fn positive_rational_from(num: Option<u8>, den: Option<u8>) -> Rational {
    Rational::new(
        i32::from(num.unwrap_or_default() % 31) + 1,
        i32::from(den.unwrap_or_default() % 31) + 1,
    )
    .unwrap()
}

fn small_i32_from(byte: Option<u8>) -> i32 {
    i32::from(byte.unwrap_or_default()) - 128
}

fn small_i64_from(first: Option<u8>, second: Option<u8>) -> i64 {
    let high = i64::from(first.unwrap_or_default()) - 128;
    let low = i64::from(second.unwrap_or_default());
    high * 256 + low
}

fn io_error_kind_from(byte: Option<u8>) -> io::ErrorKind {
    match byte.unwrap_or_default() % 6 {
        0 => io::ErrorKind::NotFound,
        1 => io::ErrorKind::UnexpectedEof,
        2 => io::ErrorKind::InvalidData,
        3 => io::ErrorKind::InvalidInput,
        4 => io::ErrorKind::Unsupported,
        _ => io::ErrorKind::PermissionDenied,
    }
}

fn expected_av_error_kind_for_io(kind: io::ErrorKind) -> AvErrorKind {
    match kind {
        io::ErrorKind::NotFound => AvErrorKind::NotFound,
        io::ErrorKind::UnexpectedEof => AvErrorKind::EndOfFile,
        io::ErrorKind::InvalidData => AvErrorKind::InvalidData,
        io::ErrorKind::InvalidInput => AvErrorKind::InvalidArgument,
        io::ErrorKind::Unsupported => AvErrorKind::Unsupported,
        _ => AvErrorKind::External,
    }
}

fn payload_from(cursor: &mut Cursor<'_>, len: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(len);
    for _ in 0..len {
        payload.push(cursor.next().unwrap_or_default());
    }
    payload
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
