use crate::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    CaseInsensitive,
    CaseSensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetMode {
    Overwrite,
    KeepExisting,
    Append,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionarySet {
    Inserted,
    Replaced,
    Kept,
    Appended,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryEntry {
    key: String,
    value: String,
}

impl DictionaryEntry {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dictionary {
    entries: Vec<DictionaryEntry>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[DictionaryEntry] {
        &self.entries
    }

    pub fn set(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> AvResult<DictionarySet> {
        self.set_with_mode(key, value, MatchMode::CaseInsensitive, SetMode::Overwrite)
    }

    pub fn set_with_mode(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        match_mode: MatchMode,
        set_mode: SetMode,
    ) -> AvResult<DictionarySet> {
        let key = validate_key(key.into())?;
        let value = validate_value(value.into())?;

        if let Some(index) = self.find_index(&key, match_mode) {
            let entry = &mut self.entries[index];
            return match set_mode {
                SetMode::Overwrite => {
                    entry.key = key;
                    entry.value = value;
                    Ok(DictionarySet::Replaced)
                }
                SetMode::KeepExisting => Ok(DictionarySet::Kept),
                SetMode::Append => {
                    entry.value.push_str(&value);
                    Ok(DictionarySet::Appended)
                }
            };
        }

        self.entries.push(DictionaryEntry { key, value });
        Ok(DictionarySet::Inserted)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.get_entry(key, MatchMode::CaseInsensitive)
            .map(DictionaryEntry::value)
    }

    pub fn get_entry(&self, key: &str, match_mode: MatchMode) -> Option<&DictionaryEntry> {
        self.entries
            .iter()
            .find(|entry| key_matches(entry.key(), key, match_mode))
    }

    pub fn get_prefixed(&self, prefix: &str, match_mode: MatchMode) -> Option<&DictionaryEntry> {
        self.entries
            .iter()
            .find(|entry| key_has_prefix(entry.key(), prefix, match_mode))
    }

    pub fn remove(&mut self, key: &str, match_mode: MatchMode) -> Option<DictionaryEntry> {
        self.find_index(key, match_mode)
            .map(|index| self.entries.remove(index))
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn find_index(&self, key: &str, match_mode: MatchMode) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| key_matches(entry.key(), key, match_mode))
    }
}

fn validate_key(key: String) -> AvResult<String> {
    if key.is_empty() {
        return Err(AvError::invalid_argument(
            "dictionary key must not be empty",
        ));
    }

    if key.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(
            "dictionary key must not contain NUL",
        ));
    }

    Ok(key)
}

fn validate_value(value: String) -> AvResult<String> {
    if value.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(
            "dictionary value must not contain NUL",
        ));
    }

    Ok(value)
}

fn key_matches(candidate: &str, key: &str, match_mode: MatchMode) -> bool {
    match match_mode {
        MatchMode::CaseSensitive => candidate == key,
        MatchMode::CaseInsensitive => ascii_eq_ignore_case(candidate, key),
    }
}

fn key_has_prefix(candidate: &str, prefix: &str, match_mode: MatchMode) -> bool {
    match match_mode {
        MatchMode::CaseSensitive => candidate.starts_with(prefix),
        MatchMode::CaseInsensitive => {
            candidate.len() >= prefix.len()
                && ascii_eq_ignore_case_bytes(
                    &candidate.as_bytes()[..prefix.len()],
                    prefix.as_bytes(),
                )
        }
    }
}

fn ascii_eq_ignore_case(left: &str, right: &str) -> bool {
    left.len() == right.len() && ascii_eq_ignore_case_bytes(left.as_bytes(), right.as_bytes())
}

fn ascii_eq_ignore_case_bytes(left: &[u8], right: &[u8]) -> bool {
    left.iter()
        .zip(right)
        .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn default_set_replaces_case_insensitive_key() {
        let mut dict = Dictionary::new();

        assert_eq!(dict.set("Title", "First").unwrap(), DictionarySet::Inserted);
        assert_eq!(
            dict.set("title", "Second").unwrap(),
            DictionarySet::Replaced
        );

        assert_eq!(dict.len(), 1);
        assert_eq!(dict.get("TITLE"), Some("Second"));
        assert_eq!(dict.entries()[0].key(), "title");
    }

    #[test]
    fn case_sensitive_mode_allows_distinct_keys() {
        let mut dict = Dictionary::new();

        dict.set_with_mode(
            "TITLE",
            "upper",
            MatchMode::CaseSensitive,
            SetMode::Overwrite,
        )
        .unwrap();
        dict.set_with_mode(
            "title",
            "lower",
            MatchMode::CaseSensitive,
            SetMode::Overwrite,
        )
        .unwrap();

        assert_eq!(dict.len(), 2);
        assert_eq!(
            dict.get_entry("TITLE", MatchMode::CaseSensitive)
                .map(DictionaryEntry::value),
            Some("upper")
        );
        assert_eq!(
            dict.get_entry("title", MatchMode::CaseSensitive)
                .map(DictionaryEntry::value),
            Some("lower")
        );
    }

    #[test]
    fn keep_existing_preserves_original_entry() {
        let mut dict = Dictionary::new();

        dict.set("artist", "first").unwrap();
        let result = dict
            .set_with_mode(
                "ARTIST",
                "second",
                MatchMode::CaseInsensitive,
                SetMode::KeepExisting,
            )
            .unwrap();

        assert_eq!(result, DictionarySet::Kept);
        assert_eq!(dict.get("artist"), Some("first"));
        assert_eq!(dict.entries()[0].key(), "artist");
    }

    #[test]
    fn append_mode_concatenates_existing_value() {
        let mut dict = Dictionary::new();

        dict.set("comment", "part1").unwrap();
        let result = dict
            .set_with_mode(
                "COMMENT",
                "+part2",
                MatchMode::CaseInsensitive,
                SetMode::Append,
            )
            .unwrap();

        assert_eq!(result, DictionarySet::Appended);
        assert_eq!(dict.get("comment"), Some("part1+part2"));
    }

    #[test]
    fn prefix_lookup_returns_first_matching_entry() {
        let mut dict = Dictionary::new();
        dict.set("artist-sort", "Lastname").unwrap();
        dict.set("album", "Record").unwrap();

        let entry = dict
            .get_prefixed("ARTIST", MatchMode::CaseInsensitive)
            .unwrap();

        assert_eq!(entry.key(), "artist-sort");
        assert_eq!(entry.value(), "Lastname");
        assert!(dict
            .get_prefixed("ARTIST", MatchMode::CaseSensitive)
            .is_none());
    }

    #[test]
    fn remove_defaults_to_requested_match_mode() {
        let mut dict = Dictionary::new();
        dict.set("language", "eng").unwrap();

        assert!(dict.remove("LANGUAGE", MatchMode::CaseSensitive).is_none());
        let removed = dict.remove("LANGUAGE", MatchMode::CaseInsensitive).unwrap();

        assert_eq!(removed.key(), "language");
        assert!(dict.is_empty());
    }

    #[test]
    fn rejects_empty_or_nul_keys_and_nul_values() {
        let mut dict = Dictionary::new();

        let empty_key = dict.set("", "value").unwrap_err();
        let nul_key = dict.set("bad\0key", "value").unwrap_err();
        let nul_value = dict.set("key", "bad\0value").unwrap_err();

        assert_eq!(empty_key.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(nul_key.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(nul_value.kind(), AvErrorKind::InvalidArgument);
        assert!(dict.is_empty());
    }
}
