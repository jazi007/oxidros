use super::{annotations::parse_annotation_apps, PResult, Span};
use crate::{
    character::{BELL, BS, CR, CR_S, FF, HT, HT_S, LF, LF_S, VT, VT_S},
    expr::{
        AnnotationAndDef, AnyDeclarator, ArrayDeclarator, Case, CaseLabel, ConstDcl, ConstExpr,
        ConstType, ConstrTypeDcl, Definition, ElementSpec, EnumDcl, FixedPoint, FixedPtType,
        Literal, Member, Module, NativeDcl, PrimitiveType, ScopedName, SequenceType, StringType,
        StructDcl, StructDef, StructForwardDcl, SwitchTypeSpec, TemplateTypeSpec, TypeDcl,
        TypeSpec, Typedef, TypedefType, UnaryOpExpr, UnionDcl, UnionDef, UnionForwardDcl,
        WStringType,
    },
    parser::{
        annotations::parse_annotation_dcl,
        components::{
            parse_component_dcl, parse_connector_dcl, parse_home_dcl, parse_porttype_dcl,
        },
        extended_data_types::{parse_bitmask_dcl, parse_bitset_dcl, parse_map_type},
        interfaces::{parse_except_dcl, parse_interface_dcl},
        template::{parse_template_module_dcl, parse_template_module_inst},
        value_types::parse_value_dcl,
    },
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::{
        complete::{anychar, digit0, digit1, hex_digit1, oct_digit0, one_of, satisfy},
        is_alphabetic, is_alphanumeric, is_hex_digit, is_oct_digit,
    },
    combinator::fail,
    error::{Error, VerboseError},
    multi::{many0, many1, separated_list1},
    sequence::{delimited, tuple},
};
use nom_greedyerror::AsStr;
use num_bigint::BigInt;
use num_traits::Zero;

pub fn lparen<'a>(lparen: &'a str) -> impl FnMut(Span<'a>) -> PResult<'a, ((), Span<'a>, ())> {
    tuple((
        skip_space_and_comment0,
        tag(lparen),
        skip_space_and_comment0,
    ))
}

pub fn rparen<'a>(rparen: &'a str) -> impl FnMut(Span<'a>) -> PResult<'a, ((), Span<'a>)> {
    tuple((skip_space_and_comment0, tag(rparen)))
}

pub fn delimiter<'a>(d: &'a str) -> impl FnMut(Span<'a>) -> PResult<'a, ((), Span<'a>, ())> {
    lparen(d)
}

fn parse_comment(input: Span) -> PResult<Span> {
    let (input, c) = alt((tag("//"), tag("/*")))(input)?;

    let (input, comment) = match c.as_str() {
        "//" => alt((take_until("\n"), take_while(|_| true)))(input)?,
        "/*" => {
            let (input, c) = take_until("*/")(input)?;
            let (input, _) = tag("*/")(input)?;
            (input, c)
        }
        _ => unreachable!(),
    };

    Ok((input, comment))
}

pub fn skip_space_and_comment0(input: Span) -> PResult<()> {
    let (input, _) = many0(alt((
        tag(" "),
        tag(LF_S),
        tag(CR_S),
        tag(HT_S),
        tag(VT_S),
        parse_comment,
    )))(input)?;

    Ok((input, ()))
}

pub fn skip_space_and_comment1(input: Span) -> PResult<()> {
    let (input, _) = many1(alt((
        tag(" "),
        tag(LF_S),
        tag(CR_S),
        tag(HT_S),
        tag(VT_S),
        parse_comment,
    )))(input)?;

    Ok((input, ()))
}

/// ```text
/// <identifier>
/// ```
pub fn parse_id(input: Span) -> PResult<String> {
    let (input, head) = satisfy(|c| c == '_' || is_alphabetic(c as u8))(input)?;
    let (input, tail) = take_while(|c| c == '_' || is_alphanumeric(c as u8))(input)?;

    let mut result = String::new();
    result.push(head);
    result.push_str(tail.as_str());
    Ok((input, result))
}

/// ```text
/// (  2) <definition> ::= <module_dcl> ";"
///                      | <const_dcl> ";"
///                      | <type_dcl> ";"
///
/// ( 71) <definition> ::+ <except_dcl> ";"
///                      | <interface_dcl> ";"
///
/// ( 98) <definition> ::+ <value_dcl> ";"
///
/// (111) <definition> ::+ <type_id_dcl> ";"
///                      | <type_prefix_dcl> ";"
///                      | <import_dcl> ";"
///
/// (133) <definition> ::+ <component_dcl> ";"
///
/// (144) <definition> ::+ <home_dcl> ";"
///
/// (153) <definition> ::+ <event_dcl> ";"
///
/// (171) <definition> ::+ <porttype_dcl> ";"
///                      | <connector_dcl> ";"
///
/// (184) <definition> ::+ <template_module_dcl> ";"
///                      | <template_module_inst> ";"
///
/// (218) <definition> ::+ <annotaion_dcl> ";"
/// ```
pub fn parse_definition(input: Span) -> PResult<AnnotationAndDef> {
    fn module(input: Span) -> PResult<Definition> {
        let (input, def) = parse_module_dcl(input)?;
        Ok((input, Definition::Module(def)))
    }

    fn const_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_const_dcl(input)?;
        Ok((input, Definition::Const(def)))
    }

    fn type_dcl<'a>(head: &str, input: Span<'a>) -> PResult<'a, Definition> {
        let (input, def) = parse_type_dcl(head, input)?;
        Ok((input, Definition::Type(def)))
    }

    fn except_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_except_dcl(input)?;
        Ok((input, Definition::Except(def)))
    }

    fn interface_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_interface_dcl(input)?;
        Ok((input, Definition::Interface(def)))
    }

    fn value_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_value_dcl(input)?;
        Ok((input, Definition::Value(def)))
    }

    fn component_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_component_dcl(input)?;
        Ok((input, Definition::Component(def)))
    }

    fn home_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_home_dcl(input)?;
        Ok((input, Definition::Home(def)))
    }

    fn port_type_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_porttype_dcl(input)?;
        Ok((input, Definition::PortType(def)))
    }

    fn connector_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_connector_dcl(input)?;
        Ok((input, Definition::Connector(def)))
    }

    fn template_module_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_template_module_dcl(input)?;
        Ok((input, Definition::TemplateModuleDcl(def)))
    }

    fn template_module_inst(input: Span) -> PResult<Definition> {
        let (input, def) = parse_template_module_inst(input)?;
        Ok((input, Definition::TemplateModuleInst(def)))
    }

    fn annotation_dcl(input: Span) -> PResult<Definition> {
        let (input, def) = parse_annotation_dcl(input)?;
        Ok((input, Definition::Annotation(def)))
    }

    let (input, _) = skip_space_and_comment0(input)?;

    let (input, annotations) = if tag::<&str, Span, Error<Span>>("@")(input).is_ok() {
        if tuple((tag("annotation"), skip_space_and_comment1))(input).is_ok() {
            let (input, result) = annotation_dcl(input)?;
            let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;
            return Ok((
                input,
                AnnotationAndDef {
                    definition: result,
                    annotations: None,
                },
            ));
        } else {
            let (input, annotation) = parse_annotation_apps(input)?;
            (input, Some(annotation))
        }
    } else {
        (input, None)
    };

    let (input, _) = skip_space_and_comment0(input)?;

    let (input, (head, _)) = tuple((
        alt((
            tag("module"),
            tag("const"),
            tag("struct"),
            tag("enum"),
            tag("union"),
            tag("native"),
            tag("bitset"),
            tag("bitmask"),
            tag("typedef"),
            tag("exception"),
            tag("interface"),
            tag("valuetype"),
            tag("component"),
            tag("home"),
            tag("porttype"),
            tag("connector"),
        )),
        skip_space_and_comment1,
    ))(input)?;

    let (input, result) = match head.as_str() {
        "module" => alt((module, template_module_dcl, template_module_inst))(input)?,
        "const" => const_dcl(input)?,
        "struct" | "enum" | "union" | "bitmask" | "bitset" | "typedef" | "native" => {
            type_dcl(head.as_str(), input)?
        }
        "exception" => except_dcl(input)?,
        "interface" => interface_dcl(input)?,
        "valuetype" => value_dcl(input)?,
        "component" => component_dcl(input)?,
        "home" => home_dcl(input)?,
        "porttype" => port_type_dcl(input)?,
        "connector" => connector_dcl(input)?,
        _ => unreachable!(),
    };

    let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;

    Ok((
        input,
        AnnotationAndDef {
            definition: result,
            annotations,
        },
    ))
}

