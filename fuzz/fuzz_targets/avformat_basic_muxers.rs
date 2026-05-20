#![no_main]

use avformat::{
    PcmS16leDemuxer, PcmS16leMuxer, RawVideoDemuxer, RawVideoMuxer, WavDemuxer, WavMuxer,
    Yuv4MpegDemuxer, Yuv4MpegMuxer,
};
use avutil::{AvErrorKind, Packet, PixelFormat, Rational, SampleFormat};
use libfuzzer_sys::fuzz_target;

const MAX_PACKETS: usize = 6;
const MAX_AUDIO_PAYLOAD: usize = 96;
const MAX_VIDEO_PAYLOAD: usize = 384;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);

    exercise_pcm_muxer(&mut cursor);
    exercise_wav_muxer(&mut cursor);
    exercise_rawvideo_muxer(&mut cursor);
    exercise_yuv4mpeg_muxer(&mut cursor);
    exercise_fixtures();
});

fn exercise_pcm_muxer(cursor: &mut Cursor<'_>) {
    let sample_rate = sample_rate_from(cursor.next());
    let channels = channel_count_from(cursor.next());
    let Ok(mut muxer) = PcmS16leMuxer::new(sample_rate, channels) else {
        return;
    };

    let frame_size = muxer.info().bytes_per_sample_frame();
    let packets = audio_packets_from(cursor, frame_size);
    let mut expected = Vec::new();
    let mut expected_packets = 0_u64;
    let mut expected_samples = 0_usize;

    for packet in &packets {
        let before = pcm_state(&muxer);
        let result = muxer.write_packet(packet);
        if packet.stream_index() == 0 && packet.data().len().is_multiple_of(frame_size) {
            result.unwrap();
            expected.extend_from_slice(packet.data());
            expected_packets += 1;
            expected_samples += packet.data().len() / frame_size;
            assert_eq!(muxer.packets(), expected_packets);
            assert_eq!(muxer.data_len(), expected.len());
            assert_eq!(muxer.info().samples_per_channel(), expected_samples);
        } else {
            assert!(result.is_err());
            assert_eq!(pcm_state(&muxer), before);
        }
    }

    assert_eq!(muxer.info().sample_rate(), sample_rate);
    assert_eq!(muxer.info().channels(), channels);
    assert_eq!(muxer.info().sample_format(), SampleFormat::S16);
    assert_eq!(muxer.render(), expected);
    let finished = muxer.finish();
    assert!(muxer.is_finished());
    assert_eq!(finished, expected);
    let err = muxer
        .write_packet(&Packet::new(vec![0; frame_size], 0))
        .unwrap_err();
    assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    assert_eq!(muxer.render(), expected);

    let mut demuxer = PcmS16leDemuxer::open(&finished, sample_rate, channels, 3).unwrap();
    let mut roundtrip = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        roundtrip.extend_from_slice(packet.data());
    }
    assert_eq!(roundtrip, expected);
}

fn exercise_wav_muxer(cursor: &mut Cursor<'_>) {
    let channels = channel_count_from(cursor.next());
    let sample_rate = sample_rate_from(cursor.next());
    let Ok(mut muxer) = WavMuxer::new_pcm_s16le(channels, sample_rate) else {
        return;
    };

    let block_align = usize::from(muxer.info().block_align());
    let packets = audio_packets_from(cursor, block_align);
    let mut expected = Vec::new();
    let mut expected_packets = 0_u64;

    for packet in &packets {
        let before = wav_state(&muxer);
        let result = muxer.write_packet(packet);
        if packet.stream_index() == 0 && packet.data().len().is_multiple_of(block_align) {
            result.unwrap();
            expected.extend_from_slice(packet.data());
            expected_packets += 1;
            assert_eq!(muxer.packets(), expected_packets);
            assert_eq!(muxer.data_len(), expected.len());
            assert_eq!(muxer.info().data_size(), expected.len());
        } else {
            assert!(result.is_err());
            assert_eq!(wav_state(&muxer), before);
        }
    }

    let rendered = muxer.render().unwrap();
    assert_wav_roundtrip(&rendered, channels, sample_rate, &expected);
    let finished = muxer.finish().unwrap();
    assert!(muxer.is_finished());
    assert_eq!(finished, rendered);
    let err = muxer
        .write_packet(&Packet::new(vec![0; block_align], 0))
        .unwrap_err();
    assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    assert_eq!(muxer.render().unwrap(), rendered);
}

