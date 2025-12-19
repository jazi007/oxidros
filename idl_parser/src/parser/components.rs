use super::{
    core::{
        delimiter, lparen, parse_id, parse_scoped_name, parse_simple_declarator, parse_type_spec,
        rparen, skip_space_and_comment0, skip_space_and_comment1,
    },
    interfaces::{parse_export, parse_raises_expr},
    PResult, Span,
};
use crate::{
    expr::{
        ComponentDcl, ComponentDef, ComponentExport, ComponentForwardDcl, ComponentHeader,
        ComponentInheritanceSpec, ConnectorDcl, ConnectorExport, ConnectorHeader,
        ConnectorInheritSpec, FactoryDcl, FactoryParamDcl, HomeDcl, HomeExport, HomeHeader,
        HomeInheritanceSpec, InterfaceType, PortBody, PortDcl, PortExport, PortRef, PortTypeDcl,
        PortTypeDef, PortTypeForwardDcl, ProvidesDcl, UsesDcl,
    },
    parser::interfaces::parse_attr_dcl,
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    multi::{many0, many1, separated_list0},
    sequence::{delimited, tuple},
};
use nom_greedyerror::AsStr;

/// ```text
/// (134) <component_dcl> ::= <component_def>
///                         | <component_forward_dcl>
/// ```
pub fn parse_component_dcl(input: Span) -> PResult<ComponentDcl> {
    fn def(input: Span) -> PResult<ComponentDcl> {
        let (input, result) = parse_component_def(input)?;
        Ok((input, ComponentDcl::Def(result)))
    }

    fn forward_dcl(input: Span) -> PResult<ComponentDcl> {
        let (input, result) = parse_component_forward_dcl(input)?;
        Ok((input, ComponentDcl::ForwardDcl(result)))
    }

    alt((def, forward_dcl))(input)
}

/// ```text
/// (135) <component_forward_dcl> ::= "component" <identifier>
/// ```
fn parse_component_forward_dcl(input: Span) -> PResult<ComponentForwardDcl> {
    let (input, _) = tuple((tag("component"), skip_space_and_comment1))(input)?;
    let (input, id) = parse_id(input)?;
    Ok((input, ComponentForwardDcl(id)))
}

/// ```text
/// (136) <component_def> ::= <component_header> "{" <component_body> "}"
/// ```
fn parse_component_def(input: Span) -> PResult<ComponentDef> {
    let (input, header) = parse_component_header(input)?;

    let (input, body) = delimited(lparen("{"), parse_componet_body, rparen("}"))(input)?;

    Ok((input, ComponentDef { header, body }))
}

/// ```text
/// (137) <component_header> ::= "component" <identifier> [ <component_inheritance_spec> ]
/// ```
fn parse_component_header(input: Span) -> PResult<ComponentHeader> {
    let (input, id) = parse_id(input)?;

    let (input, inheritance) = if tuple((skip_space_and_comment0, tag(":")))(input).is_ok() {
        let (input, (_, result)) =
            tuple((skip_space_and_comment0, parse_component_inheritance_spec))(input)?;
        (input, Some(result))
    } else {
        (input, None)
    };

    Ok((input, ComponentHeader { id, inheritance }))
}

/// ```text
/// (138) <component_inheritance_spec> ::= ":" <scoped_name>
/// ```
fn parse_component_inheritance_spec(input: Span) -> PResult<ComponentInheritanceSpec> {
    let (input, _) = tuple((tag(":"), skip_space_and_comment0))(input)?;
    let (input, name) = parse_scoped_name(input)?;
    Ok((input, ComponentInheritanceSpec(name)))
}

/// ```text
/// (139) <component_body> ::= <component_export>*
/// ```
fn parse_componet_body(input: Span) -> PResult<Vec<ComponentExport>> {
    many0(parse_compoent_export)(input)
}