/// ```text
/// (3) <module_dcl> ::= "module" <identifier> "{" <definition>+ "}"
/// ```
fn parse_module_dcl(input: Span) -> PResult<Module> {
    // <identifier>
    let (input, id) = parse_id(input)?;

    // "{" <definition>+ "}"
    let (input, (_, definitions)) = tuple((lparen("{"), definitions))(input)?;

    Ok((input, Module { id, definitions }))
}

pub fn definitions(mut input: Span) -> PResult<Vec<AnnotationAndDef>> {
    let mut result = Vec::new();

    loop {
        if let Ok((i, _)) = rparen("}")(input) {
            return Ok((i, result));
        }

        let (i, def) = parse_definition(input)?;
        result.push(def);

        input = i;
    }
}

/// ```text
/// (4) <scoped_name> ::= <identifier>
///                     | "::" <identifier>
///                     | <scoped_name> "::" <identifier>
/// ```
pub fn parse_scoped_name(input: Span) -> PResult<ScopedName> {
    fn relative(input: Span) -> PResult<ScopedName> {
        let (input, (_, result)) = tuple((tag("::"), separated_list1(tag("::"), parse_id)))(input)?;
        Ok((input, ScopedName::Relative(result)))
    }

    fn absolute(input: Span) -> PResult<ScopedName> {
        let (input, ids) = separated_list1(tag("::"), parse_id)(input)?;
        Ok((input, ScopedName::Absolute(ids)))
    }

    alt((relative, absolute))(input)
}

/// ```text
/// (5) <const_dcl> ::= "const" <const_type> <identifier> "=" <const_expr>
/// ```
pub fn parse_const_dcl(input: Span) -> PResult<ConstDcl> {
    // <const_type>
    let (input, const_type) = parse_const_type(input)?;

    // <identifier>
    let (input, _) = skip_space_and_comment1(input)?;
    let (input, id) = parse_id(input)?;

    // "="
    let (input, _) = delimiter("=")(input)?;

    // <const_expr>
    let (input, expr) = parse_const_expr(input, true)?;

    Ok((
        input,
        ConstDcl {
            const_type,
            id,
            expr,
        },
    ))
}

/// ```text
/// (6) <const_type> ::= <integer_type>
///                    | <floating_pt_type>
///                    | <fixed_pt_const_type>
///                    | <char_type>
///                    | <wide_char_type>
///                    | <boolean_type>
///                    | <octet_type>
///                    | <string_type>
///                    | <wide_string_type>
///                    | <scoped_name>
/// ```
pub fn parse_const_type(input: Span) -> PResult<ConstType> {
    if let Ok((input, result)) = alt((parse_int_words, parse_long_double))(input) {
        return Ok((input, ConstType::PrimitiveType(result)));
    }

    if let Ok((input, result)) = parse_string_type(input) {
        return Ok((input, ConstType::StringType(result)));
    }

    if let Ok((input, result)) = parse_wide_string_type(input) {
        return Ok((input, ConstType::WStringType(result)));
    }

    let (input, name) = parse_scoped_name(input)?;

    if let Some(p) = name.to_primitive() {
        return Ok((input, ConstType::PrimitiveType(p)));
    }

    if let ScopedName::Absolute(s) = &name {
        if s.len() >= 2 {
            return Ok((input, ConstType::ScopedName(name)));
        }

        match s[0].as_str() {
            "string" => Ok((input, ConstType::StringType(StringType::UnlimitedSize))),
            "wstring" => Ok((input, ConstType::WStringType(WStringType::UnlimitedSize))),
            "fixed" => Ok((input, ConstType::FixedPointConst)),
            _ => Ok((input, ConstType::ScopedName(name))),
        }
    } else {
        Ok((input, ConstType::ScopedName(name)))
    }
}

/// ```text
/// ( 7) <const_expr> ::= <or_expr>
/// (19) <positive_int_const> ::= <const_expr>
/// ```
pub fn parse_const_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    parse_or_expr(input, shift)
}

/// ```text
/// (8) <or_expr> ::= <xor_expr>
///                 | <or_expr> "|" <xor_expr>
/// ```
fn parse_or_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    let (input, left) = parse_xor_expr(input, shift)?;

    if let Ok((input, _)) = delimiter("|")(input) {
        let (input, right) = parse_or_expr(input, shift)?;
        Ok((input, ConstExpr::Or(Box::new(left), Box::new(right))))
    } else {
        Ok((input, left))
    }
}

/// ```text
/// (9) <xor_expr> ::= <and_expr>
///                  | <xor_expr> "^" <and_expr>
/// ```
fn parse_xor_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    let (input, left) = parse_and_expr(input, shift)?;

    if let Ok((input, _)) = delimiter("^")(input) {
        let (input, right) = parse_xor_expr(input, shift)?;
        Ok((input, ConstExpr::Xor(Box::new(left), Box::new(right))))
    } else {
        Ok((input, left))
    }
}

