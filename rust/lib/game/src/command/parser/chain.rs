use std::marker::PhantomData;

use crate::command::parser::{Output, Parser};
use crate::command::Spec as CommandSpec;
use crate::errors::*;

pub fn chain_2<'a, A, B, PA, PB>(
    a: &PA,
    b: &PB,
    input: &'a str,
    names: &[String],
) -> Result<Output<'a, (A, B)>, GameError>
where
    PA: Parser<A>,
    PB: Parser<B>,
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

pub struct Chain2<A, B, PA, PB>
where
    PA: Parser<A>,
    PB: Parser<B>,
{
    pub a: PA,
    pub b: PB,
    a_type: PhantomData<A>,
    b_type: PhantomData<B>,
}

impl<A, B, PA, PB> Chain2<A, B, PA, PB>
where
    PA: Parser<A>,
    PB: Parser<B>,
{
    pub fn new(a: PA, b: PB) -> Self {
        Self {
            a,
            b,
            a_type: PhantomData,
            b_type: PhantomData,
        }
    }
}

impl<A, B, PA, PB> Parser<(A, B)> for Chain2<A, B, PA, PB>
where
    PA: Parser<A>,
    PB: Parser<B>,
{
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, (A, B)>, GameError> {
        chain_2(&self.a, &self.b, input, names)
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.a.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        CommandSpec::Chain(vec![self.a.to_spec(), self.b.to_spec()])
    }
}

pub struct Chain3<A, B, C, PA, PB, PC>
where
    PA: Parser<A>,
    PB: Parser<B>,
    PC: Parser<C>,
{
    pub a: PA,
    pub b: PB,
    pub c: PC,
    a_type: PhantomData<A>,
    b_type: PhantomData<B>,
    c_type: PhantomData<C>,
}

impl<A, B, C, PA, PB, PC> Chain3<A, B, C, PA, PB, PC>
where
    PA: Parser<A>,
    PB: Parser<B>,
    PC: Parser<C>,
{
    pub fn new(a: PA, b: PB, c: PC) -> Self {
        Self {
            a,
            b,
            c,
            a_type: PhantomData,
            b_type: PhantomData,
            c_type: PhantomData,
        }
    }
}

impl<A, B, C, PA, PB, PC> Parser<(A, B, C)> for Chain3<A, B, C, PA, PB, PC>
where
    PA: Parser<A>,
    PB: Parser<B>,
    PC: Parser<C>,
{
    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, (A, B, C)>, GameError> {
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

pub struct Chain4<A, B, C, D, PA, PB, PC, PD>
where
    PA: Parser<A>,
    PB: Parser<B>,
    PC: Parser<C>,
    PD: Parser<D>,
{
    pub a: PA,
    pub b: PB,
    pub c: PC,
    pub d: PD,
    a_type: PhantomData<A>,
    b_type: PhantomData<B>,
    c_type: PhantomData<C>,
    d_type: PhantomData<D>,
}

impl<A, B, C, D, PA, PB, PC, PD> Chain4<A, B, C, D, PA, PB, PC, PD>
where
    PA: Parser<A>,
    PB: Parser<B>,
    PC: Parser<C>,
    PD: Parser<D>,
{
    pub fn new(a: PA, b: PB, c: PC, d: PD) -> Self {
        Self {
            a,
            b,
            c,
            d,
            a_type: PhantomData,
            b_type: PhantomData,
            c_type: PhantomData,
            d_type: PhantomData,
        }
    }
}

impl<A, B, C, D, PA, PB, PC, PD> Parser<(A, B, C, D)> for Chain4<A, B, C, D, PA, PB, PC, PD>
where
    PA: Parser<A>,
    PB: Parser<B>,
    PC: Parser<C>,
    PD: Parser<D>,
{
    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, (A, B, C, D)>, GameError> {
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
