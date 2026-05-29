use avutil::{AvError, AvResult, Packet, Rational};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image2Pattern {
    raw: String,
    kind: Image2PatternKind,
}

impl Image2Pattern {
    pub fn parse(pattern: impl Into<String>) -> AvResult<Self> {
        let raw = pattern.into();
        if raw.is_empty() {
            return Err(AvError::invalid_argument(
                "image2 pattern must not be empty",
            ));
        }

        let mut conversion = None;
        let mut index = 0;
        while let Some(relative) = raw[index..].find('%') {
            let percent = index + relative;
            let after_percent = percent + 1;
            let Some(next) = raw.as_bytes().get(after_percent).copied() else {
                return Err(AvError::invalid_argument(
                    "image2 pattern has trailing percent escape",
                ));
            };

            if next == b'%' {
                index = after_percent + 1;
                continue;
            }

            if conversion.is_some() {
                return Err(AvError::unsupported(
                    "image2 pattern supports only one numeric conversion",
                ));
            }

            let mut spec_end = after_percent;
            let mut width = None;
            if raw.as_bytes()[spec_end] == b'0' {
                spec_end += 1;
                let width_start = spec_end;
                while raw
                    .as_bytes()
                    .get(spec_end)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    spec_end += 1;
                }
                if spec_end == width_start {
                    return Err(AvError::invalid_argument(
                        "image2 zero-padded pattern is missing width",
                    ));
                }
                let parsed_width = raw[width_start..spec_end]
                    .parse::<usize>()
                    .map_err(|_| AvError::invalid_argument("image2 pattern width is invalid"))?;
                if parsed_width == 0 || parsed_width > 18 {
                    return Err(AvError::invalid_argument(
                        "image2 pattern width must be between 1 and 18",
                    ));
                }
                width = Some(parsed_width);
            }

            if raw.as_bytes().get(spec_end).copied() != Some(b'd') {
                return Err(AvError::unsupported(format!(
                    "unsupported image2 pattern conversion near `{}`",
                    &raw[percent..]
                )));
            }
            spec_end += 1;
            conversion = Some((percent, spec_end, width));
            index = spec_end;
        }

        let kind = if let Some((start, end, width)) = conversion {
            Image2PatternKind::Numbered {
                prefix: unescape_percent(&raw[..start])?,
                suffix: unescape_percent(&raw[end..])?,
                width,
            }
        } else {
            Image2PatternKind::Single(unescape_percent(&raw)?)
        };

        Ok(Self { raw, kind })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn is_sequence(&self) -> bool {
        matches!(self.kind, Image2PatternKind::Numbered { .. })
    }

    pub fn frame_number_for_path(&self, path: &str) -> Option<i64> {
        match &self.kind {
            Image2PatternKind::Single(single) => (single == path).then_some(0),
            Image2PatternKind::Numbered {
                prefix,
                suffix,
                width,
            } => {
                if !path.starts_with(prefix) || !path.ends_with(suffix) {
                    return None;
                }
                let number_start = prefix.len();
                let number_end = path.len().checked_sub(suffix.len())?;
                if number_start >= number_end {
                    return None;
                }

                let digits = &path[number_start..number_end];
                if width.is_some_and(|width| digits.len() != width)
                    || !digits.as_bytes().iter().all(u8::is_ascii_digit)
                {
                    return None;
                }
                digits.parse::<i64>().ok()
            }
        }
    }