/// ```text
/// (10) <and_expr> ::= <shift_expr>
///                   | <and_expr> "&" <shift_expr>
/// ```
fn parse_and_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    let (input, left) = parse_shift_expr(input, shift)?;

    if let Ok((input, _)) = delimiter("&")(input) {
        let (input, right) = parse_and_expr(input, shift)?;
        Ok((input, ConstExpr::And(Box::new(left), Box::new(right))))
    } else {
        Ok((input, left))
    }
}

/// ```text
/// (11) <shift_expr> ::= <add_expr>
///                     | <shift_expr> ">>" <add_expr>
///                     | <shift_expr> "<<" <add_expr>
/// ```
fn parse_shift_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    let (input, left) = parse_add_expr(input, shift)?;

    if let Ok((input, (_, op, _))) = if shift {
        alt((delimiter(">>"), delimiter("<<")))(input)
    } else {
        delimiter("<<")(input)
    } {
        let (input, right) = parse_shift_expr(input, shift)?;

        match op.as_str() {
            ">>" => Ok((input, ConstExpr::RShift(Box::new(left), Box::new(right)))),
            "<<" => Ok((input, ConstExpr::LShift(Box::new(left), Box::new(right)))),
            _ => unreachable!(),
        }
    } else {
        Ok((input, left))
    }
}

/// ```text
/// (12) <add_expr> ::= <mult_expr>
///                   | <add_expr> "+" <mult_expr>
///                   | <add_expr> "-" <mult_expr>
/// ```
fn parse_add_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    let (input, left) = parse_mult_expr(input, shift)?;

    if let Ok((input, (_, op, _))) = alt((delimiter("+"), delimiter("-")))(input) {
        let (input, right) = parse_add_expr(input, shift)?;

        match op.as_str() {
            "+" => Ok((input, ConstExpr::Add(Box::new(left), Box::new(right)))),
            "-" => Ok((input, ConstExpr::Sub(Box::new(left), Box::new(right)))),
            _ => unreachable!(),
        }
    } else {
        Ok((input, left))
    }
}

/// ```text
/// (13) <mult_expr> ::= <unary_expr>
///                    | <mult_expr> "*" <unary_expr>
///                    | <mult_expr> "/" <unary_expr>
///                    | <mult_expr> "%" <unary_expr>
/// ```
fn parse_mult_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    let (input, left) = parse_unary_expr(input, shift)?;

    if let Ok((input, (_, op, _))) = alt((delimiter("*"), delimiter("/"), delimiter("%")))(input) {
        let (input, right) = parse_mult_expr(input, shift)?;

        match op.as_str() {
            "*" => Ok((input, ConstExpr::Mul(Box::new(left), Box::new(right)))),
            "/" => Ok((input, ConstExpr::Div(Box::new(left), Box::new(right)))),
            "%" => Ok((input, ConstExpr::Mod(Box::new(left), Box::new(right)))),
            _ => unreachable!(),
        }
    } else {
        Ok((input, left))
    }
}

/// ```text
/// (14) <unary_expr> ::= <unary_operator> <primary_expr>
///                     | <primary_expr>
/// (15) <unary_operator> ::= "-" | "+" | "~"
/// ```
fn parse_unary_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    let (input, _) = skip_space_and_comment0(input)?;

    let (input, op) = if let Ok((input, op)) =
        alt((tag::<&str, Span, Error<Span>>("+"), tag("-"), tag("~")))(input)
    {
        (input, op)
    } else {
        (input, Span::new(""))
    };

    let (input, _) = skip_space_and_comment0(input)?;
    let (input, expr) = parse_primary_expr(input, shift)?;

    let expr = match op.as_str() {
        "-" => UnaryOpExpr::Minus(Box::new(expr)),
        "~" => UnaryOpExpr::Negate(Box::new(expr)),
        "+" => UnaryOpExpr::Plus(Box::new(expr)),
        "" => return Ok((input, expr)),
        _ => unreachable!(),
    };

    Ok((input, ConstExpr::UnaryOp(expr)))
}

/// ```text
/// (16) <primary_expr> ::= <scoped_name>
///                       | <literal>
///                       | "(" <const_expr> ")"
/// ```
fn parse_primary_expr(input: Span, shift: bool) -> PResult<ConstExpr> {
    fn expr_literal(input: Span) -> PResult<ConstExpr> {
        let (input, literal) = parse_literal(input)?;
        Ok((input, ConstExpr::Literal(literal)))
    }

    fn expr_scoped_name(input: Span) -> PResult<ConstExpr> {
        let (input, name) = parse_scoped_name(input)?;
        Ok((input, ConstExpr::ScopedName(name)))
    }

    alt((
        expr_literal,
        expr_scoped_name,
        delimited(lparen("("), |i| parse_const_expr(i, shift), rparen(")")),
    ))(input)
}

/// ```text
/// (17) <literal> ::= <integer_literal>
///                  | <floating_pt_literal>
///                  | <fixed_pt_literal>
///                  | <character_literal>
///                  | <wide_character_literal>
///                  | <boolean_literal>
///                  | <string_literal>
///                  | <wide_string_literal>
/// ```
fn parse_literal(input: Span) -> PResult<Literal> {
    alt((
        parse_num,
        parse_hex,
        parse_octal,
        parse_boolean_literal,
        parse_char,
        parse_string,
    ))(input)
}

/// ```test
/// (18) <boolean_literal> ::= "TRUE" | "FALSE"
/// ```
fn parse_boolean_literal(input: Span) -> PResult<Literal> {
    let (input, value) = alt((tag("TRUE"), tag("FALSE")))(input)?;

    match value.as_str() {
        "TRUE" => Ok((input, Literal::Boolean(true))),
        "FALSE" => Ok((input, Literal::Boolean(false))),
        _ => unreachable!(),
    }
}

fn parse_sign(input: Span) -> PResult<bool> {
    let (input, sign) =
        tag::<&str, Span, Error<Span>>("-")(input).unwrap_or((input, Span::new("")));
    match sign.as_str() {
        "-" => Ok((input, true)),
        "" => Ok((input, false)),
        _ => unreachable!(),
    }
}

