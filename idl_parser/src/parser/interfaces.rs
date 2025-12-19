use super::{
    core::{
        delimiter, lparen, parse_id, parse_member, parse_scoped_name, parse_simple_declarator,
        parse_type_spec, rparen, skip_space_and_comment0, skip_space_and_comment1,
    },
    PResult, Span,
};
use crate::{
    expr::{
        AttrDcl, AttrDeclarator, AttrRaises, AttrSpec, ExceptDcl, Export, GetExcep, InterfaceDcl,
        InterfaceDef, InterfaceForwardDcl, InterfaceHeader, OpDcl, OpTypeSpec, ParamAttribute,
        ParamDcl, Raises, ReadonlyAttrDeclarator, ReadonlyAttrSpec, ScopedName, SetExcep, TypeSpec,
    },
    parser::core::{parse_const_dcl, parse_type_dcl},
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    multi::{many0, many1, separated_list1},
    sequence::{delimited, tuple},
};
use nom_greedyerror::AsStr;
use std::mem::take;

/// ```text
/// (72) <except_dcl> ::= "exception" <identifier> "{" <member>*  "}"
/// ```
pub fn parse_except_dcl(input: Span) -> PResult<ExceptDcl> {
    let (input, id) = parse_id(input)?;

    let (input, members) = delimited(lparen("{"), many1(parse_member), rparen("}"))(input)?;

    Ok((input, ExceptDcl { id, members }))
}

/// ```text
/// (73) <interface_dcl> ::= <interface_def>
///                        | <interface_forward_dcl>
/// ```
pub fn parse_interface_dcl(input: Span) -> PResult<InterfaceDcl> {
    parse_interface(input)
}

/// ```text
/// (74) <interface_def> ::= <interface_header> "{" <interface_body> "}"
/// (75) <interface_forward_dcl> ::= <interface_kind> <identifier>
/// (76) <interface_header> ::= <interface_kind> <identifier> [ <interface_inheritance_spec> ]
/// (77) <interface_kind> ::= "interface"
/// (78) <interface_inheritance_spec> ::= ":" <interface_name> { "," <interface_name> }*
/// ```
fn parse_interface(input: Span) -> PResult<InterfaceDcl> {
    let (input, id) = parse_id(input)?;

    if tuple((skip_space_and_comment0, tag(";")))(input).is_ok() {
        return Ok((input, InterfaceDcl::ForwardDcl(InterfaceForwardDcl(id))));
    }

    let (input, inheritance) = if let Ok((input, _)) =
        tuple((skip_space_and_comment0, tag(":")))(input)
    {
        let (input, inheritance) = separated_list1(delimiter(","), parse_interface_name)(input)?;
        (input, Some(inheritance))
    } else {
        (input, None)
    };

    let header = InterfaceHeader { id, inheritance };

    let (input, body) = delimited(lparen("{"), parse_interface_body, rparen("}"))(input)?;

    Ok((input, InterfaceDcl::Def(InterfaceDef { header, body })))
}

/// ```text
/// (79) <interface_name> ::= <scoped_name>
/// ```
pub fn parse_interface_name(input: Span) -> PResult<ScopedName> {
    parse_scoped_name(input)
}

/// ```text
/// (80) <interface_body> ::= <export>*
/// ```
fn parse_interface_body(input: Span) -> PResult<Vec<Export>> {
    many0(parse_export)(input)
}

/// ```text
/// (81) <export> ::= <op_dcl> ";"
///                 | <attr_dcl> ";"
///
/// (90) <export> ::+ <type_dcl> ";"
///                 | <const_dcl> ";"
///                 | <except_dcl> ";"
/// ```
pub fn parse_export(input: Span) -> PResult<Export> {
    fn op(input: Span) -> PResult<Export> {
        let (input, op) = parse_op_dcl(input)?;
        Ok((input, Export::Op(op)))
    }

    fn attr<'a>(head: &str, input: Span<'a>) -> PResult<'a, Export> {
        let (input, attr) = parse_attr_dcl(head, input)?;
        Ok((input, Export::Attr(attr)))
    }

    fn type_dcl<'a>(head: &str, input: Span<'a>) -> PResult<'a, Export> {
        let (input, dcl) = parse_type_dcl(head, input)?;
        Ok((input, Export::Type(dcl)))
    }

    fn const_dcl(input: Span) -> PResult<Export> {
        let (input, dcl) = parse_const_dcl(input)?;
        Ok((input, Export::Const(dcl)))
    }

    fn except(input: Span) -> PResult<Export> {
        let (input, dcl) = parse_except_dcl(input)?;
        Ok((input, Export::Except(dcl)))
    }

    if let Ok((input, (head, _))) = tuple((
        alt((
            tag("struct"),
            tag("enum"),
            tag("union"),
            tag("bitset"),
            tag("bitmask"),
            tag("native"),
            tag("typedef"),
            tag("const"),
            tag("exception"),
            tag("readonly"),
            tag("attribute"),
        )),
        skip_space_and_comment1,
    ))(input)
    {
        let (input, result) = match head.as_str() {
            "const" => const_dcl(input)?,
            "except" => except(input)?,
            "readonly" | "attribute" => attr(head.as_str(), input)?,
            _ => type_dcl(head.as_str(), input)?,
        };
        let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;
        Ok((input, result))
    } else {
        let (input, _) = skip_space_and_comment0(input)?;
        let (input, result) = op(input)?;
        let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;
        Ok((input, result))
    }
}

