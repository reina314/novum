use super::{
    SetRef,
    IteratorRef,
    ObjectRef,
    ClassRef,
    StructTypeRef,
    StructValueRef,
    EnumRef,
    EnumValueRef,
    EnumConstructor,
    VectorRef,
    MatrixRef,
    SeriesRef,
    DataFrameRef,
    // GroupedDataFrameRef,
    ModuleRef,
    PathRef,
    ClosureRef,
    FunctionRef,
};

use std::{
    fmt, 
    rc::Rc,
    cell::{
        Ref,
        RefMut,
        RefCell,
    }, 
    collections::HashMap, 
};

pub type StrRef = Rc<String>;
pub type Tuple = Rc<Vec<Value>>;
pub type Dict = Rc<RefCell<HashMap<String, Value>>>;
pub type BuiltinFn = fn(Vec<Value>) -> Result<Value, String>;

#[derive(Debug, Clone)]
pub struct List {
    elements:
        Rc<RefCell<Vec<Value>>>,
}

impl List {
    #[inline]
    pub fn new(
        elements: Vec<Value>,
    ) -> Self {
        Self {
            elements:
                Rc::new(
                    RefCell::new(
                        elements
                    )
                ),
        }
    }

    #[inline]
    pub fn with_capacity(
        capacity: usize,
    ) -> Self {
        Self {
            elements:
                Rc::new(
                    RefCell::new(
                        Vec::with_capacity(
                            capacity
                        )
                    )
                ),
        }
    }

    #[inline]
    pub fn len(
        &self,
    ) -> usize {
        self.elements
            .borrow()
            .len()
    }

    #[inline]
    pub fn is_empty(
        &self,
    ) -> bool {
        self.elements
            .borrow()
            .is_empty()
    }

    #[inline]
    pub fn get(
        &self,
        index: usize,
    ) -> Option<Value> {
        self.elements
            .borrow()
            .get(index)
            .cloned()
    }

    #[inline]
    pub fn set(
        &self,
        index: usize,
        value: Value,
    ) -> Result<(), String> {
        let mut elements =
            self.elements
                .borrow_mut();

        let slot =
            elements
                .get_mut(index)
                .ok_or_else(|| {
                    format!(
                        "list index out of bounds: {}",
                        index
                    )
                })?;

        *slot =
            value;

        Ok(())
    }

    #[inline]
    pub fn push(
        &self,
        value: Value,
    ) {
        self.elements
            .borrow_mut()
            .push(value);
    }

    #[inline]
    pub fn append(
        &self,
        mut values: Vec<Value>,
    ) {
        self.elements
            .borrow_mut()
            .append(
                &mut values
            );
    }

    #[inline]
    pub fn extend(
        &self,
        values: impl IntoIterator<Item = Value>,
    ) {
        self.elements
            .borrow_mut()
            .extend(values);
    }

    #[inline]
    pub fn iter_cloned(
        &self,
    ) -> Vec<Value> {
        self.elements
            .borrow()
            .clone()
    }

