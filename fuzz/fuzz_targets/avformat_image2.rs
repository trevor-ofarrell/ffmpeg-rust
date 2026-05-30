#![no_main]

use avformat::{Image2Demuxer, Image2Entry, Image2Muxer, Image2Pattern};
use avutil::{Packet, Rational};
use libfuzzer_sys::fuzz_target;

const MAX_ENTRIES: usize = 8;
const MAX_PAYLOAD_LEN: usize = 24;
const MAX_LITERAL_LEN: usize = 18;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    let pattern = pattern_from(&mut cursor);
    let start_number = start_number_from(cursor.next());
    let frame_rate = frame_rate_from(cursor.next());
    let entry_mode = cursor.next().unwrap_or_default();
    let entries = entries_from_bytes(&pattern, start_number, entry_mode, &mut cursor);
    let packets = packets_from_bytes(&mut cursor);

    exercise_pattern(&pattern);
    exercise_demux(&pattern, entries, start_number, frame_rate);
    exercise_mux(&pattern, start_number, frame_rate, packets);
    exercise_fixtures();
});

fn exercise_pattern(pattern: &str) {
    let Ok(parsed) = Image2Pattern::parse(pattern.to_owned()) else {
        return;
    };

    for number in [0, 1, 42, 1000] {
        let path = parsed.path_for_frame_number(number).unwrap();
        let matched = parsed.frame_number_for_path(&path).unwrap();
        if parsed.is_sequence() {
            assert_eq!(matched, number);
        } else {
            assert_eq!(matched, 0);
        }
    }
    assert!(parsed.path_for_frame_number(-1).is_err());
}

fn exercise_demux(
    pattern: &str,
    entries: Vec<Image2Entry>,
    start_number: i64,
    frame_rate: Rational,
) {
    let Ok(mut demuxer) =
        Image2Demuxer::open(pattern.to_owned(), entries, start_number, frame_rate)
    else {
        return;
    };

    let info = demuxer.info().clone();
    assert_eq!(info.pattern().raw(), pattern);
    assert_eq!(info.start_number(), start_number);
    assert_eq!(info.frame_rate(), frame_rate);
    assert_eq!(info.frame_count(), demuxer.frames().len());
    if !info.pattern().is_sequence() {
        assert_eq!(info.frame_count(), 1);
    }

    let frames = demuxer.frames().to_vec();
    for frame in &frames {
        assert!(!frame.path().is_empty());
        assert!(!frame.data().is_empty());
        assert_eq!(
            info.pattern().frame_number_for_path(frame.path()),
            Some(frame.number())
        );
        if info.pattern().is_sequence() {
            assert!(frame.number() >= start_number);
        }
    }

    for (expected_pts, frame) in frames.iter().enumerate() {
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.stream_index(), 0);
        assert_eq!(packet.pts(), Some(expected_pts as i64));
        assert_eq!(packet.dts(), Some(expected_pts as i64));
        assert_eq!(packet.duration(), 1);
        assert_eq!(packet.data(), frame.data());
        assert_eq!(packet.side_data().len(), 1);
        assert_eq!(packet.side_data()[0].kind(), "image2_path");
        assert_eq!(packet.side_data()[0].data(), frame.path().as_bytes());
    }
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn exercise_demux_errors_from_start_probe_window_exhaustion(
    pattern: &str,
    entries: Vec<Image2Entry>,
    start_number: i64,
    frame_rate: Rational,
) {
    assert!(Image2Demuxer::open(pattern.to_owned(), entries, start_number, frame_rate).is_err());
}

fn exercise_demux_rejects_start_number_over_32bit_limit(
    pattern: &str,
    entries: Vec<Image2Entry>,
    start_number: i64,
    frame_rate: Rational,
) {
    assert!(Image2Demuxer::open(pattern.to_owned(), entries, start_number, frame_rate).is_err());
}