/// ```text
/// (82) <op_dcl> ::= <op_type_spec> <identifier> "(" [ <parameter_dcls> ] ")" [ <raises_expr> ]
/// ```
fn parse_op_dcl(input: Span) -> PResult<OpDcl> {
    let (input, type_spec) = parse_op_type_spec(input)?;

    let (input, _) = skip_space_and_comment1(input)?;
    let (input, id) = parse_id(input)?;

    let (input, params) = delimited(lparen("("), parse_parameter_dcls, rparen(")"))(input)?;

    let (input, raises) = if tuple((skip_space_and_comment0, tag("raises")))(input).is_ok() {
        let (input, raises) = parse_raises_expr(input)?;
        (input, Some(raises))
    } else {
        (input, None)
    };
    Ok((
        input,
        OpDcl {
            id,
            type_spec,
            params,
            raises,
        },
    ))
}

/// ```text
/// (83) <op_type_spec> ::= <type_spec>
///                       | "void"
/// ```
fn parse_op_type_spec(input: Span) -> PResult<OpTypeSpec> {
    let (input, type_spec) = parse_type_spec(input)?;

    if let TypeSpec::ScopedName(ScopedName::Absolute(name)) = &type_spec {
        if name.len() == 1 && name[0] == "void" {
            return Ok((input, OpTypeSpec::Void));
        }
    }

    Ok((input, OpTypeSpec::TypeSpec(type_spec)))
}

/// ```text
/// (84) <parameter_dcls> ::= <param_dcl> { "," <param_dcl> }*
/// ```
fn parse_parameter_dcls(input: Span) -> PResult<Vec<ParamDcl>> {
    separated_list1(delimiter(","), parse_param_dcl)(input)
}

/// ```text
/// (85) <param_dcl> ::= <param_attribute> <type_spec> <simple_declarator>
/// ```
fn parse_param_dcl(input: Span) -> PResult<ParamDcl> {
    let (input, attr) = parse_param_attribute(input)?;

    let (input, _) = skip_space_and_comment1(input)?;
    let (input, type_spec) = parse_type_spec(input)?;

    let (input, _) = skip_space_and_comment1(input)?;
    let (input, id) = parse_simple_declarator(input)?;

    Ok((
        input,
        ParamDcl {
            attr,
            type_spec,
            id,
        },
    ))
}

/// ```text
/// (86) <param_attribute> ::= "in"
///                          | "out"
///                          | "inout"
/// ```
fn parse_param_attribute(input: Span) -> PResult<ParamAttribute> {
    let (input, c) = alt((tag("in"), tag("out"), tag("inout")))(input)?;
    match c.as_str() {
        "in" => Ok((input, ParamAttribute::In)),
        "out" => Ok((input, ParamAttribute::Out)),
        "inout" => Ok((input, ParamAttribute::InOut)),
        _ => unreachable!(),
    }
}

/// ```text
/// (87) <raises_expr> ::= "raises" "(" <scoped_name> { "," <scoped_name> }* ")"
/// ```
pub fn parse_raises_expr(input: Span) -> PResult<Raises> {
    let (input, _) = tuple((tag("raises"), lparen("(")))(input)?;

    let (input, names) = separated_list1(delimiter(","), parse_scoped_name)(input)?;

    let (input, _) = rparen(")")(input)?;

    Ok((input, Raises(names)))
}

/// ```text
/// (88) <attr_dcl> ::= <readonly_attr_spec>
///                   | <attr_spec>
/// ```
pub fn parse_attr_dcl<'a>(head: &str, input: Span<'a>) -> PResult<'a, AttrDcl> {
    fn readonly(input: Span) -> PResult<AttrDcl> {
        let (input, spec) = parse_readonly_attr_spec(input)?;
        Ok((input, AttrDcl::Readonly(spec)))
    }

    fn read_write(input: Span) -> PResult<AttrDcl> {
        let (input, spec) = parse_attr_spec(input)?;
        Ok((input, AttrDcl::ReadWrite(spec)))
    }

    match head {
        "readonly" => readonly(input),
        "attribute" => read_write(input),
        _ => unreachable!(),
    }
}