fn parse_num(input: Span) -> PResult<Literal> {
    let (input, sign) = parse_sign(input)?;

    let (input, mut integer) = if let Ok((input, _)) = tag::<&str, Span, Error<Span>>("0.")(input) {
        // floating point
        (input, BigInt::zero())
    } else {
        let (input, num) = parse_decimal(input)?;
        if let Ok((input, _)) = tag::<&str, Span, Error<Span>>(".")(input) {
            // floating point
            (input, num)
        } else {
            // decimal
            let result = if sign { num * -1 } else { num };
            return Ok((input, Literal::Integer(result)));
        }
    };

    // floating- or fixed-point
    let (input, fraction) = digit1(input)?;
    if let Ok((input, c)) = alt((
        tag::<&str, Span, Error<Span>>("d"),
        tag("D"),
        tag("e"),
        tag("E"),
    ))(input)
    {
        match c.as_str() {
            "d" | "D" => {
                let mut scale = 0;
                for n in fraction.bytes() {
                    let val = n - b'0';
                    integer *= 10;
                    integer += val;
                    scale += 1;
                }
                let value = if sign { integer * -1 } else { integer };
                let result = Literal::FixedPoint(FixedPoint { value, scale });
                Ok((input, result))
            }
            "e" | "E" => {
                let (input, exp_sign) = tag::<&str, Span, VerboseError<Span>>("-")(input)
                    .unwrap_or((input, Span::new("")));
                let sign = if sign { "-" } else { "" };

                let (input, float_num) = if exp_sign.as_str() == "-" {
                    let (input, exp) = parse_decimal(input)?;
                    (input, format!("{sign}{integer}.{fraction}e-{exp}"))
                } else if let Ok((input, exp)) = parse_decimal(input) {
                    (input, format!("{sign}{integer}.{fraction}e{exp}"))
                } else {
                    (input, format!("{sign}{integer}.{fraction}"))
                };

                let result = float_num
                    .parse::<f64>()
                    .expect("failed to parse a floating point number");
                Ok((input, Literal::FloatingPoint(result)))
            }
            _ => unreachable!(),
        }
    } else {
        let sign = if sign { "-" } else { "" };
        let float_num = format!("{sign}{integer}.{fraction}");
        let result = float_num
            .parse::<f64>()
            .expect("failed to parse a floating point number");

        Ok((input, Literal::FloatingPoint(result)))
    }
}

/// [1-0][0-9]*
fn parse_decimal(input: Span) -> PResult<BigInt> {
    let (input, head) = one_of("123456789")(input)?;
    let (input, tail) = digit0(input)?;

    let mut result = BigInt::zero();
    let n = head as u8 - b'0';
    result += n;

    for n in tail.as_bytes().iter() {
        let val = *n - b'0';
        result *= 10;
        result += val;
    }

    Ok((input, result))
}

/// 0[0-8]*
fn parse_octal(input: Span) -> PResult<Literal> {
    let (input, _) = tag("0")(input)?;
    let (input, tail) = oct_digit0(input)?;

    let mut result = BigInt::zero();
    for n in tail.as_bytes().iter() {
        let val = *n - b'0';
        result *= 8;
        result += val;
    }

    Ok((input, Literal::Integer(result)))
}

fn hex_to_num(hex: char) -> u8 {
    match hex {
        'a' | 'A' => 10,
        'b' | 'B' => 11,
        'c' | 'C' => 12,
        'd' | 'D' => 13,
        'e' | 'E' => 14,
        'f' | 'F' => 15,
        _ => hex as u8 - b'0',
    }
}

/// (0x|0X)[0-9a-fA-F]+
fn parse_hex(input: Span) -> PResult<Literal> {
    let (input, _) = alt((tag("0x"), tag("0X")))(input)?;
    let (input, tail) = hex_digit1(input)?;

    let mut result = BigInt::zero();
    for n in tail.as_bytes().iter() {
        result *= 16;
        result += hex_to_num(*n as char);
    }

    Ok((input, Literal::Integer(result)))
}

fn parse_string(input: Span) -> PResult<Literal> {
    let (input, head) = alt((tag("\""), tag("L\"")))(input)?;
    let (input, mut result) = parse_string_tail(input)?;

    let (input, v) = if head.as_str() == "\"" {
        many0(parse_normal_string)(input)?
    } else {
        many0(parse_wide_string)(input)?
    };

    v.iter().for_each(|s| result.push_str(s.as_str()));
    Ok((input, Literal::String(result)))
}

fn parse_string_tail(mut input: Span) -> PResult<String> {
    let mut result = String::new();

    loop {
        if let Ok((input, _)) = tag::<&str, Span, Error<Span>>("\"")(input) {
            return Ok((input, result));
        };

        let (next, c) = parse_char_escape(input)?;
        result.push(c);
        input = next;
    }
}

fn parse_normal_string(input: Span) -> PResult<String> {
    let (input, _) = tuple((skip_space_and_comment0, tag("\"")))(input)?;
    parse_string_tail(input)
}

fn parse_wide_string(input: Span) -> PResult<String> {
    let (input, _) = tuple((skip_space_and_comment0, tag("L\"")))(input)?;
    parse_string_tail(input)
}

/// 'c', where `c` is a character or escaped character
fn parse_char(input: Span) -> PResult<Literal> {
    let (input, _) = alt((tag("'"), tag("L'")))(input)?;
    let (input, result) = parse_char_escape(input)?;
    let (input, _) = tag("'")(input)?;

    Ok((input, Literal::Char(result)))
}

fn parse_char_escape(input: Span) -> PResult<char> {
    if let Ok((input, _)) = tag::<&str, Span, Error<Span>>("\\")(input) {
        let (input, c) = anychar(input)?;
        match c {
            'n' => Ok((input, LF)),
            't' => Ok((input, HT)),
            'v' => Ok((input, VT)),
            'b' => Ok((input, BS)),
            'r' => Ok((input, CR)),
            'f' => Ok((input, FF)),
            'a' => Ok((input, BELL)),
            '\\' => Ok((input, '\\')),
            '?' => Ok((input, '?')),
            '\'' => Ok((input, '\'')),
            '"' => Ok((input, '"')),
            'o' => {
                // \ooo
                let original = input;
                let (input, high) = satisfy(|n| is_oct_digit(n as u8))(input)?;
                let (input, mid) = satisfy(|n| is_oct_digit(n as u8))(input)?;
                let (input, low) = satisfy(|n| is_oct_digit(n as u8))(input)?;

                let high = high as u32 - '0' as u32;
                let mid = mid as u32 - '0' as u32;
                let low = low as u32 - '0' as u32;

                let val = high * 8 * 8 + mid * 8 + low;
                if let Some(result) = char::from_u32(val) {
                    Ok((input, result))
                } else {
                    fail(original)
                }
            }
            'x' => {
                // \xhh
                let (input, high) = satisfy(|n| is_hex_digit(n as u8))(input)?;
                let (input, low) = satisfy(|n| is_hex_digit(n as u8))(input)?;

                let high = hex_to_num(high);
                let low = hex_to_num(low);

                let val = high * 16 + low;
                Ok((input, val as char))
            }
            'u' => {
                // \uhhhh
                let original = input;
                let (input, hex1) = satisfy(|n| is_hex_digit(n as u8))(input)?;
                let (input, hex2) = satisfy(|n| is_hex_digit(n as u8))(input)?;
                let (input, hex3) = satisfy(|n| is_hex_digit(n as u8))(input)?;
                let (input, hex4) = satisfy(|n| is_hex_digit(n as u8))(input)?;

                let hex1 = hex_to_num(hex1) as u32;
                let hex2 = hex_to_num(hex2) as u32;
                let hex3 = hex_to_num(hex3) as u32;
                let hex4 = hex_to_num(hex4) as u32;

                let val = hex1 * 16 * 16 * 16 + hex2 * 16 * 16 + hex3 * 16 + hex4;

                if let Some(result) = char::from_u32(val) {
                    Ok((input, result))
                } else {
                    fail(original)
                }
            }
            c => Ok((input, c)),
        }
    } else {
        anychar(input)
    }
}

