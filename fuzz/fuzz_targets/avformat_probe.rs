#![no_main]

use avformat::{
    avi_probe_descriptor, mov_probe_descriptor, register_avi_probe, register_mov_probe,
    ProbeDescriptor, ProbeRegistry, ProbeRequest, ProbeScore, ProbeSignature,
};
use libfuzzer_sys::fuzz_target;

const MAX_DESCRIPTORS: usize = 8;
const MAX_FIELDS: usize = 4;
const MAX_HEADER_LEN: usize = 48;
const MAX_LITERAL_LEN: usize = 24;
const MAX_SIGNATURE_LEN: usize = 12;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    let mut registry = ProbeRegistry::new();

    if cursor.next().unwrap_or_default().is_multiple_of(2) {
        let _ = register_avi_probe(&mut registry);
    }
    if cursor.next().unwrap_or_default().is_multiple_of(2) {
        let _ = register_mov_probe(&mut registry);
    }

    let descriptor_count = usize::from(cursor.next().unwrap_or_default()) % (MAX_DESCRIPTORS + 1);
    for _ in 0..descriptor_count {
        let before = registry.clone();
        let result =
            generated_descriptor(&mut cursor).and_then(|descriptor| registry.register(descriptor));
        if result.is_ok() {
            assert_eq!(registry.descriptors().len(), before.descriptors().len() + 1);
        } else {
            assert_eq!(registry, before);
        }
        assert_registry_invariants(&registry);
    }

    let header = header_from(&mut cursor);
    let path = path_from(&mut cursor);
    let mime_type = mime_type_from(&mut cursor);
    let use_extension = cursor.next().unwrap_or_default().is_multiple_of(2);
    let use_mime_type = cursor.next().unwrap_or_default().is_multiple_of(2);

    exercise_probe(
        &registry,
        &header,
        &path,
        &mime_type,
        use_extension,
        use_mime_type,
    );
    exercise_fixtures();
});

fn exercise_probe(
    registry: &ProbeRegistry,
    header: &[u8],
    path: &str,
    mime_type: &str,
    use_extension: bool,
    use_mime_type: bool,
) {
    let mut request = ProbeRequest::new(header);
    if use_extension {
        request = request.with_extension(path);
    }
    if use_mime_type {
        request = request.with_mime_type(mime_type);
    }

    assert_eq!(request.header(), header);
    let first = registry.probe(request);
    let second = registry.probe(request);
    assert_eq!(
        first.map(|matched| (matched.descriptor().name(), matched.score())),
        second.map(|matched| (matched.descriptor().name(), matched.score()))
    );

    let Some(matched) = first else {
        return;
    };

    assert!(matched.score() > ProbeScore::NONE);
    assert!(matched.score() <= ProbeScore::MAX);
    assert!(registry
        .descriptors()
        .iter()
        .any(|descriptor| descriptor.name() == matched.descriptor().name()));
    assert_match_is_explained(matched.descriptor(), matched.score(), request);
}

fn exercise_fixtures() {
    let mut registry = ProbeRegistry::new();
    register_avi_probe(&mut registry).unwrap();
    register_mov_probe(&mut registry).unwrap();

    exercise_probe(
        &registry,
        b"RIFF\0\0\0\0AVI ",
        "clip.bin",
        "application/octet-stream",
        true,
        true,
    );
    let avi = registry
        .probe(ProbeRequest::new(b"RIFF\0\0\0\0AVI ").with_extension("clip.mp4"))
        .unwrap();
    assert_eq!(avi.descriptor().name(), "avi");
    assert_eq!(avi.score(), ProbeScore::SIGNATURE);

    let mov = registry
        .probe(ProbeRequest::new(b"\0\0\0\x18ftypisom").with_extension("clip.bin"))
        .unwrap();
    assert_eq!(mov.descriptor().name(), "mov,mp4,m4a,3gp,3g2,mj2");
    assert_eq!(mov.score(), ProbeScore::SIGNATURE);

    let mov_mime = registry
        .probe(ProbeRequest::new(b"not mov").with_mime_type("Video/QuickTime"))
        .unwrap();
    assert_eq!(mov_mime.score(), ProbeScore::MIME_TYPE);
    assert!(registry
        .probe(ProbeRequest::new(b"RIFF....WAVE").with_extension("clip.bin"))
        .is_none());

    assert!(ProbeDescriptor::new("", &[], &[], &[]).is_err());
    assert!(ProbeDescriptor::new("bad\0name", &[], &[], &[]).is_err());
    assert!(ProbeDescriptor::new("bad-ext", &[""], &[], &[]).is_err());
    assert!(ProbeSignature::new(0, b"").is_err());
    assert!(registry.register(avi_probe_descriptor().unwrap()).is_err());
    assert!(mov_probe_descriptor().unwrap().signatures()[0].offset() > 0);
}