/// ```text
/// (140) <component_export> ::= <provides_dcl> ";"
///                            | <uses_dcl> ";"
///                            | <attr_dcl> ";"
///
/// (179) <component_export> ::+ <port_dcl> ";"
/// ```
fn parse_compoent_export(input: Span) -> PResult<ComponentExport> {
    fn provides(input: Span) -> PResult<ComponentExport> {
        let (input, result) = parse_provides_dcl(input)?;
        Ok((input, ComponentExport::Provides(result)))
    }

    fn uses(input: Span) -> PResult<ComponentExport> {
        let (input, result) = parse_uses_dcl(input)?;
        Ok((input, ComponentExport::Uses(result)))
    }

    fn attr<'a>(head: &str, input: Span<'a>) -> PResult<'a, ComponentExport> {
        let (input, result) = parse_attr_dcl(head, input)?;
        Ok((input, ComponentExport::Attr(result)))
    }

    fn port<'a>(head: &str, input: Span<'a>) -> PResult<'a, ComponentExport> {
        let (input, result) = parse_port_dcl(head, input)?;
        Ok((input, ComponentExport::Port(result)))
    }

    let (input, _) = skip_space_and_comment0(input)?;

    let (input, (head, _)) = tuple((
        alt((
            tag("provides"),
            tag("uses"),
            tag("readonly"),
            tag("attribute"),
            tag("port"),
            tag("mirrorport"),
        )),
        skip_space_and_comment1,
    ))(input)?;

    let (input, result) = match head.as_str() {
        "provides" => provides(input)?,
        "uses" => uses(input)?,
        "readonly" | "atrribute" => attr(head.as_str(), input)?,
        "port" | "mirrorport" => port(head.as_str(), input)?,
        _ => unreachable!(),
    };

    let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;

    Ok((input, result))
}

/// ```text
/// (141) <provides_dcl> ::= "provides" <interface_type> <identifier>
/// ```
fn parse_provides_dcl(input: Span) -> PResult<ProvidesDcl> {
    let (input, interface_type) = parse_interface_type(input)?;
    let (input, (_, id)) = tuple((skip_space_and_comment1, parse_id))(input)?;

    Ok((input, ProvidesDcl { interface_type, id }))
}

/// ```text
/// (142) <interface_type> ::= <scoped_name>
/// ```
fn parse_interface_type(input: Span) -> PResult<InterfaceType> {
    let (input, name) = parse_scoped_name(input)?;
    Ok((input, InterfaceType(name)))
}

/// ```text
/// (143) <uses_dcl> ::= "uses" <interface_type> <identifier>
/// ```
fn parse_uses_dcl(input: Span) -> PResult<UsesDcl> {
    let (input, (interface_type, _, id)) =
        tuple((parse_interface_type, skip_space_and_comment1, parse_id))(input)?;

    Ok((input, UsesDcl { interface_type, id }))
}

/// ```text
/// (145) <home_dcl> ::= <home_header> "{" <home_body> "}"
/// ```
pub fn parse_home_dcl(input: Span) -> PResult<HomeDcl> {
    let (input, header) = parse_home_header(input)?;

    let (input, body) = delimited(lparen("{"), parse_home_body, rparen("}"))(input)?;

    Ok((input, HomeDcl { header, body }))
}

/// ```text
/// (146) <home_header> ::= "home" <identifier> [ <home_inheritance_spec> ] "manages" <scoped_name>
/// ```
fn parse_home_header(input: Span) -> PResult<HomeHeader> {
    let (input, id) = parse_id(input)?;

    let (input, inheritance) = if tuple((skip_space_and_comment0, tag(":")))(input).is_ok() {
        let (input, (_, result)) =
            tuple((skip_space_and_comment0, parse_home_inheritance_spec))(input)?;
        (input, Some(result))
    } else {
        (input, None)
    };

    let (input, _) = tuple((
        skip_space_and_comment1,
        tag("manages"),
        skip_space_and_comment1,
    ))(input)?;

    let (input, manages) = parse_scoped_name(input)?;

    Ok((
        input,
        HomeHeader {
            id,
            inheritance,
            manages,
        },
    ))
}

/// ```text
/// (147) <home_inheritance_spec> ::= ":" <scoped_name>
/// ```
fn parse_home_inheritance_spec(input: Span) -> PResult<HomeInheritanceSpec> {
    let (input, _) = tuple((tag(":"), skip_space_and_comment0))(input)?;
    let (input, name) = parse_scoped_name(input)?;
    Ok((input, HomeInheritanceSpec(name)))
}

/// ```text
/// (148) <home_body> ::= <home_export>*
/// ```
fn parse_home_body(input: Span) -> PResult<Vec<HomeExport>> {
    many0(parse_home_export)(input)
}

