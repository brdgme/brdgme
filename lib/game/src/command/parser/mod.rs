use std::collections::HashSet;
use std::fmt::Display;
use std::marker::PhantomData;

use unicase::UniCase;

use crate::command::Spec as CommandSpec;
use crate::errors::GameError;

pub use self::chain::*;

pub mod chain;

const MANY_DELIM: &str = ",";

#[derive(Debug, PartialEq)]
pub struct Output<'a, T> {
    pub value: T,
    pub consumed: &'a str,
    pub remaining: &'a str,
}

pub trait Parser<T> {
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, T>, GameError>;
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

impl Parser<String> for Token {
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, String>, GameError> {
        let t_len = self.token.len();
        if input.len() < self.token.len() || UniCase(&input[..t_len]) != UniCase(&self.token) {
            return Err(GameError::Parse {
                message: None,
                expected: self.expected(names),
                offset: 0,
            });
        }
        Ok(Output {
            value: self.token.to_owned(),
            consumed: &input[..t_len],
            remaining: &input[t_len..],
        })
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

impl Parser<i32> for Int {
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, i32>, GameError> {
        let mut found_digit = false;
        let consumed_count = input
            .chars()
            .enumerate()
            .take_while(|&(i, c)| {
                if i == 0 && c == '-' {
                    true
                } else if c.is_digit(10) {
                    found_digit = true;
                    true
                } else {
                    false
                }
            })
            .count();
        if !found_digit {
            return Err(GameError::Parse {
                message: None,
                expected: self.expected(names),
                offset: 0,
            });
        }
        let consumed = &input[..consumed_count];
        let value: i32 = consumed.parse().map_err(|_| {
            GameError::Parse {
                message: Some(format!("failed to parse '{}'", consumed)),
                expected: self.expected(names),
                offset: 0,
            }
        })?;
        if let Some(min) = self.min {
            if value < min {
                return Err(GameError::Parse {
                    message: Some(format!("{} is too low", value)),
                    expected: self.expected(names),
                    offset: 0,
                });
            }
        }
        if let Some(max) = self.max {
            if value > max {
                return Err(GameError::Parse {
                    message: Some(format!("{} is too high", value)),
                    expected: self.expected(names),
                    offset: 0,
                });
            }
        }
        Ok(Output {
            value,
            consumed,
            remaining: &input[consumed_count..],
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
        TP: Parser<T>,
{
    pub parser: TP,
    pub map: F,
    t_type: PhantomData<T>,
    o_type: PhantomData<O>,
}

impl<T, O, F, TP> Map<T, O, F, TP>
    where
        F: Fn(T) -> O,
        TP: Parser<T>,
{
    pub fn new(parser: TP, map: F) -> Self {
        Self {
            parser,
            map,
            t_type: PhantomData,
            o_type: PhantomData,
        }
    }
}

impl<T, O, F, TP> Parser<O> for Map<T, O, F, TP>
    where
        F: Fn(T) -> O,
        TP: Parser<T>,
{
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

pub struct Opt<T, TP>
    where
        TP: Parser<T>,
{
    pub parser: TP,
    t_type: PhantomData<T>,
}

impl<T, TP> Opt<T, TP>
    where
        TP: Parser<T>,
{
    pub fn new(parser: TP) -> Self {
        Self {
            parser,
            t_type: PhantomData,
        }
    }
}

impl<T, TP> Parser<Option<T>> for Opt<T, TP>
    where
        TP: Parser<T>,
{
    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Option<T>>, GameError> {
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

pub struct Many<T, TP>
    where
        TP: Parser<T>,
{
    pub parser: TP,
    pub min: Option<usize>,
    pub max: Option<usize>,
    pub delim: String,
    t_type: PhantomData<T>,
}

impl<T, TP> Many<T, TP>
    where
        TP: Parser<T>,
{
    pub fn any(parser: TP) -> Self {
        Self {
            parser,
            min: None,
            max: None,
            delim: MANY_DELIM.to_string(),
            t_type: PhantomData,
        }
    }

    pub fn some(parser: TP) -> Self {
        Self {
            parser,
            min: Some(1),
            max: None,
            delim: MANY_DELIM.to_string(),
            t_type: PhantomData,
        }
    }

    pub fn bounded(parser: TP, min: usize, max: usize) -> Self {
        Self {
            parser,
            min: Some(min),
            max: Some(max),
            delim: MANY_DELIM.to_string(),
            t_type: PhantomData,
        }
    }
}

impl<T, TP> Parser<Vec<T>> for Many<T, TP>
    where
        TP: Parser<T>,
{
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, Vec<T>>, GameError> {
        let mut parsed: Vec<T> = vec![];
        if let Some(max) = self.max {
            if max == 0 || max < self.min.unwrap_or(0) {
                return Ok(Output {
                    value: parsed,
                    consumed: &input[..0],
                    remaining: input,
                });
            }
        }
        let mut first = true;
        let mut offset = 0;
        let delim = Chain2::new(
            Opt::new(Space {}),
            Chain2::new(Token::new(self.delim.to_owned()), Opt::new(Space {})),
        );
        'outer: loop {
            let mut inner_offset = offset;
            if !first {
                match delim.parse(&input[offset..], names) {
                    Ok(Output { consumed, .. }) => inner_offset += consumed.len(),
                    Err(_) => break 'outer,
                };
            } else {
                first = false;
            }
            match self.parser.parse(&input[inner_offset..], names) {
                Ok(Output {
                       value, consumed, ..
                   }) => {
                    parsed.push(value);
                    offset = inner_offset + consumed.len();
                    if let Some(max) = self.max {
                        if parsed.len() == max {
                            break 'outer;
                        }
                    }
                }
                Err(_) => {
                    break 'outer;
                }
            };
        }
        if let Some(min) = self.min {
            if parsed.len() < min {
                return Err(GameError::Parse {
                    message: Some(format!(
                        "expected at least {} items but could only parse {}",
                        min,
                        parsed.len()
                    )),
                    expected: vec![],
                    offset: 0,
                });
            }
        }
        Ok(Output {
            value: parsed,
            consumed: &input[..offset],
            remaining: &input[offset..],
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.parser
            .expected(names)
            .iter()
            .map(|e| match (self.min, self.max) {
                (None, None) => format!("any number of {}", e),
                (Some(min), None) => format!("{} or more {}", min, e),
                (None, Some(max)) => format!("up to {} {}", max, e),
                (Some(min), Some(max)) => format!("between {} and {} {}", min, max, e),
            })
            .collect()
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Many {
            spec: Box::new(self.parser.to_spec()),
            min: self.min,
            max: self.max,
            delim: self.delim.to_owned(),
        }
    }
}

struct Space {}

impl Parser<String> for Space {
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, String>, GameError> {
        let consumed = input.chars().take_while(|c| c.is_whitespace()).count();
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

pub struct OneOf<T, TP: Parser<T> + ?Sized> {
    pub parsers: Vec<Box<TP>>,
    t_type: PhantomData<T>,
}

impl<T, TP: Parser<T> + ?Sized> OneOf<T, TP> {
    pub fn new(parsers: Vec<Box<TP>>) -> Self {
        Self {
            parsers,
            t_type: PhantomData,
        }
    }
}

impl<T, TP: Parser<T> + ?Sized> Parser<T> for OneOf<T, TP> {
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, T>, GameError> {
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
                    if e_consumed > error_consumed {
                        errors = vec![e];
                        error_consumed = e_consumed;
                    } else if e_consumed == error_consumed {
                        errors.push(e);
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

pub fn comma_list<T: Display>(items: &[T], last: &str) -> String {
    match items.len() {
        0 => "".to_string(),
        1 => format!("{}", items[0]),
        2 => format!("{} {} {}", items[0], last, items[1]),
        _ => format!("{}, {}", items[0], comma_list(&items[1..], last)),
    }
}

pub fn comma_list_or<T: Display>(items: &[T]) -> String {
    comma_list(items, "or")
}

pub fn comma_list_and<T: Display>(items: &[T]) -> String {
    comma_list(items, "and")
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

fn shared_prefix(s1: &str, s2: &str) -> usize {
    let mut s1i = s1.chars();
    let mut s2i = s2.chars();
    let mut len = 0usize;
    loop {
        match (s1i.next(), s2i.next()) {
            (Some(s1c), Some(s2c)) => {
                if s1c != s2c {
                    return len;
                }
                len += 1;
            }
            _ => return len,
        }
    }
}

impl<T> Parser<T> for Enum<T>
    where
        T: ToString + Clone,
{
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, T>, GameError> {
        let input_lower = input.to_lowercase();
        let mut matched: Vec<&T> = vec![];
        let mut match_len: usize = 0;
        // Exact matches are prioritised, a shorter full match will happen over a longer partial
        // match.
        let mut full_match = false;
        // Track which values have been searched to avoid duplicates.
        let mut searched: HashSet<String> = HashSet::new();
        for v in &self.values {
            let v_str = v.clone().to_string().to_lowercase();
            if searched.contains(&v_str) {
                // This is a duplicate, skip it.
                continue;
            }
            searched.insert(v_str.clone());
            let v_len = v_str.len();
            let matching = shared_prefix(&input_lower, &v_str);
            if self.exact && matching < v_len {
                // The input isn't long enough and we require exact match, skip it.
                continue;
            }
            if matching > 0 && matching >= match_len && (!full_match || matching == v_len) {
                if matching == v_len {
                    full_match = true
                }
                if matching > match_len {
                    matched = vec![v];
                    match_len = matching;
                } else {
                    matched.push(v);
                }
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
                    comma_list_and(&matched
                        .iter()
                        .map(|m| m.to_string())
                        .collect::<Vec<String>>()),
                )),
                expected: self.expected(names),
                offset: 0,
            }),
        }
    }

    fn expected(&self, _names: &[String]) -> Vec<String> {
        let mut values = self.values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>();
        values.sort();
        values
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Enum {
            values: self.values.iter().cloned().map(|v| v.to_string()).collect(),
            exact: self.exact,
        }
    }
}

pub struct Doc<T, TP: Parser<T>> {
    pub name: String,
    pub desc: Option<String>,
    pub parser: TP,
    t_type: PhantomData<T>,
}

impl<T, TP: Parser<T>> Doc<T, TP> {
    pub fn name<I: Into<String>>(name: I, parser: TP) -> Self {
        Self {
            name: name.into(),
            desc: None,
            parser,
            t_type: PhantomData,
        }
    }

    pub fn name_desc<I: Into<String>>(name: I, desc: I, parser: TP) -> Self {
        Self {
            name: name.into(),
            desc: Some(desc.into()),
            parser,
            t_type: PhantomData,
        }
    }
}

impl<T, TP: Parser<T>> Parser<T> for Doc<T, TP> {
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, T>, GameError> {
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

impl ToString for PlayerNum {
    fn to_string(&self) -> String {
        self.name.to_owned()
    }
}

pub struct Player {}

impl Player {
    fn player_nums(&self, names: &[String]) -> Vec<PlayerNum> {
        names
            .iter()
            .enumerate()
            .map(|(p, name)| {
                PlayerNum {
                    num: p,
                    name: name.to_string(),
                }
            })
            .collect::<Vec<PlayerNum>>()
    }
}

impl Parser<usize> for Player {
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, usize>, GameError> {
        Map::new(Enum::partial(self.player_nums(names)), |pn| pn.num).parse(input, names)
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        Enum::partial(self.player_nums(names)).expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Player
    }
}

pub struct AfterSpace<T, TP: Parser<T>> {
    pub parser: TP,
    t_type: PhantomData<T>,
}

impl<T, TP: Parser<T>> AfterSpace<T, TP> {
    pub fn new(parser: TP) -> Self {
        Self {
            parser,
            t_type: PhantomData,
        }
    }
}

impl<T, TP: Parser<T>> Parser<T> for AfterSpace<T, TP> {
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, T>, GameError> {
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
        let mut parser = Many::any(Int {
            min: None,
            max: None,
        });
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
        parser.delim = ";".to_string();
        assert_eq!(
            Output {
                value: vec![3, 4, 5],
                consumed: "3; 4; 5",
                remaining: "",
            },
            parser
                .parse("3; 4; 5", &[])
                .expect("expected '3; 4; 5' to parse")
        );
    }

    #[test]
    fn test_one_of_works() {
        let parsers: Vec<Box<Parser<String>>> = vec![
            Box::new(Token::new("blah")),
            Box::new(Map::new(Many::any(Token::new("fart")), |v| v.join(" "))),
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
                consumed: "fart, fart, fart",
                remaining: "",
            },
            parser
                .parse("fart, fart, fart", &[])
                .expect("expected 'fart, fart, fart' to parse")
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
}
