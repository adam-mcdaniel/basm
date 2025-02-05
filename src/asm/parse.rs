// parser.rs
use nom::{
    branch::alt, bytes::complete::{tag, take_while1, take_while_m_n}, character::complete::{anychar, char, digit1, hex_digit1, multispace0, space0}, combinator::{cut, eof, map, map_res, opt, recognize}, error::{convert_error, ParseError, VerboseError}, multi::{many0, many1, separated_list0}, sequence::{delimited, pair, preceded, terminated, tuple}, IResult
};

use super::*;

// ---------------------------------------------------------------------
// Error and Location
// ---------------------------------------------------------------------

/// The error type we will use.
pub type Error<'a> = VerboseError<&'a str>;

/// A convenient alias for our IResult with that error type.
pub type Res<'a, T> = IResult<&'a str, T, Error<'a>>;


//
// Parsers
//

// A helper to “eat” optional whitespace before and after a parser.
fn ws<'a, F: 'a, O>(inner: F) -> impl FnMut(&'a str) -> Res<'a, O>
where
    F: FnMut(&'a str) -> Res<'a, O>,
{
    // delimited(multispace0, inner, multispace0)
    delimited(space0, inner, space0)
    // preceded(multispace0, inner)
}

/// Parse an identifier: a letter or underscore followed by alphanumerics or underscores.
fn parse_identifier(input: &str) -> Res<Symbol> {
    let (input, _) = space0(input)?;
    let first_char = |c: char| c.is_ascii_alphabetic() || c == '_';
    let other_char = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let (input, id) = recognize(tuple((
        take_while1(first_char),
        opt(take_while1(other_char)),
    )))(input)?;
    Ok((input, id.into()))
}

/// A register is simply an identifier (e.g. "R0", "SP", "HP").
fn parse_register(input: &str) -> Res<StaticLocation> {
    // Parse a name in `REGISTER_NAMES` 
    let (input, _) = space0(input)?;
    let (input, id) = parse_identifier(input)?;
    if REGISTER_NAMES.contains(&id.as_str()) {
        Ok((input, StaticLocation::register(id.as_str())))
    } else {
        return Err(nom::Err::Error(nom::error::VerboseError::from_error_kind(
            input,
            nom::error::ErrorKind::Fail,
        )));
    }

    // let (input, id) = parse_identifier(input)?;
    // Ok((input, StaticLocation::register(id.as_str())))
}

/// Parse an immediate (number) and return it as an Operand.
fn parse_immediate_literal(input: &str) -> Res<u64> {
    alt((
        // Parse a hexadecimal number
        map_res(
            preceded(tag("0x"), ws(hex_digit1)),
            |hex_str: &str| u64::from_str_radix(hex_str, 16),
        ),

        // Parse an octal number
        map_res(
            preceded(tag("0o"), ws(take_while1(|c: char| c.is_ascii_digit()))),
            |octal_str: &str| u64::from_str_radix(octal_str, 8),
        ),

        // Parse a decimal number
        map_res(ws(digit1), |digit_str: &str| {
            digit_str.parse::<u64>()
        }),
        
        // Parse a character literal
        map(ws(parse_char_literal), |c| c as u64),
    ))(input)
}

/// Parse an immediate (number) and return it as an Operand.
fn parse_immediate(input: &str) -> Res<Operand> {
    map(ws(parse_immediate_literal), Operand::Immediate)(input)
}

/// Parses a single escaped character (e.g., `'\n'`, `'\t'`, `'\''`, `'\xNN'`, `'\u{NNNN}'`).
fn parse_escape(input: &str) -> Res<char> {
    preceded(
        char('\\'),
        alt((
            map(char('n'), |_| '\n'),
            map(char('r'), |_| '\r'),
            map(char('t'), |_| '\t'),
            map(char('\\'), |_| '\\'),
            map(char('\''), |_| '\''),
            map(char('"'), |_| '\"'),
            map_res(preceded(tag("x"), take_while_m_n(2, 2, |c: char| c.is_ascii_hexdigit())), 
                    |hex| u8::from_str_radix(hex, 16).map(|b| b as char)),
            map_res(preceded(tag("u{"), recognize(hex_digit1)), 
                    |hex| u32::from_str_radix(hex, 16).ok()
                          .and_then(std::char::from_u32)
                          .ok_or_else(|| nom::Err::Error((hex, nom::error::ErrorKind::Char)))),
        )),
    )(input)
}

/// Parses a single character, either normal or escaped.
fn parse_char(input: &str) -> Res<char> {
    alt((parse_escape, anychar))(input)
}

/// Parses a full character literal, e.g., `'a'` or `'\n'`
fn parse_char_literal(input: &str) -> Res<char> {
    delimited(char('\''), parse_char, char('\''))(input)
}
/// Parse a dynamic location:
///   - Either `SP[<register>]` (stack dereference)
///   - Or `HP[<register>]` (heap dereference)
///   - Or a plain register.
fn parse_dynamic_location(input: &str) -> Res<DynamicLocation> {
    let (input, result) = alt((
        // map(
        //     delimited(tag("["), ws(tag("SP")), char(']')),
        //     |_| DynamicLocation::DerefStack(StaticLocation::register("SP")),
        // ),
        // map(
        // delimited(tag("["), ws(tag("HP")), char(']')),
        //     |_| DynamicLocation::DerefStack(StaticLocation::register("HP")),
        // ),
        map(
            delimited(tag("["), ws(parse_register), char(']')),
            |reg| DynamicLocation::DerefStack(reg),
        ),
        // map(
        //     delimited(delimited(tag("["), ws(tag("HBP")), ws(tag("+"))), ws(parse_register), char(']')),
        //     |reg| DynamicLocation::DerefStack(reg),
        // ),
        map(parse_register, |reg| DynamicLocation::Static(reg)),
    ))(input)?;
    Ok((input, result))
}

/// Parse an operand: either an immediate number or a dynamic location.
fn parse_operand(input: &str) -> Res<Operand> {
    alt((
        parse_immediate,
        map(ws(parse_dynamic_location), Operand::Location),
    ))(input)
}

/// Parse the `hex_dump` instruction:
fn parse_hex_dump(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("hex_dump"))(input)?;
    Ok((input, BasicBlockOp::HexDump))
}

/// Parse the `dec_dump` instruction:
fn parse_dec_dump(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("dec_dump"))(input)?;
    Ok((input, BasicBlockOp::DecimalDump))
}