fn exercise_mux(pattern: &str, start_number: i64, frame_rate: Rational, packets: Vec<Packet>) {
    let Ok(mut muxer) = Image2Muxer::new(pattern.to_owned(), start_number, frame_rate) else {
        return;
    };

    assert_eq!(muxer.info().pattern().raw(), pattern);
    assert_eq!(muxer.info().start_number(), start_number);
    assert_eq!(muxer.info().frame_rate(), frame_rate);
    assert_eq!(muxer.info().frame_count(), 0);
    assert!(muxer.entries().is_empty());

    for packet in packets.iter().take(MAX_ENTRIES) {
        let before_count = muxer.info().frame_count();
        let before_len = muxer.entries().len();
        let result = muxer.write_packet(packet);
        if result.is_ok() {
            assert_eq!(muxer.info().frame_count(), before_count + 1);
            assert_eq!(muxer.entries().len(), before_len + 1);

            let entry = muxer.entries().last().unwrap();
            assert!(!entry.path().is_empty());
            assert_eq!(entry.data(), packet.data());
            if let Some(number) = start_number.checked_add(before_count as i64) {
                assert_eq!(
                    entry.path(),
                    muxer
                        .info()
                        .pattern()
                        .path_for_frame_number(number)
                        .unwrap()
                );
            }
        } else {
            assert_eq!(muxer.info().frame_count(), before_count);
            assert_eq!(muxer.entries().len(), before_len);
        }
    }

    let rendered = muxer.render();
    assert_eq!(rendered, muxer.entries());
    let finished = muxer.finish();
    assert!(muxer.is_finished());
    assert_eq!(finished, rendered);
    let finished_len = muxer.entries().len();
    assert!(muxer.write_packet(&Packet::new(b"x".to_vec(), 0)).is_err());
    assert_eq!(muxer.entries().len(), finished_len);

    if !finished.is_empty() {
        let mut demuxer = Image2Demuxer::open(
            pattern.to_owned(),
            finished.clone(),
            start_number,
            frame_rate,
        )
        .unwrap();
        for entry in &finished {
            let packet = demuxer.read_packet().unwrap().unwrap();
            assert_eq!(packet.data(), entry.data());
        }
        assert!(demuxer.read_packet().unwrap().is_none());
    }
}

fn exercise_fixtures() {
    let rate_25 = Rational::new(25, 1).unwrap();
    exercise_demux(
        "frame-%03d.png",
        vec![
            entry("frame-002.png", b"two"),
            entry("frame-000.png", b"zero"),
            entry("frame-001.png", b"one"),
        ],
        0,
        rate_25,
    );
    exercise_demux(
        "frame-%3d.png",
        vec![
            entry("frame-000.png", b"zero"),
            entry("frame-001.png", b"one"),
        ],
        0,
        Rational::ONE,
    );
    exercise_demux(
        "cover%%final.png",
        vec![entry("cover%final.png", b"cover")],
        0,
        Rational::ONE,
    );
    exercise_demux(
        "cover%%final.png",
        vec![entry("cover%final.png", b"cover")],
        7,
        Rational::ONE,
    );
    exercise_demux(
        "frame-%d.png",
        vec![entry("frame-0.png", b"zero"), entry("frame-2.png", b"two")],
        1,
        Rational::ONE,
    );
    exercise_demux(
        "frame-%d.png",
        vec![entry("frame-5.png", b"five")],
        1,
        Rational::ONE,
    );
    exercise_demux(
        "frame-%d.png",
        vec![entry("frame-2147483647.png", b"max")],
        i32::MAX as i64,
        Rational::ONE,
    );
    exercise_demux_rejects_start_number_over_32bit_limit(
        "frame-%d.png",
        vec![entry("frame-0.png", b"zero")],
        i64::from(i32::MAX) + 1,
        Rational::ONE,
    );
    exercise_demux_errors_from_start_probe_window_exhaustion(
        "frame-%03d.ppm",
        vec![entry("frame-006.ppm", b"six")],
        1,
        Rational::ONE,
    );
    exercise_demux(
        "frame-%03d.ppm",
        vec![
            entry("frame-999.ppm", b"nine_nine_nine"),
            entry("frame-1000.ppm", b"thousand"),
        ],
        999,
        Rational::new(25, 1).unwrap(),
    );
    exercise_demux(
        "frame-%020d.ppm",
        vec![
            entry("frame-00000000000000000001.ppm", b"one"),
            entry("frame-00000000000000000002.ppm", b"two"),
        ],
        1,
        Rational::ONE,
    );
    exercise_mux(
        "frame-%03d.png",
        2,
        rate_25,
        vec![
            Packet::new(b"two".to_vec(), 0),
            Packet::new(b"three".to_vec(), 0),
        ],
    );
    exercise_mux(
        "cover.png",
        0,
        Rational::ONE,
        vec![
            Packet::new(b"cover".to_vec(), 0),
            Packet::new(b"second".to_vec(), 0),
        ],
    );
    exercise_mux(
        "frame-%d.png",
        i64::MAX,
        Rational::ONE,
        vec![Packet::new(b"overflow".to_vec(), 0)],
    );
    let _ = Image2Entry::new("", Vec::new());
}

