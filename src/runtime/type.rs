#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub enum Type {
    Unit,
    Null,

    Int,
    Float,
    Bool,
    Str,

    Tuple,
    List,
    Set,
    Dict,

    Vector,
    Matrix,

    Series,
    DataFrame,
    GroupedDataFrame,

    Range,

    Function,
    Builtin,
    Iterator,
    BoundMethod,

    Object,
    Struct,
    Module,

    Enum,
    EnumValue,
    EnumConstructor,

    Path,
}

impl Type {
    pub fn name(self) -> &'static str {
        match self {
            Self::Int => "Int",
            Self::Float => "Float",
            Self::Bool => "Bool",
            Self::Str => "Str",

            Self::Tuple => "Tuple",
            Self::List => "List",
            Self::Set => "Set",
            Self::Dict => "Dict",

            Self::Vector => "Vector",
            Self::Matrix => "Matrix",
            
            Self::Series => "Series",
            Self::DataFrame => "DataFrame",
            Self::GroupedDataFrame => "GroupedDataFrame",

            Self::Object => "Object",
            Self::Struct => "Struct",
            Self::Module => "Module",

            Self::Enum => "Enum",
            Self::EnumValue => "EnumValue",
            Self::EnumConstructor => "EnumConstructor",

            Self::Path => "Path",

            Self::Range => "Range",

            Self::Function => "Function",
            Self::Builtin => "Builtin",
            Self::Iterator => "Iterator",
            Self::BoundMethod => "BoundMethod",

            Self::Unit => "Unit",
            Self::Null => "Null",
        }
    }
}