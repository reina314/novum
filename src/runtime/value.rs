use super::{
    SetRef,
    FuncRef,
    IteratorRef,
    ObjectRef,
    StructRef,
    EnumRef,
    EnumValueRef,
    EnumConstructor,
    VectorRef,
    MatrixRef,
    SeriesRef,
    DataFrameRef,
    GroupedDataFrameRef,
    ModuleRef,
    BoundMethod,
    Type,
};
use std::{
    fmt, 
    rc::Rc,
    cell::RefCell, 
    collections::HashMap, 
};

pub type StrRef = Rc<String>;
pub type Tuple = Rc<Vec<Value>>;
pub type List = Rc<RefCell<Vec<Value>>>;
pub type Dict = Rc<RefCell<HashMap<String, Value>>>;
pub type BuiltinFn = fn(Vec<Value>) -> Result<Value, String>;

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(StrRef),

    Tuple(Tuple),
    List(List),
    Set(SetRef),
    Dict(Dict),

    Vector(VectorRef),
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
    Iterator(IteratorRef),
    Builtin(BuiltinFn),

    BoundMethod(BoundMethod),
    
    Unit,
    Null,
}

impl Value {
    pub fn value_type(&self) -> Type {
        match self {
            Self::Unit => Type::Unit,
            Self::Null => Type::Null,

            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::Str(_) => Type::Str,

            Self::Tuple(_) => Type::Tuple,
            Self::List(_) => Type::List,
            Self::Set(_) => Type::Set,
            Self::Dict(_) => Type::Dict,

            Self::Vector(_) => Type::Vector,
            Self::Matrix(_) => Type::Matrix,

            Self::Series(_) => Type::Series,
            Self::DataFrame(_) => Type::DataFrame,
            Self::GroupedDataFrame(_) => Type::GroupedDataFrame,

            Self::Object(_) => Type::Object,
            Self::Struct(_) => Type::Struct,
            Self::Module(_) => Type::Module,

            Self::Enum(_) => Type::Enum,
            Self::EnumValue(_) => Type::EnumValue,
            Self::EnumConstructor(_) => Type::EnumConstructor,
            
            Self::Range(..) => Type::Range,

            Self::Func(_) => Type::Function,
            Self::Builtin(_) => Type::Builtin,
            Self::Iterator(_) => Type::Iterator,
            Self::BoundMethod(_) => Type::BoundMethod,
        }
    }

    pub fn type_name(&self) -> &'static str {
        self.value_type().name()
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
            (
                Self::Int(x),
                Self::Int(y)
            ) => x == y,
            (
                Self::Float(x),
                Self::Float(y)
            ) => x == y,

            (Self::Int(x), Self::Float(y)) => (*x as f64) == *y,
            (Self::Float(x), Self::Int(y)) => *x == (*y as f64),

            (Self::Str(x), Self::Str(y)) => x == y,
            (Self::Bool(x), Self::Bool(y)) => x == y,

            (// recursive element-wise
                Self::Tuple(a),
                Self::Tuple(b)
            ) => {
                if a.len() != b.len() {
                    false
                } else {
                    for (lhs, rhs) in a.iter().zip(b.iter()) {
                        if !Self::eq_values(lhs, rhs)? {
                            return Ok(false);
                        }
                    }

                    true
                }
            }
            
            (// recursive element-wise
                Self::List(x),
                Self::List(y)
            ) => {
                let x = x.borrow();
                let y = y.borrow();

                Self::eq_slices(
                    &x,
                    &y,
                )?
            },

            (// element-wise, order-insensitive
                Self::Set(a),
                Self::Set(b),
            ) => {
                let a = a.borrow();
                let b = b.borrow();

                if a.len() != b.len() {
                    false
                } else {
                    for value in a.values() {
                        if !b.contains(value)? {
                            return Ok(false);
                        }
                    }

                    true
                }
            }

            (// recursive key/value-wise
                Self::Dict(x),
                Self::Dict(y),
            ) => {
                let x = x.borrow();
                let y = y.borrow();

                if x.len() != y.len() {
                    false
                } else {
                    for (key, value) in x.iter() {
                        let Some(other) = y.get(key) else {
                            return Ok(false);
                        };

                        if !Self::eq_values(
                            value,
                            other,
                        )? {
                            return Ok(false);
                        }
                    }

                    true
                }
            }

            (// element-wise
                Self::Vector(a),
                Self::Vector(b)    
            ) => {
                let a = a.borrow();
                let b = b.borrow();

                a.as_slice() == b.as_slice()
            },

            (// recursive element-wise
                Self::Matrix(a),
                Self::Matrix(b)
            ) => {
                let a = a.borrow();
                let b = b.borrow();

                a.rows() == b.rows()
                    && a.cols() == b.cols()
                    && a.as_slice() == b.as_slice()
            },

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

