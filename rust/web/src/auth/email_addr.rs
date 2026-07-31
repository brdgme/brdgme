/// An email address in canonical form (trimmed, lowercased). The only way to
/// build one is `canonicalize_email`, so a `CanonicalEmail` is always canonical
/// by construction. Store and compare this type - never a raw `String`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalEmail(String);

impl CanonicalEmail {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for CanonicalEmail {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

pub fn canonicalize_email(raw: &str) -> CanonicalEmail {
    CanonicalEmail(raw.trim().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_trims_and_lowercases() {
        assert_eq!(canonicalize_email(" Foo@X.COM ").as_str(), "foo@x.com");
    }

    #[test]
    fn canonicalize_leaves_canonical_input_unchanged() {
        assert_eq!(canonicalize_email("foo@x.com").as_str(), "foo@x.com");
    }

    #[test]
    fn canonicalize_whitespace_only_becomes_empty() {
        assert_eq!(canonicalize_email("   ").as_str(), "");
    }
}