/// ```text
/// (89) <readonly_attr_spec> ::= "readonly" "attribute" <type_spec> <readonly_attr_declarator>
/// ```
fn parse_readonly_attr_spec(input: Span) -> PResult<ReadonlyAttrSpec> {
    let (input, _) = tuple((tag("attribute"), skip_space_and_comment1))(input)?;

    let (input, type_spec) = parse_type_spec(input)?;

    let (input, _) = skip_space_and_comment1(input)?;
    let (input, declarator) = parse_readonly_attr_declarator(input)?;

    Ok((
        input,
        ReadonlyAttrSpec {
            type_spec,
            declarator,
        },
    ))
}

/// ```text
/// (90) <readonly_attr_declarator> ::= <simple_declarator> <raises_expr>
///                                   | <simple_declarator> { "," <simple_declarator> }*
/// ```
fn parse_readonly_attr_declarator(input: Span) -> PResult<ReadonlyAttrDeclarator> {
    let (input, mut ids) = separated_list1(
        tuple((skip_space_and_comment0, tag(","), skip_space_and_comment0)),
        parse_simple_declarator,
    )(input)?;

    if ids.len() == 1 && tuple((skip_space_and_comment1, tag("raises")))(input).is_ok() {
        let (input, _) = skip_space_and_comment1(input)?;
        let (input, raises) = parse_raises_expr(input)?;
        let s = take(&mut ids[0]);
        return Ok((input, ReadonlyAttrDeclarator::WithRaises(s, raises)));
    }

    Ok((input, ReadonlyAttrDeclarator::IDs(ids)))
}

/// ```text
/// (91) <attr_spec> ::= "attribute" <type_spec> <attr_declarator>
/// ```
fn parse_attr_spec(input: Span) -> PResult<AttrSpec> {
    let (input, type_spec) = parse_type_spec(input)?;

    let (input, _) = skip_space_and_comment1(input)?;
    let (input, declarator) = parse_attr_declarator(input)?;

    Ok((
        input,
        AttrSpec {
            type_spec,
            declarator,
        },
    ))
}

/// ```text
/// (92) <attr_declarator> ::= <simple_declarator> <attr_raises_expr>
///                          | <simple_declarator> { "," <simple_declarator> }*
/// ```
fn parse_attr_declarator(input: Span) -> PResult<AttrDeclarator> {
    let (input, mut ids) = separated_list1(delimiter(","), parse_simple_declarator)(input)?;

    if ids.len() == 1
        && tuple((
            skip_space_and_comment1,
            alt((tag("getraises"), tag("setraises"))),
        ))(input)
        .is_ok()
    {
        let (input, _) = skip_space_and_comment1(input)?;
        let (input, raises) = parse_attr_raises_expr(input)?;
        let s = take(&mut ids[0]);
        return Ok((input, AttrDeclarator::WithRaises(s, raises)));
    }

    Ok((input, AttrDeclarator::IDs(ids)))
}

/// ```text
/// (93) <attr_raises_expr> ::= <get_excep_expr> [ <set_excep_expr> ]
///                           | <set_excep_expr>
/// ```
fn parse_attr_raises_expr(input: Span) -> PResult<AttrRaises> {
    fn get_excep(input: Span) -> PResult<AttrRaises> {
        let (input, get_excep) = parse_get_excep_expr(input)?;

        if tuple((skip_space_and_comment0, tag("setraises")))(input).is_ok() {
            let (input, _) = skip_space_and_comment0(input)?;
            let (input, set_excep) = parse_set_excep_expr(input)?;
            Ok((input, AttrRaises::GetExcep(get_excep, Some(set_excep))))
        } else {
            Ok((input, AttrRaises::GetExcep(get_excep, None)))
        }
    }

    fn set_excep(input: Span) -> PResult<AttrRaises> {
        let (input, set_excep) = parse_set_excep_expr(input)?;
        Ok((input, AttrRaises::SetExcep(set_excep)))
    }

    alt((get_excep, set_excep))(input)
}

/// ```text
/// (94) <get_excep_expr> ::= "getraises" <exception_list>
/// ```
fn parse_get_excep_expr(input: Span) -> PResult<GetExcep> {
    let (input, _) = tuple((tag("getraises"), skip_space_and_comment0))(input)?;
    let (input, names) = parse_exception_list(input)?;
    Ok((input, GetExcep(names)))
}

/// ```text
/// (95) <set_excep_expr> ::= "setraises" <exception_list>
/// ```
fn parse_set_excep_expr(input: Span) -> PResult<SetExcep> {
    let (input, _) = tuple((tag("setraises"), skip_space_and_comment0))(input)?;
    let (input, names) = parse_exception_list(input)?;
    Ok((input, SetExcep(names)))
}

/// ```text
/// (96) <exception_list> ::= "(" <scoped_name> { "," <scoped_name> }* ")"
/// ```
fn parse_exception_list(input: Span) -> PResult<Vec<ScopedName>> {
    delimited(
        lparen("("),
        separated_list1(delimiter(","), parse_scoped_name),
        rparen(")"),
    )(input)
}