fn exercise_rawvideo_muxer(cursor: &mut Cursor<'_>) {
    let pixel_format = pixel_format_from(cursor.next());
    let width = video_dimension_from(cursor.next(), pixel_format, true);
    let height = video_dimension_from(cursor.next(), pixel_format, false);
    let frame_rate = positive_rate_from(cursor.next());
    let Ok(mut muxer) = RawVideoMuxer::new(width, height, pixel_format, frame_rate) else {
        return;
    };

    let frame_size = muxer.info().frame_size();
    let packets = video_packets_from(cursor, frame_size);
    let mut expected = Vec::new();
    let mut expected_frames = 0_usize;

    for packet in &packets {
        let before = rawvideo_state(&muxer);
        let result = muxer.write_packet(packet);
        if packet.stream_index() == 0 && packet.data().len() == frame_size {
            result.unwrap();
            expected.extend_from_slice(packet.data());
            expected_frames += 1;
            assert_eq!(muxer.info().frame_count(), expected_frames);
            assert_eq!(muxer.data_len(), expected.len());
        } else {
            assert!(result.is_err());
            assert_eq!(rawvideo_state(&muxer), before);
        }
    }

    assert_eq!(muxer.info().width(), width);
    assert_eq!(muxer.info().height(), height);
    assert_eq!(muxer.info().pixel_format(), pixel_format);
    assert_eq!(muxer.info().frame_rate(), frame_rate);
    assert_eq!(muxer.render(), expected);
    let finished = muxer.finish();
    assert!(muxer.is_finished());
    assert_eq!(finished, expected);
    let err = muxer
        .write_packet(&Packet::new(vec![0; frame_size], 0))
        .unwrap_err();
    assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    assert_eq!(muxer.render(), expected);

    let mut demuxer =
        RawVideoDemuxer::open(&finished, width, height, pixel_format, frame_rate).unwrap();
    let mut roundtrip = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        roundtrip.extend_from_slice(packet.data());
    }
    assert_eq!(roundtrip, expected);
}

fn exercise_yuv4mpeg_muxer(cursor: &mut Cursor<'_>) {
    let width = even_u32_from(cursor.next());
    let height = even_u32_from(cursor.next());
    let frame_rate = positive_rate_from(cursor.next());
    let sample_aspect_ratio = sample_aspect_ratio_from(cursor.next());
    let Ok(mut muxer) = Yuv4MpegMuxer::new(width, height, frame_rate, sample_aspect_ratio) else {
        return;
    };

    let frame_size = muxer.info().frame_size();
    let packets = video_packets_from(cursor, frame_size);
    let mut expected = Vec::new();
    let mut expected_frames = 0_usize;

    for packet in &packets {
        let before = y4m_state(&muxer);
        let result = muxer.write_packet(packet);
        if packet.stream_index() == 0 && packet.data().len() == frame_size {
            result.unwrap();
            expected.extend_from_slice(packet.data());
            expected_frames += 1;
            assert_eq!(muxer.frame_count(), expected_frames);
            assert_eq!(muxer.payload_len(), expected.len());
        } else {
            assert!(result.is_err());
            assert_eq!(y4m_state(&muxer), before);
        }
    }

    assert_eq!(muxer.info().width(), width);
    assert_eq!(muxer.info().height(), height);
    assert_eq!(muxer.info().pixel_format(), PixelFormat::Yuv420p);
    assert_eq!(muxer.info().frame_rate(), frame_rate);
    assert_eq!(muxer.info().sample_aspect_ratio(), sample_aspect_ratio);
    let rendered = muxer.render();
    assert_y4m_roundtrip(&rendered, &expected, sample_aspect_ratio);
    let finished = muxer.finish();
    assert!(muxer.is_finished());
    assert_eq!(finished, rendered);
    let err = muxer
        .write_packet(&Packet::new(vec![0; frame_size], 0))
        .unwrap_err();
    assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    assert_eq!(muxer.render(), rendered);
}

