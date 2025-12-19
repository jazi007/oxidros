use num_bigint::BigInt;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Module {
    pub id: String,
    pub definitions: Vec<AnnotationAndDef>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Const {
    pub id: String,
    pub expr: ConstExpr,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct AnnotationAndDef {
    pub annotations: Option<Vec<AnnotationAppl>>,
    pub definition: Definition,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Definition {
    Module(Module),
    Const(ConstDcl),
    Type(TypeDcl),
    Except(ExceptDcl),
    Interface(InterfaceDcl),
    Value(ValueDcl),
    Component(ComponentDcl),
    Home(HomeDcl),
    PortType(PortTypeDcl),
    Connector(ConnectorDcl),
    TemplateModuleDcl(TemplateModuleDcl),
    TemplateModuleInst(TemplateModuleInst),
    Annotation(AnnotationDcl),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ScopedName {
    Absolute(Vec<String>),
    Relative(Vec<String>),
}

impl ScopedName {
    pub fn to_primitive(&self) -> Option<PrimitiveType> {
        if let ScopedName::Absolute(s) = self {
            if s.len() == 1 {
                return PrimitiveType::new(s[0].as_str());
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Identifier {
    pub annotation: Option<AnnotationAppl>,
    pub id: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PrimitiveType {
    Short,            // signed 2^16
    Long,             // signed 2^32
    LongLong,         // signed 2^64
    UnsignedShort,    // unsigned 2^16
    UnsignedLong,     // unsigned 2^32
    UnsignedLongLong, // unsigned 2^64
    Float,            // 32-bit floating point number
    Double,           // 64-bit floating point number
    LongDouble,       // 128 bits floating point number
    Char,             // 8-bit character
    WChar,            // implementation dependent wide character
    Boolean,          // boolean
    Any,              // any type

    // an opaque 8-bit quantity that is guaranteed not to undergo any change by the middleware
    Octet,

    // Extended types
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
}

impl PrimitiveType {
    pub fn new(c: &str) -> Option<Self> {
        match c {
            "char" => Some(PrimitiveType::Char),
            "wchar" => Some(PrimitiveType::WChar),
            "boolean" => Some(PrimitiveType::Boolean),
            "octet" => Some(PrimitiveType::Octet),
            "short" => Some(PrimitiveType::Short),
            "long" => Some(PrimitiveType::Long),
            "double" => Some(PrimitiveType::Double),
            "float" => Some(PrimitiveType::Float),
            "any" => Some(PrimitiveType::Any),
            "int8" => Some(PrimitiveType::Int8),
            "uint8" => Some(PrimitiveType::Uint8),
            "int16" => Some(PrimitiveType::Int16),
            "uint16" => Some(PrimitiveType::Uint16),
            "int32" => Some(PrimitiveType::Int32),
            "uint32" => Some(PrimitiveType::Uint32),
            "int64" => Some(PrimitiveType::Int64),
            "uint64" => Some(PrimitiveType::Uint64),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ConstExpr {
    Or(Box<ConstExpr>, Box<ConstExpr>),
    Xor(Box<ConstExpr>, Box<ConstExpr>),
    And(Box<ConstExpr>, Box<ConstExpr>),
    RShift(Box<ConstExpr>, Box<ConstExpr>),
    LShift(Box<ConstExpr>, Box<ConstExpr>),
    Add(Box<ConstExpr>, Box<ConstExpr>),
    Sub(Box<ConstExpr>, Box<ConstExpr>),
    Mul(Box<ConstExpr>, Box<ConstExpr>),
    Div(Box<ConstExpr>, Box<ConstExpr>),
    Mod(Box<ConstExpr>, Box<ConstExpr>),
    UnaryOp(UnaryOpExpr),
    Literal(Literal),
    ScopedName(ScopedName),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum UnaryOpExpr {
    Minus(Box<ConstExpr>),
    Plus(Box<ConstExpr>),
    Negate(Box<ConstExpr>),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum StringType {
    UnlimitedSize,
    Sized(ConstExpr),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum WStringType {
    UnlimitedSize,
    Sized(ConstExpr),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ConstType {
    PrimitiveType(PrimitiveType),
    ScopedName(ScopedName),
    StringType(StringType),
    WStringType(WStringType),
    FixedPointConst, // constant fixed point number
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Literal {
    Char(char),
    String(String),
    Integer(BigInt),
    FixedPoint(FixedPoint),
    FloatingPoint(f64),
    Boolean(bool),
}

/// `FixedPoint` represents a fixed point number.
/// The actual value is `value / 10^scale`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct FixedPoint {
    pub value: BigInt,
    pub scale: u64,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ConstDcl {
    pub const_type: ConstType,
    pub id: String,
    pub expr: ConstExpr,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct StructForwardDcl(pub String);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum TypeSpec {
    PrimitiveType(PrimitiveType),
    ScopedName(ScopedName),
    Template(Box<TemplateTypeSpec>),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Member {
    pub annotations: Option<Vec<AnnotationAppl>>,
    pub type_spec: TypeSpec,
    pub declarators: Vec<AnyDeclarator>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct StructDef {
    pub id: String,
    pub members: Vec<Member>,
    pub inheritance: Option<ScopedName>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum StructDcl {
    Def(StructDef),
    ForwardDcl(StructForwardDcl),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct EnumDcl {
    pub id: String,
    pub variants: Vec<String>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum CaseLabel {
    Case(ConstExpr),
    Default,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Case {
    pub labels: Vec<CaseLabel>,
    pub spec: ElementSpec,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ElementSpec {
    pub type_spec: TypeSpec,
    pub declarator: AnyDeclarator,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum SwitchTypeSpec {
    PrimitiveType(PrimitiveType),
    ScopedName(ScopedName),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct UnionDef {
    pub id: String,
    pub switch_type_spec: SwitchTypeSpec,
    pub body: Vec<Case>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum UnionDcl {
    Def(UnionDef),
    ForwardDcl(UnionForwardDcl),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct UnionForwardDcl(pub String);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ConstrTypeDcl {
    Struct(StructDcl),
    Union(UnionDcl),
    Enum(EnumDcl),
    Bitset(BitsetDcl),
    Bitmask(BitmaskDcl),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum TypeDcl {
    ConstrType(ConstrTypeDcl),
    Native(NativeDcl),
    Typedef(Typedef),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct NativeDcl(pub String);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Typedef {
    pub type_dcl: TypedefType,
    pub declarators: Vec<AnyDeclarator>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum TypedefType {
    Simple(TypeSpec),
    Constr(ConstrTypeDcl),
    Template(TemplateTypeSpec),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum SequenceType {
    Unlimited(TypeSpec),
    Limited(TypeSpec, ConstExpr),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum TemplateTypeSpec {
    Sequence(SequenceType),
    String(StringType),
    WString(WStringType),
    FixedPoint(FixedPtType),
    Map(MapType),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct FixedPtType {
    pub total_digits: ConstExpr,      // up to 31
    pub fractional_digits: ConstExpr, // less than or equal to total_digits
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum AnyDeclarator {
    Simple(String),
    Array(ArrayDeclarator),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ArrayDeclarator {
    pub id: String,
    pub array_size: Vec<ConstExpr>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ExceptDcl {
    pub id: String,
    pub members: Vec<Member>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct InterfaceHeader {
    pub id: String,
    pub inheritance: Option<Vec<ScopedName>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct InterfaceForwardDcl(pub String);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum OpTypeSpec {
    TypeSpec(TypeSpec),
    Void,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ParamAttribute {
    In,
    Out,
    InOut,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ParamDcl {
    pub attr: ParamAttribute,
    pub type_spec: TypeSpec,
    pub id: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Raises(pub Vec<ScopedName>);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct OpDcl {
    pub id: String,
    pub type_spec: OpTypeSpec,
    pub params: Vec<ParamDcl>,
    pub raises: Option<Raises>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ReadonlyAttrDeclarator {
    WithRaises(String, Raises),
    IDs(Vec<String>),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ReadonlyAttrSpec {
    pub type_spec: TypeSpec,
    pub declarator: ReadonlyAttrDeclarator,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct SetExcep(pub Vec<ScopedName>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GetExcep(pub Vec<ScopedName>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AttrRaises {
    GetExcep(GetExcep, Option<SetExcep>),
    SetExcep(SetExcep),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AttrDeclarator {
    WithRaises(String, AttrRaises),
    IDs(Vec<String>),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct AttrSpec {
    pub type_spec: TypeSpec,
    pub declarator: AttrDeclarator,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum AttrDcl {
    Readonly(ReadonlyAttrSpec),
    ReadWrite(AttrSpec),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Export {
    Op(OpDcl),
    Attr(AttrDcl),
    Type(TypeDcl),
    Const(ConstDcl),
    Except(ExceptDcl),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct InterfaceDef {
    pub header: InterfaceHeader,
    pub body: Vec<Export>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum InterfaceDcl {
    Def(InterfaceDef),
    ForwardDcl(InterfaceForwardDcl),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct InitParamDcl {
    pub type_spec: TypeSpec,
    pub id: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct InitDcl {
    pub id: String,
    pub params: Vec<InitParamDcl>,
    pub raises: Option<Raises>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct StateMember {
    pub visibility: Visibility,
    pub type_spec: TypeSpec,
    pub declarators: Vec<AnyDeclarator>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ValueElement {
    Export(Export),
    StateMember(StateMember),
    InitDcl(InitDcl),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ValueInheritanceSpec {
    pub value: Option<ScopedName>,
    pub interface: Option<ScopedName>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ValueHeader {
    pub id: String,
    pub inheritance: ValueInheritanceSpec,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ValueDef {
    pub header: ValueHeader,
    pub elements: Vec<ValueElement>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ValueForwardDcl(pub String);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ValueDcl {
    Def(ValueDef),
    ForwardDcl(ValueForwardDcl),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct FactoryParamDcl {
    pub type_spec: TypeSpec,
    pub id: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct FactoryDcl {
    pub id: String,
    pub params: Vec<FactoryParamDcl>,
    pub raises: Option<Raises>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum HomeExport {
    Export(Export),
    FactoryDcl(FactoryDcl),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HomeHeader {
    pub id: String,
    pub inheritance: Option<HomeInheritanceSpec>,
    pub manages: ScopedName,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct HomeDcl {
    pub header: HomeHeader,
    pub body: Vec<HomeExport>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HomeInheritanceSpec(pub ScopedName);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ComponentForwardDcl(pub String);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ComponentInheritanceSpec(pub ScopedName);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ComponentHeader {
    pub id: String,
    pub inheritance: Option<ComponentInheritanceSpec>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct InterfaceType(pub ScopedName);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ProvidesDcl {
    pub interface_type: InterfaceType,
    pub id: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct UsesDcl {
    pub interface_type: InterfaceType,
    pub id: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ComponentExport {
    Provides(ProvidesDcl),
    Uses(UsesDcl),
    Attr(AttrDcl),
    Port(PortDcl),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ComponentDef {
    pub header: ComponentHeader,
    pub body: Vec<ComponentExport>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ComponentDcl {
    Def(ComponentDef),
    ForwardDcl(ComponentForwardDcl),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct PortTypeForwardDcl(pub String);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PortDcl {
    Port(ScopedName, String),
    MirrorPort(ScopedName, String),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PortRef {
    Provides(ProvidesDcl),
    Uses(UsesDcl),
    Port(PortDcl),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum PortExport {
    PortRef(PortRef),
    Attr(AttrDcl),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct PortBody {
    pub port_ref: PortRef,
    pub port_export: Vec<PortExport>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct PortTypeDef {
    pub id: String,
    pub body: PortBody,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum PortTypeDcl {
    Def(PortTypeDef),
    ForwardDcl(PortTypeForwardDcl),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ConnectorInheritSpec(pub ScopedName);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ConnectorExport {
    PortRef(PortRef),
    Attr(AttrDcl),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ConnectorHeader {
    pub id: String,
    pub inheritance: Option<ConnectorInheritSpec>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct ConnectorDcl {
    pub header: ConnectorHeader,
    pub export: Vec<ConnectorExport>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum FormalParameterType {
    Typename,
    Interface,
    ValueType,
    EventType,
    Struct,
    Union,
    Exception,
    Enum,
    Sequence,
    Const(ConstType),
    SequenceType(SequenceType),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct FormalParameter {
    pub parameter_type: FormalParameterType,
    pub id: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ActualParameter {
    TypeSpec(TypeSpec),
    ConstExpr(ConstExpr),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct TemplateModuleInst {
    pub name: ScopedName,
    pub params: Vec<ActualParameter>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct TemplateModuleRef {
    pub name: ScopedName,
    pub params: Vec<String>,
    pub id: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum TplDefinition {
    Definition(AnnotationAndDef),
    TemplateModuleRef(TemplateModuleRef),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct TemplateModuleDcl {
    pub id: String,
    pub params: Vec<FormalParameter>,
    pub definitions: Vec<TplDefinition>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct MapType {
    pub key: TypeSpec,
    pub value: TypeSpec,
    pub size: Option<ConstExpr>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct BitfieldSpec {
    pub bits: ConstExpr,
    pub destination_type: Option<PrimitiveType>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Bitfield {
    pub spec: BitfieldSpec,
    pub ids: Vec<String>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct BitsetDcl {
    pub id: String,
    pub name: Option<ScopedName>,
    pub fields: Vec<Bitfield>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct BitValue(pub String);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct BitmaskDcl {
    pub id: String,
    pub values: Vec<BitValue>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AnnotationHeader(pub String);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct AnnotationApplParam {
    pub id: String,
    pub expr: ConstExpr,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum AnnotationApplParams {
    ConstExpr(ConstExpr),
    ApplParams(Vec<AnnotationApplParam>),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct AnnotationAppl {
    pub name: ScopedName,
    pub params: Option<AnnotationApplParams>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct AnnotationMember {
    pub member_type: ConstType,
    pub declarator: String,
    pub default_value: Option<ConstExpr>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum AnnotationBody {
    AnnotationMember(AnnotationMember),
    Enum(EnumDcl),
    Const(ConstDcl),
    Typedef(Typedef),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct AnnotationDcl {
    pub header: AnnotationHeader,
    pub body: Vec<AnnotationBody>,
}