fn assert_match_is_explained(
    descriptor: &ProbeDescriptor,
    score: ProbeScore,
    request: ProbeRequest<'_>,
) {
    match score {
        ProbeScore::SIGNATURE => {
            assert!(descriptor
                .signatures()
                .iter()
                .any(|signature| signature_matches(signature, request.header())));
        }
        ProbeScore::MIME_TYPE => {
            let mime_type = request.mime_type().unwrap();
            assert!(descriptor
                .mime_types()
                .iter()
                .any(|known| known.eq_ignore_ascii_case(mime_type)));
        }
        ProbeScore::EXTENSION => {
            let extension = request.extension().unwrap();
            assert!(descriptor
                .extensions()
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension)));
        }
        _ => unreachable!("probe registry should not return zero-score matches"),
    }
}

fn signature_matches(signature: &ProbeSignature, header: &[u8]) -> bool {
    let Some(end) = signature.offset().checked_add(signature.bytes().len()) else {
        return false;
    };
    header.get(signature.offset()..end) == Some(signature.bytes())
}

fn assert_registry_invariants(registry: &ProbeRegistry) {
    for (index, descriptor) in registry.descriptors().iter().enumerate() {
        assert!(!descriptor.name().is_empty());
        assert!(!descriptor.name().as_bytes().contains(&0));
        assert!(registry.descriptors()[..index]
            .iter()
            .all(|known| !known.name().eq_ignore_ascii_case(descriptor.name())));
        assert!(descriptor
            .extensions()
            .iter()
            .all(|extension| !extension.is_empty() && !extension.as_bytes().contains(&0)));
        assert!(descriptor
            .mime_types()
            .iter()
            .all(|mime_type| !mime_type.is_empty() && !mime_type.as_bytes().contains(&0)));
        assert!(descriptor
            .signatures()
            .iter()
            .all(|signature| !signature.bytes().is_empty()));
    }
}

fn generated_descriptor(cursor: &mut Cursor<'_>) -> avutil::AvResult<ProbeDescriptor> {
    let name = descriptor_name_from(cursor);
    let extensions = strings_from(cursor, field_string_from);
    let mime_types = strings_from(cursor, mime_type_from);
    let signatures = byte_fields_from(cursor);
    let offset_signatures = offset_byte_fields_from(cursor);

    let extension_refs = extensions.iter().map(String::as_str).collect::<Vec<_>>();
    let mime_refs = mime_types.iter().map(String::as_str).collect::<Vec<_>>();
    let signature_refs = signatures.iter().map(Vec::as_slice).collect::<Vec<&[u8]>>();
    let offset_refs = offset_signatures
        .iter()
        .map(|(offset, bytes)| (*offset, bytes.as_slice()))
        .collect::<Vec<_>>();

    ProbeDescriptor::new_with_offset_signatures(
        name,
        &extension_refs,
        &mime_refs,
        &signature_refs,
        &offset_refs,
    )
}

fn strings_from(
    cursor: &mut Cursor<'_>,
    mut value_from: impl FnMut(&mut Cursor<'_>) -> String,
) -> Vec<String> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_FIELDS + 1);
    (0..count).map(|_| value_from(cursor)).collect()
}

fn byte_fields_from(cursor: &mut Cursor<'_>) -> Vec<Vec<u8>> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_FIELDS + 1);
    (0..count).map(|_| signature_bytes_from(cursor)).collect()
}

