#![no_main]

use avformat::PcmS16leDemuxer;
use avutil::SampleFormat;
use libfuzzer_sys::fuzz_target;

const VALID_STEREO: &[u8] = &[0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0];
const PARTIAL_THREE_CHANNEL: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
const ODD_STEREO_PACKET: &[u8] = &[0, 1, 2, 3, 4];

fuzz_target!(|data: &[u8]| {
    let sample_rate = data
        .first()
        .copied()
        .map(sample_rate_from)
        .unwrap_or(48_000);
    let channels = data.get(1).copied().map(channels_from).unwrap_or(2);
    let packet_samples = data.get(2).copied().map(packet_samples_from).unwrap_or(2);
    let payload = data.get(3..).unwrap_or(&[]);

    exercise_pcm(payload, sample_rate, channels, packet_samples);
    exercise_pcm(VALID_STEREO, 48_000, 2, 2);
    exercise_pcm(PARTIAL_THREE_CHANNEL, 48_000, 3, 1024);
    exercise_pcm(&[], 48_000, 2, 1024);
    exercise_pcm_s16le_muxer(VALID_STEREO);
    exercise_pcm_s16le_muxer(PARTIAL_THREE_CHANNEL);
    exercise_pcm_s16le_muxer(ODD_STEREO_PACKET);
    exercise_pcm_s16le_muxer(&[]);
});

fn exercise_pcm(input: &[u8], sample_rate: u32, channels: u16, packet_samples: usize) {
    let Ok(mut demuxer) = PcmS16leDemuxer::open(input, sample_rate, channels, packet_samples)
    else {
        return;
    };

    let info = demuxer.info().clone();
    assert_eq!(info.sample_format(), SampleFormat::S16);
    assert!(info.sample_rate() > 0);
    assert!(info.channels() > 0);
    assert!(info.packet_samples() > 0);
    assert_eq!(
        info.bytes_per_sample_frame(),
        usize::from(info.channels()) * 2
    );
    assert_eq!(
        info.packet_size(),
        info.bytes_per_sample_frame() * info.packet_samples()
    );
    let accounted_bytes = info.total_samples_per_channel() * info.bytes_per_sample_frame();
    assert!(accounted_bytes <= input.len());
    assert!(input.len().saturating_sub(accounted_bytes) < info.bytes_per_sample_frame());

    let mut expected_pts = 0_i64;
    for _ in 0..32 {
        match demuxer.read_packet() {
            Ok(Some(packet)) => {
                assert_eq!(packet.stream_index(), 0);
                assert_eq!(packet.pts(), Some(expected_pts));
                assert_eq!(packet.dts(), Some(expected_pts));
                assert!(packet.duration() >= 0);
                assert!(packet.data().len() <= info.packet_size());
                let packet_accounted_bytes =
                    packet.duration() as usize * info.bytes_per_sample_frame();
                assert!(packet_accounted_bytes <= packet.data().len());
                assert!(
                    packet.data().len().saturating_sub(packet_accounted_bytes)
                        < info.bytes_per_sample_frame()
                );
                assert_eq!(packet.side_data()[0].kind(), "pcm_sample_fmt");
                assert_eq!(packet.side_data()[0].data(), b"s16");
                assert_eq!(packet.side_data()[1].kind(), "pcm_channels");
                assert_eq!(
                    packet.side_data()[1].data(),
                    info.channels().to_string().as_bytes()
                );
                expected_pts += packet.duration();
            }
            Ok(None) | Err(_) => break,
        }
    }
}

fn sample_rate_from(byte: u8) -> u32 {
    match byte % 7 {
        0 => 0,
        1 => 1,
        2 => 8_000,
        3 => 44_100,
        4 => 48_000,
        5 => 192_000,
        _ => u32::MAX,
    }
}

fn exercise_pcm_s16le_muxer(payload: &[u8]) {
    let Ok(mut muxer) = avformat::PcmS16leMuxer::new(48_000, 2) else {
        return;
    };
    let payload_len = payload.len();
    let bytes_per_sample_frame = muxer.info().bytes_per_sample_frame();
    let expected_samples_per_channel = payload_len / bytes_per_sample_frame;

    muxer.write_packet(&avutil::Packet::new(payload.to_vec(), 0)).unwrap();

    assert!(muxer.packets() == 1);
    assert_eq!(muxer.data_len(), payload_len);
    assert_eq!(muxer.render(), payload);
    assert_eq!(muxer.info().samples_per_channel(), expected_samples_per_channel);

    muxer.write_packet(&avutil::Packet::new(payload.to_vec(), 0)).unwrap();
    assert_eq!(muxer.packets(), 2);
    assert_eq!(muxer.data_len(), payload_len * 2);
    assert_eq!(muxer.info().samples_per_channel(), expected_samples_per_channel * 2);
}

fn channels_from(byte: u8) -> u16 {
    match byte % 6 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 8,
        _ => u16::MAX,
    }
}

fn packet_samples_from(byte: u8) -> usize {
    match byte % 6 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 4,
        4 => 1024,
        _ => 4096,
    }
}