/// ```text
/// (20) <type_dcl> ::= <constr_type_dcl>
///                   | <native_dcl>
///                   | <typedef_dcl>
/// ```
pub fn parse_type_dcl<'a>(head: &str, input: Span<'a>) -> PResult<'a, TypeDcl> {
    fn constr_type<'a>(head: &str, input: Span<'a>) -> PResult<'a, TypeDcl> {
        let (input, dcl) = parse_constr_type_dcl(head, input)?;
        Ok((input, TypeDcl::ConstrType(dcl)))
    }

    fn native(input: Span) -> PResult<TypeDcl> {
        let (input, dcl) = parse_native_dcl(input)?;
        Ok((input, TypeDcl::Native(dcl)))
    }

    fn typedef(input: Span) -> PResult<TypeDcl> {
        let (input, dcl) = parse_typedef_dcl(input)?;
        Ok((input, TypeDcl::Typedef(dcl)))
    }

    match head {
        "struct" | "enum" | "union" | "bitmask" | "bitset" => constr_type(head, input),
        "native" => native(input),
        "typedef" => typedef(input),
        _ => unreachable!(),
    }
}

/// ```text
/// ( 21) <type_spec> ::= <simple_type_spec>
/// (216) <type_spec> ::+ <template_type_spec>
/// ```
pub fn parse_type_spec(input: Span) -> PResult<TypeSpec> {
    fn template_type_spec(input: Span) -> PResult<TypeSpec> {
        let (input, t) = parse_template_type_spec(input)?;
        Ok((input, TypeSpec::Template(Box::new(t))))
    }

    alt((template_type_spec, parse_simple_type_spec))(input)
}

/// ```text
/// (22) <simple_type_spec> ::= <base_type_spec> | <scoped_name>
/// ```
fn parse_simple_type_spec(input: Span) -> PResult<TypeSpec> {
    if let Ok((input, result)) = alt((parse_int_words, parse_long_double))(input) {
        return Ok((input, TypeSpec::PrimitiveType(result)));
    }

    let (input, name) = parse_scoped_name(input)?;

    if let Some(p) = name.to_primitive() {
        Ok((input, TypeSpec::PrimitiveType(p)))
    } else {
        Ok((input, TypeSpec::ScopedName(name)))
    }
}

// ```text
// (23) <base_type_spec> ::= <integer_type>
//                         | <floating_pt_type>
//                         | <char_type>
//                         | <wide_char_type>
//                         | <boolean_type>
//                         | <octet_type>
//
// (69) <base_type_spec> ::+ <any_type>
// (70) <any_type> ::= "any"
// ```

/// Parse integer types consisting more than or equal to 2 words.
pub fn parse_int_words(input: Span) -> PResult<PrimitiveType> {
    fn long_long(input: Span) -> PResult<PrimitiveType> {
        let (input, _) = tuple((tag("long"), skip_space_and_comment1, tag("long")))(input)?;
        Ok((input, PrimitiveType::LongLong))
    }

    fn unsigned_short(input: Span) -> PResult<PrimitiveType> {
        let (input, _) = tuple((tag("unsigned"), skip_space_and_comment1, tag("short")))(input)?;
        Ok((input, PrimitiveType::UnsignedLong))
    }

    fn unsigned_long_long(input: Span) -> PResult<PrimitiveType> {
        let (input, _) = tuple((
            tag("unsigned"),
            skip_space_and_comment1,
            tag("long"),
            skip_space_and_comment1,
            tag("long"),
        ))(input)?;
        Ok((input, PrimitiveType::UnsignedLongLong))
    }

    fn unsigned_long(input: Span) -> PResult<PrimitiveType> {
        let (input, _) = tuple((tag("unsigned"), skip_space_and_comment1, tag("long")))(input)?;
        Ok((input, PrimitiveType::UnsignedLong))
    }

    alt((long_long, unsigned_short, unsigned_long_long, unsigned_long))(input)
}

fn parse_long_double(input: Span) -> PResult<PrimitiveType> {
    let (input, _) = tuple((tag("long"), skip_space_and_comment1, tag("double")))(input)?;
    Ok((input, PrimitiveType::LongDouble))
}

// ```text
// (24) <floating_pt_type> ::= "float"
//                           | "double"
//                           | "long" "double"
//
// (25) <integer_type> ::= <signed_int>
//                       | <unsigned_int>
//
// (26) <signed_int> ::= <signed_short_int>
//                     | <signed_long_int>
//                     | <signed_longlong_int>
//
// (27) <signed_short_int> ::= "short"
// (28) <signed_long_int> ::= "long"
// (29) <signed_longlong_int> ::= "long" "long"
//
// (30) <unsigned_int> ::= <unsigned_short_int>
//                       | <unsigned_long_int>
//                       | <unsigned_longlong_int>
//
// (31) <unsigned_short_int> ::= "unsigned" "short"
// (32) <unsigned_long_int> ::= "unsigned" "long"
// (33) <unsigned_longlong_int> ::= "unsigned" "long" "long"
//
// (34) <char_type> ::= "char"
// (35) <wide_char_type> ::= "wchar"
// (36) <boolean_type> ::= "boolean"
// (37) <octet_type> ::= "octet"
//
// (206) <signed_int> ::+ <signed_tiny_int>
// (207) <unsigned_int> ::+ <unsigned_tiny_int>
// (208) <signed_tiny_int>     ::= "int8"
// (209) <unsigned_tiny_int>   ::= "uint8"
// (210) <signed_short_int>    ::+ "int16"
// (211) <signed_long_int>     ::+ "int32"
// (212) <signed_longlong_int> ::+ "int64"
// (213) <signed_short_int>    ::+ "int16"
// (214) <signed_long_int>     ::+ "int32"
// (215) <signed_longlong_int> ::+ "int64"
// ```

