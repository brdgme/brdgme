/// Wraps text to a maximum width, preserving existing newlines.
pub fn word_wrap(s: &str, width: usize) -> String {
    s.split('\n')
        .map(|segment| wrap_segment(segment, width))
        .collect::<Vec<String>>()
        .join("\n")
}

fn wrap_segment(s: &str, width: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in s.split(' ') {
        if current.is_empty() {
            current = word.to_owned();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_owned();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_string_unchanged() {
        assert_eq!(word_wrap("hello", 10), "hello");
    }

    #[test]
    fn exact_width_boundary() {
        assert_eq!(word_wrap("hello", 5), "hello");
        assert_eq!(word_wrap("hi there", 8), "hi there");
    }

    #[test]
    fn wrap_on_space() {
        assert_eq!(word_wrap("hello world foo", 11), "hello world\nfoo");
        assert_eq!(word_wrap("a b c d e", 3), "a b\nc d\ne");
    }

    #[test]
    fn existing_newlines_preserved() {
        assert_eq!(word_wrap("hello\nworld", 20), "hello\nworld");
        assert_eq!(
            word_wrap("aaa bbb ccc\nddd eee", 7),
            "aaa bbb\nccc\nddd eee"
        );
    }

    #[test]
    fn overlong_token_unbroken() {
        assert_eq!(word_wrap("superlongword", 5), "superlongword");
        assert_eq!(
            word_wrap("hi superlongword bye", 5),
            "hi\nsuperlongword\nbye"
        );
    }
}