/// ```text
/// (149) <home_export> ::= <export>
///                       | <factory_dcl> ";"
/// ```
fn parse_home_export(input: Span) -> PResult<HomeExport> {
    fn export(input: Span) -> PResult<HomeExport> {
        let (input, result) = parse_export(input)?;
        Ok((input, HomeExport::Export(result)))
    }

    fn factory_dcl(input: Span) -> PResult<HomeExport> {
        let (input, result) = parse_factory_dcl(input)?;
        let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;
        Ok((input, HomeExport::FactoryDcl(result)))
    }

    let (input, _) = skip_space_and_comment0(input)?;
    alt((export, factory_dcl))(input)
}

/// ```text
/// (150) <factory_dcl> ::= "factory" <identifier> "(" [ <factory_param_dcls> ] ")" [ <raises_expr> ]
/// ```
fn parse_factory_dcl(input: Span) -> PResult<FactoryDcl> {
    let (input, _) = tuple((tag("factory"), skip_space_and_comment1))(input)?;

    let (input, id) = parse_simple_declarator(input)?;

    let (input, params) = delimited(
        lparen("("),
        separated_list0(delimiter(","), parse_factory_param_dcl),
        rparen(")"),
    )(input)?;

    let (input, raises) = if tuple((skip_space_and_comment0, tag("raises")))(input).is_ok() {
        let (input, (_, raises)) = tuple((skip_space_and_comment0, parse_raises_expr))(input)?;
        (input, Some(raises))
    } else {
        (input, None)
    };

    Ok((input, FactoryDcl { id, params, raises }))
}

// ```text
// (151) <factory_param_dcls> ::= <factory_param_dcl> { ","  <factory_param_dcl> }*
// ```

/// ```text
/// (152) <factory_param_dcl> ::= "in" <type_spec> <simple_declarator>
/// ```
fn parse_factory_param_dcl(input: Span) -> PResult<FactoryParamDcl> {
    let (input, _) = tuple((tag("in"), skip_space_and_comment1))(input)?;
    let (input, type_spec) = parse_type_spec(input)?;
    let (input, _) = skip_space_and_comment1(input)?;
    let (input, id) = parse_simple_declarator(input)?;
    Ok((input, FactoryParamDcl { type_spec, id }))
}

/// ```text
/// (172) <porttype_dcl> ::= <porttype_def>
///                        | <porttype_forward_dcl>
/// (173) <porttype_forward_dcl> ::= "porttype" <identifier>
/// (174) <porttype_def> ::= "porttype" <identifier> "{" <port_body> "}"
/// ```
pub fn parse_porttype_dcl(input: Span) -> PResult<PortTypeDcl> {
    parse_porttype(input)
}

fn parse_porttype(input: Span) -> PResult<PortTypeDcl> {
    let (input, id) = parse_id(input)?;

    if tuple((skip_space_and_comment0, tag(";")))(input).is_ok() {
        return Ok((input, PortTypeDcl::ForwardDcl(PortTypeForwardDcl(id))));
    }

    let (input, body) = delimited(lparen("{"), parse_port_body, rparen("}"))(input)?;
    Ok((input, PortTypeDcl::Def(PortTypeDef { id, body })))
}

/// ```text
/// (175) <port_body> ::= <port_ref> <port_export>*
/// ```
fn parse_port_body(input: Span) -> PResult<PortBody> {
    let (input, port_ref) = parse_port_ref(input)?;
    let (input, port_export) = many0(parse_port_export)(input)?;
    Ok((
        input,
        PortBody {
            port_ref,
            port_export,
        },
    ))
}

/// ```text
/// (176) <port_ref> ::= <provides_dcl> ";"
///                    | <uses_dcl> ";"
///                    | <port_dcl> ";"
/// ```
fn parse_port_ref(input: Span) -> PResult<PortRef> {
    fn provides(input: Span) -> PResult<PortRef> {
        let (input, result) = parse_provides_dcl(input)?;
        Ok((input, PortRef::Provides(result)))
    }

    fn uses(input: Span) -> PResult<PortRef> {
        let (input, result) = parse_uses_dcl(input)?;
        Ok((input, PortRef::Uses(result)))
    }

    fn port<'a>(head: &str, input: Span<'a>) -> PResult<'a, PortRef> {
        let (input, result) = parse_port_dcl(head, input)?;
        Ok((input, PortRef::Port(result)))
    }

    let (input, (head, _)) = tuple((
        alt((tag("provides"), tag("uses"), tag("port"), tag("mirrorport"))),
        skip_space_and_comment1,
    ))(input)?;

    let (input, result) = match head.as_str() {
        "provides" => provides(input)?,
        "uses" => uses(input)?,
        "port" | "mirrorport" => port(head.as_str(), input)?,
        _ => unreachable!(),
    };

    let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;

    Ok((input, result))
}