/// ```text
/// ( 38) <template_type_spec> ::= <sequence_type>
///                              | <string_type>
///                              | <wide_string_type>
///                              | <fixed_pt_type>
///
/// (197) <template_type_spec> ::+ <map_type>
/// ```
fn parse_template_type_spec(input: Span) -> PResult<TemplateTypeSpec> {
    fn sequence(input: Span) -> PResult<TemplateTypeSpec> {
        let (input, t) = parse_sequence_type(input)?;
        Ok((input, TemplateTypeSpec::Sequence(t)))
    }

    fn string(input: Span) -> PResult<TemplateTypeSpec> {
        let (input, t) = parse_string_type(input)?;
        Ok((input, TemplateTypeSpec::String(t)))
    }

    fn wide_string(input: Span) -> PResult<TemplateTypeSpec> {
        let (input, t) = parse_wide_string_type(input)?;
        Ok((input, TemplateTypeSpec::WString(t)))
    }

    fn fixed_pt(input: Span) -> PResult<TemplateTypeSpec> {
        let (input, t) = parse_fixed_pt_type(input)?;
        Ok((input, TemplateTypeSpec::FixedPoint(t)))
    }

    fn map_type(input: Span) -> PResult<TemplateTypeSpec> {
        let (input, t) = parse_map_type(input)?;
        Ok((input, TemplateTypeSpec::Map(t)))
    }

    if tuple((
        alt((tag("string"), tag("wstring"))),
        skip_space_and_comment1,
    ))(input)
    .is_ok()
    {
        let (input, s) = alt((tag("string"), tag("wstring")))(input)?;
        match s.as_str() {
            "string" => return Ok((input, TemplateTypeSpec::String(StringType::UnlimitedSize))),
            "wstring" => return Ok((input, TemplateTypeSpec::WString(WStringType::UnlimitedSize))),
            _ => unreachable!(),
        }
    }

    alt((sequence, string, wide_string, fixed_pt, map_type))(input)
}

/// ```text
/// (39) <sequence_type> ::= "sequence" "<" <type_spec> "," <positive_int_const> ">"
///                        | "sequence" "<" <type_spec> ">"
/// ```
pub fn parse_sequence_type(input: Span) -> PResult<SequenceType> {
    let (input, _) = tuple((tag("sequence"), skip_space_and_comment0, lparen("<")))(input)?;
    let (input, type_spec) = parse_type_spec(input)?;

    let (input, _) = skip_space_and_comment0(input)?;
    let (input, c) = alt((tag(","), tag(">")))(input)?;

    match c.as_str() {
        "," => {
            let (input, _) = skip_space_and_comment0(input)?;
            let (input, expr) = parse_const_expr(input, false)?;
            let (input, _) = rparen(">")(input)?;
            Ok((input, SequenceType::Limited(type_spec, expr)))
        }
        ">" => Ok((input, SequenceType::Unlimited(type_spec))),
        _ => {
            unreachable!()
        }
    }
}

/// ```text
/// "<" <positive_int_const> ">"
/// ```
fn parse_max_size(input: Span) -> PResult<ConstExpr> {
    let (input, expr) = delimited(lparen("<"), |i| parse_const_expr(i, false), rparen(">"))(input)?;
    Ok((input, expr))
}

/// ```text
/// (40) <string_type> ::= "string" "<" <positive_int_const> ">"
///                      | "string"
/// ```
///
/// This function parses only `"string" "<" <positive_int_const> ">"`.
fn parse_string_type(input: Span) -> PResult<StringType> {
    let (input, _) = tag("string")(input)?;
    let (input, size) = parse_max_size(input)?;
    Ok((input, StringType::Sized(size)))
}

/// ```text
/// (41) <wide_string_type> ::= "wstring" "<" <positive_int_const> ">"
///                           | "wstring"
/// ```
///
/// This function parses only `"string" "<" <positive_int_const> ">"`.
fn parse_wide_string_type(input: Span) -> PResult<WStringType> {
    let (input, _) = tag("wstring")(input)?;
    let (input, size) = parse_max_size(input)?;
    Ok((input, WStringType::Sized(size)))
}

/// ```text
/// (42) <fixed_pt_type> ::= "fixed" "<" <positive_int_const> "," <positive_int_const> ">"
/// ```
fn parse_fixed_pt_type(input: Span) -> PResult<FixedPtType> {
    let (input, _) = tuple((tag("fixed"), lparen("<")))(input)?;

    let (input, total_digits) = parse_const_expr(input, false)?;

    let (input, _) = delimiter(",")(input)?;

    let (input, fractional_digits) = parse_const_expr(input, false)?;

    let (input, _) = rparen(">")(input)?;

    Ok((
        input,
        FixedPtType {
            total_digits,
            fractional_digits,
        },
    ))
}

// ```text
// (43) <fixed_pt_const_type> ::= "fixed"
// ```

/// ```text
/// ( 44) <constr_type_dcl> ::= <struct_dcl>
///                           | <union_dcl>
///                           | <enum_dcl>
///
/// (198) <constr_type_dcl> ::+ <bitset_dcl>
///                           | <bitmask_dcl>
/// ```
fn parse_constr_type_dcl<'a>(head: &str, input: Span<'a>) -> PResult<'a, ConstrTypeDcl> {
    fn struct_type(input: Span) -> PResult<ConstrTypeDcl> {
        let (input, t) = parse_struct_dcl(input)?;
        Ok((input, ConstrTypeDcl::Struct(t)))
    }

    fn enum_type(input: Span) -> PResult<ConstrTypeDcl> {
        let (input, t) = parse_enum_dcl(input)?;
        Ok((input, ConstrTypeDcl::Enum(t)))
    }

    fn union_type(input: Span) -> PResult<ConstrTypeDcl> {
        let (input, t) = parse_union_dcl(input)?;
        Ok((input, ConstrTypeDcl::Union(t)))
    }

    fn bitset(input: Span) -> PResult<ConstrTypeDcl> {
        let (input, t) = parse_bitset_dcl(input)?;
        Ok((input, ConstrTypeDcl::Bitset(t)))
    }

    fn bitmask(input: Span) -> PResult<ConstrTypeDcl> {
        let (input, t) = parse_bitmask_dcl(input)?;
        Ok((input, ConstrTypeDcl::Bitmask(t)))
    }

    match head {
        "struct" => struct_type(input),
        "union" => union_type(input),
        "enum" => enum_type(input),
        "bitset" => bitset(input),
        "bitmask" => bitmask(input),
        _ => unreachable!(),
    }
}

/// ```text
/// (45) <struct_dcl> ::= <struct_def>
///                     | <struct_forward_dcl>
/// ```
fn parse_struct_dcl(input: Span) -> PResult<StructDcl> {
    let (input, _) = skip_space_and_comment0(input)?;
    parse_struct(input)
}

