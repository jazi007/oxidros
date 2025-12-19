use super::{
    core::{
        lparen, parse_const_expr, parse_const_type, parse_id, parse_scoped_name,
        parse_simple_declarator, rparen, skip_space_and_comment0, skip_space_and_comment1,
    },
    PResult, Span,
};
use crate::{
    expr::{
        AnnotationAppl, AnnotationApplParam, AnnotationApplParams, AnnotationBody, AnnotationDcl,
        AnnotationHeader, AnnotationMember, ConstType,
    },
    parser::core::{delimiter, parse_const_dcl, parse_enum_dcl, parse_typedef_dcl},
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    multi::{many0, many1, separated_list1},
    sequence::{delimited, tuple},
};

/// ```text
/// (219) <annotation_dcl> ::= <annotation_header> "{" <annotation_body> "}"
/// ```
pub fn parse_annotation_dcl(input: Span) -> PResult<AnnotationDcl> {
    let (input, header) = parse_annotation_header(input)?;
    let (input, body) = delimited(lparen("{"), parse_annotation_body, rparen("}"))(input)?;

    Ok((input, AnnotationDcl { header, body }))
}

/// ```text
/// (220) <annotation_header> ::= "@annotation" <identifier>
/// ```
fn parse_annotation_header(input: Span) -> PResult<AnnotationHeader> {
    let (input, (_, _, id)) =
        tuple((tag("@annotation"), skip_space_and_comment1, parse_id))(input)?;
    Ok((input, AnnotationHeader(id)))
}

/// ```text
/// (221) <annotation_body> ::= { <annotation_member>
///                               | <enum_dcl> ";"
///                               | <const_dcl> ";"
///                               | <typedef_dcl> ";" }*
/// ```
fn parse_annotation_body(input: Span) -> PResult<Vec<AnnotationBody>> {
    fn annotation_member(input: Span) -> PResult<AnnotationBody> {
        let (input, result) = parse_annotation_member(input)?;
        Ok((input, AnnotationBody::AnnotationMember(result)))
    }

    fn enum_dcl(input: Span) -> PResult<AnnotationBody> {
        let (input, _) = tuple((tag("enum"), skip_space_and_comment1))(input)?;
        let (input, result) = parse_enum_dcl(input)?;
        let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;
        Ok((input, AnnotationBody::Enum(result)))
    }

    fn const_dcl(input: Span) -> PResult<AnnotationBody> {
        let (input, _) = tuple((tag("const"), skip_space_and_comment1))(input)?;
        let (input, result) = parse_const_dcl(input)?;
        let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;
        Ok((input, AnnotationBody::Const(result)))
    }

    fn typedef_dcl(input: Span) -> PResult<AnnotationBody> {
        let (input, result) = parse_typedef_dcl(input)?;
        let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;
        Ok((input, AnnotationBody::Typedef(result)))
    }

    fn body(input: Span) -> PResult<AnnotationBody> {
        let (input, _) = skip_space_and_comment0(input)?;
        alt((enum_dcl, const_dcl, typedef_dcl, annotation_member))(input)
    }

    many0(body)(input)
}

/// ```text
/// (222) <annotation_member> ::= <annotation_member_type> <simple_declarator> [ "default" <const_expr> ] ";"
/// ```
fn parse_annotation_member(input: Span) -> PResult<AnnotationMember> {
    let (input, (member_type, _, declarator)) = tuple((
        parse_annotation_member_type,
        skip_space_and_comment1,
        parse_simple_declarator,
    ))(input)?;

    let (input, default_value) =
        if let Ok((input, _)) = tuple((skip_space_and_comment0, tag("default")))(input) {
            let (input, (_, expr)) =
                tuple((skip_space_and_comment1, |i| parse_const_expr(i, true)))(input)?;
            (input, Some(expr))
        } else {
            (input, None)
        };

    let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;

    Ok((
        input,
        AnnotationMember {
            member_type,
            declarator,
            default_value,
        },
    ))
}

/// ```text
/// (223) <annotation_member_type> ::= <const_type> | <any_const_type> | <scoped_name>
/// (224) <any_const_type> ::= "any"
/// ```
fn parse_annotation_member_type(input: Span) -> PResult<ConstType> {
    parse_const_type(input)
}

/// ```text
/// (225) <annotation_appl> ::= "@" <scoped_name> [ "(" <annotation_appl_params> ")" ]
/// ```
///
/// `<annotation_appl>` is never used in the specification; OMG IDL 4.2.
fn parse_annotation_appl(input: Span) -> PResult<AnnotationAppl> {
    let (input, (_, name)) = tuple((tag("@"), parse_scoped_name))(input)?;

    let (input, params) = if let Ok((input, _)) = lparen("(")(input) {
        let (input, params) = parse_annotation_appl_params(input)?;
        let (input, _) = rparen(")")(input)?;
        (input, Some(params))
    } else {
        (input, None)
    };

    Ok((input, AnnotationAppl { name, params }))
}

/// ```text
/// (226) <annotation_appl_params> ::= <const_expr>
///                                  | <annotation_appl_param> { "," <annotation_appl_param> }*
/// ```
fn parse_annotation_appl_params(input: Span) -> PResult<AnnotationApplParams> {
    fn appl_params(input: Span) -> PResult<AnnotationApplParams> {
        let (input, result) = separated_list1(delimiter(","), parse_annotation_appl_param)(input)?;
        Ok((input, AnnotationApplParams::ApplParams(result)))
    }

    fn const_expr(input: Span) -> PResult<AnnotationApplParams> {
        let (input, result) = parse_const_expr(input, true)?;
        Ok((input, AnnotationApplParams::ConstExpr(result)))
    }

    alt((appl_params, const_expr))(input)
}

/// ```text
/// (227) <annotation_appl_param> ::= <identifier> "=" <const_expr>
/// ```
fn parse_annotation_appl_param(input: Span) -> PResult<AnnotationApplParam> {
    let (input, id) = parse_id(input)?;
    let (input, _) = delimiter("=")(input)?;
    let (input, expr) = parse_const_expr(input, true)?;
    Ok((input, AnnotationApplParam { id, expr }))
}

pub fn parse_annotation_apps(input: Span) -> PResult<Vec<AnnotationAppl>> {
    many1(delimited(
        skip_space_and_comment0,
        parse_annotation_appl,
        skip_space_and_comment0,
    ))(input)
}