/// Parse the `set` instruction:
///   [<dynamic_location>] = <operand>
fn parse_set(input: &str) -> Res<BasicBlockOp> {
    let (input, dest) = parse_dynamic_location(input)?;
    let (input, _) = ws(tag("="))(input)?;
    let (input, src) = parse_operand(input)?;
    Ok((input, BasicBlockOp::Set {dest, src}))
}

/// Parse the `ne` instruction.
fn parse_lea(input: &str) -> Res<BasicBlockOp> {
    let (input, dest) = parse_dynamic_location(input)?;
    let (input, _) = ws(tag("lea"))(input)?;
    let (input, src) = parse_dynamic_location(input)?;
    let mut negative = false;
    let (input, offset) = opt(
        alt((
            preceded(pair(space0, ws(char('+'))), parse_operand),
            preceded(pair(space0, ws(char('-'))), map(parse_operand, |op| {negative=true; op})),
        ))
    )(input)?;

    Ok((input, BasicBlockOp::GetAddr {dest, src, offset, negative}))
}


/// Parse the `inc` instruction:
///   inc <dynamic_location> [<imm>]
fn parse_inc(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("inc"))(input)?;
    let (input, op) = parse_dynamic_location(input)?;
    let (input, imm) = opt(preceded(ws(tag(",")), parse_immediate_literal))(input)?;

    Ok((input, BasicBlockOp::Inc(op, imm)))
}

/// Parse the `dec` instruction:
///   dec <dynamic_location> [<imm>]
fn parse_dec(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("dec"))(input)?;
    let (input, op) = parse_dynamic_location(input)?;
    let (input, imm) = opt(preceded(ws(tag(",")), parse_immediate_literal))(input)?;

    Ok((input, BasicBlockOp::Dec(op, imm)))
}