/// ```text
/// ( 46) <struct_def> ::= "struct" <identifier> "{" <member>+ "}"
///
/// ( 48) <struct_forward_dcl> ::= "struct" <identifier>
///
/// (195) <struct_def> ::+ "struct" <identifier> ":" <scoped_name> "{" <member>* "}"
///                      | "struct" <identifier> "{" "}"
/// ```
fn parse_struct(input: Span) -> PResult<StructDcl> {
    // <identifier>
    let (input, id) = parse_id(input)?;

    if tuple((skip_space_and_comment0, tag(";")))(input).is_ok() {
        return Ok((input, StructDcl::ForwardDcl(StructForwardDcl(id))));
    }

    let (input, _) = skip_space_and_comment0(input)?;

    let (input, inheritance) = if let Ok((input, _)) = tag::<&str, Span, Error<Span>>(":")(input) {
        let (input, (_, name)) = tuple((skip_space_and_comment0, parse_scoped_name))(input)?;
        (input, Some(name))
    } else {
        (input, None)
    };

    // "{" <member>* "}"
    let (input, (_, members)) = tuple((lparen("{"), members))(input)?;

    Ok((
        input,
        StructDcl::Def(StructDef {
            id,
            members,
            inheritance,
        }),
    ))
}

fn members(mut input: Span) -> PResult<Vec<Member>> {
    let mut result = Vec::new();

    loop {
        if let Ok((i, _)) = rparen("}")(input) {
            return Ok((i, result));
        }

        let (i, member) = parse_member(input)?;
        result.push(member);

        input = i;
    }
}

/// ```text
/// (47) <member> ::= <type_spec> <declarators> ";"
/// ```
pub fn parse_member(input: Span) -> PResult<Member> {
    let (input, _) = skip_space_and_comment0(input)?;

    let (input, annotations) = if tag::<&str, Span, Error<Span>>("@")(input).is_ok() {
        let (input, annotation) = parse_annotation_apps(input)?;
        (input, Some(annotation))
    } else {
        (input, None)
    };

    // <type_spec>
    let (input, type_spec) = parse_type_spec(input)?;

    // <declarators>
    let (input, _) = skip_space_and_comment1(input)?;
    let (input, declarators) = parse_declarators(input)?;

    // ";"
    let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;

    Ok((
        input,
        Member {
            annotations,
            type_spec,
            declarators,
        },
    ))
}

/// ```text
/// (49) <union_dcl> ::= <union_def>
///                    | <union_forward_dcl>
/// ```
fn parse_union_dcl(input: Span) -> PResult<UnionDcl> {
    parse_union(input)
}

/// ```text
/// (50) <union_def> ::= "union" <identifier> "switch" "(" <switch_type_spec> ")" "{" <switch_body> "}"
/// (56) <union_forward_dcl> ::= "union" <identifier>
/// ```
fn parse_union(input: Span) -> PResult<UnionDcl> {
    // <identifier>
    let (input, id) = parse_id(input)?;

    if tuple((skip_space_and_comment0, tag(";")))(input).is_ok() {
        return Ok((input, UnionDcl::ForwardDcl(UnionForwardDcl(id))));
    }

    // "switch"
    let (input, _) = tuple((skip_space_and_comment1, tag("switch")))(input)?;

    // "(" <switch_type_spec> ")"
    let (input, switch_type_spec) =
        delimited(lparen("("), parse_switch_type_spec, rparen(")"))(input)?;

    // "{" <switch_body> "}"
    let (input, body) = delimited(lparen("{"), parse_switch_body, rparen("}"))(input)?;

    Ok((
        input,
        UnionDcl::Def(UnionDef {
            id,
            switch_type_spec,
            body,
        }),
    ))
}

/// ```text
/// ( 51) <switch_type_spec> ::= <integer_type>
///                            | <char_type>
///                            | <boolean_type>
///                            | <scoped_name>
///
/// (196) <switch_type_spec> ::+ <wide_char_type>
///                            | <octet_type>
/// ```
fn parse_switch_type_spec(input: Span) -> PResult<SwitchTypeSpec> {
    if let Ok((input, result)) = parse_int_words(input) {
        return Ok((input, SwitchTypeSpec::PrimitiveType(result)));
    }

    let (input, name) = parse_scoped_name(input)?;

    if let ScopedName::Absolute(s) = &name {
        if s.len() >= 2 {
            return Ok((input, SwitchTypeSpec::ScopedName(name)));
        }

        match s[0].as_str() {
            "short" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Short))),
            "long" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Long))),
            "char" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Char))),
            "boolean" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Boolean))),
            "int8" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Int8))),
            "uint8" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Uint8))),
            "int16" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Int16))),
            "uint16" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Uint16))),
            "int32" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Int32))),
            "uint32" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Uint32))),
            "int64" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Int64))),
            "uint64" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Uint64))),
            "wchar" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::WChar))),
            "octet" => Ok((input, SwitchTypeSpec::PrimitiveType(PrimitiveType::Octet))),
            _ => Ok((input, SwitchTypeSpec::ScopedName(name))),
        }
    } else {
        Ok((input, SwitchTypeSpec::ScopedName(name)))
    }
}

/// ```text
/// (52) <switch_body> ::= <case>+
/// ```
fn parse_switch_body(input: Span) -> PResult<Vec<Case>> {
    many1(parse_case)(input)
}

/// ```text
/// (53) <case> ::= <case_label>+ <element_spec> ";"
/// ```
fn parse_case(input: Span) -> PResult<Case> {
    let (input, labels) = many1(parse_case_label)(input)?;

    let (input, _) = skip_space_and_comment0(input)?;
    let (input, spec) = parse_element_spec(input)?;

    let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;

    Ok((input, Case { labels, spec }))
}

/// ```text
/// (54) <case_label> ::= "case" <const_expr> ":"
///                     | "default" ":"
/// ```
fn parse_case_label(input: Span) -> PResult<CaseLabel> {
    fn case(input: Span) -> PResult<CaseLabel> {
        let (input, _) = tuple((tag("case"), skip_space_and_comment1))(input)?;

        let (input, expr) = parse_const_expr(input, true)?;

        let (input, _) = tuple((skip_space_and_comment0, tag(":")))(input)?;

        Ok((input, CaseLabel::Case(expr)))
    }

    fn default(input: Span) -> PResult<CaseLabel> {
        let (input, _) = tuple((tag("default"), skip_space_and_comment0, tag(":")))(input)?;
        Ok((input, CaseLabel::Default))
    }

    let (input, _) = skip_space_and_comment0(input)?;
    alt((case, default))(input)
}

/// ```text
/// (55) <element_spec> ::= <type_spec> <declarator>
/// ```
fn parse_element_spec(input: Span) -> PResult<ElementSpec> {
    let (input, type_spec) = parse_type_spec(input)?;

    let (input, _) = skip_space_and_comment1(input)?;
    let (input, declarator) = parse_declarator(input)?;

    Ok((
        input,
        ElementSpec {
            type_spec,
            declarator,
        },
    ))
}