fn exercise_fixtures() {
    let mut pcm = PcmS16leMuxer::new(48_000, 2).unwrap();
    pcm.write_packet(&Packet::new(vec![0, 0, 1, 0], 0)).unwrap();
    assert_eq!(pcm.finish(), vec![0, 0, 1, 0]);

    let mut wav = WavMuxer::new_pcm_s16le(1, 44_100).unwrap();
    wav.write_packet(&Packet::new(vec![1, 0, 2, 0], 0)).unwrap();
    assert_wav_roundtrip(&wav.finish().unwrap(), 1, 44_100, &[1, 0, 2, 0]);

    let mut raw =
        RawVideoMuxer::new(2, 2, PixelFormat::Gray8, Rational::new(24, 1).unwrap()).unwrap();
    raw.write_packet(&Packet::new(vec![0, 1, 2, 3], 0)).unwrap();
    assert_eq!(raw.finish(), vec![0, 1, 2, 3]);

    let mut raw_gray16 = RawVideoMuxer::new(
        2,
        1,
        PixelFormat::Gray16Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    raw_gray16
        .write_packet(&Packet::new((0..4).collect(), 0))
        .unwrap();
    let raw_gray16_output = raw_gray16.finish();
    let mut raw_gray16_demuxer = RawVideoDemuxer::open(
        &raw_gray16_output,
        2,
        1,
        PixelFormat::Gray16Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gray16_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..4).collect::<Vec<_>>()
    );
    assert!(raw_gray16_demuxer.read_packet().unwrap().is_none());

    let mut raw_gray10 = RawVideoMuxer::new(
        2,
        1,
        PixelFormat::Gray10Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    raw_gray10
        .write_packet(&Packet::new((0..4).collect(), 0))
        .unwrap();
    let raw_gray10_output = raw_gray10.finish();
    let mut raw_gray10_demuxer = RawVideoDemuxer::open(
        &raw_gray10_output,
        2,
        1,
        PixelFormat::Gray10Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gray10_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..4).collect::<Vec<_>>()
    );
    assert!(raw_gray10_demuxer.read_packet().unwrap().is_none());

    let mut raw_gray32 = RawVideoMuxer::new(
        2,
        1,
        PixelFormat::Gray32Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    raw_gray32
        .write_packet(&Packet::new((0..8).collect(), 0))
        .unwrap();
    let raw_gray32_output = raw_gray32.finish();
    let mut raw_gray32_demuxer = RawVideoDemuxer::open(
        &raw_gray32_output,
        2,
        1,
        PixelFormat::Gray32Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gray32_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..8).collect::<Vec<_>>()
    );
    assert!(raw_gray32_demuxer.read_packet().unwrap().is_none());

    let mut raw_grayf16 = RawVideoMuxer::new(
        2,
        1,
        PixelFormat::GrayF16Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    raw_grayf16
        .write_packet(&Packet::new((0..4).collect(), 0))
        .unwrap();
    let raw_grayf16_output = raw_grayf16.finish();
    let mut raw_grayf16_demuxer = RawVideoDemuxer::open(
        &raw_grayf16_output,
        2,
        1,
        PixelFormat::GrayF16Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_grayf16_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..4).collect::<Vec<_>>()
    );
    assert!(raw_grayf16_demuxer.read_packet().unwrap().is_none());

    let mut raw_grayf32 = RawVideoMuxer::new(
        2,
        1,
        PixelFormat::GrayF32Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    raw_grayf32
        .write_packet(&Packet::new((0..8).collect(), 0))
        .unwrap();
    let raw_grayf32_output = raw_grayf32.finish();
    let mut raw_grayf32_demuxer = RawVideoDemuxer::open(
        &raw_grayf32_output,
        2,
        1,
        PixelFormat::GrayF32Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_grayf32_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..8).collect::<Vec<_>>()
    );
    assert!(raw_grayf32_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrp =
        RawVideoMuxer::new(2, 1, PixelFormat::Gbrp, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrp
        .write_packet(&Packet::new((0..6).collect(), 0))
        .unwrap();
    let raw_gbrp_output = raw_gbrp.finish();
    let mut raw_gbrp_demuxer = RawVideoDemuxer::open(
        &raw_gbrp_output,
        2,
        1,
        PixelFormat::Gbrp,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrp_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..6).collect::<Vec<_>>()
    );
    assert!(raw_gbrp_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrp9 =
        RawVideoMuxer::new(2, 1, PixelFormat::Gbrp9Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrp9
        .write_packet(&Packet::new((0..12).collect(), 0))
        .unwrap();
    let raw_gbrp9_output = raw_gbrp9.finish();
    let mut raw_gbrp9_demuxer = RawVideoDemuxer::open(
        &raw_gbrp9_output,
        2,
        1,
        PixelFormat::Gbrp9Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrp9_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..12).collect::<Vec<_>>()
    );
    assert!(raw_gbrp9_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrp10 =
        RawVideoMuxer::new(2, 1, PixelFormat::Gbrp10Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrp10
        .write_packet(&Packet::new((0..12).collect(), 0))
        .unwrap();
    let raw_gbrp10_output = raw_gbrp10.finish();
    let mut raw_gbrp10_demuxer = RawVideoDemuxer::open(
        &raw_gbrp10_output,
        2,
        1,
        PixelFormat::Gbrp10Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrp10_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..12).collect::<Vec<_>>()
    );
    assert!(raw_gbrp10_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrp12 =
        RawVideoMuxer::new(2, 1, PixelFormat::Gbrp12Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrp12
        .write_packet(&Packet::new((0..12).collect(), 0))
        .unwrap();
    let raw_gbrp12_output = raw_gbrp12.finish();
    let mut raw_gbrp12_demuxer = RawVideoDemuxer::open(
        &raw_gbrp12_output,
        2,
        1,
        PixelFormat::Gbrp12Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrp12_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..12).collect::<Vec<_>>()
    );
    assert!(raw_gbrp12_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrp14 =
        RawVideoMuxer::new(2, 1, PixelFormat::Gbrp14Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrp14
        .write_packet(&Packet::new((0..12).collect(), 0))
        .unwrap();
    let raw_gbrp14_output = raw_gbrp14.finish();
    let mut raw_gbrp14_demuxer = RawVideoDemuxer::open(
        &raw_gbrp14_output,
        2,
        1,
        PixelFormat::Gbrp14Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrp14_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..12).collect::<Vec<_>>()
    );
    assert!(raw_gbrp14_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrp16 =
        RawVideoMuxer::new(2, 1, PixelFormat::Gbrp16Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrp16
        .write_packet(&Packet::new((0..12).collect(), 0))
        .unwrap();
    let raw_gbrp16_output = raw_gbrp16.finish();
    let mut raw_gbrp16_demuxer = RawVideoDemuxer::open(
        &raw_gbrp16_output,
        2,
        1,
        PixelFormat::Gbrp16Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrp16_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..12).collect::<Vec<_>>()
    );
    assert!(raw_gbrp16_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrap =
        RawVideoMuxer::new(2, 1, PixelFormat::Gbrap, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrap
        .write_packet(&Packet::new((0..8).collect(), 0))
        .unwrap();
    let raw_gbrap_output = raw_gbrap.finish();
    let mut raw_gbrap_demuxer = RawVideoDemuxer::open(
        &raw_gbrap_output,
        2,
        1,
        PixelFormat::Gbrap,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrap_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..8).collect::<Vec<_>>()
    );
    assert!(raw_gbrap_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrap16 =
        RawVideoMuxer::new(2, 1, PixelFormat::Gbrap16Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrap16
        .write_packet(&Packet::new((0..16).collect(), 0))
        .unwrap();
    let raw_gbrap16_output = raw_gbrap16.finish();
    let mut raw_gbrap16_demuxer = RawVideoDemuxer::open(
        &raw_gbrap16_output,
        2,
        1,
        PixelFormat::Gbrap16Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrap16_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..16).collect::<Vec<_>>()
    );
    assert!(raw_gbrap16_demuxer.read_packet().unwrap().is_none());

    let mut raw_gbrapf32 =
        RawVideoMuxer::new(1, 1, PixelFormat::GbrapF32Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_gbrapf32
        .write_packet(&Packet::new((0..16).collect(), 0))
        .unwrap();
    let raw_gbrapf32_output = raw_gbrapf32.finish();
    let mut raw_gbrapf32_demuxer = RawVideoDemuxer::open(
        &raw_gbrapf32_output,
        1,
        1,
        PixelFormat::GbrapF32Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_gbrapf32_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..16).collect::<Vec<_>>()
    );
    assert!(raw_gbrapf32_demuxer.read_packet().unwrap().is_none());

    let mut raw_ya8 =
        RawVideoMuxer::new(2, 1, PixelFormat::Ya8, Rational::new(24, 1).unwrap()).unwrap();
    raw_ya8
        .write_packet(&Packet::new(vec![0x10, 0xff, 0x80, 0x40], 0))
        .unwrap();
    let raw_ya8_output = raw_ya8.finish();
    let mut raw_ya8_demuxer = RawVideoDemuxer::open(
        &raw_ya8_output,
        2,
        1,
        PixelFormat::Ya8,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_ya8_demuxer.read_packet().unwrap().unwrap().data(),
        &[0x10, 0xff, 0x80, 0x40]
    );
    assert!(raw_ya8_demuxer.read_packet().unwrap().is_none());

    let mut raw_ya16 =
        RawVideoMuxer::new(2, 1, PixelFormat::Ya16Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_ya16
        .write_packet(&Packet::new((0..8).collect(), 0))
        .unwrap();
    let raw_ya16_output = raw_ya16.finish();
    let mut raw_ya16_demuxer = RawVideoDemuxer::open(
        &raw_ya16_output,
        2,
        1,
        PixelFormat::Ya16Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_ya16_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..8).collect::<Vec<_>>()
    );
    assert!(raw_ya16_demuxer.read_packet().unwrap().is_none());

    let mut raw_rgb0 =
        RawVideoMuxer::new(2, 1, PixelFormat::Rgb0, Rational::new(24, 1).unwrap()).unwrap();
    raw_rgb0
        .write_packet(&Packet::new((0..8).collect(), 0))
        .unwrap();
    let raw_rgb0_output = raw_rgb0.finish();
    let mut raw_rgb0_demuxer = RawVideoDemuxer::open(
        &raw_rgb0_output,
        2,
        1,
        PixelFormat::Rgb0,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_rgb0_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..8).collect::<Vec<_>>()
    );
    assert!(raw_rgb0_demuxer.read_packet().unwrap().is_none());

    let mut raw_rgb565 =
        RawVideoMuxer::new(2, 1, PixelFormat::Rgb565Le, Rational::new(24, 1).unwrap()).unwrap();
    raw_rgb565
        .write_packet(&Packet::new((0..4).collect(), 0))
        .unwrap();
    let raw_rgb565_output = raw_rgb565.finish();
    let mut raw_rgb565_demuxer = RawVideoDemuxer::open(
        &raw_rgb565_output,
        2,
        1,
        PixelFormat::Rgb565Le,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_rgb565_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..4).collect::<Vec<_>>()
    );
    assert!(raw_rgb565_demuxer.read_packet().unwrap().is_none());

    let mut raw_yuyv422 =
        RawVideoMuxer::new(2, 1, PixelFormat::Yuyv422, Rational::new(24, 1).unwrap()).unwrap();
    raw_yuyv422
        .write_packet(&Packet::new((0..4).collect(), 0))
        .unwrap();
    let raw_yuyv422_output = raw_yuyv422.finish();
    let mut raw_yuyv422_demuxer = RawVideoDemuxer::open(
        &raw_yuyv422_output,
        2,
        1,
        PixelFormat::Yuyv422,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_yuyv422_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..4).collect::<Vec<_>>()
    );
    assert!(raw_yuyv422_demuxer.read_packet().unwrap().is_none());

    let mut raw_nv12 =
        RawVideoMuxer::new(4, 2, PixelFormat::Nv12, Rational::new(24, 1).unwrap()).unwrap();
    raw_nv12
        .write_packet(&Packet::new((0..12).collect(), 0))
        .unwrap();
    let raw_nv12_output = raw_nv12.finish();
    let mut raw_nv12_demuxer = RawVideoDemuxer::open(
        &raw_nv12_output,
        4,
        2,
        PixelFormat::Nv12,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_nv12_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..12).collect::<Vec<_>>()
    );
    assert!(raw_nv12_demuxer.read_packet().unwrap().is_none());

    let mut raw_bgr444 =
        RawVideoMuxer::new(2, 1, PixelFormat::Bgr444Be, Rational::new(24, 1).unwrap()).unwrap();
    raw_bgr444
        .write_packet(&Packet::new((0..4).collect(), 0))
        .unwrap();
    let raw_bgr444_output = raw_bgr444.finish();
    let mut raw_bgr444_demuxer = RawVideoDemuxer::open(
        &raw_bgr444_output,
        2,
        1,
        PixelFormat::Bgr444Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_bgr444_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..4).collect::<Vec<_>>()
    );
    assert!(raw_bgr444_demuxer.read_packet().unwrap().is_none());

    let mut raw_rgb48 =
        RawVideoMuxer::new(2, 1, PixelFormat::Bgr48Be, Rational::new(24, 1).unwrap()).unwrap();
    raw_rgb48
        .write_packet(&Packet::new((0..12).collect(), 0))
        .unwrap();
    let raw_rgb48_output = raw_rgb48.finish();
    let mut raw_rgb48_demuxer = RawVideoDemuxer::open(
        &raw_rgb48_output,
        2,
        1,
        PixelFormat::Bgr48Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_rgb48_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..12).collect::<Vec<_>>()
    );
    assert!(raw_rgb48_demuxer.read_packet().unwrap().is_none());

    let mut raw_rgba64 =
        RawVideoMuxer::new(1, 1, PixelFormat::Bgra64Be, Rational::new(24, 1).unwrap()).unwrap();
    raw_rgba64
        .write_packet(&Packet::new((0..8).collect(), 0))
        .unwrap();
    let raw_rgba64_output = raw_rgba64.finish();
    let mut raw_rgba64_demuxer = RawVideoDemuxer::open(
        &raw_rgba64_output,
        1,
        1,
        PixelFormat::Bgra64Be,
        Rational::new(24, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_rgba64_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..8).collect::<Vec<_>>()
    );
    assert!(raw_rgba64_demuxer.read_packet().unwrap().is_none());

    let mut raw_yuv411 =
        RawVideoMuxer::new(4, 3, PixelFormat::Yuv411p, Rational::new(25, 1).unwrap()).unwrap();
    raw_yuv411
        .write_packet(&Packet::new((0..18).collect(), 0))
        .unwrap();
    let raw_yuv411_output = raw_yuv411.finish();
    let mut raw_yuv411_demuxer = RawVideoDemuxer::open(
        &raw_yuv411_output,
        4,
        3,
        PixelFormat::Yuv411p,
        Rational::new(25, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_yuv411_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..18).collect::<Vec<_>>()
    );
    assert!(raw_yuv411_demuxer.read_packet().unwrap().is_none());

    let mut raw_yuv410 =
        RawVideoMuxer::new(4, 4, PixelFormat::Yuv410p, Rational::new(25, 1).unwrap()).unwrap();
    raw_yuv410
        .write_packet(&Packet::new((0..18).collect(), 0))
        .unwrap();
    let raw_yuv410_output = raw_yuv410.finish();
    let mut raw_yuv410_demuxer = RawVideoDemuxer::open(
        &raw_yuv410_output,
        4,
        4,
        PixelFormat::Yuv410p,
        Rational::new(25, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_yuv410_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..18).collect::<Vec<_>>()
    );
    assert!(raw_yuv410_demuxer.read_packet().unwrap().is_none());

    let mut raw_yuv440 =
        RawVideoMuxer::new(3, 2, PixelFormat::Yuv440p, Rational::new(25, 1).unwrap()).unwrap();
    raw_yuv440
        .write_packet(&Packet::new((0..12).collect(), 0))
        .unwrap();
    let raw_yuv440_output = raw_yuv440.finish();
    let mut raw_yuv440_demuxer = RawVideoDemuxer::open(
        &raw_yuv440_output,
        3,
        2,
        PixelFormat::Yuv440p,
        Rational::new(25, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw_yuv440_demuxer.read_packet().unwrap().unwrap().data(),
        &(0..12).collect::<Vec<_>>()
    );
    assert!(raw_yuv440_demuxer.read_packet().unwrap().is_none());

    let mut y4m = Yuv4MpegMuxer::new(2, 2, Rational::new(25, 1).unwrap(), None).unwrap();
    y4m.write_packet(&Packet::new(vec![0, 1, 2, 3, 4, 5], 0))
        .unwrap();
    assert_y4m_roundtrip(&y4m.finish(), &[0, 1, 2, 3, 4, 5], None);
}

fn assert_wav_roundtrip(bytes: &[u8], channels: u16, sample_rate: u32, expected: &[u8]) {
    let mut demuxer = WavDemuxer::open(bytes).unwrap();
    assert_eq!(demuxer.info().channels(), channels);
    assert_eq!(demuxer.info().sample_rate(), sample_rate);
    assert_eq!(demuxer.info().sample_format(), SampleFormat::S16);
    assert_eq!(demuxer.info().bits_per_sample(), 16);
    assert_eq!(demuxer.info().data_size(), expected.len());
    let packet = demuxer.read_packet().unwrap().unwrap();
    assert_eq!(packet.data(), expected);
    assert_eq!(
        packet.duration() as usize,
        expected.len() / usize::from(demuxer.info().block_align())
    );
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn assert_y4m_roundtrip(bytes: &[u8], expected: &[u8], sample_aspect_ratio: Option<Rational>) {
    let mut demuxer = Yuv4MpegDemuxer::open(bytes).unwrap();
    assert_eq!(demuxer.info().sample_aspect_ratio(), sample_aspect_ratio);
    let mut roundtrip = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        roundtrip.extend_from_slice(packet.data());
    }
    assert_eq!(roundtrip, expected);
}

fn audio_packets_from(cursor: &mut Cursor<'_>, frame_size: usize) -> Vec<Packet> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_PACKETS + 1);
    let mut packets = Vec::with_capacity(count);
    for _ in 0..count {
        let stream_index = stream_index_from(cursor.next());
        let len = payload_len_for_mode(cursor, frame_size, MAX_AUDIO_PAYLOAD);
        packets.push(Packet::new(payload_from(cursor, len), stream_index));
    }
    packets
}

fn video_packets_from(cursor: &mut Cursor<'_>, frame_size: usize) -> Vec<Packet> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_PACKETS + 1);
    let mut packets = Vec::with_capacity(count);
    for _ in 0..count {
        let stream_index = stream_index_from(cursor.next());
        let len = payload_len_for_mode(cursor, frame_size, MAX_VIDEO_PAYLOAD);
        packets.push(Packet::new(payload_from(cursor, len), stream_index));
    }
    packets
}

fn payload_len_for_mode(cursor: &mut Cursor<'_>, valid_len: usize, max_len: usize) -> usize {
    match cursor.next().unwrap_or_default() % 5 {
        0 => valid_len.min(max_len),
        1 => valid_len.saturating_sub(1).min(max_len),
        2 => valid_len.saturating_add(1).min(max_len),
        3 => usize::from(cursor.next().unwrap_or_default()) % (max_len + 1),
        _ => 0,
    }
}

fn payload_from(cursor: &mut Cursor<'_>, len: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(len);
    for _ in 0..len {
        payload.push(cursor.next().unwrap_or_default());
    }
    payload
}

fn stream_index_from(byte: Option<u8>) -> usize {
    usize::from(byte.unwrap_or_default().is_multiple_of(5))
}

fn sample_rate_from(byte: Option<u8>) -> u32 {
    match byte.unwrap_or_default() % 6 {
        0 => 0,
        1 => 8_000,
        2 => 44_100,
        3 => 48_000,
        4 => 96_000,
        _ => u32::from(byte.unwrap_or_default()) + 1,
    }
}

fn channel_count_from(byte: Option<u8>) -> u16 {
    match byte.unwrap_or_default() % 7 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 6,
        4 => 8,
        _ => u16::from(byte.unwrap_or_default() % 12),
    }
}

fn pixel_format_from(byte: Option<u8>) -> PixelFormat {
    let formats = PixelFormat::ALL;
    formats[usize::from(byte.unwrap_or_default()) % formats.len()]
}

fn video_dimension_from(byte: Option<u8>, pixel_format: PixelFormat, is_width: bool) -> usize {
    let mut value = usize::from(byte.unwrap_or_default() % 8) + 1;
    let (log2_chroma_w, log2_chroma_h) = pixel_format.log2_chroma();
    let divisor = if is_width {
        1_usize << log2_chroma_w
    } else {
        1_usize << log2_chroma_h
    };
    if divisor > 1 && value % divisor != 0 {
        value += divisor - (value % divisor);
    }
    value
}

fn even_u32_from(byte: Option<u8>) -> u32 {
    let value = u32::from(byte.unwrap_or_default() % 4) + 1;
    value * 2
}

fn positive_rate_from(byte: Option<u8>) -> Rational {
    match byte.unwrap_or_default() % 4 {
        0 => Rational::ONE,
        1 => Rational::new(24, 1).unwrap(),
        2 => Rational::new(30000, 1001).unwrap(),
        _ => Rational::new(60, 1).unwrap(),
    }
}

fn sample_aspect_ratio_from(byte: Option<u8>) -> Option<Rational> {
    match byte.unwrap_or_default() % 4 {
        0 => None,
        1 => Some(Rational::ONE),
        2 => Some(Rational::new(4, 3).unwrap()),
        _ => Some(Rational::new(16, 15).unwrap()),
    }
}

fn pcm_state(muxer: &PcmS16leMuxer) -> (u64, usize, usize, Vec<u8>) {
    (
        muxer.packets(),
        muxer.data_len(),
        muxer.info().samples_per_channel(),
        muxer.render(),
    )
}

fn wav_state(muxer: &WavMuxer) -> (u64, usize, usize) {
    (muxer.packets(), muxer.data_len(), muxer.info().data_size())
}

fn rawvideo_state(muxer: &RawVideoMuxer) -> (usize, usize, Vec<u8>) {
    (muxer.info().frame_count(), muxer.data_len(), muxer.render())
}

fn y4m_state(muxer: &Yuv4MpegMuxer) -> (usize, usize, Vec<u8>) {
    (muxer.frame_count(), muxer.payload_len(), muxer.render())
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