/// ```text
/// (177) <port_export> ::= <port_ref>
///                       | <attr_dcl> ";"
/// ```
fn parse_port_export(input: Span) -> PResult<PortExport> {
    fn port_ref(input: Span) -> PResult<PortExport> {
        let (input, result) = parse_port_ref(input)?;
        Ok((input, PortExport::PortRef(result)))
    }

    fn attr<'a>(head: &str, input: Span<'a>) -> PResult<'a, PortExport> {
        let (input, result) = parse_attr_dcl(head, input)?;
        let (input, _) = tuple((skip_space_and_comment0, tag(";")))(input)?;
        Ok((input, PortExport::Attr(result)))
    }

    let (input, _) = skip_space_and_comment0(input)?;
    if let Ok((input, (head, _))) = tuple((
        alt((tag("readonly"), tag("attribute"))),
        skip_space_and_comment1,
    ))(input)
    {
        attr(head.as_str(), input)
    } else {
        port_ref(input)
    }
}

/// ```text
/// (178) <port_dcl> ::= { "port" | "mirrorport" } <scoped_name> <identifier>
/// ```
fn parse_port_dcl<'a>(head: &str, input: Span<'a>) -> PResult<'a, PortDcl> {
    let (input, (name, _, id)) =
        tuple((parse_scoped_name, skip_space_and_comment1, parse_id))(input)?;

    match head {
        "port" => Ok((input, PortDcl::Port(name, id))),
        "mirrorport" => Ok((input, PortDcl::MirrorPort(name, id))),
        _ => unreachable!(),
    }
}

/// ```text
/// (180) <connector_dcl> ::= <connector_header> "{" <connector_export>+ "}"
/// ```
pub fn parse_connector_dcl(input: Span) -> PResult<ConnectorDcl> {
    let (input, header) = parse_connector_header(input)?;
    let (input, export) =
        delimited(lparen("{"), many1(parse_connector_export), rparen("}"))(input)?;

    Ok((input, ConnectorDcl { header, export }))
}

/// ```text
/// (181) <connector_header> ::= "connector" <identifier> [ <connector_inherit_spec> ]
/// ```
fn parse_connector_header(input: Span) -> PResult<ConnectorHeader> {
    let (input, id) = parse_id(input)?;

    let (input, inheritance) = if tuple((skip_space_and_comment0, tag(":")))(input).is_ok() {
        let (input, (_, result)) =
            tuple((skip_space_and_comment0, parse_connector_inherit_spec))(input)?;
        (input, Some(result))
    } else {
        (input, None)
    };

    Ok((input, ConnectorHeader { id, inheritance }))
}

/// ```text
/// (182) <connector_inherit_spec> ::= ":" <scoped_name>
/// ```
fn parse_connector_inherit_spec(input: Span) -> PResult<ConnectorInheritSpec> {
    let (input, _) = delimiter(":")(input)?;
    let (input, name) = parse_scoped_name(input)?;
    Ok((input, ConnectorInheritSpec(name)))
}

/// ```text
/// (183) <connector_export> ::= <port_ref>
///                            | <attr_dcl> ";"
/// ```
fn parse_connector_export(input: Span) -> PResult<ConnectorExport> {
    fn port_ref(input: Span) -> PResult<ConnectorExport> {
        let (input, result) = parse_port_ref(input)?;
        Ok((input, ConnectorExport::PortRef(result)))
    }

    fn attr<'a>(head: &str, input: Span<'a>) -> PResult<'a, ConnectorExport> {
        let (input, result) = parse_attr_dcl(head, input)?;
        let (input, _) = tuple((skip_space_and_comment0, tag(":")))(input)?;
        Ok((input, ConnectorExport::Attr(result)))
    }

    let (input, _) = skip_space_and_comment0(input)?;
    if let Ok((input, (head, _))) = tuple((
        alt((tag("readonly"), tag("attribute"))),
        skip_space_and_comment1,
    ))(input)
    {
        attr(head.as_str(), input)
    } else {
        port_ref(input)
    }
}