    #[inline]
    pub fn as_vec(
        &self,
    ) -> Ref<'_, Vec<Value>> {
        self.elements
            .borrow()
    }

    #[inline]
    pub fn as_vec_mut(
        &self,
    ) -> RefMut<'_, Vec<Value>> {
        self.elements
            .borrow_mut()
    }

    pub fn repeat(
        &self,
        count: usize,
    ) -> Result<Self, String> {
        if count == 0 {
            return Ok(
                Self::new(
                    Vec::new()
                )
            );
        }

        let elements =
            self.elements
                .borrow();

        let capacity =
            elements
                .len()
                .checked_mul(count)
                .ok_or_else(|| {
                    "list repetition size overflow"
                        .to_string()
                })?;

        let mut result =
            Vec::with_capacity(
                capacity
            );

        for _ in 0..count {
            result.extend(
                elements.iter().cloned()
            );
        }

        Ok(
            Self::new(
                result
            )
        )
    }
}

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

    StructType(StructTypeRef),
    Struct(StructValueRef),

    Object(ObjectRef),
    Class(ClassRef),

    Vector(VectorRef),
    Matrix(MatrixRef),

    Series(SeriesRef),
    DataFrame(DataFrameRef),
    // GroupedDataFrame(GroupedDataFrameRef),

    Module(ModuleRef),

    Enum(EnumRef),
    EnumValue(EnumValueRef),
    EnumConstructor(EnumConstructor),

    Path(PathRef),

    Range(i64, i64, bool),
    
    FunctionProto(FunctionRef),
    Closure(ClosureRef),
    Iterator(IteratorRef),
    Builtin(BuiltinFn),
    
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

            Self::Tuple(_) => "Tuple",
            Self::List(_) => "List",
            Self::Set(_) => "Set",
            Self::Dict(_) => "Dict",

            Self::StructType(_) => "StructType",
            Self::Struct(_) => "Struct",

            Self::Object(_) => "Object",
            Self::Class(_) => "Class",

            Self::Vector(_) => "Vector",
            Self::Matrix(_) => "Matrix",
            
            Self::Series(_) => "Series",
            Self::DataFrame(_) => "DataFrame",
            // Self::GroupedDataFrame(_) => "GroupedDataFrame",

            Self::Module(_) => "Module",

            Self::Enum(_) => "Enum",
            Self::EnumValue(_) => "EnumValue",
            Self::EnumConstructor(_) => "EnumConstructor",

            Self::Path(_) => "Path",

            Self::Range(..) => "Range",

            Self::FunctionProto(_) => "Function",
            Self::Closure(_) => "Closure",
            Self::Builtin(_) => "Builtin",
            Self::Iterator(_) => "Iterator",

            Self::Unit => "Unit",
            Self::Null => "Null",
        }
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

    pub fn eq_values(
        a: &Self,
        b: &Self
    ) -> Result<bool, String> {
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
                let x 
                    = x.as_vec();
                let y 
                    = y.as_vec();

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

            (// element-wise
                Self::Matrix(a),
                Self::Matrix(b)
            ) => {
                let a =
                    a.borrow();

                let b =
                    b.borrow();

                if a.rows() != b.rows()
                    || a.cols() != b.cols()
                {
                    false
                } else {
                    for row in 0..a.rows() {
                        for col in 0..a.cols() {
                            if a.get(row, col)
                                != b.get(row, col)
                            {
                                return Ok(false);
                            }
                        }
                    }

                    true
                }
            },

            (Self::Series(a),Self::Series(b)) => Rc::ptr_eq(a, b),

            (Self::DataFrame(a),Self::DataFrame(b),) => Rc::ptr_eq(a, b),

            // (Self::Object(x), Self::Object(y)) => Rc::ptr_eq(x, y),

            (Self::Iterator(_), Self::Iterator(_)) => false,

            (Self::Builtin(a), Self::Builtin(b)) => *a as usize == *b as usize,

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

            (
                Self::Path(a),
                Self::Path(b),
            ) => {
                a.as_path() == b.as_path()
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

            Self::List(v) => write!(f, "{:?}", v),

            Self::Set(set) => 
            write!(f, "{:?}", set.borrow().values()),

            Self::Dict(v) => write!(f, "{:?}", v.borrow()),

            Self::StructType(v) => write!(f, "{}", v),

            Self::Struct(v) => write!(f, "{}", v),

            Self::Vector(v) => write!(f, "{:?}", v.borrow()),

            Self::Matrix(v) => write!(f, "{:?}", v.borrow()),

            Self::Series(v) => write!(f, "{:?}", v),

            Self::DataFrame(df) => write!(f, "{:?}", df),

            // Self::GroupedDataFrame(grouped) => write!(f, "<grouped dataframe: {}>", grouped.group_column()),

            Self::Object(v) => write!(f, "{:?}", v.borrow()),

            Self::Class(class) => write!(f, "<class {}>", class.name()),

            Self::Module(module) => write!(f,"<module {}>",module.borrow().name()),

            Self::Enum(definition) => write!(f, "<enum {}>", definition.name()),

            Self::EnumValue(value) => write!(f, "{:?}", value),

            Self::EnumConstructor(constructor) => write!(f, "{:?}", constructor),

            Self::Path(path) => write!(f, "{:?}", path),

            Self::Range(a,b,inclusive) => if *inclusive { write!(f,"{a}..={b}") } else { write!(f,"{a}..{b}") },

            Self::FunctionProto(function) => write!(f, "<function arity={}>", function.arity),

            Self::Closure(v) => write!(f, "<closure arity={}>", v.function.arity),

            Self::Iterator(_) => write!(f, "<iterator>"),

            Self::Builtin(_) => write!(f, "<builtin>"),

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
                let elements =
                    list.as_vec();

                write!(f, "[")?;

                for (
                    index,
                    value,
                ) in elements.iter().enumerate()
                {
                    if index > 0 {
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

            Self::StructType(v) => 
                write!(f, "{}", v),

            Self::Struct(v) => 
                write!(f, "{}", v),

            Self::Matrix(matrix) => 
                matrix.borrow().fmt_display(f),

            Self::Series(series) => 
                series.fmt_display(f),

            Self::DataFrame(df) =>
                df.fmt_display(f),

            Self::Object(object) => object.borrow().fmt_display(f),

            Self::Class(class) => 
            write!(f, "<class {}>", class.name()),

            Self::Module(module) => write!(f, "<module {}>", module.borrow().name()),

            Self::Enum(definition) =>
                write!(f, "<enum {}>", definition.name()),

            Self::EnumValue(value) =>
                write!(f, "{}", value),

            Self::EnumConstructor(constructor) =>
                write!(f, "{}", constructor),

            Self::Path(path) =>
                write!(f, "{:#?}", path),

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

            Self::FunctionProto(function) => write!(f, "<function arity={}", function.arity),

            Self::Closure(v) => write!(f, "<closure arity={}>", v.function.arity),

            Self::Builtin(_) =>
                write!(f, "<builtin>"),

            Self::Iterator(_) =>
                write!(f, "<iterator>"),

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

