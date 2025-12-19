use super::{
    core::{
        lparen, parse_declarators, parse_id, parse_scoped_name, parse_simple_declarator,
        parse_type_spec, rparen, skip_space_and_comment0, skip_space_and_comment1,
    },
    interfaces::parse_raises_expr,
    PResult, Span,
};
use crate::{
    expr::{
        InitDcl, InitParamDcl, ScopedName, StateMember, ValueDcl, ValueDef, ValueElement,
        ValueForwardDcl, ValueHeader, ValueInheritanceSpec, Visibility,
    },
    parser::{
        core::delimiter,
        interfaces::{parse_export, parse_interface_name},
    },
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    multi::{many0, separated_list0},
    sequence::{delimited, tuple},
};
use nom_greedyerror::AsStr;

/// ```text
/// (99) <value_dcl> ::= <value_def>
///                    | <value_forward_dcl>
/// ```
pub fn parse_value_dcl(input: Span) -> PResult<ValueDcl> {
    parse_value(input)
}

/// ```text
/// (100) <value_def> ::= <value_header> "{" <value_element>* "}"
/// (101) <value_header> ::= <value_kind> <identifier> [ <value_inheritance_spec> ]
/// (102) <value_kind> ::= "valuetype"
/// (110) <value_forward_dcl> ::= <value_kind> <identifier>
/// ```
fn parse_value(input: Span) -> PResult<ValueDcl> {
    let (input, id) = parse_id(input)?;

    if tuple((skip_space_and_comment0, tag(";")))(input).is_ok() {
        return Ok((input, ValueDcl::ForwardDcl(ValueForwardDcl(id))));
    }

    let (input, inheritance) = parse_value_inheritance_spec(input)?;

    let (input, elements) = delimited(lparen("{"), many0(parse_value_element), rparen("}"))(input)?;
    Ok((
        input,
        ValueDcl::Def(ValueDef {
            header: ValueHeader { id, inheritance },
            elements,
        }),
    ))
}

/// ```text
/// (103) <value_inheritance_spec> ::= [ ":" <value_name> ] [ "supports" <interface_name> ]
/// ```
fn parse_value_inheritance_spec(input: Span) -> PResult<ValueInheritanceSpec> {
    let (input, value) = if let Ok((input, _)) = delimiter(":")(input) {
        let (input, value) = parse_value_name(input)?;
        (input, Some(value))
    } else {
        (input, None)
    };

    let (input, interface) = if let Ok((input, _)) = tuple((
        skip_space_and_comment1,
        tag("supports"),
        skip_space_and_comment0,
    ))(input)
    {
        let (input, interface) = parse_interface_name(input)?;
        (input, Some(interface))
    } else {
        (input, None)
    };

    Ok((input, ValueInheritanceSpec { value, interface }))
}

/// ```text
/// (104) <value_name> ::= <scoped_name>
/// ```
fn parse_value_name(input: Span) -> PResult<ScopedName> {
    parse_scoped_name(input)
}

/// ```text
/// (105) <value_element> ::= <export>
///                         | <state_member>
///                         | <init_dcl>
/// ```
fn parse_value_element(input: Span) -> PResult<ValueElement> {
    fn export(input: Span) -> PResult<ValueElement> {
        let (input, result) = parse_export(input)?;
        Ok((input, ValueElement::Export(result)))
    }

    fn state_member(input: Span) -> PResult<ValueElement> {
        let (input, result) = parse_state_member(input)?;
        Ok((input, ValueElement::StateMember(result)))
    }

    fn init_dcl(input: Span) -> PResult<ValueElement> {
        let (input, result) = parse_init_dcl(input)?;
        Ok((input, ValueElement::InitDcl(result)))
    }

    let (input, _) = skip_space_and_comment0(input)?;
    alt((export, state_member, init_dcl))(input)
}

/// ```text
/// (106) <state_member> ::= ( "public" | "private" ) <type_spec> <declarators> ";"
/// ```
fn parse_state_member(input: Span) -> PResult<StateMember> {
    let (input, visibility) = alt((tag("public"), tag("private")))(input)?;
    let (input, _) = skip_space_and_comment1(input)?;

    let (input, type_spec) = parse_type_spec(input)?;

    let (input, _) = skip_space_and_comment1(input)?;
    let (input, declarators) = parse_declarators(input)?;

    let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;

    let visibility = match visibility.as_str() {
        "public" => Visibility::Public,
        "private" => Visibility::Private,
        _ => unreachable!(),
    };

    Ok((
        input,
        StateMember {
            visibility,
            type_spec,
            declarators,
        },
    ))
}

/// ```text
/// (107) <init_dcl> ::= "factory" <identifier> "(" [ init_param_dcls ] ")" [ <raises_expr> ] ";"
/// (108) <init_param_dcls> ::= <init_param_dcl> { "," <init_param_dcl> }*
/// ```
fn parse_init_dcl(input: Span) -> PResult<InitDcl> {
    let (input, _) = tuple((tag("factory"), skip_space_and_comment1))(input)?;
    let (input, id) = parse_id(input)?;

    // [ init_param_dcls ]
    let (input, params) = delimited(
        lparen("("),
        separated_list0(delimiter(","), parse_init_param_dcl),
        rparen(")"),
    )(input)?;

    let (input, raises) = if tuple((skip_space_and_comment0, tag("raises")))(input).is_ok() {
        let (input, (_, raises)) = tuple((skip_space_and_comment0, parse_raises_expr))(input)?;
        (input, Some(raises))
    } else {
        (input, None)
    };

    Ok((input, InitDcl { id, params, raises }))
}

/// ```text
/// (109) <init_param_dcl> ::= "in" <type_spec> <simple_declarator>
/// ```
fn parse_init_param_dcl(input: Span) -> PResult<InitParamDcl> {
    let (input, _) = tag("in")(input)?;

    let (input, type_spec) = delimited(
        skip_space_and_comment1,
        parse_type_spec,
        skip_space_and_comment1,
    )(input)?;

    let (input, id) = parse_simple_declarator(input)?;

    Ok((input, InitParamDcl { type_spec, id }))
}