/// ```text
/// (57) <enum_dcl> ::= "enum" <identifier> "{" <enumerator> { "," <enumerator> }* "}"
/// ```
pub fn parse_enum_dcl(input: Span) -> PResult<EnumDcl> {
    // <identifier>
    let (input, id) = parse_id(input)?;

    // "{" <enumerator> { "," <enumerator> }* "}"
    let (input, variants) = delimited(
        lparen("{"),
        separated_list1(delimiter(","), parse_enumerator),
        rparen("}"),
    )(input)?;

    Ok((input, EnumDcl { id, variants }))
}

/// ```text
/// (58) <enumerator> ::= <identifier>
/// ```
fn parse_enumerator(input: Span) -> PResult<String> {
    parse_id(input)
}

/// ```text
/// (60) <fixed_array_size> ::= "[" <positive_int_const> "]"
/// ```
fn parse_fixed_array_size(input: Span) -> PResult<ConstExpr> {
    delimited(lparen("["), |i| parse_const_expr(i, true), rparen("]"))(input)
}

/// ```text
/// (61) <native_dcl> ::= "native" <simple_declarator>
/// ```
fn parse_native_dcl(input: Span) -> PResult<NativeDcl> {
    let (input, _) = tuple((tag("native"), skip_space_and_comment1))(input)?;
    let (input, id) = parse_simple_declarator(input)?;

    Ok((input, NativeDcl(id)))
}

/// ```text
/// (62) <simple_declarator> ::= <identifier>
/// ```
pub fn parse_simple_declarator(input: Span) -> PResult<String> {
    parse_id(input)
}

/// ```text
/// (63) <typedef_dcl> ::= "typedef" <type_declarator>
/// ```
pub fn parse_typedef_dcl(input: Span) -> PResult<Typedef> {
    parse_type_declarator(input)
}

/// ```text
/// (64) <type_declarator> ::= { <simple_type_spec> | <template_type_spec> | <constr_type_dcl> } <any_declarators>
/// (65) <any_declalators> ::= <any_declarator> { "," <any_declarator> }*
/// ```
fn parse_type_declarator(input: Span) -> PResult<Typedef> {
    fn simple(input: Span) -> PResult<TypedefType> {
        let (input, t) = parse_simple_type_spec(input)?;
        Ok((input, TypedefType::Simple(t)))
    }

    fn template(input: Span) -> PResult<TypedefType> {
        let (input, t) = parse_template_type_spec(input)?;
        Ok((input, TypedefType::Template(t)))
    }

    fn constr<'a>(head: &str, input: Span<'a>) -> PResult<'a, TypedefType> {
        let (input, t) = parse_constr_type_dcl(head, input)?;
        Ok((input, TypedefType::Constr(t)))
    }

    // <simple_type_spec> | <template_type_spec> | <constr_type_dcl>
    let (input, type_dcl) = if let Ok((input, (head, _))) = tuple((
        alt((
            tag("struct"),
            tag("enum"),
            tag("union"),
            tag("bitset"),
            tag("bitmask"),
            tag("native"),
        )),
        skip_space_and_comment1,
    ))(input)
    {
        constr(head.as_str(), input)?
    } else {
        alt((template, simple))(input)?
    };

    let (input, _) = skip_space_and_comment1(input)?;

    // <any_declalators>
    let (input, declarators) = separated_list1(delimiter(","), parse_any_declarator)(input)?;

    Ok((
        input,
        Typedef {
            type_dcl,
            declarators,
        },
    ))
}

/// ```text
/// (66) <any_declarator> ::= <simple_declarator>
///                         | <array_declarator>
/// (59) <array_declarator> ::= <identifier> <fixed_array_size>+
/// ```
fn parse_any_declarator(input: Span) -> PResult<AnyDeclarator> {
    let (input, id) = parse_id(input)?;

    let (input, array_size) = many0(parse_fixed_array_size)(input)?;

    if array_size.is_empty() {
        Ok((input, AnyDeclarator::Simple(id)))
    } else {
        Ok((
            input,
            AnyDeclarator::Array(ArrayDeclarator { id, array_size }),
        ))
    }
}

/// ```text
/// (67) <declarators> ::= <declarator> { "," <declarator> }*
/// ```
pub fn parse_declarators(input: Span) -> PResult<Vec<AnyDeclarator>> {
    separated_list1(delimiter(","), parse_declarator)(input)
}

/// ```text
/// ( 68) <declarator> ::= <simple_declarator>
/// (217) <declarator> ::+ <array_declarator>
/// ```
fn parse_declarator(input: Span) -> PResult<AnyDeclarator> {
    parse_any_declarator(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::*;
    use num_bigint::BigInt;
    use num_traits::FromPrimitive;

    #[test]
    fn expr() {
        let input = Span::new("1 >> 3");
        parse_const_expr(input, true).unwrap();

        let input = Span::new("1+3");
        parse_const_expr(input, true).unwrap();

        let input = Span::new("1+-3");
        parse_const_expr(input, true).unwrap();

        let input = Span::new("1*3+4");
        parse_const_expr(input, true).unwrap();

        let input = Span::new("1 >> 4");
        parse_const_expr(input, true).unwrap();
    }

    #[test]
    fn literal() {
        let input = Span::new(r#""\(text\)""#);
        assert_eq!(
            parse_string(input).unwrap().1,
            Literal::String(r#"(text)"#.to_string())
        );

        let input = Span::new("0.0");
        assert_eq!(parse_literal(input).unwrap().1, Literal::FloatingPoint(0.0));

        let input = Span::new("1234");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::Integer(BigInt::from_usize(1234).unwrap())
        );

        let input = Span::new("0xabcd");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::Integer(BigInt::from_usize(0xabcd).unwrap())
        );

        let input = Span::new("0XEF45");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::Integer(BigInt::from_usize(0xef45).unwrap())
        );

        let input = Span::new("0127");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::Integer(BigInt::from_usize(0o127).unwrap())
        );

        let input = Span::new("-567");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::Integer(BigInt::from_isize(-567).unwrap())
        );

        let input = Span::new("-567.04e");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::FloatingPoint(-567.04)
        );

        let input = Span::new("-567.04e4");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::FloatingPoint(-567.04e4)
        );

        let input = Span::new("-567.04e-4");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::FloatingPoint(-567.04e-4)
        );

        let input = Span::new("567.04d");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::FixedPoint(FixedPoint {
                value: BigInt::from_isize(56704).unwrap(),
                scale: 2
            })
        );

        let input = Span::new("-567.04d");
        assert_eq!(
            parse_literal(input).unwrap().1,
            Literal::FixedPoint(FixedPoint {
                value: BigInt::from_isize(-56704).unwrap(),
                scale: 2
            })
        );

        let input = Span::new("'\n'");
        assert_eq!(parse_literal(input).unwrap().1, Literal::Char('\n'));
    }
}