fn offset_byte_fields_from(cursor: &mut Cursor<'_>) -> Vec<(usize, Vec<u8>)> {
    let count = usize::from(cursor.next().unwrap_or_default()) % (MAX_FIELDS + 1);
    (0..count)
        .map(|_| {
            let offset = match cursor.next().unwrap_or_default() % 8 {
                0 => 0,
                1 => 4,
                2 => 8,
                3 => 16,
                4 => usize::MAX,
                _ => usize::from(cursor.next().unwrap_or_default()),
            };
            (offset, signature_bytes_from(cursor))
        })
        .collect()
}

fn descriptor_name_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 12 {
        0 => "avi".to_owned(),
        1 => "AVI".to_owned(),
        2 => "mov,mp4,m4a,3gp,3g2,mj2".to_owned(),
        3 => "wav".to_owned(),
        4 => "matroska".to_owned(),
        5 => "raw".to_owned(),
        6 => String::new(),
        7 => "bad\0name".to_owned(),
        _ => field_string_from(cursor),
    }
}

fn field_string_from(cursor: &mut Cursor<'_>) -> String {
    let len = usize::from(cursor.next().unwrap_or_default()) % (MAX_LITERAL_LEN + 1);
    let mut output = String::with_capacity(len);
    for _ in 0..len {
        output.push(match cursor.next().unwrap_or_default() % 18 {
            0 => 'a',
            1 => 'A',
            2 => 'v',
            3 => 'i',
            4 => 'm',
            5 => 'p',
            6 => '4',
            7 => 'w',
            8 => '.',
            9 => '/',
            10 => '\\',
            11 => '-',
            12 => '_',
            13 => '+',
            14 => ' ',
            15 => '\0',
            16 => 'x',
            _ => 'Z',
        });
    }
    output
}

fn mime_type_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 10 {
        0 => "video/x-msvideo".to_owned(),
        1 => "Video/X-MsVideo".to_owned(),
        2 => "video/quicktime".to_owned(),
        3 => "Video/QuickTime".to_owned(),
        4 => "video/mp4".to_owned(),
        5 => "application/octet-stream".to_owned(),
        6 => String::new(),
        7 => "bad\0mime".to_owned(),
        _ => field_string_from(cursor),
    }
}

fn path_from(cursor: &mut Cursor<'_>) -> String {
    match cursor.next().unwrap_or_default() % 12 {
        0 => "clip.avi".to_owned(),
        1 => "CLIP.AVI".to_owned(),
        2 => "clip.mp4".to_owned(),
        3 => "clip.mov".to_owned(),
        4 => "clip.bin".to_owned(),
        5 => r"C:\media\CLIP.MP4".to_owned(),
        6 => "/tmp/clip".to_owned(),
        7 => ".hidden".to_owned(),
        8 => "trailing.".to_owned(),
        _ => field_string_from(cursor),
    }
}

fn header_from(cursor: &mut Cursor<'_>) -> Vec<u8> {
    match cursor.next().unwrap_or_default() % 8 {
        0 => b"RIFF\0\0\0\0AVI ".to_vec(),
        1 => b"RIFF....WAVE".to_vec(),
        2 => b"\0\0\0\x18ftypisom".to_vec(),
        3 => b"xxxxftypmp42".to_vec(),
        _ => {
            let len = usize::from(cursor.next().unwrap_or_default()) % (MAX_HEADER_LEN + 1);
            (0..len)
                .map(|_| cursor.next().unwrap_or_default())
                .collect()
        }
    }
}

fn signature_bytes_from(cursor: &mut Cursor<'_>) -> Vec<u8> {
    match cursor.next().unwrap_or_default() % 10 {
        0 => b"RIFF".to_vec(),
        1 => b"AVI ".to_vec(),
        2 => b"ftyp".to_vec(),
        3 => b"WAVE".to_vec(),
        4 => b"\0\0\0\x18".to_vec(),
        5 => Vec::new(),
        _ => {
            let len = usize::from(cursor.next().unwrap_or_default()) % (MAX_SIGNATURE_LEN + 1);
            (0..len)
                .map(|_| cursor.next().unwrap_or_default())
                .collect()
        }
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