    fn eq_slices(
        a: &[Value],
        b: &[Value],
    ) -> Result<bool, String> {
        if a.len() != b.len() {
            return Ok(false);
        }

        for (x, y) in a.iter().zip(b.iter()) {
            if !Self::eq_values(x, y)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::Str(v) => write!(f, "{v:?}"),

            Self::Tuple(values) => {
                write!(f, "(")?;

                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{:?}", value)?;
                }

                if values.len() == 1 {
                    write!(f, ",")?;
                }

                write!(f, ")")
            }

            Self::List(v) => write!(f, "{:?}", v.borrow()),

            Self::Set(set) => 
            write!(f, "{:?}", set.borrow().values()),

            Self::Dict(v) => write!(f, "{:?}", v.borrow()),

            Self::Vector(v) => write!(f, "{:?}", v.borrow()),

            Self::Matrix(v) => write!(f, "{:?}", v.borrow()),

            Self::Series(v) => write!(f, "{:?}", v),

            Self::DataFrame(df) => write!(f, "{:?}", df),

            Self::GroupedDataFrame(grouped) => write!(f, "<grouped dataframe: {}>", grouped.group_column()),

            Self::Object(v) => write!(f, "{:?}", v.borrow()),

            Self::Struct(def) => write!(f, "<struct {}>", def.name),

            Self::Module(module) => write!(f,"<module {}>",module.borrow().name()),

            Self::Enum(definition) => write!(f, "<enum {}>", definition.name()),

            Self::EnumValue(value) => write!(f, "{:?}", value),

            Self::EnumConstructor(constructor) => write!(f, "{:?}", constructor),

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

            Self::Tuple(values) => {
                write!(f, "(")?;

                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{}", value)?;
                }

                if values.len() == 1 {
                    write!(f, ",")?;
                }

                write!(f, ")")
            }

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

            Self::Set(set) => {
                let set =
                    set.borrow();

                write!(f, "{{")?;

                for (i, value)
                    in set.values().iter().enumerate()
                {
                    if i > 0 {
                        write!(f, ", ")?;
                    }

                    write!(
                        f,
                        "{}",
                        value
                    )?;
                }

                write!(f, "}}")
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
                write!(f, "<enum {}>", definition.name()),

            Self::EnumValue(value) =>
                write!(f, "{}", value),

            Self::EnumConstructor(constructor) =>
                write!(f, "{}", constructor),

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

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool { 
        Self::eq_values(self, other)
            .unwrap_or(false) 
    }
}

fn format_float(value: f64) -> String {
    let s = format!("{value:.8}");

    let s = s
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();

    if s.contains('.') {
        s
    } else {
        format!("{s}.0")
    }
}


pub trait FromValue: Sized {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value>;

    fn expected_type() -> Type;
}

impl FromValue for i64 {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value> {
        match value {
            Value::Int(v) =>
                Ok(v),

            other =>
                Err(other),
        }
    }

    fn expected_type() -> Type {
        Type::Int
    }
}

impl FromValue for bool {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value> {
        match value {
            Value::Bool(b) =>
                Ok(b),

            other =>
                Err(other),
        }
    }

    fn expected_type() -> Type {
        Type::Bool
    }
}

impl FromValue for List {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value> {
        match value {
            Value::List(list) =>
                Ok(list),

            other =>
                Err(other),
        }
    }

    fn expected_type() -> Type {
        Type::List
    }
}

impl FromValue for SetRef {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value> {
        match value {
            Value::Set(value) =>
                Ok(value),

            other =>
                Err(other),
        }
    }

    fn expected_type() -> Type {
        Type::Set
    }
}

impl FromValue for StrRef {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value> {
        match value {
            Value::Str(value) =>
                Ok(value),

            other =>
                Err(other),
        }
    }

    fn expected_type() -> Type {
        Type::Str
    }
}

impl FromValue for VectorRef {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value> {
        match value {
            Value::Vector(vector) =>
                Ok(vector),

            other =>
                Err(other),
        }
    }

    fn expected_type() -> Type {
        Type::Vector
    }
}

impl FromValue for MatrixRef {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value> {
        match value {
            Value::Matrix(matrix) =>
                Ok(matrix),

            other =>
                Err(other),
        }
    }

    fn expected_type() -> Type {
        Type::Matrix
    }
}

impl FromValue for IteratorRef {
    fn from_value(
        value: Value,
    ) -> Result<Self, Value> {
        match value {
            Value::Iterator(iterator) =>
                Ok(iterator),

            other =>
                Err(other),
        }
    }

    fn expected_type() -> Type {
        Type::Iterator
    }
}

