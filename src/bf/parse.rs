use super::*;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, none_of},
    combinator::{all_consuming, map},
    error::{convert_error, VerboseError},
    multi::many0,
    IResult,
};
// ---------------------------------------------------------------------
// Error and Location
// ---------------------------------------------------------------------

/// The error type we will use.
pub type Error<'a> = VerboseError<&'a str>;

/// A convenient alias for our IResult with that error type.
pub type Res<'a, T> = IResult<&'a str, T, Error<'a>>;

fn parse_comments(input: &str) -> Res<()> {
    let (input, _) = many0(none_of("[]<>+-.,#$"))(input)?;
    Ok((input, ()))
}

/// Parses a single operator into an `Op`
fn parse_op(input: &str) -> Res<Op> {
    let (input, _) = parse_comments(input)?;
    let (input, result) = alt((
        map(tag("[-]"), |_| Op::Zero), // Detect `[-]` as Zero
        map(char('>'), |_| Op::Move(1)),
        map(char('<'), |_| Op::Move(-1)),
        map(char('+'), |_| Op::Add(1)),
        map(char('-'), |_| Op::Add(-1)),
        map(char('.'), |_| Op::Put),
        map(char(','), |_| Op::Get),
        map(char('['), |_| Op::While),
        map(char(']'), |_| Op::End),
        map(char('#'), |_| Op::HexDump),
        map(char('$'), |_| Op::DecDump),
    ))(input)?;
    let (input, _) = parse_comments(input)?;

    Ok((input, result))
}

/// Parses multiple operations while **coalescing** `Move(n)` and `Add(n)`.
fn parse_ops(input: &str) -> Res<Vec<Op>> {
    let (input, ops) = all_consuming(many0(parse_op))(input)?;
    let mut optimized_ops: Vec<Op> = Vec::new();

    for op in ops {
        if let Some(last) = optimized_ops.last_mut() {
            if last.coalesce(op) {
                continue;
            }
        }
        optimized_ops.push(op);
    }

    Ok((input, optimized_ops))
}

pub fn parse(input: &str) -> Result<Vec<Op>, String> {
    match parse_ops(input) {
        Ok((rest, program)) => {
            if rest.is_empty() {
                Ok(program)
            } else {
                Err(format!(
                    "Failed to parse the entire input. Remaining: {}",
                    rest
                ))
            }
        }
        Err(e) => match e {
            nom::Err::Error(e) => Err(format!("Error: {}", convert_error(input, e))),
            nom::Err::Failure(e) => Err(format!("Failure: {}", convert_error(input, e))),
            nom::Err::Incomplete(_) => Err("Incomplete input".into()),
        },
    }
}
