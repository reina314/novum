use super::{
    FuncRef,
    IteratorObj,
    ObjectRef,
    StructRef,
    EnumRef,
    EnumValueRef,
    EnumConstructor,
    MatrixRef,
    SeriesRef,
    DataFrameRef,
    GroupedDataFrameRef,
    ModuleRef,
    BoundMethod,
};
use std::{
    fmt, 
    rc::Rc,
    cell::RefCell, 
    collections::HashMap, 
};

pub type List = Rc<RefCell<Vec<Value>>>;
pub type Dict = Rc<RefCell<HashMap<String, Value>>>;
pub type BuiltinFn = fn(Vec<Value>) -> Result<Value, String>;

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Rc<String>),

    List(List),
    Dict(Dict),
    Matrix(MatrixRef),
    Series(SeriesRef),
    DataFrame(DataFrameRef),
    GroupedDataFrame(GroupedDataFrameRef),

    Object(ObjectRef),
    Struct(StructRef),
    Module(ModuleRef),

    Enum(EnumRef),
    EnumValue(EnumValueRef),
    EnumConstructor(EnumConstructor),

    Range(i64, i64, bool),
    
    Func(FuncRef),
    Iterator(IteratorObj),
    Builtin(BuiltinFn),

    BoundMethod(BoundMethod),
    
    Unit,
    Null,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::Bool(_) => "Bool",
            Self::Str(_) => "Str",

            Self::List(_) => "List",
            Self::Dict(_) => "Dict",
            Self::Matrix(_) => "Matrix",
            Self::Series(_) => "Series",
            Self::DataFrame(_) => "DataFrame",
            Self::GroupedDataFrame(_) => "GroupedDataFrame",

            Self::Object(_) => "Object",
            Self::Struct(_) => "Struct",
            Self::Module(_) => "Module",

            Self::Enum(_) => "Enum",
            Self::EnumValue(_) => "EnumValue",
            Self::EnumConstructor(_) => "EnumConstructor",

            Self::Range(..) => "Range",

            Self::Func(_) => "Function",
            Self::Iterator(_) => "Iterator",
            Self::Builtin(_) => "Builtin",

            Self::BoundMethod(_) => "Method",

            Self::Unit => "Unit",
            Self::Null => "Null",
        }
    }

    pub fn truthy_bool(&self) -> Option<bool> {
        match self { Self::Bool(v) => Some(*v), _ => None }
    }

    pub fn negate(self) -> Result<Self, String> {
        match self {
            Self::Int(n) => 
                n
                    .checked_neg()
                    .map(Self::Int)
                    .ok_or_else(
                        || "integer overflow in negation".into()
                    ),
            Self::Float(n) => 
                Ok(Self::Float(-n)),

            other => 
                Err(format!(
                    "unary '-' is not defined for {}", other.type_name()
                )),
        }
    }

    pub fn eq_values(a: &Self, b: &Self) -> Result<bool, String> {
        Ok(match (a, b) {
            (Self::Int(x), Self::Int(y)) => x == y,
            (Self::Float(x), Self::Float(y)) => x == y,

            (Self::Int(x), Self::Float(y)) => (*x as f64) == *y,
            (Self::Float(x), Self::Int(y)) => *x == (*y as f64),

            (Self::Str(x), Self::Str(y)) => x == y,
            (Self::Bool(x), Self::Bool(y)) => x == y,

            (Self::List(x), Self::List(y)) => Rc::ptr_eq(x, y),
            (Self::Dict(x), Self::Dict(y)) => Rc::ptr_eq(x, y),
            (Self::Matrix(a), Self::Matrix(b)) => Rc::ptr_eq(a, b),
            (Self::Series(a),Self::Series(b)) => Rc::ptr_eq(a, b),
            (Self::DataFrame(a),Self::DataFrame(b),) => Rc::ptr_eq(a, b),

            (Self::Object(x), Self::Object(y)) => Rc::ptr_eq(x, y),

            (Self::Func(x), Self::Func(y)) => Rc::ptr_eq(x, y),

            (Self::Iterator(_), Self::Iterator(_)) => false,

            (Self::Builtin(a), Self::Builtin(b)) => *a as usize == *b as usize,
            
            (Self::BoundMethod(_), Self::BoundMethod(_),) => false,

            (Self::Range(a1,b1,c1), Self::Range(a2,b2,c2)) => (a1,b1,c1)==(a2,b2,c2),

            (Self::Module(a), Self::Module(b)) => Rc::ptr_eq(a, b),

            (Self::Enum(a),Self::Enum(b),) => Rc::ptr_eq(a, b),
            (Self::EnumValue(a),Self::EnumValue(b),
            ) => {
                if a.enum_name() != b.enum_name()
                    || a.variant() != b.variant()
                    || a.fields().len() != b.fields().len()
                {
                    false
                } else {
                    for (left, right)
                        in a.fields()
                            .iter()
                            .zip(b.fields().iter())
                    {
                        if !Self::eq_values(
                            left,
                            right,
                        )? {
                            return Ok(false);
                        }
                    }

                    true
                }
            }

            (Self::Unit, Self::Unit) => true,
            (Self::Null, Self::Null) => true,
            
            _ => return Err(format!("comparison not defined between {} and {}", a.type_name(), b.type_name())),
        })
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::Str(v) => write!(f, "{v:?}"),

            Self::List(v) => write!(f, "{:?}", v.borrow()),
            Self::Dict(v) => write!(f, "{:?}", v.borrow()),
            Self::Matrix(v) => write!(f, "{:?}", v.borrow()),
            Self::Series(v) => write!(f, "{:?}", v),
            Self::DataFrame(df) => write!(f, "{:?}", df),
            Self::GroupedDataFrame(grouped) => write!(f, "<grouped dataframe: {}>", grouped.group_column()),

            Self::Object(v) => write!(f, "{:?}", v.borrow()),
            Self::Struct(def) => write!(f, "<struct {}>", def.name),
            Self::Module(module) => write!(f,"<module {}>",module.borrow().name()),

            Self::Enum(definition) =>
            write!(
                f,
                "<enum {}>",
                definition.name()
            ),
            Self::EnumValue(value) => {
                write!(
                    f,
                    "{}.{}",
                    value.enum_name(),
                    value.variant()
                )?;

                if !value.fields().is_empty() {
                    write!(f, "(")?;

                    for (i, field) in
                        value.fields().iter().enumerate()
                    {
                        if i > 0 {
                            write!(f, ", ")?;
                        }

                        write!(f, "{:?}", field)?;
                    }

                    write!(f, ")")?;
                }

                Ok(())
            }
            Self::EnumConstructor(..) => write!(f, "<enum constructor>"),

            Self::Range(a,b,inclusive) => if *inclusive { write!(f,"{a}..={b}") } else { write!(f,"{a}..{b}") },
            
            Self::Func(v) => write!(f, "{v}"),
            Self::Iterator(_) => write!(f, "<iterator>"),
            Self::Builtin(_) => write!(f, "<builtin>"),

            Self::BoundMethod(method) => write!(f, "{:?}", method),

            Self::Unit => write!(f, "<unit>"),
            Self::Null => write!(f, "<null>"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(v) =>
                write!(f, "{v}"),

            Self::Float(v) =>
                write!(f, "{}", format_float(*v)),

            Self::Bool(v) =>
                write!(f, "{v}"),

            Self::Str(v) =>
                write!(f, "{v}"),

            Self::Null =>
                write!(f, "null"),

            Self::Unit =>
                write!(f, "()"),

            Self::List(list) => {
                let list = list.borrow();

                write!(f, "[")?;

                for (i, value) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{value}")?;
                }

                write!(f, "]")
            }

            Self::Dict(dict) => {
                let dict = dict.borrow();

                let mut entries =
                    dict.iter()
                        .collect::<Vec<_>>();

                entries.sort_by(
                    |(key_a, _), (key_b, _)| {
                        key_a.cmp(key_b)
                    }
                );

                write!(f, "{{")?;

                for (i, (key, value)) in
                    entries.iter().enumerate()
                {
                    if i > 0 {
                        write!(f, ", ")?;
                    }

                    write!(
                        f,
                        "{:?}: {}",
                        key,
                        value
                    )?;
                }

                write!(f, "}}")
            }

            Self::Matrix(matrix) => 
                matrix.borrow().fmt_display(f),

            Self::Series(series) => 
                series.fmt_display(f),

            Self::DataFrame(df) =>
                df.fmt_display(f),

            Self::Object(object) => 
                object.borrow().fmt_display(f),

            Self::Struct(def) =>
                write!(f, "<struct {}>", def.name),

            Self::Module(module) =>
                write!(f, "<module {}>", module.borrow().name()),

            Self::Enum(definition) =>
                write!(
                    f,
                    "<enum {}>",
                    definition.name()
                ),

            Self::EnumValue(value) => {
                write!(
                    f,
                    "{}.{}",
                    value.enum_name(),
                    value.variant()
                )?;

                if !value.fields().is_empty() {
                    write!(f, "(")?;

                    for (i, field) in
                        value.fields().iter().enumerate()
                    {
                        if i > 0 {
                            write!(f, ", ")?;
                        }

                        write!(f, "{}", field)?;
                    }

                    write!(f, ")")?;
                }

                Ok(())
            }
            Self::EnumConstructor(..) =>
                write!(f, "<enum constructor>"),

            Self::Range(
                start,
                end,
                inclusive,
            ) => {
                if *inclusive {
                    write!(
                        f,
                        "{}..={}",
                        start,
                        end
                    )
                } else {
                    write!(
                        f,
                        "{}..{}",
                        start,
                        end
                    )
                }
            }

            Self::Func(_) =>
                write!(f, "<function>"),

            Self::Builtin(_) =>
                write!(f, "<builtin>"),

            Self::Iterator(_) =>
                write!(f, "<iterator>"),

            Self::BoundMethod(method) =>
                write!(f, "{}", method),

            _ => write!(f, "{:?}", self),
        }
    }
}

fn format_float(value: f64) -> String {
    let s = format!("{value:.8}");

    s.trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool { Self::eq_values(self, other).unwrap_or(false) }
}