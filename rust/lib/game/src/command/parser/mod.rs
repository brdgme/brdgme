use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;

use unicase::UniCase;

use crate::command::Spec as CommandSpec;
use crate::errors::GameError;

pub use self::chain::*;

pub mod chain;

#[derive(Debug, PartialEq)]
pub struct Output<'a, T> {
    pub value: T,
    pub consumed: &'a str,
    pub remaining: &'a str,
}

pub trait Parser {
    type T;

    /// `GameError::Parse.offset` is the byte position of the failure measured
    /// from the start of the input slice this parser was given. Leaf parsers
    /// report 0. Only `Chain` adds.
    fn parse<'a>(&self, input: &'a str, names: &[String])
    -> Result<Output<'a, Self::T>, GameError>;
    fn expected(&self, names: &[String]) -> Vec<String>;
    fn to_spec(&self) -> CommandSpec;
}

pub struct Token {
    pub token: String,
}

impl Token {
    pub fn new<T>(token: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            token: token.into(),
        }
    }
}

impl Parser for Token {
    type T = String;

    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, String>, GameError> {
        let t_len = self.token.len();
        // get() returns None when t_len exceeds the input or is not a char
        // boundary of the input; both are mismatches, never panics.
        match input.get(..t_len) {
            Some(prefix) if UniCase::new(prefix) == UniCase::new(&self.token) => Ok(Output {
                value: self.token.to_owned(),
                consumed: prefix,
                remaining: &input[t_len..],
            }),
            _ => Err(GameError::Parse {
                message: None,
                expected: self.expected(names),
                offset: 0,
            }),
        }
    }

    fn expected(&self, _names: &[String]) -> Vec<String> {
        vec![self.token.to_owned()]
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Token(self.token.to_owned())
    }
}

pub struct Int {
    pub min: Option<i32>,
    pub max: Option<i32>,
}

impl Int {
    pub fn any() -> Self {
        Int {
            min: None,
            max: None,
        }
    }

    pub fn positive() -> Self {
        Int {
            min: Some(1),
            max: None,
        }
    }

    pub fn not_negative() -> Self {
        Int {
            min: Some(0),
            max: None,
        }
    }

    pub fn bounded(min: i32, max: i32) -> Self {
        Int {
            min: Some(min),
            max: Some(max),
        }
    }

    fn expected_output(&self) -> String {
        match (self.min, self.max) {
            (None, None) => "number".to_string(),
            (Some(min), None) => format!("number {} or higher", min),
            (None, Some(max)) => format!("number {} or lower", max),
            (Some(min), Some(max)) => format!("number between {} and {}", min, max),
        }
    }
}

impl Parser for Int {
    type T = i32;

    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, i32>, GameError> {
        let mut found_digit = false;
        // Byte length of the accepted prefix. The accepted chars are all
        // 1-byte ASCII today, but a byte length keeps this slice-safe if the
        // accepted set ever grows (see the Space/Enum multi-byte panics this
        // file previously had).
        let consumed_len = input
            .char_indices()
            .take_while(|&(i, c)| {
                if i == 0 && c == '-' {
                    true
                } else if c.is_ascii_digit() {
                    found_digit = true;
                    true
                } else {
                    false
                }
            })
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        if !found_digit {
            return Err(GameError::Parse {
                message: None,
                expected: self.expected(names),
                offset: 0,
            });
        }
        let consumed = &input[..consumed_len];
        let value: i32 = consumed.parse().map_err(|_| GameError::Parse {
            message: Some(format!("failed to parse '{}'", consumed)),
            expected: self.expected(names),
            offset: 0,
        })?;
        if let Some(min) = self.min
            && value < min
        {
            return Err(GameError::Parse {
                message: Some(format!("{} is too low", value)),
                expected: self.expected(names),
                offset: 0,
            });
        }
        if let Some(max) = self.max
            && value > max
        {
            return Err(GameError::Parse {
                message: Some(format!("{} is too high", value)),
                expected: self.expected(names),
                offset: 0,
            });
        }
        Ok(Output {
            value,
            consumed,
            remaining: &input[consumed_len..],
        })
    }

    fn expected(&self, _names: &[String]) -> Vec<String> {
        vec![self.expected_output()]
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Int {
            min: self.min,
            max: self.max,
        }
    }
}

pub struct Map<T, O, F, TP>
where
    F: Fn(T) -> O,
    TP: Parser<T = T>,
{
    pub parser: TP,
    pub map: F,
}

impl<T, O, F, TP> Map<T, O, F, TP>
where
    F: Fn(T) -> O,
    TP: Parser<T = T>,
{
    pub fn new(parser: TP, map: F) -> Self {
        Self { parser, map }
    }
}

