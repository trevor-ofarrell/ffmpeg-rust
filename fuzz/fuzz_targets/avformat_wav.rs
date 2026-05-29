#![no_main]

use avformat::WavDemuxer;
use avutil::SampleFormat;
use libfuzzer_sys::fuzz_target;

const VALID_WAV: &[u8] = b"RIFF,\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x80\xbb\0\0\0\xee\x02\0\x04\0\x10\0data\x08\0\0\0\0\0\x01\0\x02\0\x03\0";
const TOO_SMALL_RIFF_WAV: &[u8] = b"RIFF\0\0\0\0";

fuzz_target!(|data: &[u8]| {
    exercise_wav(data);
    exercise_wav(VALID_WAV);
    exercise_wav(TOO_SMALL_RIFF_WAV);
});

fn exercise_wav(input: &[u8]) {
    let Ok(mut demuxer) = WavDemuxer::open(input) else {
        return;
    };

    let info = demuxer.info().clone();
    assert_eq!(info.sample_format(), SampleFormat::S16);
    assert_eq!(info.bits_per_sample(), 16);
    assert_eq!(
        usize::from(info.block_align()),
        usize::from(info.channels()) * 2
    );
    assert_eq!(
        info.byte_rate(),
        info.sample_rate() * u32::from(info.block_align())
    );
    assert_eq!(
        info.samples_per_channel(),
        info.data_size() / usize::from(info.block_align())
    );

    let Ok(Some(packet)) = demuxer.read_packet() else {
        return;
    };
    assert_eq!(packet.stream_index(), 0);
    assert_eq!(packet.pts(), Some(0));
    assert_eq!(packet.dts(), Some(0));
    assert_eq!(packet.duration() as usize, info.samples_per_channel());
    assert_eq!(packet.data().len(), info.data_size());
    assert!(demuxer.read_packet().unwrap().is_none());
}