/// Parse the `push` instruction:
///   push <operand>
fn parse_push(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("push"))(input)?;
    let (input, op) = parse_operand(input)?;
    Ok((input, BasicBlockOp::Push(op)))
}

/// Parse the `pop` instruction:
///   pop [<dynamic_location>]
fn parse_pop(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("pop"))(input)?;
    // Optionally follow with a dynamic location (e.g. "pop R0" or "pop SP[R0]")
    let (input, loc) = opt(preceded(space0, parse_dynamic_location))(input)?;
    Ok((input, BasicBlockOp::Pop(loc)))
}

/// Parse the `getchar` instruction:
///   getchar [<dynamic_location>]
fn parse_getchar(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("getchar"))(input)?;
    let (input, loc) = opt(preceded(space0, parse_dynamic_location))(input)?;
    Ok((input, BasicBlockOp::GetChar(loc)))
}

/// Parse the `putchar` instruction:
///   putchar <operand>
fn parse_putchar(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("putchar"))(input)?;
    let (input, op) = parse_operand(input)?;
    Ok((input, BasicBlockOp::PutChar(op)))
}

/// Parse the `putchar` instruction:
///   putchar <operand>
fn parse_putint(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("putint"))(input)?;
    let (input, op) = parse_operand(input)?;
    Ok((input, BasicBlockOp::PutInt(op)))
}