impl<T, O, F, TP> Parser for Map<T, O, F, TP>
where
    F: Fn(T) -> O,
    TP: Parser<T = T>,
{
    type T = O;

    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, O>, GameError> {
        let child_parse = self.parser.parse(input, names)?;
        Ok(Output {
            value: (self.map)(child_parse.value),
            consumed: child_parse.consumed,
            remaining: child_parse.remaining,
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.parser.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        self.parser.to_spec()
    }
}

pub struct Opt<TP>
where
    TP: Parser,
{
    pub parser: TP,
}

impl<TP> Opt<TP>
where
    TP: Parser,
{
    pub fn new(parser: TP) -> Self {
        Self { parser }
    }
}

impl<T, TP> Parser for Opt<TP>
where
    TP: Parser<T = T>,
{
    type T = Option<T>;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        Ok(match self.parser.parse(input, names) {
            Ok(output) => Output {
                value: Some(output.value),
                consumed: output.consumed,
                remaining: output.remaining,
            },
            Err(_) => Output {
                value: None,
                consumed: &input[..0],
                remaining: input,
            },
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.parser
            .expected(names)
            .iter()
            .map(|e| format!("optional {}", e))
            .collect()
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Opt(Box::new(self.parser.to_spec()))
    }
}

/// Repetition combinator.
///
/// Progress invariant: every iteration of the parse loop must consume at
/// least one byte of input (via the delimiter or the item). A parser that
/// succeeds consuming nothing (`Opt`, `Token::new("")`, an empty `Chain`)
/// would otherwise loop forever, so both this impl and the `Spec::Many`
/// impl stop as soon as an iteration makes no progress.
pub struct Many<TP, DP>
where
    TP: Parser,
    DP: Parser,
{
    pub parser: TP,
    pub min: Option<usize>,
    pub max: Option<usize>,
    pub delim: Option<DP>,
}

impl<TP> Many<TP, Space>
where
    TP: Parser,
{
    pub fn any_spaced(parser: TP) -> Self {
        Self {
            parser,
            min: None,
            max: None,
            delim: Some(Space {}),
        }
    }

    pub fn some_spaced(parser: TP) -> Self {
        Self {
            parser,
            min: Some(1),
            max: None,
            delim: Some(Space {}),
        }
    }

    pub fn bounded_spaced(parser: TP, min: usize, max: usize) -> Self {
        Self {
            parser,
            min: Some(min),
            max: Some(max),
            delim: Some(Space {}),
        }
    }
}

impl<TP, DP> Parser for Many<TP, DP>
where
    TP: Parser,
    DP: Parser,
{
    type T = Vec<TP::T>;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        let mut parsed: Self::T = vec![];
        let mut first = true;
        let mut offset = 0;
        'outer: loop {
            // Checked at the top of the loop exactly like the spec impl
            // (`CommandSpec::Many`), so degenerate configs (`max == 0`, or
            // `max < min`) fall through to the min check below instead of
            // returning early with an empty Ok (lg F8).
            if let Some(max) = self.max
                && parsed.len() >= max
            {
                break 'outer;
            }
            let mut inner_offset = offset;
            if !first {
                if let Some(d) = self.delim.as_ref() {
                    match d.parse(&input[offset..], names) {
                        Ok(Output { consumed, .. }) => inner_offset += consumed.len(),
                        Err(_) => break 'outer,
                    };
                }
            } else {
                first = false;
            }
            match self.parser.parse(&input[inner_offset..], names) {
                Ok(Output {
                    value, consumed, ..
                }) => {
                    parsed.push(value);
                    let new_offset = inner_offset + consumed.len();
                    // Progress invariant (see the struct doc comment): stop
                    // when neither the delimiter nor the item consumed
                    // anything, otherwise this loop never ends (lg F6).
                    let progressed = new_offset > offset;
                    offset = new_offset;
                    if !progressed {
                        break 'outer;
                    }
                }
                Err(_) => {
                    break 'outer;
                }
            };
        }
        if let Some(min) = self.min
            && parsed.len() < min
        {
            return Err(GameError::Parse {
                message: Some(format!(
                    "expected at least {} items but could only parse {}",
                    min,
                    parsed.len()
                )),
                expected: vec![],
                offset,
            });
        }
        Ok(Output {
            value: parsed,
            consumed: &input[..offset],
            remaining: &input[offset..],
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        many_expected(self.parser.expected(names), self.min, self.max)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Many {
            spec: Box::new(self.parser.to_spec()),
            min: self.min,
            max: self.max,
            delim: self.delim.as_ref().map(|d| Box::new(d.to_spec())),
        }
    }
}

pub struct Space {}

impl Parser for Space {
    type T = String;

    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, String>, GameError> {
        // Byte length of the leading whitespace run. trim_start strips the
        // same set of chars as char::is_whitespace, so this is always a char
        // boundary; a char count here would byte-slice mid-char on multi-byte
        // whitespace such as U+00A0 NBSP.
        let consumed = input.len() - input.trim_start().len();
        if consumed == 0 {
            return Err(GameError::Parse {
                message: None,
                expected: self.expected(names),
                offset: 0,
            });
        }
        Ok(Output {
            value: input[..consumed].to_owned(),
            consumed: &input[..consumed],
            remaining: &input[consumed..],
        })
    }

    fn expected(&self, _names: &[String]) -> Vec<String> {
        vec!["whitespace".to_string()]
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Space
    }
}

pub struct OneOf<TP: Parser + ?Sized> {
    pub parsers: Vec<Box<TP>>,
}

impl<TP: Parser + ?Sized> OneOf<TP> {
    pub fn new(parsers: Vec<Box<TP>>) -> Self {
        Self { parsers }
    }
}

impl<TP: Parser + ?Sized> Parser for OneOf<TP> {
    type T = TP::T;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        let mut errors: Vec<GameError> = vec![];
        let mut error_consumed: usize = 0;
        for p in &self.parsers {
            match p.parse(input, names) {
                Ok(output) => return Ok(output),
                Err(e) => {
                    let mut e_consumed = 0;
                    if let GameError::Parse { offset, .. } = e {
                        e_consumed = offset;
                    }
                    match e_consumed.cmp(&error_consumed) {
                        Ordering::Greater => {
                            errors = vec![e];
                            error_consumed = e_consumed;
                        }
                        Ordering::Equal => errors.push(e),
                        _ => {}
                    }
                }
            }
        }

        let error_messages = &errors
            .iter()
            .filter_map(|e| {
                if let GameError::Parse { ref message, .. } = *e {
                    message.to_owned()
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();
        Err(GameError::Parse {
            message: if error_messages.is_empty() {
                None
            } else {
                Some(comma_list_or(error_messages))
            },
            expected: errors
                .iter()
                .flat_map(|e| match *e {
                    GameError::Parse { ref expected, .. } => expected.clone(),
                    _ => vec![],
                })
                .collect(),
            offset: error_consumed,
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.parsers
            .iter()
            .flat_map(|p| p.expected(names))
            .collect()
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::OneOf(self.parsers.iter().map(|p| p.to_spec()).collect())
    }
}

pub fn comma_list<T: fmt::Display>(items: &[T], last: &str) -> String {
    match items.len() {
        0 => "".to_string(),
        1 => format!("{}", items[0]),
        2 => format!("{} {} {}", items[0], last, items[1]),
        _ => format!("{}, {}", items[0], comma_list(&items[1..], last)),
    }
}

pub fn comma_list_or<T: fmt::Display>(items: &[T]) -> String {
    comma_list(items, "or")
}

pub fn comma_list_and<T: fmt::Display>(items: &[T]) -> String {
    comma_list(items, "and")
}

pub(crate) fn add_offset(e: GameError, by: usize) -> GameError {
    match e {
        GameError::Parse {
            message,
            expected,
            offset,
        } => GameError::Parse {
            message,
            expected,
            offset: offset + by,
        },
        other => other,
    }
}

pub(crate) fn many_expected(
    inner: Vec<String>,
    min: Option<usize>,
    max: Option<usize>,
) -> Vec<String> {
    inner
        .iter()
        .map(|e| match (min, max) {
            (None, None) => format!("any number of {}", e),
            (Some(min), None) => format!("{} or more {}", min, e),
            (None, Some(max)) => format!("up to {} {}", max, e),
            (Some(min), Some(max)) => format!("between {} and {} {}", min, max, e),
        })
        .collect()
}

pub struct Enum<T>
where
    T: ToString + Clone,
{
    pub values: Vec<T>,
    pub exact: bool,
}

impl<T> Enum<T>
where
    T: ToString + Clone,
{
    pub fn exact(values: Vec<T>) -> Self {
        Self {
            values,
            exact: true,
        }
    }

    pub fn partial(values: Vec<T>) -> Self {
        Self {
            values,
            exact: false,
        }
    }
}

/// Case-insensitive shared prefix of `input` and `value`, compared per char
/// via char::to_lowercase. Returns byte lengths `(input_bytes, value_bytes)`
/// of the matched prefix in each ORIGINAL string; both are char boundaries
/// of their own string, so they are safe slice indices. Byte lengths are
/// tracked separately because case-insensitively equal prefixes can differ
/// in byte length between the two strings.
fn shared_prefix(input: &str, value: &str) -> (usize, usize) {
    let mut input_bytes = 0usize;
    let mut value_bytes = 0usize;
    let mut vi = value.chars();
    for ic in input.chars() {
        match vi.next() {
            Some(vc) if ic.to_lowercase().eq(vc.to_lowercase()) => {
                input_bytes += ic.len_utf8();
                value_bytes += vc.len_utf8();
            }
            _ => break,
        }
    }
    (input_bytes, value_bytes)
}

impl<T> Parser for Enum<T>
where
    T: ToString + Clone,
{
    type T = T;
    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        let mut matched: Vec<&T> = vec![];
        // Byte length of `input` consumed by the current best match(es).
        let mut match_len: usize = 0;
        // Candidates are ranked by (bytes of input matched, then whether the
        // whole value was matched). Longest wins; a full match only breaks a
        // tie against an equal-length partial match. Replacing on a strictly
        // better key - rather than appending on ties - is what makes the
        // outcome independent of value declaration order (lg F5).
        let mut full_match = false;
        // Track which values have been searched to avoid duplicates.
        let mut searched: HashSet<String> = HashSet::new();
        for v in &self.values {
            let v_str = v.clone().to_string();
            let v_key = v_str.to_lowercase();
            if searched.contains(&v_key) {
                // This is a duplicate, skip it.
                continue;
            }
            searched.insert(v_key);
            let (matching, v_matching) = shared_prefix(input, &v_str);
            // Whether the whole value was matched, measured in the value's
            // own bytes (comparing input bytes to value bytes would misfire
            // whenever case folding changes byte length).
            let full = v_matching == v_str.len();
            if self.exact && !full {
                // The input isn't long enough and we require exact match, skip it.
                continue;
            }
            if matching == 0 {
                continue;
            }
            match (matching.cmp(&match_len), full.cmp(&full_match)) {
                // Strictly longer match: it becomes the sole candidate.
                (Ordering::Greater, _) => {
                    matched = vec![v];
                    match_len = matching;
                    full_match = full;
                }
                // Same length, but a full match beats a partial one.
                (Ordering::Equal, Ordering::Greater) => {
                    matched = vec![v];
                    full_match = full;
                }
                // Genuinely ambiguous: same length, same match kind.
                (Ordering::Equal, Ordering::Equal) => matched.push(v),
                // Shorter, or an equal-length partial against a full match.
                _ => {}
            }
        }
        match matched.len() {
            1 => Ok(Output {
                value: matched[0].to_owned(),
                consumed: &input[..match_len],
                remaining: &input[match_len..],
            }),
            0 => Err(GameError::Parse {
                message: None,
                expected: self.expected(names),
                offset: 0,
            }),
            _ => Err(GameError::Parse {
                message: Some(format!(
                    "matched {}, more input is required to uniquely match one",
                    comma_list_and(
                        &matched
                            .iter()
                            .map(|m| m.to_string())
                            .collect::<Vec<String>>()
                    ),
                )),
                expected: self.expected(names),
                offset: 0,
            }),
        }
    }

    fn expected(&self, _names: &[String]) -> Vec<String> {
        let mut values = self
            .values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>();
        values.sort();
        values
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Enum {
            values: self.values.iter().map(|v| v.to_string()).collect(),
            exact: self.exact,
        }
    }
}

pub struct Doc<TP: Parser> {
    pub name: String,
    pub desc: Option<String>,
    pub parser: TP,
}

impl<TP: Parser> Doc<TP> {
    pub fn name<I: Into<String>>(name: I, parser: TP) -> Self {
        Self {
            name: name.into(),
            desc: None,
            parser,
        }
    }

    pub fn name_desc<I: Into<String>>(name: I, desc: I, parser: TP) -> Self {
        Self {
            name: name.into(),
            desc: Some(desc.into()),
            parser,
        }
    }
}

impl<TP: Parser> Parser for Doc<TP> {
    type T = TP::T;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        self.parser.parse(input, names)
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.parser.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Doc {
            name: self.name.to_owned(),
            desc: self.desc.to_owned(),
            spec: Box::new(self.parser.to_spec()),
        }
    }
}

#[derive(Clone)]
struct PlayerNum {
    num: usize,
    name: String,
}

impl fmt::Display for PlayerNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub struct Player {}

impl Player {
    fn player_nums(&self, names: &[String]) -> Vec<PlayerNum> {
        names
            .iter()
            .enumerate()
            .map(|(p, name)| PlayerNum {
                num: p,
                name: name.to_string(),
            })
            .collect::<Vec<PlayerNum>>()
    }
}

impl Parser for Player {
    type T = usize;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        Map::new(Enum::partial(self.player_nums(names)), |pn| pn.num).parse(input, names)
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        Enum::partial(self.player_nums(names)).expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Player
    }
}

pub struct AfterSpace<TP: Parser> {
    pub parser: TP,
}

impl<TP: Parser> AfterSpace<TP> {
    pub fn new(parser: TP) -> Self {
        Self { parser }
    }
}

impl<TP: Parser> Parser for AfterSpace<TP> {
    type T = TP::T;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        let pair = chain_2(&Space {}, &self.parser, input, names)?;
        Ok(Output {
            value: pair.value.1,
            consumed: pair.consumed,
            remaining: pair.remaining,
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.parser.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Chain(vec![CommandSpec::Space, self.parser.to_spec()])
    }
}

impl Parser for CommandSpec {
    type T = serde_json::Value;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        match self {
            CommandSpec::Int { min, max } => {
                let out = Int {
                    min: *min,
                    max: *max,
                }
                .parse(input, names)?;
                Ok(Output {
                    value: serde_json::Value::Number(out.value.into()),
                    consumed: out.consumed,
                    remaining: out.remaining,
                })
            }
            CommandSpec::Token(token) => {
                let out = Token::new(token.clone()).parse(input, names)?;
                Ok(Output {
                    value: serde_json::Value::String(out.value),
                    consumed: out.consumed,
                    remaining: out.remaining,
                })
            }
            CommandSpec::Enum { values, exact } => {
                let out = if *exact {
                    Enum::exact(values.clone()).parse(input, names)?
                } else {
                    Enum::partial(values.clone()).parse(input, names)?
                };
                Ok(Output {
                    value: serde_json::Value::String(out.value),
                    consumed: out.consumed,
                    remaining: out.remaining,
                })
            }
            CommandSpec::OneOf(specs) => {
                let mut errors: Vec<GameError> = vec![];
                let mut error_consumed: usize = 0;
                for s in specs {
                    match s.parse(input, names) {
                        Ok(output) => return Ok(output),
                        Err(e) => {
                            let mut e_consumed = 0;
                            if let GameError::Parse { offset, .. } = e {
                                e_consumed = offset;
                            }
                            match e_consumed.cmp(&error_consumed) {
                                Ordering::Greater => {
                                    errors = vec![e];
                                    error_consumed = e_consumed;
                                }
                                Ordering::Equal => errors.push(e),
                                _ => {}
                            }
                        }
                    }
                }
                let error_messages = &errors
                    .iter()
                    .filter_map(|e| {
                        if let GameError::Parse { ref message, .. } = *e {
                            message.to_owned()
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<String>>();
                Err(GameError::Parse {
                    message: if error_messages.is_empty() {
                        None
                    } else {
                        Some(comma_list_or(error_messages))
                    },
                    expected: errors
                        .iter()
                        .flat_map(|e| match *e {
                            GameError::Parse { ref expected, .. } => expected.clone(),
                            _ => vec![],
                        })
                        .collect(),
                    offset: error_consumed,
                })
            }
            CommandSpec::Chain(specs) => {
                let mut values = vec![];
                let mut consumed_len = 0;
                let mut remaining = input;
                for s in specs {
                    let out = s
                        .parse(remaining, names)
                        .map_err(|e| add_offset(e, consumed_len))?;
                    values.push(out.value);
                    consumed_len += out.consumed.len();
                    remaining = out.remaining;
                }
                Ok(Output {
                    value: serde_json::Value::Array(values),
                    consumed: &input[..consumed_len],
                    remaining,
                })
            }
            CommandSpec::Many {
                spec,
                min,
                max,
                delim,
            } => {
                let mut values = vec![];
                let mut consumed_len = 0;
                let mut remaining = input;
                let mut first = true;
                loop {
                    if let Some(max_val) = max
                        && values.len() >= *max_val
                    {
                        break;
                    }
                    let mut inner_remaining = remaining;
                    let mut delim_len = 0;
                    if !first && let Some(d) = delim {
                        match d.parse(remaining, names) {
                            Ok(out) => {
                                inner_remaining = out.remaining;
                                delim_len = out.consumed.len();
                            }
                            Err(_) => break,
                        }
                    }
                    match spec.parse(inner_remaining, names) {
                        Ok(out) => {
                            let step = delim_len + out.consumed.len();
                            values.push(out.value);
                            consumed_len += step;
                            remaining = out.remaining;
                            first = false;
                            // Progress invariant, see the typed `Many` impl:
                            // a zero-width iteration would loop forever.
                            if step == 0 {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                if let Some(min_val) = min
                    && values.len() < *min_val
                {
                    return Err(GameError::Parse {
                        message: Some(format!(
                            "expected at least {} items but could only parse {}",
                            min_val,
                            values.len()
                        )),
                        expected: vec![],
                        offset: consumed_len,
                    });
                }
                Ok(Output {
                    value: serde_json::Value::Array(values),
                    consumed: &input[..consumed_len],
                    remaining,
                })
            }
            CommandSpec::Opt(spec) => Ok(match spec.parse(input, names) {
                Ok(out) => Output {
                    value: out.value,
                    consumed: out.consumed,
                    remaining: out.remaining,
                },
                Err(_) => Output {
                    value: serde_json::Value::Null,
                    consumed: &input[..0],
                    remaining: input,
                },
            }),
            CommandSpec::Doc { spec, .. } => spec.parse(input, names),
            CommandSpec::Player => {
                let out = Player {}.parse(input, names)?;
                Ok(Output {
                    value: serde_json::Value::Number(out.value.into()),
                    consumed: out.consumed,
                    remaining: out.remaining,
                })
            }
            CommandSpec::Space => {
                let out = Space {}.parse(input, names)?;
                Ok(Output {
                    value: serde_json::Value::String(out.value),
                    consumed: out.consumed,
                    remaining: out.remaining,
                })
            }
        }
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        match self {
            CommandSpec::Int { min, max } => Int {
                min: *min,
                max: *max,
            }
            .expected(names),
            CommandSpec::Token(token) => Token::new(token.clone()).expected(names),
            CommandSpec::Enum { values, exact } => {
                if *exact {
                    Enum::exact(values.clone()).expected(names)
                } else {
                    Enum::partial(values.clone()).expected(names)
                }
            }
            CommandSpec::OneOf(specs) => specs.iter().flat_map(|s| s.expected(names)).collect(),
            CommandSpec::Chain(specs) => specs
                .iter()
                .find(|s| !matches!(s, CommandSpec::Space))
                .or_else(|| specs.first())
                .map(|s| s.expected(names))
                .unwrap_or_default(),
            CommandSpec::Many { spec, min, max, .. } => {
                many_expected(spec.expected(names), *min, *max)
            }
            CommandSpec::Opt(spec) => spec
                .expected(names)
                .iter()
                .map(|e| format!("optional {}", e))
                .collect(),
            CommandSpec::Doc { spec, .. } => spec.expected(names),
            CommandSpec::Player => Player {}.expected(names),
            CommandSpec::Space => Space {}.expected(names),
        }
    }

    fn to_spec(&self) -> CommandSpec {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_parser_works() {
        let mut parser = Int {
            min: None,
            max: None,
        };
        parser
            .parse("fart", &[])
            .expect_err("expected 'fart' to produce an error");
        assert_eq!(
            Output {
                value: 10,
                consumed: "10",
                remaining: "",
            },
            parser.parse("10", &[]).expect("expected '10' to parse")
        );
        assert_eq!(
            Output {
                value: 10,
                consumed: "10",
                remaining: " with bacon and cheese",
            },
            parser
                .parse("10 with bacon and cheese", &[])
                .expect("expected '10 with bacon and cheese' to parse")
        );
        assert_eq!(
            Output {
                value: -10,
                consumed: "-10",
                remaining: " with bacon and cheese",
            },
            parser
                .parse("-10 with bacon and cheese", &[])
                .expect("expected '-10 with bacon and cheese' to parse")
        );
        parser
            .parse("-", &[])
            .expect_err("expected '-' to produce an error");
        parser.min = Some(-5);
        parser
            .parse("-6", &[])
            .expect_err("expected '-6' to produce an error when minimum is set");
        parser.max = Some(100);
        parser
            .parse("101", &[])
            .expect_err("expected '101' to produce an error when maximum is set");
    }

    #[test]
    fn map_parser_works() {
        let parser = Map::new(
            Int {
                min: None,
                max: None,
            },
            |i| i.to_string(),
        );
        assert_eq!(
            Output {
                value: "123".to_string(),
                consumed: "00123",
                remaining: "bacon",
            },
            parser
                .parse("00123bacon", &[])
                .expect("expected '00123bacon' to parse")
        )
    }

    #[test]
    fn opt_parser_works() {
        let parser = Opt::new(Int {
            min: None,
            max: None,
        });
        assert_eq!(
            Output {
                value: Some(123),
                consumed: "00123",
                remaining: "bacon",
            },
            parser
                .parse("00123bacon", &[])
                .expect("expected '00123bacon' to parse")
        );
        assert_eq!(
            Output {
                value: None,
                consumed: "",
                remaining: "bacon",
            },
            parser
                .parse("bacon", &[])
                .expect("expected 'bacon' to parse")
        );
    }

    #[test]
    fn token_parser_works() {
        let parser = Token::new("blah");
        assert_eq!(
            Output {
                value: "blah".to_string(),
                consumed: "BlAh",
                remaining: "bacon",
            },
            parser
                .parse("BlAhbacon", &[])
                .expect("expected 'BlAhbacon' to parse")
        );
        parser
            .parse("ClAhbacon", &[])
            .expect_err("expected 'ClAhbacon' to produce an error");
    }

    #[test]
    fn many_parser_works() {
        let mut parser = Many {
            parser: Int {
                min: None,
                max: None,
            },
            min: None,
            max: None,
            delim: Some(Token::new(", ")),
        };
        assert_eq!(
            Output {
                value: vec![3, 4, 5],
                consumed: "3, 4, 5",
                remaining: "",
            },
            parser
                .parse("3, 4, 5", &[])
                .expect("expected '3, 4, 5' to parse")
        );
        parser.min = Some(5);
        parser
            .parse("3, 4, 5", &[])
            .expect_err("expected '3, 4, 5' with a min of 5 to produce an error");
        parser.max = Some(5);
        assert_eq!(
            Output {
                value: vec![3, 4, 5, 6, 7],
                consumed: "3, 4, 5, 6, 7",
                remaining: ", 8, 9, 10",
            },
            parser
                .parse("3, 4, 5, 6, 7, 8, 9, 10", &[])
                .expect("expected '3, 4, 5, 6, 7, 8, 9, 10' to parse")
        );
        parser.min = None;
        parser.delim = Some(Token::new(";"));
        assert_eq!(
            Output {
                value: vec![3, 4, 5],
                consumed: "3;4;5",
                remaining: "",
            },
            parser
                .parse("3;4;5", &[])
                .expect("expected '3; 4; 5' to parse")
        );
    }

    #[test]
    fn many_degenerate_bounds_match_the_spec_impl() {
        // lg F8: the typed impl used to return Ok(empty) for `max == 0` or
        // `max < min` via an early return that skipped the min check, while
        // the spec impl broke out of its loop and failed the min check. Same
        // grammar, different success/failure - the exact drift the parity
        // helper guards against.
        let parser: Many<Int, Space> = Many {
            parser: Int::any(),
            min: Some(2),
            max: Some(1),
            delim: Some(Space {}),
        };
        parser
            .parse("1 2", &[])
            .expect_err("min 2 with max 1 must fail the min check");
        assert_typed_spec_parity(&parser, &["1 2", "1", ""]);

        let parser: Many<Int, Space> = Many {
            parser: Int::any(),
            min: Some(1),
            max: Some(0),
            delim: Some(Space {}),
        };
        parser
            .parse("1 2", &[])
            .expect_err("min 1 with max 0 must fail the min check");
        assert_typed_spec_parity(&parser, &["1 2", "1", ""]);

        // max == 0 without a min still succeeds consuming nothing.
        let parser: Many<Int, Space> = Many {
            parser: Int::any(),
            min: None,
            max: Some(0),
            delim: Some(Space {}),
        };
        let out = parser
            .parse("1 2", &[])
            .expect("max 0 with no min must succeed with an empty value");
        assert!(out.value.is_empty());
        assert_eq!(out.consumed, "");
        assert_eq!(out.remaining, "1 2");
        assert_typed_spec_parity(&parser, &["1 2", ""]);
    }

    #[test]
    fn many_zero_width_item_terminates() {
        // lg F6: `Opt` always succeeds consuming nothing, so with no
        // delimiter every iteration made zero progress and the loop pushed
        // values forever (unbounded Vec growth with max = None).
        let parser: Many<Opt<Token>, Space> = Many {
            parser: Opt::new(Token::new("x")),
            min: None,
            max: None,
            delim: None,
        };
        let out = parser
            .parse("y", &[])
            .expect("a zero-width Many must terminate and succeed");
        assert_eq!(out.value, vec![None]);
        assert_eq!(out.consumed, "");
        assert_eq!(out.remaining, "y");
        assert_typed_spec_parity(&parser, &["y", ""]);
    }

    #[test]
    fn spec_many_zero_width_item_terminates() {
        // lg F6: `Chain(vec![])` succeeds consuming nothing, so the spec
        // Many loop had the same unbounded-growth defect as the typed one.
        let spec = CommandSpec::Many {
            spec: Box::new(CommandSpec::Chain(vec![])),
            min: None,
            max: None,
            delim: None,
        };
        let out = spec
            .parse("y", &[])
            .expect("a zero-width spec Many must terminate and succeed");
        assert_eq!(out.consumed, "");
        assert_eq!(out.remaining, "y");
        assert_eq!(
            out.value,
            serde_json::Value::Array(vec![serde_json::Value::Array(vec![])])
        );
    }

    #[test]
    fn test_one_of_works() {
        let parsers: Vec<Box<dyn Parser<T = String>>> = vec![
            Box::new(Token::new("blah")),
            Box::new(Map::new(Many::any_spaced(Token::new("fart")), |v| {
                v.join(" ")
            })),
        ];
        let parser = OneOf::new(parsers);
        assert_eq!(
            Output {
                value: "blah".to_string(),
                consumed: "blah",
                remaining: "",
            },
            parser.parse("blah", &[]).expect("expected 'blah' to parse")
        );
        assert_eq!(
            Output {
                value: "fart fart fart".to_string(),
                consumed: "fart fart fart",
                remaining: "",
            },
            parser
                .parse("fart fart fart", &[])
                .expect("expected 'fart fart fart' to parse")
        );
    }

    #[test]
    fn test_enum_works() {
        let parser = Enum::partial(vec!["fart", "cheese", "dog", "bacon", "farty"]);
        assert_eq!(
            Output {
                value: "cheese",
                consumed: "c",
                remaining: "",
            },
            parser.parse("c", &[]).expect("expected 'c' to parse")
        );
        parser
            .parse("hat", &[])
            .expect_err("expected 'hat' to produce error");
        parser
            .parse("far", &[])
            .expect_err("expected 'far' to produce error");
        assert_eq!(
            Output {
                value: "fart",
                consumed: "fart",
                remaining: "",
            },
            parser.parse("fart", &[]).expect("expected 'fart' to parse")
        );
        assert_eq!(
            Output {
                value: "farty",
                consumed: "farty",
                remaining: "",
            },
            parser
                .parse("farty", &[])
                .expect("expected 'farty' to parse")
        );
        assert_eq!(
            Output {
                value: "dog",
                consumed: "DoG",
                remaining: "log",
            },
            parser
                .parse("DoGlog", &[])
                .expect("expected 'DoGlog' to parse")
        );
    }

    #[test]
    fn enum_full_match_wins_ties_in_either_declaration_order() {
        // lg F5: a full match that ties the current best length used to be
        // appended instead of replacing the partial, so ["abc", "ab"] with
        // input "ab" produced a spurious "matched ab and abc" ambiguity
        // error while ["ab", "abc"] parsed fine.
        for values in [vec!["abc", "ab"], vec!["ab", "abc"]] {
            let parser = Enum::partial(values.clone());
            assert_eq!(
                Output {
                    value: "ab",
                    consumed: "ab",
                    remaining: "",
                },
                parser
                    .parse("ab", &[])
                    .unwrap_or_else(|e| panic!("values {:?}: {}", values, e)),
            );
            assert_eq!(
                Output {
                    value: "abc",
                    consumed: "abc",
                    remaining: "",
                },
                parser
                    .parse("abc", &[])
                    .unwrap_or_else(|e| panic!("values {:?}: {}", values, e)),
            );
        }
    }

    #[test]
    fn enum_longest_match_wins_in_either_declaration_order() {
        // lg F5, second half: the ranking key is (matched length, then full
        // match), so the longer partial match wins regardless of which value
        // was declared first. Pre-fix ["ab", "abcd"] consumed only "ab" while
        // ["abcd", "ab"] consumed "abc" for the same input.
        for values in [vec!["ab", "abcd"], vec!["abcd", "ab"]] {
            let parser = Enum::partial(values.clone());
            assert_eq!(
                Output {
                    value: "abcd",
                    consumed: "abc",
                    remaining: "x",
                },
                parser
                    .parse("abcx", &[])
                    .unwrap_or_else(|e| panic!("values {:?}: {}", values, e)),
            );
        }
    }

    #[test]
    fn player_name_prefix_of_another_name_parses_longest() {
        // lg F5 reachability: Player builds Enum::partial from player names,
        // which are user-chosen, so prefix pairs are ordinary. Both orderings
        // must resolve the same way.
        for names in [
            vec!["Bo".to_string(), "Bobby".to_string()],
            vec!["Bobby".to_string(), "Bo".to_string()],
        ] {
            let parser = Player {};
            let bobby = names.iter().position(|n| n == "Bobby").unwrap();
            let bo = names.iter().position(|n| n == "Bo").unwrap();
            assert_eq!(
                bobby,
                parser
                    .parse("bobb", &names)
                    .unwrap_or_else(|e| panic!("names {:?}: {}", names, e))
                    .value,
                "a longer partial name match must win: {:?}",
                names
            );
            assert_eq!(
                bo,
                parser
                    .parse("bo", &names)
                    .unwrap_or_else(|e| panic!("names {:?}: {}", names, e))
                    .value,
                "an exact full name match must win ties: {:?}",
                names
            );
        }
    }

    #[test]
    fn after_space_parser_works() {
        let parser = AfterSpace::new(Token::new("blah"));
        parser
            .parse("blah", &[])
            .expect_err("expected 'blah' to produce error");
        assert_eq!(
            Output {
                value: "blah".to_string(),
                consumed: " BlAh",
                remaining: "bacon",
            },
            parser
                .parse(" BlAhbacon", &[])
                .expect("expected ' BlAhbacon' to parse")
        );
    }

    // --- Typed parser vs spec parser parity (drift guard) ---
    //
    // `Parser` is implemented twice: once for the typed combinators above and
    // again for the serializable `Spec` (see `impl Parser for CommandSpec`).
    // The dual implementation is retained deliberately (see
    // docs/parser-autocomplete-handover.md §6 D5), so these tests guard
    // against the two implementations drifting apart: for representative
    // command shapes the typed parse and the `to_spec()`-derived spec parse
    // must agree on consumption - same success/failure, and on success the
    // same `remaining`. Values are not compared: the typed and spec
    // implementations return different value types, and `suggest` only ever
    // consumes `remaining` from spec-parse results.
    fn assert_typed_spec_parity<P>(parser: &P, inputs: &[&str])
    where
        P: Parser,
    {
        let spec = parser.to_spec();
        assert_eq!(
            parser.expected(&[]),
            spec.expected(&[]),
            "typed and spec parsers disagree on expected()"
        );
        for input in inputs {
            let typed_result = parser.parse(input, &[]);
            let spec_result = spec.parse(input, &[]);
            match (&typed_result, &spec_result) {
                (Ok(typed), Ok(spec)) => assert_eq!(
                    typed.remaining, spec.remaining,
                    "typed and spec parsers disagree on remaining for input {:?}",
                    input
                ),
                (Err(_), Err(_)) => {}
                _ => panic!(
                    "typed and spec parsers disagree on success for input {:?}: typed {}, spec {}",
                    input,
                    if typed_result.is_ok() {
                        "succeeded"
                    } else {
                        "failed"
                    },
                    if spec_result.is_ok() {
                        "succeeded"
                    } else {
                        "failed"
                    },
                ),
            }
        }
    }

    #[test]
    fn splendor_take_typed_spec_parity() {
        // Mirrors splendor-2's take_parser: Many(Enum) inside nested
        // Chain/Space/Doc.
        let parser = Chain2::new(
            Doc::name_desc(
                "take",
                "take 3 different tokens, or 2 of the same token",
                Token::new("take"),
            ),
            AfterSpace::new(Doc::name_desc(
                "tokens",
                "the tokens to take",
                Many::some_spaced(Enum::partial(vec![
                    "Diamond", "Sapphire", "Emerald", "Ruby", "Onyx",
                ])),
            )),
        );
        // Structural sanity check: the derived spec must match the shape that
        // suggest.rs's splendor_take_spec() helper hard-codes.
        assert_eq!(
            parser.to_spec(),
            CommandSpec::Chain(vec![
                CommandSpec::Doc {
                    name: "take".into(),
                    desc: Some("take 3 different tokens, or 2 of the same token".into()),
                    spec: Box::new(CommandSpec::Token("take".into())),
                },
                CommandSpec::Chain(vec![
                    CommandSpec::Space,
                    CommandSpec::Doc {
                        name: "tokens".into(),
                        desc: Some("the tokens to take".into()),
                        spec: Box::new(CommandSpec::Many {
                            spec: Box::new(CommandSpec::Enum {
                                values: vec![
                                    "Diamond".into(),
                                    "Sapphire".into(),
                                    "Emerald".into(),
                                    "Ruby".into(),
                                    "Onyx".into(),
                                ],
                                exact: false,
                            }),
                            min: Some(1),
                            max: None,
                            delim: Some(Box::new(CommandSpec::Space)),
                        }),
                    },
                ]),
            ])
        );
        assert_typed_spec_parity(
            &parser,
            &[
                "take diamond sapphire emerald", // valid full command
                "take dia sap em",               // unique prefixes
                "take x",                        // garbage
                "take dia sap emsa",             // mid-word stop
                "take diamond ",                 // trailing space after a token
                "take ",                         // trailing space, no tokens
                "take",                          // no space after the token
                "",                              // empty input
            ],
        );
    }

    #[test]
    fn jaipur_sell_typed_spec_parity() {
        // Mirrors jaipur-2's sell_parser
        // (rust/game/jaipur-2/src/command.rs:60-91): a Many nested inside a
        // OneOf. The trade-good enum is trimmed from jaipur's six goods
        // (Diamond, Gold, Silver, Cloth, Spice, Leather) to the first three -
        // coverage of the shape, not the value list, is what matters.
        let goods = vec!["Diamond", "Gold", "Silver"];
        let sell_quantity: Box<dyn Parser<T = (usize, String)>> = Box::new(Map::new(
            Chain2::new(
                Int::positive(),
                AfterSpace::new(Enum::partial(goods.clone())),
            ),
            |(quantity, good): (i32, &str)| (quantity as usize, good.to_string()),
        ));
        let sell_many: Box<dyn Parser<T = (usize, String)>> = Box::new(Map::new(
            Many::some_spaced(Enum::partial(goods)),
            |goods: Vec<&str>| {
                (
                    goods.len(),
                    goods.first().copied().unwrap_or("Diamond").to_string(),
                )
            },
        ));
        let parser = Chain2::new(
            Doc::name_desc(
                "sell",
                "sell goods for tokens, eg. sell 2 dia or sell dia dia",
                Token::new("sell"),
            ),
            AfterSpace::new(OneOf::new(vec![sell_quantity, sell_many])),
        );
        assert_typed_spec_parity(
            &parser,
            &[
                "sell 2 diamond",     // valid quantity form
                "sell diamond gold",  // valid repeated-good form
                "sell 2 dia",         // unique prefix
                "sell dia gol",       // unique prefixes
                "sell x",             // garbage
                "sell dia gol silva", // mid-word stop
                "sell 0 diamond",     // below the Int minimum
                "sell 2",             // missing the good
                "sell diamond ",      // trailing space
                "sell ",              // trailing space, no argument
                "sell",               // no space after the token
                "",                   // empty input
            ],
        );
    }

    #[test]
    fn opt_typed_spec_parity() {
        // Compact Opt-containing shape: an optional ` <int>` after a token,
        // exercising the Opt arm of both implementations.
        let parser = Chain2::new(Token::new("buy"), Opt::new(AfterSpace::new(Int::any())));
        assert_typed_spec_parity(
            &parser,
            &[
                "buy 3",  // present
                "buy -3", // present, negative
                "buy",    // absent
                "buy ",   // absent, trailing space left over
                "buy x",  // absent, junk left over
                "buy3",   // absent, no space
                "",       // empty input
            ],
        );
    }

    #[test]
    fn space_parser_handles_multibyte_whitespace() {
        // U+00A0 NBSP is 2-byte whitespace; iOS autocorrect inserts it in
        // place of a regular space. Must not panic (char count != byte len).
        let parser = Space {};
        assert_eq!(
            Output {
                value: "\u{a0}".to_string(),
                consumed: "\u{a0}",
                remaining: "x",
            },
            parser
                .parse("\u{a0}x", &[])
                .expect("expected NBSP to parse as whitespace")
        );
        // Mixed ASCII + NBSP + ideographic space run.
        assert_eq!(
            Output {
                value: " \u{a0}\u{3000}".to_string(),
                consumed: " \u{a0}\u{3000}",
                remaining: "go",
            },
            parser
                .parse(" \u{a0}\u{3000}go", &[])
                .expect("expected mixed whitespace run to parse")
        );
        // Non-whitespace multi-byte char must still error, not panic.
        parser
            .parse("é", &[])
            .expect_err("expected 'é' to produce an error");
    }

    #[test]
    fn token_parser_handles_multibyte_input() {
        // "nñ" is 3 bytes; byte index 2 (the token's length) is inside 'ñ'.
        // Must be a mismatch, not a panic.
        let parser = Token::new("no");
        parser
            .parse("nñ", &[])
            .expect_err("expected 'nñ' to produce an error for token 'no'");
        // Multi-byte input longer than the token still mismatches cleanly.
        parser
            .parse("ñofurther", &[])
            .expect_err("expected 'ñofurther' to produce an error for token 'no'");
        // A multi-byte token still matches multi-byte input exactly.
        let parser = Token::new("sí");
        assert_eq!(
            Output {
                value: "sí".to_string(),
                consumed: "sí",
                remaining: "!",
            },
            parser.parse("sí!", &[]).expect("expected 'sí!' to parse")
        );
    }

    #[test]
    fn int_parser_stops_cleanly_at_multibyte_chars() {
        let parser = Int {
            min: None,
            max: None,
        };
        assert_eq!(
            Output {
                value: 12,
                consumed: "12",
                remaining: "é",
            },
            parser.parse("12é", &[]).expect("expected '12é' to parse")
        );
        parser
            .parse("é12", &[])
            .expect_err("expected 'é12' to produce an error");
        // Non-ASCII digits are rejected, not consumed.
        parser
            .parse("١٢", &[])
            .expect_err("expected Arabic-Indic digits to produce an error");
    }

    #[test]
    fn enum_parser_handles_multibyte_values() {
        // lg F3: shared_prefix returned chars, Enum sliced bytes.
        let parser = Enum::partial(vec!["café", "dog"]);
        assert_eq!(
            Output {
                value: "café",
                consumed: "café",
                remaining: "x",
            },
            parser
                .parse("caféx", &[])
                .expect("expected 'caféx' to parse")
        );
        // Partial prefix stopping before the multi-byte char.
        assert_eq!(
            Output {
                value: "café",
                consumed: "caf",
                remaining: "",
            },
            parser.parse("caf", &[]).expect("expected 'caf' to parse")
        );
        // Case-insensitive multi-byte match.
        assert_eq!(
            Output {
                value: "café",
                consumed: "CAFÉ",
                remaining: "",
            },
            parser.parse("CAFÉ", &[]).expect("expected 'CAFÉ' to parse")
        );
    }

    #[test]
    fn enum_parser_multibyte_player_name() {
        // lg F3 reachability: Player builds Enum::partial from user names.
        let names = vec!["José".to_string(), "Bob".to_string()];
        let parser = Player {};
        assert_eq!(
            Output {
                value: 0,
                consumed: "josé",
                remaining: "",
            },
            parser
                .parse("josé", &names)
                .expect("expected player name 'josé' to parse")
        );
    }

    #[test]
    fn exact_enum_matches_multibyte_values() {
        // lg F4: chars-vs-bytes comparison made exact multi-byte values
        // unmatchable, and broke full-match priority.
        let parser = Enum::exact(vec!["café", "dog"]);
        assert_eq!(
            Output {
                value: "café",
                consumed: "café",
                remaining: "",
            },
            parser.parse("café", &[]).expect("expected 'café' to parse")
        );
        parser
            .parse("caf", &[])
            .expect_err("expected partial 'caf' to error under exact");
        // Full-match priority with multi-byte values: the exact-length full
        // match must beat the equal-input-length partial of a longer value.
        let parser = Enum::partial(vec!["café", "cafét"]);
        assert_eq!(
            Output {
                value: "café",
                consumed: "café",
                remaining: "",
            },
            parser
                .parse("café", &[])
                .expect("expected full match 'café' to win over partial 'cafét'")
        );
    }

    #[test]
    fn doc_spec_expected_delegates_to_inner() {
        let spec = CommandSpec::Doc {
            name: "tokens".into(),
            desc: None,
            spec: Box::new(CommandSpec::Enum {
                values: vec!["Diamond".into(), "Sapphire".into()],
                exact: false,
            }),
        };
        assert_eq!(spec.expected(&[]), vec!["Diamond", "Sapphire"]);
    }

    #[test]
    fn many_spec_expected_applies_cardinality() {
        let spec = CommandSpec::Many {
            spec: Box::new(CommandSpec::Token("card".into())),
            min: Some(1),
            max: Some(2),
            delim: None,
        };
        assert_eq!(spec.expected(&[]), vec!["between 1 and 2 card"]);
    }

    #[test]
    fn chain_offset_propagation() {
        let spec = CommandSpec::Chain(vec![
            CommandSpec::Token("play".into()),
            CommandSpec::Space,
            CommandSpec::Token("card".into()),
        ]);
        let err = spec
            .parse("play x", &[])
            .expect_err("expected 'play x' to fail");
        match err {
            GameError::Parse { offset, .. } => assert_eq!(offset, 5),
            _ => panic!("expected Parse error"),
        }
    }

    #[test]
    fn one_of_furthest_branch_wins_expected() {
        let spec = CommandSpec::OneOf(vec![
            CommandSpec::Chain(vec![
                CommandSpec::Token("play".into()),
                CommandSpec::Space,
                CommandSpec::Token("card".into()),
            ]),
            CommandSpec::Chain(vec![
                CommandSpec::Token("play".into()),
                CommandSpec::Space,
                CommandSpec::Token("tile".into()),
            ]),
        ]);
        let err = spec
            .parse("play x", &[])
            .expect_err("expected 'play x' to fail");
        match err {
            GameError::Parse {
                expected, offset, ..
            } => {
                assert_eq!(offset, 5);
                assert!(expected.contains(&"card".to_string()));
                assert!(expected.contains(&"tile".to_string()));
            }
            _ => panic!("expected Parse error"),
        }
    }

    #[test]
    fn one_of_all_fail_at_zero_accumulates_all_expected() {
        let spec = CommandSpec::OneOf(vec![
            CommandSpec::Token("buy".into()),
            CommandSpec::Token("sell".into()),
        ]);
        let err = spec.parse("x", &[]).expect_err("expected 'x' to fail");
        match err {
            GameError::Parse {
                expected, offset, ..
            } => {
                assert_eq!(offset, 0);
                assert_eq!(expected, vec!["buy", "sell"]);
            }
            _ => panic!("expected Parse error"),
        }
    }
}
