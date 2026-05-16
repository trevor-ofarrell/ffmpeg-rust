use avutil::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProbeScore(u8);

impl ProbeScore {
    pub const NONE: Self = Self(0);
    pub const EXTENSION: Self = Self(50);
    pub const MIME_TYPE: Self = Self(75);
    pub const SIGNATURE: Self = Self(100);
    pub const MAX: Self = Self(100);

    pub fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeDescriptor {
    name: String,
    extensions: Vec<String>,
    mime_types: Vec<String>,
    signatures: Vec<Vec<u8>>,
}

impl ProbeDescriptor {
    pub fn new(
        name: impl Into<String>,
        extensions: &[&str],
        mime_types: &[&str],
        signatures: &[&[u8]],
    ) -> AvResult<Self> {
        let name = validate_text("probe descriptor name", name.into())?;
        let extensions = extensions
            .iter()
            .map(|extension| normalize_extension(extension))
            .collect::<AvResult<Vec<_>>>()?;
        let mime_types = mime_types
            .iter()
            .map(|mime_type| validate_text("probe MIME type", mime_type.to_ascii_lowercase()))
            .collect::<AvResult<Vec<_>>>()?;
        let signatures = signatures
            .iter()
            .map(validate_signature)
            .collect::<AvResult<Vec<_>>>()?;

        Ok(Self {
            name,
            extensions,
            mime_types,
            signatures,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    pub fn mime_types(&self) -> &[String] {
        &self.mime_types
    }

    pub fn signatures(&self) -> &[Vec<u8>] {
        &self.signatures
    }

    fn score(&self, request: &ProbeRequest<'_>) -> ProbeScore {
        if self
            .signatures
            .iter()
            .any(|signature| request.header.starts_with(signature))
        {
            return ProbeScore::SIGNATURE;
        }

        if let Some(mime_type) = request.mime_type {
            if self
                .mime_types
                .iter()
                .any(|known| known.eq_ignore_ascii_case(mime_type))
            {
                return ProbeScore::MIME_TYPE;
            }
        }

        if let Some(extension) = request.extension {
            if self
                .extensions
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
            {
                return ProbeScore::EXTENSION;
            }
        }

        ProbeScore::NONE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeRequest<'a> {
    header: &'a [u8],
    extension: Option<&'a str>,
    mime_type: Option<&'a str>,
}

impl<'a> ProbeRequest<'a> {
    pub fn new(header: &'a [u8]) -> Self {
        Self {
            header,
            extension: None,
            mime_type: None,
        }
    }

    pub fn with_extension(mut self, extension_or_path: &'a str) -> Self {
        self.extension = extension_from_path(extension_or_path);
        self
    }

    pub fn with_mime_type(mut self, mime_type: &'a str) -> Self {
        self.mime_type = Some(mime_type);
        self
    }

    pub fn header(&self) -> &'a [u8] {
        self.header
    }

    pub fn extension(&self) -> Option<&'a str> {
        self.extension
    }

    pub fn mime_type(&self) -> Option<&'a str> {
        self.mime_type
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeMatch<'a> {
    descriptor: &'a ProbeDescriptor,
    score: ProbeScore,
}

impl<'a> ProbeMatch<'a> {
    pub fn descriptor(&self) -> &'a ProbeDescriptor {
        self.descriptor
    }

    pub fn score(&self) -> ProbeScore {
        self.score
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProbeRegistry {
    descriptors: Vec<ProbeDescriptor>,
}

impl ProbeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn descriptors(&self) -> &[ProbeDescriptor] {
        &self.descriptors
    }

    pub fn register(&mut self, descriptor: ProbeDescriptor) -> AvResult<()> {
        if self
            .descriptors
            .iter()
            .any(|known| known.name().eq_ignore_ascii_case(descriptor.name()))
        {
            return Err(AvError::invalid_argument(format!(
                "duplicate probe descriptor `{}`",
                descriptor.name()
            )));
        }

        self.descriptors.push(descriptor);
        Ok(())
    }

    pub fn probe(&self, request: ProbeRequest<'_>) -> Option<ProbeMatch<'_>> {
        let mut best = None;
        let mut best_score = ProbeScore::NONE;

        for descriptor in &self.descriptors {
            let score = descriptor.score(&request);
            if score > best_score {
                best = Some(ProbeMatch { descriptor, score });
                best_score = score;
            }
        }

        best
    }
}

fn validate_text(label: &str, value: String) -> AvResult<String> {
    if value.is_empty() {
        return Err(AvError::invalid_argument(format!(
            "{label} must not be empty"
        )));
    }

    if value.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(format!(
            "{label} must not contain NUL"
        )));
    }

    Ok(value)
}

fn validate_signature(signature: &&[u8]) -> AvResult<Vec<u8>> {
    if signature.is_empty() {
        return Err(AvError::invalid_argument(
            "probe signature must not be empty",
        ));
    }

    Ok(signature.to_vec())
}

fn normalize_extension(extension: &str) -> AvResult<String> {
    let extension = extension.trim_start_matches('.');
    validate_text("probe extension", extension.to_ascii_lowercase())
}

fn extension_from_path(path: &str) -> Option<&str> {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let extension = file_name
        .rsplit_once('.')
        .map_or(file_name, |(_, extension)| extension);
    (!extension.is_empty()).then_some(extension.trim_start_matches('.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::AvErrorKind;

    #[test]
    fn descriptor_validation_rejects_empty_or_invalid_fields() {
        assert_eq!(
            ProbeDescriptor::new("", &[], &[], &[]).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert!(ProbeDescriptor::new("bad\0name", &[], &[], &[]).is_err());
        assert!(ProbeDescriptor::new("wav", &[""], &[], &[]).is_err());
        assert!(ProbeDescriptor::new("wav", &[], &[""], &[]).is_err());
        assert!(ProbeDescriptor::new("wav", &[], &[], &[b""]).is_err());
    }

    #[test]
    fn signature_match_scores_higher_than_extension() {
        let mut registry = ProbeRegistry::new();
        registry
            .register(descriptor("mp3", &["mp3"], &[], &[]))
            .unwrap();
        registry
            .register(descriptor("wav", &["wav"], &["audio/wav"], &[b"RIFF"]))
            .unwrap();

        let matched = registry
            .probe(ProbeRequest::new(b"RIFF....WAVE").with_extension("song.mp3"))
            .unwrap();

        assert_eq!(matched.descriptor().name(), "wav");
        assert_eq!(matched.score(), ProbeScore::SIGNATURE);
    }

    #[test]
    fn mime_type_scores_higher_than_extension() {
        let mut registry = ProbeRegistry::new();
        registry
            .register(descriptor("raw", &["bin"], &[], &[]))
            .unwrap();
        registry
            .register(descriptor("matroska", &["mkv"], &["video/x-matroska"], &[]))
            .unwrap();

        let matched = registry
            .probe(
                ProbeRequest::new(b"")
                    .with_extension("capture.bin")
                    .with_mime_type("Video/X-Matroska"),
            )
            .unwrap();

        assert_eq!(matched.descriptor().name(), "matroska");
        assert_eq!(matched.score(), ProbeScore::MIME_TYPE);
    }

    #[test]
    fn extension_matching_is_case_insensitive_and_path_aware() {
        let mut registry = ProbeRegistry::new();
        registry
            .register(descriptor("waveform", &["WAV"], &[], &[]))
            .unwrap();

        let matched = registry
            .probe(ProbeRequest::new(b"").with_extension(r"C:\media\CLIP.WAV"))
            .unwrap();

        assert_eq!(matched.descriptor().name(), "waveform");
        assert_eq!(matched.score(), ProbeScore::EXTENSION);
    }

    #[test]
    fn duplicate_descriptor_names_are_rejected_case_insensitively() {
        let mut registry = ProbeRegistry::new();
        registry.register(descriptor("wav", &[], &[], &[])).unwrap();

        let err = registry
            .register(descriptor("WAV", &[], &[], &[]))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    }

    #[test]
    fn no_match_returns_none_and_ties_keep_registration_order() {
        let mut registry = ProbeRegistry::new();
        registry
            .register(descriptor("first", &["dat"], &[], &[]))
            .unwrap();
        registry
            .register(descriptor("second", &["dat"], &[], &[]))
            .unwrap();

        assert!(registry.probe(ProbeRequest::new(b"")).is_none());

        let matched = registry
            .probe(ProbeRequest::new(b"").with_extension("sample.dat"))
            .unwrap();
        assert_eq!(matched.descriptor().name(), "first");
    }

    fn descriptor(
        name: &str,
        extensions: &[&str],
        mime_types: &[&str],
        signatures: &[&[u8]],
    ) -> ProbeDescriptor {
        ProbeDescriptor::new(name, extensions, mime_types, signatures).unwrap()
    }
}