/// A helper for binary operations (add, sub, etc.) that take the form:
///   <op> <lhs>, <rhs>, <dest>
/// where lhs and rhs are operands and dest is a dynamic location.
fn parse_binary_op<'a, F>(
    op_name: &'static str,
    constructor: F,
) -> impl FnMut(&'a str) -> Res<'a, BasicBlockOp>
where
    F: Fn(Operand, Operand, DynamicLocation) -> BasicBlockOp,
{
    move |input: &str| {
        let (input, dest) = parse_dynamic_location(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = ws(tag(op_name))(input)?;
        let (input, _) = space0(input)?;
        let (input, lhs) = parse_operand(input)?;
        let (input, rhs) = opt(preceded(ws(char(',')), parse_operand))(input)?;
        match rhs {
            // Pass the dst as the lhs 
            None => Ok((input, constructor(Operand::Location(dest), lhs, dest))),
            Some(rhs) => Ok((input, constructor(lhs, rhs, dest)))
        }
    }
}

/// Parse the `add` instruction.
fn parse_add(input: &str) -> Res<BasicBlockOp> {
    parse_binary_op("add", |lhs, rhs, dest| BasicBlockOp::Add { lhs, rhs, dest })(input)
}

/// Parse the `sub` instruction.
fn parse_sub(input: &str) -> Res<BasicBlockOp> {
    parse_binary_op("sub", |lhs, rhs, dest| BasicBlockOp::Sub { lhs, rhs, dest })(input)
}

/// Parse the `mul` instruction.
fn parse_mul(input: &str) -> Res<BasicBlockOp> {
    parse_binary_op("mul", |lhs, rhs, dest| BasicBlockOp::Mul { lhs, rhs, dest })(input)
}

/// Parse the `div` instruction.
fn parse_div(input: &str) -> Res<BasicBlockOp> {
    parse_binary_op("div", |lhs, rhs, dest| BasicBlockOp::Div { lhs, rhs, dest })(input)
}

/// Parse the `mod` instruction.
fn parse_mod(input: &str) -> Res<BasicBlockOp> {
    parse_binary_op("mod", |lhs, rhs, dest| BasicBlockOp::Mod { lhs, rhs, dest })(input)
}

/// Parse the `eq` instruction.
fn parse_eq(input: &str) -> Res<BasicBlockOp> {
    parse_binary_op("eq", |lhs, rhs, dest| BasicBlockOp::Eq { lhs, rhs, dest })(input)
}

/// Parse the `ne` instruction.
fn parse_ne(input: &str) -> Res<BasicBlockOp> {
    parse_binary_op("ne", |lhs, rhs, dest| BasicBlockOp::Ne { lhs, rhs, dest })(input)
}

/// Parse the unary `neg` instruction:
///   neg <src>, <dest>
fn parse_neg(input: &str) -> Res<BasicBlockOp> {
    let (input, _) = ws(tag("neg"))(input)?;
    let (input, _) = space0(input)?;
    let (input, src) = parse_operand(input)?;
    let (input, _) = ws(char(','))(input)?;
    let (input, dest) = parse_dynamic_location(input)?;
    Ok((input, BasicBlockOp::Neg { src, dest }))
}

/// Parse a label line: an identifier immediately followed by a colon.
fn parse_label(input: &str) -> Res<Symbol> {
    let (input, _) = space0(input)?;
    let (input, result) = map(terminated(parse_identifier, ws(char(':'))), Symbol::from)(input)?;
    let (input, _) = cut(parse_end_of_lines)(input)?;
    Ok((input, result))
}

/// Parse the `quit` instruction:
///   quit
fn parse_quit(input: &str) -> Res<Op> {
    let (input, _) = space0(input)?;
    let (input, _) = tag("quit")(input)?;
    let (input, _) = cut(parse_end_of_lines)(input)?;
    Ok((input, Op::Quit(next_basic_block_number())))
}

/// Parse the `jmp` instruction:
///   jmp <label>
fn parse_jmp(input: &str) -> Res<Op> {
    let (input, _) = ws(tag("jmp"))(input)?;
    let (input, _) = space0(input)?;
    let (input, label) = parse_identifier(input)?;
    let (input, _) = cut(parse_end_of_lines)(input)?;
    Ok((input, Op::Jmp(next_basic_block_number(), label)))
}

/// Parse the `call` instruction:
///   call <label>
fn parse_call(input: &str) -> Res<Op> {
    let (input, _) = ws(tag("call"))(input)?;
    let (input, _) = space0(input)?;
    let (input, label) = parse_identifier(input)?;
    let (input, _) = cut(parse_end_of_lines)(input)?;
    Ok((input, Op::Call(next_basic_block_number(), label)))
}

/// Parse the `ret` instruction:
///  ret
fn parse_ret(input: &str) -> Res<Op> {
    let (input, _) = ws(tag("ret"))(input)?;
    let (input, _) = cut(parse_end_of_lines)(input)?;
    Ok((input, Op::Return(next_basic_block_number())))
}

/// Parse the `jmp_if` instruction:
///   jmp_if <dynamic_location>, <label>
fn parse_jmp_if(input: &str) -> Res<Op> {
    let (input, _) = ws(tag("jmp_if"))(input)?;
    let (input, _) = space0(input)?;
    let (input, loc) = parse_dynamic_location(input)?;
    let (input, _) = ws(char(','))(input)?;
    let (input, label) = parse_identifier(input)?;
    let (input, _) = cut(parse_end_of_lines)(input)?;
    Ok((input, Op::JmpIf(next_basic_block_number(), loc, label)))
}

fn parse_end_of_line(input: &str) -> Res<()> {
    let (input, _) = space0(input)?;
    let (input, _) = char('\n')(input)?;
    Ok((input, ()))
}

fn parse_end_of_lines(input: &str) -> Res<()> {
    alt((
        map(many1(parse_end_of_line), |_| ()),
        map(eof, |_| ()),
    ))(input)
}

/// Parse one “line” of assembly. This line can be a basic block op,
/// a control op, or a label.
fn parse_basic_block_op(input: &str) -> Res<BasicBlockOp> {
    let (input, op) = ws(alt((
        parse_hex_dump,
        parse_dec_dump,
        parse_inc,
        parse_dec,
        parse_set,
        parse_lea,
        parse_getchar,
        parse_putchar,
        parse_putint,
        parse_push,
        parse_pop,
        parse_add,
        parse_sub,
        parse_mul,
        parse_div,
        parse_mod,
        parse_neg,
        parse_eq,
        parse_ne,
    )))(input)?;
    let (input, _) = cut(parse_end_of_lines)(input)?;
    Ok((input, op))
}

fn parse_basic_block(label: Option<Symbol>, input: &str) -> Res<BasicBlock> {
    let (input, _) = space0(input)?;
    // let (input, ops) = separated_list0(multispace0, parse_basic_block_op)(input)?;
    let (input, ops) = many0(parse_basic_block_op)(input)?;
    Ok((input, BasicBlock::new(label, ops)))
    //     map(parse_push, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_pop, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_getchar, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_putchar, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_add, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_sub, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_mul, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_div, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_mod, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_neg, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_eq, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_ne, |op| Instruction::BasicBlockOp(op)),
    //     map(parse_jmp, |op| Instruction::ControlOp(op)),
    //     map(parse_jmp_if, |op| Instruction::ControlOp(op)),
    //     map(parse_label, |lbl| Instruction::Label(lbl)),
    // ))(input)
}

fn parse_labeled_basic_block(input: &str) -> Res<Op> {
    // First try to parse a label
    let (input, _) = space0(input)?;
    let (input, label) = opt(parse_label)(input)?;
    let (input, _) = space0(input)?;
    // Then parse the basic block op
    if let Some(label) = label {
        let (input, bb) = parse_basic_block(Some(label.clone()), input)?;
        Ok((input, Op::Label(label, bb)))
    } else {
        let (input, bb) = parse_basic_block(None, input)?;
        Ok((input, Op::BasicBlock(bb)))
    }
}

fn parse_op(input: &str) -> Res<Op> {
    if input == "" {
        return Err(nom::Err::Error(nom::error::VerboseError::from_error_kind(
            input,
            nom::error::ErrorKind::Fail,
        )));
    }
    // BasicBlock(BasicBlock),
    // Label(Symbol, BasicBlock),
    // Jmp(Symbol),
    // JmpIf(DynamicLocation, Symbol),
    let (input, _) = space0(input)?;
    let (input, op) = alt((
        parse_quit,
        parse_call,
        parse_ret,
        parse_jmp_if,
        parse_jmp,
        parse_labeled_basic_block,
    ))(input)?;

    Ok((input, op))
}

/// Parse a full program – a list of instructions separated by optional whitespace.
fn parse_program(input: &str) -> Res<Program> {
    let (input, _) = multispace0(input)?;
    // let (input, result) = map(separated_list0(multispace0, parse_op), Program)(input)?;
    let (input, mut result) = map(many0(parse_op), Program)(input)?;
    result.push(Op::Quit(next_basic_block_number()));
    let (input, _) = multispace0(input)?;

    Ok((input, result))
}

fn strip_comments(input: &str) -> String {
    let mut output = String::new();
    let mut input_chars = input.chars().peekable();
    while let Some(c) = input_chars.next() {
        if c == '/' {
            if let Some('/') = input_chars.peek() {
                // Skip the rest of the line
                while let Some(c) = input_chars.next() {
                    if c == '\n' {
                        break;
                    }
                }
            } else if let Some('*') = input_chars.peek() {
                // Skip the block comment
                while let Some(c) = input_chars.next() {
                    if c == '*' {
                        if let Some('/') = input_chars.peek() {
                            input_chars.next();
                            break;
                        }
                    }
                }
            } else {
                output.push(c);
            }
        } else if c == ';' {
            // Skip the rest of the line
            while let Some(c) = input_chars.next() {
                if c == '\n' {
                    break;
                }
            }
        } else {
            output.push(c);
        }
    }
    output
}

pub fn parse(input: &str) -> Result<Program, String> {
    match parse_program(&strip_comments(input)) {
        Ok((rest, program)) => {
            if rest.is_empty() {
                Ok(program)
            } else {
                Err(format!("Failed to parse the entire input. Remaining: {}", rest))
            }
        }
        Err(e) => {
            match e {
                nom::Err::Error(e) => Err(format!("Error: {}", convert_error(input, e))),
                nom::Err::Failure(e) => Err(format!("Failure: {}", convert_error(input, e))),
                nom::Err::Incomplete(_) => Err("Incomplete input".into()),
            }
        },
    }
}