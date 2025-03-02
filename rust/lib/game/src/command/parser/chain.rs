use crate::command::Spec as CommandSpec;
use crate::command::parser::{Output, Parser};
use crate::errors::*;

type Chain2Output<PA, PB> = (<PA as Parser>::T, <PB as Parser>::T);

pub fn chain_2<'a, PA, PB>(
    a: &PA,
    b: &PB,
    input: &'a str,
    names: &[String],
) -> Result<Output<'a, Chain2Output<PA, PB>>, GameError>
where
    PA: Parser,
    PB: Parser,
{
    let lhs = a.parse(input, names)?;
    let rhs = b.parse(lhs.remaining, names)?;
    let consumed = lhs.consumed.len() + rhs.consumed.len();
    Ok(Output {
        value: (lhs.value, rhs.value),
        consumed: &input[..consumed],
        remaining: &input[consumed..],
    })
}

pub struct Chain2<PA, PB>
where
    PA: Parser,
    PB: Parser,
{
    pub a: PA,
    pub b: PB,
}

impl<PA, PB> Chain2<PA, PB>
where
    PA: Parser,
    PB: Parser,
{
    pub fn new(a: PA, b: PB) -> Self {
        Self { a, b }
    }
}

impl<PA, PB> Parser for Chain2<PA, PB>
where
    PA: Parser,
    PB: Parser,
{
    type T = (PA::T, PB::T);

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        chain_2(&self.a, &self.b, input, names)
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.a.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Chain(vec![self.a.to_spec(), self.b.to_spec()])
    }
}

pub struct Chain3<PA, PB, PC>
where
    PA: Parser,
    PB: Parser,
    PC: Parser,
{
    pub a: PA,
    pub b: PB,
    pub c: PC,
}

impl<PA, PB, PC> Chain3<PA, PB, PC>
where
    PA: Parser,
    PB: Parser,
    PC: Parser,
{
    pub fn new(a: PA, b: PB, c: PC) -> Self {
        Self { a, b, c }
    }
}

impl<PA, PB, PC> Parser for Chain3<PA, PB, PC>
where
    PA: Parser,
    PB: Parser,
    PC: Parser,
{
    type T = (PA::T, PB::T, PC::T);

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        let head = self.a.parse(input, names)?;
        let tail = chain_2(&self.b, &self.c, head.remaining, names)?;
        let consumed = head.consumed.len() + tail.consumed.len();
        Ok(Output {
            value: (head.value, tail.value.0, tail.value.1),
            consumed: &input[..consumed],
            remaining: &input[consumed..],
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.a.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Chain(vec![self.a.to_spec(), self.b.to_spec(), self.c.to_spec()])
    }
}

pub struct Chain4<PA, PB, PC, PD>
where
    PA: Parser,
    PB: Parser,
    PC: Parser,
    PD: Parser,
{
    pub a: PA,
    pub b: PB,
    pub c: PC,
    pub d: PD,
}

impl<PA, PB, PC, PD> Chain4<PA, PB, PC, PD>
where
    PA: Parser,
    PB: Parser,
    PC: Parser,
    PD: Parser,
{
    pub fn new(a: PA, b: PB, c: PC, d: PD) -> Self {
        Self { a, b, c, d }
    }
}

impl<PA, PB, PC, PD> Parser for Chain4<PA, PB, PC, PD>
where
    PA: Parser,
    PB: Parser,
    PC: Parser,
    PD: Parser,
{
    type T = (PA::T, PB::T, PC::T, PD::T);

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        let head = chain_2(&self.a, &self.b, input, names)?;
        let tail = chain_2(&self.c, &self.d, head.remaining, names)?;
        let consumed = head.consumed.len() + tail.consumed.len();
        Ok(Output {
            value: (head.value.0, head.value.1, tail.value.0, tail.value.1),
            consumed: &input[..consumed],
            remaining: &input[consumed..],
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.a.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Chain(vec![
            self.a.to_spec(),
            self.b.to_spec(),
            self.c.to_spec(),
            self.d.to_spec(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain2_parser_works() {
        use crate::command::parser::{Int, Token};
        let parser = Chain2::new(
            Int {
                min: None,
                max: None,
            },
            Token {
                token: "egg".to_string(),
            },
        );
        assert_eq!(
            Output {
                value: (123, "egg".to_string()),
                consumed: "123egg",
                remaining: "  chairs",
            },
            parser
                .parse("123egg  chairs", &[])
                .expect("expected '123egg  chairs' to parse")
        )
    }
}
