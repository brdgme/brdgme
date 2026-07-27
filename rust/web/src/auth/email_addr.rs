/// Canonical form of an email address: trimmed, lowercased. Every boundary
/// that stores or looks up an address must call this first.
pub fn canonicalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_trims_and_lowercases() {
        assert_eq!(canonicalize_email(" Foo@X.COM "), "foo@x.com");
    }

    #[test]
    fn canonicalize_leaves_canonical_input_unchanged() {
        assert_eq!(canonicalize_email("foo@x.com"), "foo@x.com");
    }

    #[test]
    fn canonicalize_whitespace_only_becomes_empty() {
        assert_eq!(canonicalize_email("   "), "");
    }
}