fn entries_from_bytes(
    pattern: &str,
    start_number: i64,
    mode: u8,
    cursor: &mut Cursor<'_>,
) -> Vec<Image2Entry> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_ENTRIES + 1);
    let parsed = Image2Pattern::parse(pattern.to_owned()).ok();
    let base_number = start_number.clamp(0, 8);
    let mut entries = Vec::new();

    for index in 0..count {
        let payload = payload_from_bytes(cursor);
        let path = match parsed.as_ref() {
            Some(parsed) if mode % 5 != 4 => {
                let number = match mode % 5 {
                    0 => base_number + index as i64,
                    1 => base_number + index as i64 + i64::from(index >= count / 2),
                    2 => base_number,
                    3 => base_number.saturating_sub(1) + index as i64,
                    _ => base_number + index as i64,
                };
                parsed
                    .path_for_frame_number(number)
                    .unwrap_or_else(|_| arbitrary_path(cursor))
            }
            _ => arbitrary_path(cursor),
        };

        let _ = Image2Entry::new(path.clone(), payload.clone());
        if let Ok(entry) = Image2Entry::new(path, payload) {
            entries.push(entry);
        }
    }

    entries
}

fn packets_from_bytes(cursor: &mut Cursor<'_>) -> Vec<Packet> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_ENTRIES + 1);
    let mut packets = Vec::new();
    for _ in 0..count {
        let stream_index = usize::from(cursor.next().unwrap_or_default().is_multiple_of(3));
        packets.push(Packet::new(payload_from_bytes(cursor), stream_index));
    }
    packets
}

fn pattern_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 15 {
        0 => "cover.png".to_owned(),
        1 => "frame-%d.png".to_owned(),
        2 => "frame-%03d.png".to_owned(),
        3 => "frame-%%-%d.jpeg".to_owned(),
        4 => String::new(),
        5 => "frame-%x.png".to_owned(),
        6 => "frame-%d-%d.png".to_owned(),
        7 => "frame-%0d.png".to_owned(),
        8 => "frame-%99d.png".to_owned(),
        9 => "frame-%.png".to_owned(),
        10 => format!("{}-%d.dat", literal_from_bytes(cursor)),
        11 => format!("{}-%03d", literal_from_bytes(cursor)),
        12 => literal_from_bytes(cursor),
        13 => "frame-%3d.png".to_owned(),
        _ => format!(
            "{}%%{}",
            literal_from_bytes(cursor),
            literal_from_bytes(cursor)
        ),
    }
}

fn arbitrary_path(cursor: &mut Cursor<'_>) -> String {
    let path = literal_from_bytes(cursor);
    if path.is_empty() {
        "x.bin".to_owned()
    } else {
        path
    }
}

fn literal_from_bytes(cursor: &mut Cursor<'_>) -> String {
    let len = usize::from(cursor.next().unwrap_or_default()) % (MAX_LITERAL_LEN + 1);
    let mut output = String::with_capacity(len);
    for _ in 0..len {
        output.push(match cursor.next().unwrap_or_default() % 16 {
            0 => 'a',
            1 => '0',
            2 => '1',
            3 => '.',
            4 => '-',
            5 => '_',
            6 => '/',
            7 => '%',
            8 => 'd',
            9 => 'p',
            10 => 'n',
            11 => 'g',
            12 => 'x',
            13 => ' ',
            14 => '[',
            _ => ']',
        });
    }
    output
}

fn payload_from_bytes(cursor: &mut Cursor<'_>) -> Vec<u8> {
    let len = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD_LEN + 1);
    let mut payload = Vec::with_capacity(len);
    for _ in 0..len {
        payload.push(cursor.next().unwrap_or_default());
    }
    payload
}

fn start_number_from(byte: Option<u8>) -> i64 {
    match byte.unwrap_or_default() % 8 {
        0 => -1,
        1 => 0,
        2 => 1,
        3 => 2,
        4 => 8,
        5 => i64::MAX,
        6 => i64::MAX - 1,
        _ => 42,
    }
}

fn frame_rate_from(byte: Option<u8>) -> Rational {
    match byte.unwrap_or_default() % 6 {
        0 => Rational::ZERO,
        1 => Rational::ONE,
        2 => Rational::new(24, 1).unwrap(),
        3 => Rational::new(30000, 1001).unwrap(),
        4 => Rational::new(-1, 1).unwrap(),
        _ => Rational::from_raw(1, 0),
    }
}

fn entry(path: &str, data: &[u8]) -> Image2Entry {
    Image2Entry::new(path, data.to_vec()).unwrap()
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