    pub fn path_for_frame_number(&self, number: i64) -> AvResult<String> {
        if number < 0 {
            return Err(AvError::invalid_argument(
                "image2 frame number must not be negative",
            ));
        }

        match &self.kind {
            Image2PatternKind::Single(single) => Ok(single.clone()),
            Image2PatternKind::Numbered {
                prefix,
                suffix,
                width,
            } => {
                let digits = if let Some(width) = width {
                    format!("{number:0width$}")
                } else {
                    number.to_string()
                };
                let mut path = String::with_capacity(prefix.len() + digits.len() + suffix.len());
                path.push_str(prefix);
                path.push_str(&digits);
                path.push_str(suffix);
                Ok(path)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Image2PatternKind {
    Single(String),
    Numbered {
        prefix: String,
        suffix: String,
        width: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image2Entry {
    path: String,
    data: Vec<u8>,
}

impl Image2Entry {
    pub fn new(path: impl Into<String>, data: Vec<u8>) -> AvResult<Self> {
        let path = path.into();
        if path.is_empty() {
            return Err(AvError::invalid_argument(
                "image2 entry path must not be empty",
            ));
        }
        if data.is_empty() {
            return Err(AvError::invalid_data(
                "image2 entry payload must not be empty",
            ));
        }
        Ok(Self { path, data })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image2Info {
    pattern: Image2Pattern,
    start_number: i64,
    frame_rate: Rational,
    frame_count: usize,
}

impl Image2Info {
    pub fn pattern(&self) -> &Image2Pattern {
        &self.pattern
    }

    pub fn start_number(&self) -> i64 {
        self.start_number
    }

    pub fn frame_rate(&self) -> Rational {
        self.frame_rate
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image2Frame {
    number: i64,
    path: String,
    data: Vec<u8>,
}

impl Image2Frame {
    pub fn number(&self) -> i64 {
        self.number
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image2Demuxer {
    info: Image2Info,
    frames: Vec<Image2Frame>,
    next_index: usize,
}

impl Image2Demuxer {
    pub fn open(
        pattern: impl Into<String>,
        entries: Vec<Image2Entry>,
        start_number: i64,
        frame_rate: Rational,
    ) -> AvResult<Self> {
        validate_frame_rate(frame_rate)?;
        if start_number < 0 {
            return Err(AvError::invalid_argument(
                "image2 start number must not be negative",
            ));
        }
        if entries.is_empty() {
            return Err(AvError::invalid_argument(
                "image2 demuxer requires at least one entry",
            ));
        }

        let pattern = Image2Pattern::parse(pattern)?;
        let frames = build_frames(&pattern, entries, start_number)?;
        let frame_count = frames.len();
        Ok(Self {
            info: Image2Info {
                pattern,
                start_number,
                frame_rate,
                frame_count,
            },
            frames,
            next_index: 0,
        })
    }

    pub fn info(&self) -> &Image2Info {
        &self.info
    }

    pub fn frames(&self) -> &[Image2Frame] {
        &self.frames
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        let Some(frame) = self.frames.get(self.next_index) else {
            return Ok(None);
        };

        let pts = i64::try_from(self.next_index)
            .map_err(|_| AvError::invalid_data("image2 packet PTS does not fit i64"))?;
        let mut packet = Packet::new(frame.data.clone(), 0);
        packet.set_pts(Some(pts));
        packet.set_dts(Some(pts));
        packet.set_duration(1)?;
        packet.push_side_data(avutil::SideData::new(
            "image2_path",
            frame.path.as_bytes().to_vec(),
        )?);
        self.next_index += 1;
        Ok(Some(packet))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image2Muxer {
    info: Image2Info,
    entries: Vec<Image2Entry>,
    next_number: i64,
    finished: bool,
}

impl Image2Muxer {
    pub fn new(
        pattern: impl Into<String>,
        start_number: i64,
        frame_rate: Rational,
    ) -> AvResult<Self> {
        validate_frame_rate(frame_rate)?;
        if start_number < 0 {
            return Err(AvError::invalid_argument(
                "image2 start number must not be negative",
            ));
        }

        let pattern = Image2Pattern::parse(pattern)?;
        Ok(Self {
            info: Image2Info {
                pattern,
                start_number,
                frame_rate,
                frame_count: 0,
            },
            entries: Vec::new(),
            next_number: start_number,
            finished: false,
        })
    }

    pub fn info(&self) -> &Image2Info {
        &self.info
    }

    pub fn entries(&self) -> &[Image2Entry] {
        &self.entries
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after image2 muxer is finished",
            ));
        }
        if packet.stream_index() != 0 {
            return Err(AvError::invalid_argument(format!(
                "image2 muxer only accepts stream 0, got stream {}",
                packet.stream_index()
            )));
        }
        if packet.data().is_empty() {
            return Err(AvError::invalid_data(
                "image2 packet payload must not be empty",
            ));
        }
        if !self.info.pattern.is_sequence() && !self.entries.is_empty() {
            return Err(AvError::invalid_argument(
                "single-image image2 output accepts exactly one packet",
            ));
        }

        let path = self.info.pattern.path_for_frame_number(self.next_number)?;
        if self.entries.iter().any(|entry| entry.path() == path) {
            return Err(AvError::invalid_data(format!(
                "duplicate image2 output path `{path}`"
            )));
        }

        let entry = Image2Entry::new(path, packet.data().to_vec())?;
        let next_number = self
            .next_number
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("image2 frame number overflow"))?;
        let frame_count = self
            .info
            .frame_count
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("image2 frame count overflow"))?;

        self.entries.push(entry);
        self.next_number = next_number;
        self.info.frame_count = frame_count;
        Ok(())
    }

    pub fn render(&self) -> Vec<Image2Entry> {
        self.entries.clone()
    }

    pub fn finish(&mut self) -> Vec<Image2Entry> {
        self.finished = true;
        self.render()
    }
}

fn build_frames(
    pattern: &Image2Pattern,
    entries: Vec<Image2Entry>,
    start_number: i64,
) -> AvResult<Vec<Image2Frame>> {
    if !pattern.is_sequence() {
        if entries.len() != 1 {
            return Err(AvError::invalid_argument(
                "single-image image2 input requires exactly one entry",
            ));
        }
        let mut entries = entries;
        let entry = entries.pop().expect("single entry exists");
        let Some(number) = pattern.frame_number_for_path(&entry.path) else {
            return Err(AvError::invalid_data(format!(
                "image2 entry `{}` does not match pattern `{}`",
                entry.path,
                pattern.raw()
            )));
        };
        return Ok(vec![Image2Frame {
            number,
            path: entry.path,
            data: entry.data,
        }]);
    }

    let mut by_number = BTreeMap::new();
    for entry in entries {
        let Some(number) = pattern.frame_number_for_path(&entry.path) else {
            return Err(AvError::invalid_data(format!(
                "image2 entry `{}` does not match pattern `{}`",
                entry.path,
                pattern.raw()
            )));
        };
        if number < start_number {
            return Err(AvError::invalid_data(format!(
                "image2 frame number {number} is before start number {start_number}"
            )));
        }
        if by_number.insert(number, entry).is_some() {
            return Err(AvError::invalid_data(format!(
                "duplicate image2 frame number {number}"
            )));
        }
    }

    let mut expected = start_number;
    let mut frames = Vec::with_capacity(by_number.len());
    for (number, entry) in by_number {
        if number != expected {
            return Err(AvError::invalid_data(format!(
                "image2 sequence is missing frame number {expected}"
            )));
        }
        frames.push(Image2Frame {
            number,
            path: entry.path,
            data: entry.data,
        });
        expected = expected
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_data("image2 frame number overflow"))?;
    }

    Ok(frames)
}

fn validate_frame_rate(frame_rate: Rational) -> AvResult<()> {
    if frame_rate.num() <= 0 || frame_rate.den() <= 0 {
        return Err(AvError::invalid_argument(
            "image2 frame rate must be positive",
        ));
    }
    Ok(())
}

fn unescape_percent(value: &str) -> AvResult<String> {
    let mut output = String::with_capacity(value.len());
    let mut index = 0;
    while let Some(relative) = value[index..].find('%') {
        let percent = index + relative;
        output.push_str(&value[index..percent]);
        if value.as_bytes().get(percent + 1).copied() != Some(b'%') {
            return Err(AvError::invalid_argument(
                "image2 literal percent must be escaped as `%%`",
            ));
        }
        output.push('%');
        index = percent + 2;
    }
    output.push_str(&value[index..]);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_numbered_sequence_in_frame_order() {
        let entries = vec![
            entry("frame-002.png", b"two"),
            entry("frame-000.png", b"zero"),
            entry("frame-001.png", b"one"),
        ];
        let mut demuxer =
            Image2Demuxer::open("frame-%03d.png", entries, 0, Rational::new(25, 1).unwrap())
                .unwrap();

        assert!(demuxer.info().pattern().is_sequence());
        assert_eq!(demuxer.info().frame_count(), 3);
        assert_eq!(demuxer.info().frame_rate(), Rational::new(25, 1).unwrap());
        assert_eq!(
            demuxer
                .frames()
                .iter()
                .map(Image2Frame::path)
                .collect::<Vec<_>>(),
            vec!["frame-000.png", "frame-001.png", "frame-002.png"]
        );

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data(), b"zero");
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.duration(), 1);
        assert_eq!(first.side_data()[0].kind(), "image2_path");
        assert_eq!(first.side_data()[0].data(), b"frame-000.png");

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), b"one");
        assert_eq!(second.pts(), Some(1));
        let third = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(third.data(), b"two");
        assert_eq!(third.pts(), Some(2));
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn supports_single_image_inputs() {
        let mut demuxer = Image2Demuxer::open(
            "cover.png",
            vec![entry("cover.png", b"png bytes")],
            0,
            Rational::new(1, 1).unwrap(),
        )
        .unwrap();

        assert!(!demuxer.info().pattern().is_sequence());
        assert_eq!(demuxer.info().frame_count(), 1);
        assert_eq!(demuxer.frames()[0].number(), 0);
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), b"png bytes");
        assert_eq!(packet.pts(), Some(0));
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn validates_patterns_and_frame_rate() {
        assert!(Image2Pattern::parse("").is_err());
        assert!(Image2Pattern::parse("frame-%x.png").is_err());
        assert!(Image2Pattern::parse("frame-%d-%d.png").is_err());
        assert!(Image2Pattern::parse("frame-%0d.png").is_err());
        assert!(Image2Pattern::parse("frame-%99d.png").is_err());
        assert!(Image2Pattern::parse("frame-%.png").is_err());
        assert!(Image2Pattern::parse("frame-%%-%d.png").is_ok());

        assert!(Image2Demuxer::open(
            "frame-%d.png",
            vec![entry("frame-0.png", b"x")],
            -1,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(Image2Demuxer::open(
            "frame-%d.png",
            vec![entry("frame-0.png", b"x")],
            0,
            Rational::new(-1, 1).unwrap(),
        )
        .is_err());
    }

    #[test]
    fn rejects_unmatched_duplicate_non_contiguous_and_empty_entries() {
        assert!(Image2Entry::new("", b"x".to_vec()).is_err());
        assert!(Image2Entry::new("frame-0.png", Vec::new()).is_err());
        assert!(
            Image2Demuxer::open("frame-%d.png", Vec::new(), 0, Rational::new(1, 1).unwrap(),)
                .is_err()
        );
        assert!(Image2Demuxer::open(
            "cover.png",
            vec![entry("cover.png", b"x"), entry("other.png", b"y")],
            0,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(Image2Demuxer::open(
            "frame-%03d.png",
            vec![entry("frame-1.png", b"x")],
            1,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(Image2Demuxer::open(
            "frame-%d.png",
            vec![entry("frame-0.png", b"x"), entry("frame-0.png", b"y")],
            0,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(Image2Demuxer::open(
            "frame-%d.png",
            vec![entry("frame-0.png", b"x"), entry("frame-2.png", b"z")],
            0,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
    }

    #[test]
    fn rejects_unpadded_numeric_aliases_for_same_frame_number() {
        let err = Image2Demuxer::open(
            "frame-%d.png",
            vec![
                entry("frame-1.png", b"one"),
                entry("frame-01.png", b"alias"),
            ],
            1,
            Rational::new(1, 1).unwrap(),
        )
        .unwrap_err();

        assert_eq!(err.kind(), avutil::AvErrorKind::InvalidData);
    }

    #[test]
    fn reads_nonzero_start_number_with_zero_based_packet_timestamps() {
        let entries = vec![
            entry("frame-011.ppm", b"eleven"),
            entry("frame-010.ppm", b"ten"),
        ];
        let mut demuxer =
            Image2Demuxer::open("frame-%03d.ppm", entries, 10, Rational::new(30, 1).unwrap())
                .unwrap();

        assert_eq!(demuxer.info().start_number(), 10);
        assert_eq!(
            demuxer
                .frames()
                .iter()
                .map(|frame| (frame.number(), frame.path()))
                .collect::<Vec<_>>(),
            vec![(10, "frame-010.ppm"), (11, "frame-011.ppm")]
        );

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data(), b"ten");
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.duration(), 1);
        assert_eq!(first.side_data()[0].data(), b"frame-010.ppm");

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), b"eleven");
        assert_eq!(second.pts(), Some(1));
        assert_eq!(second.dts(), Some(1));
        assert_eq!(second.duration(), 1);
        assert_eq!(second.side_data()[0].data(), b"frame-011.ppm");
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_writes_numbered_sequence_and_round_trips_through_demuxer() {
        let mut muxer =
            Image2Muxer::new("frame-%03d.png", 2, Rational::new(25, 1).unwrap()).unwrap();

        muxer
            .write_packet(&Packet::new(b"two".to_vec(), 0))
            .unwrap();
        muxer
            .write_packet(&Packet::new(b"three".to_vec(), 0))
            .unwrap();

        assert!(muxer.info().pattern().is_sequence());
        assert_eq!(muxer.info().start_number(), 2);
        assert_eq!(muxer.info().frame_rate(), Rational::new(25, 1).unwrap());
        assert_eq!(muxer.info().frame_count(), 2);
        assert_eq!(muxer.entries()[0].path(), "frame-002.png");
        assert_eq!(muxer.entries()[1].path(), "frame-003.png");
        assert_eq!(muxer.entries()[0].data(), b"two");

        let entries = muxer.finish();
        let mut demuxer =
            Image2Demuxer::open("frame-%03d.png", entries, 2, Rational::new(25, 1).unwrap())
                .unwrap();
        assert_eq!(demuxer.read_packet().unwrap().unwrap().data(), b"two");
        assert_eq!(demuxer.read_packet().unwrap().unwrap().data(), b"three");
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_supports_single_image_output() {
        let mut muxer =
            Image2Muxer::new("cover%%final.png", 0, Rational::new(1, 1).unwrap()).unwrap();

        muxer
            .write_packet(&Packet::new(b"png bytes".to_vec(), 0))
            .unwrap();

        assert!(!muxer.info().pattern().is_sequence());
        assert_eq!(muxer.entries()[0].path(), "cover%final.png");
        assert_eq!(muxer.entries()[0].data(), b"png bytes");
        assert_eq!(muxer.info().frame_count(), 1);
    }

    #[test]
    fn muxer_formats_unpadded_and_zero_padded_paths() {
        let unpadded = Image2Pattern::parse("img-%d.jpeg").unwrap();
        let padded = Image2Pattern::parse("img-%04d.jpeg").unwrap();

        assert_eq!(unpadded.path_for_frame_number(12).unwrap(), "img-12.jpeg");
        assert_eq!(padded.path_for_frame_number(12).unwrap(), "img-0012.jpeg");
        assert_eq!(
            padded.path_for_frame_number(12_345).unwrap(),
            "img-12345.jpeg"
        );
        assert!(padded.path_for_frame_number(-1).is_err());
    }

    #[test]
    fn muxer_rejects_invalid_parameters_packets_and_duplicate_single_path() {
        assert!(Image2Muxer::new("", 0, Rational::new(1, 1).unwrap()).is_err());
        assert!(Image2Muxer::new("frame-%d.png", -1, Rational::new(1, 1).unwrap()).is_err());
        assert!(Image2Muxer::new("frame-%d.png", 0, Rational::new(0, 1).unwrap()).is_err());

        let mut muxer = Image2Muxer::new("frame-%d.png", 0, Rational::new(1, 1).unwrap()).unwrap();
        let wrong_stream = muxer.write_packet(&Packet::new(b"x".to_vec(), 1));
        assert_eq!(
            wrong_stream.unwrap_err().kind(),
            avutil::AvErrorKind::InvalidArgument
        );
        let empty_packet = muxer.write_packet(&Packet::new(Vec::new(), 0));
        assert_eq!(
            empty_packet.unwrap_err().kind(),
            avutil::AvErrorKind::InvalidData
        );
        assert_eq!(muxer.info().frame_count(), 0);
        assert!(muxer.entries().is_empty());

        let mut single = Image2Muxer::new("cover.png", 0, Rational::new(1, 1).unwrap()).unwrap();
        single.write_packet(&Packet::new(b"x".to_vec(), 0)).unwrap();
        let duplicate = single.write_packet(&Packet::new(b"y".to_vec(), 0));
        assert_eq!(
            duplicate.unwrap_err().kind(),
            avutil::AvErrorKind::InvalidArgument
        );
        assert_eq!(single.info().frame_count(), 1);
        assert_eq!(single.entries().len(), 1);
    }

    #[test]
    fn muxer_finish_prevents_more_writes() {
        let mut muxer = Image2Muxer::new("frame-%d.png", 0, Rational::new(1, 1).unwrap()).unwrap();

        muxer.write_packet(&Packet::new(b"x".to_vec(), 0)).unwrap();
        let entries = muxer.finish();
        let err = muxer
            .write_packet(&Packet::new(b"y".to_vec(), 0))
            .unwrap_err();

        assert!(muxer.is_finished());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path(), "frame-0.png");
        assert_eq!(err.kind(), avutil::AvErrorKind::InvalidArgument);
        assert_eq!(muxer.info().frame_count(), 1);
    }

    fn entry(path: &str, data: &[u8]) -> Image2Entry {
        Image2Entry::new(path, data.to_vec()).unwrap()
    }
}
