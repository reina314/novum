use crate::{
    error::{
        Error, 
        ErrorKind, 
        Result, 
        StackFrame
    }, 
    interpreter::{ModuleLoader, operator}, 
    runtime::{
        BoundMethod, ControlFlow, DataFrameRef, EnumConstructor, EnumDef as RuntimeEnumDef, EnumValue, EnumValueRef, Env, FuncRef, Function, GroupedDataFrame, Dict, GroupedDataFrameRef, IteratorObj, IteratorRef, List, MethodReceiver, Module, ModuleContext, ModulePath, ModuleRef, ObjectRef, Series, SeriesRef, StructDefinition, Value, FromValue, Vector, VectorRef, MatrixRef, Type, StrRef, SetRef,
    }, stdlib, 
    syntax::{
        BinOp, Expr, ExprKind, IndexExpr, ListItem, Program, 
        ast::{
            EnumDef as AstEnumDef,
            MatchArm,
            Pattern,
            Visibility
        }
    },
};
use std::{
    cell::RefCell,
    rc::Rc,
    collections::HashMap,
};

pub struct Interpreter {
    env: Env,
    stack: Vec<StackFrame>,
    loop_depth: usize,
    function_depth: usize,
    module_loader: ModuleLoader,
    module_stack: Vec<ModuleContext>,
}

impl Default for Interpreter {
    fn default() -> Self { Self::new() }
}

impl Interpreter {
    pub fn new() -> Self {
        let env = Env::global();

        let module_loader = ModuleLoader::new(
            std::env::current_dir()
                .expect(
                    "failed to get current directory"
                )
        );

        let mut interpreter = Interpreter { 
            env,
            stack: Vec::new(),
            loop_depth: 0,
            function_depth: 0,
            module_loader,
            module_stack: Vec::new(),
        };

        // eager loading
        interpreter.install_builtins();
        interpreter.install_standard_enums();

        interpreter
    }

    fn install_builtins(&mut self) {
        stdlib::install_builtins(
            &self.env
        )
    }

    fn install_standard_enums(&mut self) {
        // ---------------------------------------------------------
        // Option
        // ---------------------------------------------------------
        let mut option =
            RuntimeEnumDef::new("Option");

        option
            .add_variant(
                "Some",
                1,
            )
            .expect("valid Option");

        option
            .add_variant(
                "None",
                0,
            )
            .expect("valid Option");

        self.env.define(
            "Option",
            Value::Enum(
                Rc::new(option)
            ),
        );

        // ---------------------------------------------------------
        // Result
        // ---------------------------------------------------------
        let mut result =
            RuntimeEnumDef::new("Result");

        result
            .add_variant(
                "Ok",
                1,
            )
            .expect("valid Result");

        result
            .add_variant(
                "Err",
                1,
            )
            .expect("valid Result");

        self.env.define(
            "Result",
            Value::Enum(
                Rc::new(result)
            ),
        );
    }

    fn next_iterator_value(
        &mut self,
        iterator: &mut IteratorObj,
        whole: &Expr,
    ) -> Result<Option<Value>> {
        match iterator {
            IteratorObj::List {
                data,
                index,
            } => {
                let value =
                    data.borrow()
                        .get(*index)
                        .cloned();

                if value.is_some() {
                    *index += 1;
                }

                Ok(value)
            }

            IteratorObj::Str {
                data,
                index,
            } => {
                let value =
                    data.get(*index)
                        .copied();

                match value {
                    Some(ch) => {
                        *index += 1;

                        Ok(
                            Some(
                                Value::Str(
                                    Rc::new(
                                        ch.to_string()
                                    )
                                )
                            )
                        )
                    }

                    None =>
                        Ok(None),
                }
            }

            IteratorObj::Range {
                current,
                end,
            } => {
                if *current >= *end {
                    return Ok(None);
                }

                let value =
                    *current;

                *current += 1;

                Ok(
                    Some(
                        Value::Int(value)
                    )
                )
            }

            IteratorObj::Map {
                source,
                function,
            } => {
                let input =
                    {
                        let mut source =
                            source.borrow_mut();

                        self.next_iterator_value(
                            &mut source,
                            whole,
                        )?
                    };

                match input {
                    Some(value) => {
                        let value =
                        self.call_iterator_callback(
                            function.clone(),
                            vec![value],
                            whole,
                        )?;

                        Ok(Some(value))
                    }

                    None =>
                        Ok(None),
                }
            }

            IteratorObj::Filter {
                source,
                predicate,
            } => {
                loop {
                    let input =
                        {
                            let mut source =
                                source.borrow_mut();

                            self.next_iterator_value(
                                &mut source,
                                whole,
                            )?
                        };

                    let value =
                        match input {
                            Some(value) =>
                                value,

                            None =>
                                return Ok(None),
                        };

                    let result = 
                        self.call_iterator_callback(
                            predicate.clone(),
                            vec![
                                value.clone()
                            ],
                            whole,
                        )?;

                    match result {
                        Value::Bool(true) => {
                            return Ok(
                                Some(value)
                            );
                        }

                        Value::Bool(false) => {
                            continue;
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "filter predicate must return Bool, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    }
                }
            }
        
            IteratorObj::Enumerate { 
                source,
                index 
            } => {
                let value =
                    self.next_from_iterator(
                        source,
                        whole,
                    )?;

                match value {
                    Some(value) => {
                        let current = *index;

                        *index = index
                            .checked_add(1)
                            .ok_or_else(|| {
                                self.error(
                                    ErrorKind::Overflow,
                                    "iterator index overflow",
                                    whole,
                                )
                            })?;

                        Ok(Some(
                            Value::Tuple(Rc::new(
                                vec![
                                    Value::Int(
                                        current as i64
                                    ),
                                    value,
                                ]
                            ))
                        ))
                    }

                    None =>
                        Ok(None),
                }
            }
        
            IteratorObj::Zip {
                left,
                right,
            } => {
                let l =
                    self.next_from_iterator(
                        left,
                        whole,
                    )?;

                let l = match l {
                    Some(value) => value,
                    None => return Ok(None),
                };

                let r = 
                    self.next_from_iterator(
                            right,
                            whole,
                        )?;

                let r = match r {
                    Some(value) => value,
                    None => return Ok(None),
                };

                Ok(Some(
                    Value::Tuple(Rc::new(
                        vec![
                            l,
                            r,
                        ]
                    ))
                ))
            }
        
            IteratorObj::Take {
                source,
                remaining,
            } => {
                if *remaining == 0 {
                    return Ok(None);
                }

                let value =
                    self.next_from_iterator(
                        source,
                        whole,
                    )?;

                match value {
                    Some(value) => {
                        *remaining -= 1;

                        Ok(Some(value))
                    }

                    None =>
                        Ok(None),
                }
            }
        
            IteratorObj::Skip {
                source,
                remaining,
            } => {
                while *remaining > 0 {
                    match self.next_from_iterator(
                        source,
                        whole,
                    )? {
                        Some(_) => {
                            *remaining -= 1;
                        }

                        None =>
                            return Ok(None),
                    }
                }

                self.next_from_iterator(
                    source,
                    whole,
                )
            }
        }
    }

    fn next_from_iterator(
        &mut self,
        iterator: &IteratorRef,
        whole: &Expr,
    ) -> Result<Option<Value>> {
        let mut iterator =
            iterator.borrow_mut();

        self.next_iterator_value(
            &mut iterator,
            whole,
        )
    }

    /// Helper function to validate Value type
    fn expect_type(
        &self,
        actual: &Value,
        expected: Type,
        expr: &Expr,
    ) -> Result<()> {
        if actual.value_type() == expected {
            Ok(())
        } else {
            Err(
                self.error(
                    ErrorKind::Type,
                    format!(
                        "expected {}, got {}",
                        expected.name(),
                        actual.type_name()
                    ),
                    expr,
                )
            )
        }
    }

    /// Helper funtion to validate i64 and f64
    fn expect_number(
        &self,
        value: Value,
        expr: &Expr,
    ) -> Result<f64> {
        match value {
            Value::Int(value) =>
                Ok(value as f64),

            Value::Float(value) =>
                Ok(value),

            other => {
                Err(
                    self.error(
                        ErrorKind::Type,
                        format!(
                            "expected numeric value, got {}",
                            other.type_name()
                        ),
                        expr,
                    )
                )
            }
        }
    }

    /// Helper function to validate Value type and extract data
    fn expect<T: FromValue>(
        &self,
        value: Value,
        expr: &Expr,
    ) -> Result<T> {
        match T::from_value(value) {
            Ok(value) =>
                Ok(value),

            Err(actual) =>
                Err(
                    self.error(
                        ErrorKind::Type,
                        format!(
                            "expected {}, got {}",
                            T::expected_type().name(),
                            actual.type_name(),
                        ),
                        expr,
                    )
                ),
        }
    }

    pub fn eval_program(
        &mut self,
        program: &Program
    ) -> Result<ControlFlow> {
        let mut last = Value::Unit;

        for expr in &program.statements {
            match self.eval(expr)? {
                ControlFlow::Value(v) 
                    => last = v,
                ControlFlow::Return(value) 
                    => return Err(
                        self.error(
                            ErrorKind::Control,
                            format!(
                                "unexpected top-level return: {}",
                                value
                            ), 
                            expr,
                        )
                    ),
                ControlFlow::Break 
                    => return Err(
                        self.error(
                            ErrorKind::Control,
                            "unexpected top-level break",
                            expr,
                        )
                    ),
                ControlFlow::Continue
                    => return Err(
                        self.error(
                            ErrorKind::Control,
                            "unexpected top-level continue",
                            expr,
                        )
                    )
            }
        }

        Ok(ControlFlow::Value(last))
    }

    pub fn eval(
        &mut self,
        expr: &Expr
    ) -> Result<ControlFlow> {
        use ExprKind::*;

        match &expr.kind {
            Int(n) => Ok(ControlFlow::Value(Value::Int(*n))),
            Float(n) => Ok(ControlFlow::Value(Value::Float(*n))),
            Str(s) => Ok(ControlFlow::Value(Value::Str(Rc::new(s.clone())))),
            Bool(v) => Ok(ControlFlow::Value(Value::Bool(*v))),
            Ident(name) => self.lookup(name, expr),

            Tuple(elements) => {
                let mut values =
                    Vec::with_capacity(
                        elements.len()
                    );

                for element in elements {
                    match self.eval(element)? {
                        ControlFlow::Value(value) => {
                            values.push(value);
                        }

                        other => {
                            return Ok(other);
                        }
                    }
                }

                Ok(
                    ControlFlow::Value(
                        Value::Tuple(
                            Rc::new(values)
                        )
                    )
                )
            }
            TupleIndex { object, index } => {
                let value =
                    self.eval_value(object)?;

                match value {
                    Value::Tuple(tuple) => {
                        tuple
                            .get(*index)
                            .cloned()
                            .map(ControlFlow::Value)
                            .ok_or_else(|| {
                                self.error(
                                    ErrorKind::Index,
                                    format!(
                                        "tuple index out of range: {}",
                                        index
                                    ),
                                    expr,
                                )
                            })
                    }

                    other => {
                        Err(
                            self.error(
                                ErrorKind::Type,
                                format!(
                                    "{} cannot be indexed by tuple index",
                                    other.type_name()
                                ),
                                expr,
                            )
                        )
                    }
                }
            }

            List(items) => self.eval_list(items, expr),
            Dict(entries) => self.eval_dict(entries, expr),

            StructDecl {
                visibility,
                name,
                fields,
                methods,
            } => self.eval_struct_decl(
                    *visibility,
                    name,
                    fields,
                    methods,
                    expr,
            ),
            EnumDecl(definition) => {
                self.eval_enum_decl(
                    definition,
                    expr,
                )
            }

            Import(parts) => self.eval_import(parts, expr),

            Let {
                visibility,
                pattern,
                value,
            } => self.eval_let(
                *visibility, 
                pattern, 
                value
            ),

            Assign(
                name, 
                value
            ) => {
                self.eval_assign(
                    name, 
                    value, 
                )
            }

            AssignIndex(obj, index, rhs) => self.eval_assign_index(obj, index, rhs, expr),
            AssignField(obj, name, rhs) => self.eval_assign_field(obj, name, rhs, expr),

            Drop(name) => {
                if self.env.remove_local(name).is_none() {
                    return Err(self.error(ErrorKind::Name, format!("{} does not exist in current scope", name), expr));
                }
                Ok(ControlFlow::Value(Value::Unit))
            }

            Binary(BinOp::And, lhs, rhs) => self.eval_and(lhs, rhs, expr),
            Binary(BinOp::Or, lhs, rhs) => self.eval_or(lhs, rhs, expr),
            Binary(op, lhs, rhs) => {
                let l = self.eval_value(lhs)?;
                let r = self.eval_value(rhs)?;

                operator::apply_binop(*op, l, r)
                    .map(ControlFlow::Value)
                    .map_err(|e| {
                        self.error(
                            ErrorKind::Runtime,
                            e,
                            expr,
                        )
                    })
            }

            Neg(e) => {
                let value = self.eval_value(e)?;

                match value {
                    Value::Series(series) => {
                        let values: Vec<Value> = series
                            .data()
                            .iter()
                            .cloned()
                            .map(|value| {
                                match value {
                                    Value::Null => Ok(Value::Null),

                                    value => value
                                        .negate()
                                        .map_err(|msg| {
                                            self.error(
                                                ErrorKind::Type,
                                                msg,
                                                expr,
                                            )
                                        }),
                                }
                            })
                            .collect::<Result<_>>()?;

                        Ok(ControlFlow::Value(
                            Value::Series(Rc::new(
                                Series::new(
                                    series.name(),
                                    values,
                                )
                            ))
                        ))
                    }

                    value => {
                        value
                            .negate()
                            .map(ControlFlow::Value)
                            .map_err(|msg| {
                                self.error(
                                    ErrorKind::Type,
                                    msg,
                                    expr,
                                )
                            })
                    }
                }
            }

            Not(e) => {
                let v = self.eval_value(e)?;

                let b: bool = self.expect(
                    v,
                    e,
                )?;

                Ok(ControlFlow::Value(
                    Value::Bool(!b)
                ))
            }

            If(cond, then_branch, else_branch) => {
                let v = self.eval_value(cond)?;

                let b: bool = self.expect(
                    v,
                    cond,
                )?;

                match b {
                    true => self.eval(then_branch),
                    false => match else_branch { 
                        Some(e) => self.eval(e), 
                        None => Ok(ControlFlow::Value(Value::Unit)) 
                    }
                }
            }

            While(cond, body) 
                => self.eval_while(cond, body, expr),
            Break => {
                if self.loop_depth == 0 { 
                    Err(
                        self.error(
                            ErrorKind::Control,
                            "break outside loop", expr
                        )
                    ) 
                } else { 
                    Ok(ControlFlow::Break) 
                }
            }
            Continue => {
                if self.loop_depth == 0 { 
                    Err(
                        self.error(
                            ErrorKind::Control,
                            "continue outside loop", expr
                        )
                    ) 
                } else { 
                    Ok(ControlFlow::Continue) 
                }
            }
            Return(value) 
                => self.eval_return(value),
            For {
                pattern,
                iterable,
                body
            } => self.eval_for(
                    pattern,
                    iterable,
                    body,
                    expr
                ),
            Match {
                value,
                arms,
            } => self.eval_match(
                    value,
                    arms,
                    expr,
                ),

            Try(inner) => self.eval_try(expr, inner),

            Range {
                start,
                end,
                inclusive,
            } => {
                self.eval_range(
                    start,
                    end,
                    *inclusive,
                    expr,
                )
            }

            Block(exprs) => self.eval_block(exprs, true),

            Lambda(params, body) => Ok(ControlFlow::Value(Value::Func(Rc::new(Function {
                name: None,
                params: params.clone(),
                body: Rc::new((**body).clone()),
                closure: self.env.clone(),
            })))),

            Call(callee, args) => self.eval_call(callee, args, expr),

            Field(obj, name) => self.eval_field(obj, name, expr),

            Index(obj, index) => self.eval_index(obj, index, expr),

            Null => Ok(ControlFlow::Value(Value::Null)),
            Unit => Ok(ControlFlow::Value(Value::Unit)),
        }
    }

    fn eval_value(&mut self, expr: &Expr) -> Result<Value> {
        match self.eval(expr)? {
            ControlFlow::Value(v) => Ok(v),
            ControlFlow::Return(_) =>
                Err(
                    self.error(
                        ErrorKind::Control,
                        "return cannot be used where a value is required",
                        expr,
                    )
                ),
            ControlFlow::Break =>
                Err(
                    self.error(
                        ErrorKind::Control,
                        "break cannot be used where a value is required",
                        expr,
                    )
                ),
            ControlFlow::Continue =>
                Err(
                    self.error(
                        ErrorKind::Control,
                        "continue cannot be used where a value is required",
                        expr,
                    )
                )
        }
    }

    fn lookup(&self, name: &str, expr: &Expr) -> Result<ControlFlow> {
        self.env.get(name)
            .map(ControlFlow::Value)
            .ok_or_else(|| self.error(ErrorKind::Name, format!("{} is undefined", name), expr))
    }

    fn eval_list(
        &mut self,
        items: &[ListItem],
        expr: &Expr,
    ) -> Result<ControlFlow> {
        let mut values =
            Vec::new();

        for item in items {
            match item {
                ListItem::Expr(item) => {
                    values.push(
                        self.eval_value(item)?
                    );
                }

                ListItem::Range(range) => {
                    let iterator =
                        self.eval_iterable(
                            range,
                            expr,
                        )?;

                    while let Some(value) =
                        self.next_from_iterator(
                            &iterator,
                            expr,
                        )?
                    {
                        values.push(value);
                    }
                }
            }
        }

        Ok(
            ControlFlow::Value(
                Value::List(
                    Rc::new(
                        RefCell::new(values)
                    )
                )
            )
        )
    }

    fn eval_dict(
        &mut self,
        entries: &[(String, Expr)],
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let mut map = std::collections::HashMap::new();

        for (key, value_expr) in entries {
            let value = self.eval_value(value_expr)?;

            if map.contains_key(key) {
                return Err(self.error(
                    ErrorKind::Runtime,
                    format!("duplicate dictionary key: {}", key),
                    whole,
                ));
            }

            map.insert(key.clone(), value);
        }

        Ok(ControlFlow::Value(
            Value::Dict(Rc::new(RefCell::new(map)))
        ))
    }

    fn eval_struct_decl(
        &mut self,
        visibility: Visibility,
        name: &str,
        fields: &[String],
        methods: &[(String, Box<Expr>)],
        whole: &Expr,
    ) -> Result<ControlFlow> {
        if self.env.contains_local(name) {
            return Err(self.error(
                ErrorKind::Name,
                format!(
                    "struct '{}' is already defined in this scope",
                    name
                ),
                whole,
            ));
        }

                let mut method_map = std::collections::HashMap::new();

        for (method_name, method_expr) in methods {
            let value = self.eval_value(method_expr)?;

            let function = match value {
                Value::Func(function) => function,

                _ => {
                    return Err(self.error(
                        ErrorKind::Type,
                        format!(
                            "struct method '{}' must be a function",
                            method_name
                        ),
                        method_expr,
                    ));
                }
            };

            method_map.insert(
                method_name.clone(),
                function,
            );
        }

        let definition =
            StructDefinition::new(
                name,
                fields.to_vec(),
                method_map,
            );

        let value =
            Value::Struct(
                Rc::new(
                    definition
                )
            );

        self.env
            .declare(
                name.to_owned(),
                value,
            )
            .map_err(|message| {
                self.error(
                    ErrorKind::Name,
                    message,
                    whole,
                )
            })?;

        if visibility == Visibility::Public {
            if let Some(context) =
                self.module_stack.last_mut()
            {
                context.export(
                    name.to_owned()
                );
            } else {
                return Err(
                    self.error(
                        ErrorKind::Name,
                        "'pub struct' is only allowed at module scope",
                        whole,
                    )
                );
            }
        }

        Ok(
            ControlFlow::Value(
                Value::Unit
            )
        )
    }

    fn eval_enum_decl(
        &mut self,
        definition: &AstEnumDef,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let mut enum_def =
            RuntimeEnumDef::new(
                definition.name.clone()
            );

        for variant
            in &definition.variants
        {
            enum_def
                .add_variant(
                    variant.name.clone(),
                    variant.fields.len(),
                )
                .map_err(|message| {
                    self.error(
                        ErrorKind::Name,
                        message,
                        whole,
                    )
                })?;
        }

        let enum_ref =
            Rc::new(enum_def);

        self.env.define(
            definition.name.clone(),
            Value::Enum(
                enum_ref
            ),
        );

        Ok(
            ControlFlow::Value(
                Value::Unit
            )
        )
    }

    fn bind_module_path(
        &mut self,
        path: &ModulePath,
        module: ModuleRef,
        whole: &Expr,
    ) -> Result<()> {
        let parts =
            path.parts();

        if parts.is_empty() {
            return Err(
                self.error(
                    ErrorKind::Import,
                    "empty module path",
                    whole,
                )
            );
        }

        // Single component:
        //
        // import math
        //
        // env["math"] = module
        if parts.len() == 1 {
            let name =
                parts[0].clone();

            if self.env.contains_local(&name) {
                return Err(
                    self.error(
                        ErrorKind::Name,
                        format!(
                            "name '{}' is already defined in this scope",
                            name
                        ),
                        whole,
                    )
                );
            }

            self.env.define(
                name,
                Value::Module(module),
            );

            return Ok(());
        }

        // ---------------------------------------------------------
        // Build nested namespace:
        //
        // tests
        //   └── modules
        //         └── visibility
        //
        // Intermediate namespaces are always public/exported.
        // ---------------------------------------------------------

        let root_name =
            parts[0].clone();

        let root_module =
            if let Some(Value::Module(existing)) =
                self.env.get(&root_name)
            {
                existing
            } else {
                let namespace =
                    Rc::new(
                        RefCell::new(
                            Module::new(
                                root_name.clone()
                            )
                        )
                    );

                self.env.define(
                    root_name.clone(),
                    Value::Module(
                        namespace.clone()
                    )
                );

                namespace
            };

        let mut current =
            root_module;

        for component in &parts[1..parts.len() - 1] {
            let child =
                {
                    let current_ref =
                        current.borrow();

                    current_ref
                        .get_internal(component)
                };

            let child =
                match child {
                    Some(Value::Module(module)) =>
                        module,

                    Some(_) => {
                        return Err(
                            self.error(
                                ErrorKind::Name,
                                format!(
                                    "'{}' in module path is not a module",
                                    component
                                ),
                                whole,
                            )
                        );
                    }

                    None => {
                        let module =
                            Rc::new(
                                RefCell::new(
                                    Module::new(
                                        component.clone()
                                    )
                                )
                            );

                        {
                            let mut current_ref =
                                current.borrow_mut();

                            // IMPORTANT:
                            // intermediate namespace nodes are public
                            current_ref.set_exported(
                                component.clone(),
                                Value::Module(
                                    module.clone()
                                )
                            );
                        }

                        module
                    }
                };

            current =
                child;
        }

        // ---------------------------------------------------------
        // Attach final module.
        // ---------------------------------------------------------

        let leaf_name =
            parts.last()
                .expect("non-empty module path")
                .clone();

        {
            let mut current_ref =
                current.borrow_mut();

            current_ref.set_exported(
                leaf_name,
                Value::Module(
                    module
                )
            );
        }

        Ok(())
    }

    fn eval_import(
        &mut self,
        parts: &[String],
        whole: &Expr,
    ) -> Result<ControlFlow> {
        // =========================================================
        // 1. Validate import path
        // =========================================================

        if parts.is_empty() {
            return Err(
                self.error(
                    ErrorKind::Name,
                    "empty module name",
                    whole,
                )
            );
        }

        let requested =
            ModulePath::new(
                parts.to_vec()
            );

        // =========================================================
        // 2. Lazy standard-library module
        //
        // Only module bodies are lazy.
        // Builtin functions were already installed in Interpreter::new().
        // =========================================================

        if parts.len() == 1 {
            let name =
                &parts[0];

            if let Some(module) =
                crate::stdlib::load_module(name)
            {
                if self.env.contains_local(name) {
                    return Err(
                        self.error(
                            ErrorKind::Name,
                            format!(
                                "name '{}' is already defined in this scope",
                                name
                            ),
                            whole,
                        )
                    );
                }

                self.env.define(
                    name.clone(),
                    Value::Module(module),
                );

                return Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                );
            }
        }

        // =========================================================
        // 3. Resolve physical file
        // =========================================================

        let canonical =
            self.module_loader
                .resolve(
                    &requested
                )
                .map_err(|mut error| {
                    if error.span.is_none() { 
                        error.span = Some(whole.span); 
                    }
                    if error.stack.is_empty() { 
                        error.stack = self.stack.clone(); 
                    }

                    error
                })?;

        // =========================================================
        // 4. Cache lookup
        //
        // If this file was already successfully evaluated,
        // do not execute it again.
        // =========================================================

        if let Some(module) =
            self.module_loader
                .get_cached(&canonical)
        {
            self.bind_module_path(
                &requested,
                module,
                whole,
            )?;

            return Ok(
                ControlFlow::Value(
                    Value::Unit
                )
            );
        }

        // =========================================================
        // 5. Cyclic import detection
        // =========================================================

        if self.module_stack
            .iter()
            .any(|context| {
                context.file_path == canonical
            })
        {
            let mut chain =
                self.module_stack
                    .iter()
                    .map(ModuleContext::name)
                    .collect::<Vec<_>>();

            chain.push(
                requested.name()
            );

            return Err(
                self.error(
                    ErrorKind::Runtime,
                    format!(
                        "cyclic module import: {}",
                        chain.join(" -> ")
                    ),
                    whole,
                )
            );
        }

        // =========================================================
        // 6. Load + parse
        // =========================================================

        let program =
            self.module_loader
                .load_program(
                    &canonical
                )
                .map_err(|mut error| {
                    if error.span.is_none() { 
                        error.span = Some(whole.span); 
                    }
                    if error.stack.is_empty() { 
                        error.stack = self.stack.clone(); 
                    }

                    error
                })?;

        // =========================================================
        // 7. Create module environment
        // =========================================================

        let module_env =
            self.env.child();

        let previous_env =
            std::mem::replace(
                &mut self.env,
                module_env,
            );

        // =========================================================
        // 8. Push module context
        // =========================================================

        self.module_stack.push(
            ModuleContext::new(
                requested.clone(),
                canonical.clone(),
            )
        );

        // =========================================================
        // 9. Evaluate module
        // =========================================================

        let result =
            self.eval_program(
                &program
            );

        // Always restore module stack.
        let context =
            self.module_stack
                .pop()
                .ok_or_else(|| {
                    self.error(
                        ErrorKind::Runtime,
                        "module stack underflow",
                        whole,
                    )
                })?;

        // Always restore previous environment.
        let module_env =
            std::mem::replace(
                &mut self.env,
                previous_env,
            );

        // Module execution must complete normally.
        match result? {
            ControlFlow::Value(_) => {}

            ControlFlow::Return(_) => {
                return Err(
                    self.error(
                        ErrorKind::Control,
                        "return cannot escape module scope",
                        whole,
                    )
                );
            }

            ControlFlow::Break => {
                return Err(
                    self.error(
                        ErrorKind::Control,
                        "break cannot escape module scope",
                        whole,
                    )
                );
            }

            ControlFlow::Continue => {
                return Err(
                    self.error(
                        ErrorKind::Control,
                        "continue cannot escape module scope",
                        whole,
                    )
                );
            }
        }

        // =========================================================
        // 10. Build runtime Module
        // =========================================================

        let mut module =
            Module::new(
                requested.name()
            );

        for (name, value)
            in module_env.local_values()
        {
            module.set(
                name.clone(),
                value,
            );

            if context.is_exported(
                &name
            ) {
                module.export(name);
            }
        }

        let module =
            Rc::new(
                RefCell::new(
                    module
                )
            );

        // =========================================================
        // 11. Cache fully evaluated module
        // =========================================================

        self.module_loader
            .cache(
                canonical,
                module.clone(),
            );

        // =========================================================
        // 12. Attach nested namespace
        // =========================================================

        self.bind_module_path(
            &requested,
            module,
            whole,
        )?;

        Ok(
            ControlFlow::Value(
                Value::Unit
            )
        )
    }

    fn eval_let(
        &mut self,
        visibility: Visibility,
        pattern: &Pattern,
        value_expr: &Expr,
    ) -> Result<ControlFlow> {
        let value =
            match self.eval(value_expr)? {
                ControlFlow::Value(value) =>
                    value,

                other =>
                    return Ok(other),
            };

        let mut bindings =
            HashMap::new();

        let matched =
            match_pattern(
                pattern,
                &value,
                &mut bindings,
            )
            .map_err(|message| {
                self.error(
                    ErrorKind::Runtime,
                    message,
                    value_expr,
                )
            })?;

        if !matched {
            return Err(
                self.error(
                    ErrorKind::Runtime,
                    "let pattern does not match value",
                    value_expr,
                )
            );
        }

        // ---------------------------------------------------------
        // For module-level public declarations, record exports.
        //
        // Local/block-scoped `pub let` is rejected because `pub`
        // only makes sense at module scope.
        // ---------------------------------------------------------

        if visibility == Visibility::Public {
            if self.module_stack.is_empty() {
                return Err(
                    self.error(
                        ErrorKind::Name,
                        "'pub' declaration is only allowed at module scope",
                        value_expr,
                    )
                );
            }

            for name in bindings.keys() {
                if name == "_" {
                    return Err(
                        self.error(
                            ErrorKind::Name,
                            "cannot export wildcard binding",
                            value_expr,
                        )
                    );
                }

                if let Some(context) =
                    self.module_stack.last_mut()
                {
                    context.export(
                        name.clone()
                    );
                }
            }
        }

        self.env
            .declare_all(bindings)
            .map_err(|message| {
                self.error(
                    ErrorKind::Name,
                    message,
                    value_expr,
                )
            })?;

        Ok(
            ControlFlow::Value(
                Value::Unit
            )
        )
    }

    /// Helper for `eval_let()` and `eval_for()`
    fn match_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        value: &Value,
        whole: &Expr,
    ) -> Result<HashMap<String, Value>> {
        let mut bindings =
            HashMap::new();

        let matched =
            match_pattern(
                pattern,
                value,
                &mut bindings,
            )
            .map_err(|message| {
                self.error(
                    ErrorKind::Runtime,
                    message,
                    whole,
                )
            })?;

        if !matched {
            return Err(
                self.error(
                    ErrorKind::Runtime,
                    "pattern does not match value",
                    whole,
                )
            );
        }

        Ok(bindings)
    }

    fn eval_assign(
        &mut self,
        name: &str,
        value_expr: &Expr,
    ) -> Result<ControlFlow> {
        let value =
            self.eval_value(value_expr)?;

        self.env.assign_or_define(
            name,
            value,
        );

        Ok(
            ControlFlow::Value(
                Value::Unit
            )
        )
    }

    fn eval_assign_index(
        &mut self,
        obj: &Expr,
        index: &IndexExpr,
        rhs: &Expr,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let target = self.eval_value(obj)?;

        match (target, index) {
            // =========================================================
            // List[index] = value
            // =========================================================
            (
                Value::List(list),
                IndexExpr::Single(index),
            ) => {
                let idx =
                    self.eval_index_int(
                        index,
                        whole,
                    )?;

                let value =
                    self.eval_value(rhs)?;

                let mut list =
                    list.borrow_mut();

                if idx >= list.len() {
                    return Err(self.error(
                        ErrorKind::Index,
                        format!(
                            "index out of range: {}",
                            idx
                        ),
                        whole,
                    ));
                }

                list[idx] = value.clone();

                Ok(ControlFlow::Value(value))
            }

            // =========================================================
            // List[start..end] = value
            //
            // unsupported yet
            // =========================================================
            (
                Value::List(_),
                IndexExpr::Range { .. },
            ) => {
                Err(self.error(
                    ErrorKind::Runtime,
                    "list slice assignment is not implemented yet",
                    whole,
                ))
            }

            // =========================================================
            // Dict["key"] = value
            // =========================================================
            (
                Value::Dict(dict),
                IndexExpr::Single(index),
            ) => {
                let v = self.eval_value(index)?;

                let key: String = self.expect::<StrRef>(
                    v,
                    whole,
                )?.as_ref().clone();

                let value =
                    self.eval_value(rhs)?;

                dict.borrow_mut().insert(
                    key,
                    value.clone(),
                );

                Ok(ControlFlow::Value(value))
            }

            // =========================================================
            // Dict[...] = ...
            //
            // tuple indexing is not supported
            // =========================================================
            (
                Value::Dict(_),
                IndexExpr::Tuple(_),
            ) => {
                Err(self.error(
                    ErrorKind::Index,
                    "tuple indexing is not supported for Dict",
                    whole,
                ))
            }

            // =========================================================
            // String[index] = ...
            //
            // String is immutable
            // =========================================================
            (
                Value::Str(_),
                IndexExpr::Single(_),
            ) => {
                Err(self.error(
                    ErrorKind::Runtime,
                    "String values are immutable",
                    whole,
                ))
            }

            // =========================================================
            // Matrix[row, col] = value
            // =========================================================
            (
                Value::Matrix(matrix),
                IndexExpr::Tuple(indices),
            ) => {
                if indices.len() != 2 {
                    return Err(self.error(
                        ErrorKind::Index,
                        format!(
                            "Matrix assignment expects exactly 2 indices, got {}",
                            indices.len()
                        ),
                        whole,
                    ));
                }

                let row =
                    self.eval_matrix_single_index(
                        &indices[0],
                        "row",
                        whole,
                    )?;

                let col =
                    self.eval_matrix_single_index(
                        &indices[1],
                        "column",
                        whole,
                    )?;

                let value =
                    self.eval_value(rhs)?;

                let numeric = 
                    self.expect_number(value, whole)?;

                matrix
                    .borrow_mut()
                    .set(
                        row,
                        col,
                        numeric,
                    )
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Runtime,
                            message,
                            whole,
                        )
                    })?;

                Ok(ControlFlow::Value(
                    Value::Float(numeric)
                ))
            }

            // =========================================================
            // Matrix[index] = ...
            //
            // Single index is intentionally unsupported.
            // =========================================================
            (
                Value::Matrix(_),
                IndexExpr::Single(_),
            ) => {
                Err(self.error(
                    ErrorKind::Index,
                    "Matrix assignment requires two indices: Matrix[row, col]",
                    whole,
                ))
            }

            // =========================================================
            // Matrix[slice] = ...
            //
            // Slice assignment is intentionally deferred.
            // =========================================================
            (
                Value::Matrix(_),
                IndexExpr::Range { .. },
            ) => {
                Err(self.error(
                    ErrorKind::Index,
                    "Matrix slice assignment is not implemented yet",
                    whole,
                ))
            }

            // =========================================================
            // Matrix tuple with >2 dimensions
            // =========================================================
            (
                Value::Matrix(_),
                IndexExpr::Tuple(_),
            ) => {
                unreachable!(
                    "Matrix tuple assignment should be handled above"
                )
            }

            // =========================================================
            // Unsupported indexing target
            // =========================================================
            (
                other,
                _,
            ) => {
                Err(self.error(
                    ErrorKind::Type,
                    format!(
                        "invalid indexed assignment on {}",
                        other.type_name()
                    ),
                    whole,
                ))
            }
        }
    }

    fn eval_assign_field(
        &mut self,
        obj: &Expr,
        name: &str,
        rhs: &Expr,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let target = self.eval_value(obj)?;
        let value = self.eval_value(rhs)?;

        match target {
            Value::Object(object) => {
                let mut object = object.borrow_mut();

                if object.get_method(name).is_some() {
                    return Err(self.error(
                        ErrorKind::Runtime,
                        format!(
                            "cannot assign to object method '{}'",
                            name
                        ),
                        whole,
                    ));
                }

                object.set_field(
                    name.to_owned(),
                    value.clone(),
                );

                Ok(ControlFlow::Value(value))
            }

            // Deny module modification
            Value::Module(module) => {
                Err(self.error(
                    ErrorKind::Runtime,
                    format!(
                        "cannot modify module '{}'",
                        module.borrow().name()
                    ),
                    whole,
                ))
            }

            other => Err(self.error(
                ErrorKind::Type,
                format!(
                    "cannot assign field '{}' on {}",
                    name,
                    other.type_name()
                ),
                whole,
            )),
        }
    }

    /// Helper for `eval_index()`
    fn eval_matrix_single_index(
        &mut self,
        index: &IndexExpr,
        axis: &str,
        whole: &Expr,
    ) -> Result<usize> {
        match index {
            IndexExpr::Single(expr) => {
                let v = self.eval_value(expr)?;
                
                let v: i64 = self.expect(
                    v,
                    whole,
                )?;

                if v >= 0 { Ok(v as usize) }
                else {
                    Err(self.error(
                        ErrorKind::Index,
                        format!(
                            "negative Matrix {} index",
                            axis
                        ),
                        whole,
                    ))
                }
            }

            IndexExpr::Range { .. } => {
                Err(self.error(
                    ErrorKind::Index,
                    format!(
                        "Matrix {} slicing is not implemented yet",
                        axis
                    ),
                    whole,
                ))
            }

            IndexExpr::Tuple(_) => {
                Err(self.error(
                    ErrorKind::Index,
                    format!(
                        "nested Matrix {} index is not supported",
                        axis
                    ),
                    whole,
                ))
            }
        }
    }

    fn eval_index(
        &mut self,
        obj: &Expr,
        index: &IndexExpr,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let value = self.eval_value(obj)?;

        match (value, index) {
            // =========================================================
            // List[index]
            // =========================================================
            (
                Value::List(list),
                IndexExpr::Single(index),
            ) => {
                let idx =
                    self.eval_index_int(
                        index,
                        whole,
                    )?;

                let list_ref =
                    list.borrow();

                list_ref
                    .get(idx)
                    .cloned()
                    .map(ControlFlow::Value)
                    .ok_or_else(|| {
                        self.error(
                            ErrorKind::Index,
                            format!(
                                "index out of range: {}",
                                idx
                            ),
                            whole,
                        )
                    })
            }

            // =========================================================
            // List[start..end]
            // =========================================================
            (
                Value::List(list),
                IndexExpr::Range {
                    start,
                    end,
                    inclusive,
                },
            ) => {
                let len =
                    list.borrow().len();

                let (s, e) =
                    self.resolve_slice(
                        start.as_deref(),
                        end.as_deref(),
                        *inclusive,
                        len,
                        whole,
                    )?;

                let out =
                    list.borrow()[s..e]
                        .to_vec();

                Ok(ControlFlow::Value(
                    Value::List(
                        Rc::new(
                            RefCell::new(out)
                        )
                    )
                ))
            }

            // =========================================================
            // String[index]
            // =========================================================
            (
                Value::Str(s),
                IndexExpr::Single(index),
            ) => {
                let idx =
                    self.eval_index_int(
                        index,
                        whole,
                    )?;

                s.chars()
                    .nth(idx)
                    .map(|c| {
                        ControlFlow::Value(
                            Value::Str(
                                Rc::new(
                                    c.to_string()
                                )
                            )
                        )
                    })
                    .ok_or_else(|| {
                        self.error(
                            ErrorKind::Index,
                            format!(
                                "index out of range: {}",
                                idx
                            ),
                            whole,
                        )
                    })
            }

            // =========================================================
            // String[start..end]
            // =========================================================
            (
                Value::Str(s),
                IndexExpr::Range {
                    start,
                    end,
                    inclusive,
                },
            ) => {
                let chars =
                    s.chars()
                        .collect::<Vec<_>>();

                let (start_idx, end_idx) =
                    self.resolve_slice(
                        start.as_deref(),
                        end.as_deref(),
                        *inclusive,
                        chars.len(),
                        whole,
                    )?;

                let out =
                    chars[start_idx..end_idx]
                        .iter()
                        .collect::<String>();

                Ok(ControlFlow::Value(
                    Value::Str(
                        Rc::new(out)
                    )
                ))
            }

            // =========================================================
            // Dict["key"]
            // =========================================================
            (
                Value::Dict(dict),
                IndexExpr::Single(index),
            ) => {
                let v = self.eval_value(index)?;

                let key = self.expect::<StrRef>(
                    v,
                    whole,
                )?.as_ref().clone();

                dict.borrow()
                    .get(&key)
                    .cloned()
                    .map(ControlFlow::Value)
                    .ok_or_else(|| {
                        self.error(
                            ErrorKind::Index,
                            format!(
                                "dictionary key not found: {:?}",
                                key
                            ),
                            whole,
                        )
                    })
            }

            // =========================================================
            // Matrix[row, col]
            // =========================================================
            (
                Value::Matrix(matrix),
                IndexExpr::Tuple(indices),
            ) => {
                if indices.len() != 2 {
                    return Err(self.error(
                        ErrorKind::Index,
                        format!(
                            "Matrix indexing expects exactly 2 indices, got {}",
                            indices.len()
                        ),
                        whole,
                    ));
                }

                let row_is_single =
                    matches!(
                        &indices[0],
                        IndexExpr::Single(_)
                    );

                let col_is_single =
                    matches!(
                        &indices[1],
                        IndexExpr::Single(_)
                    );

                let matrix_ref =
                    matrix.borrow();

                let (row_start, row_end) =
                    self.resolve_matrix_axis(
                        &indices[0],
                        matrix_ref.rows(),
                        "row",
                        whole,
                    )?;

                let (col_start, col_end) =
                    self.resolve_matrix_axis(
                        &indices[1],
                        matrix_ref.cols(),
                        "column",
                        whole,
                    )?;

                // -----------------------------------------
                // A[i, j] -> scalar
                // -----------------------------------------
                if row_is_single && col_is_single {
                    let value =
                        matrix_ref
                            .get(
                                row_start,
                                col_start,
                            )
                            .ok_or_else(|| {
                                self.error(
                                    ErrorKind::Index,
                                    format!(
                                        "Matrix index out of bounds: ({}, {})",
                                        row_start,
                                        col_start
                                    ),
                                    whole,
                                )
                            })?;

                    return Ok(
                        ControlFlow::Value(
                            Value::Float(value)
                        )
                    );
                }

                // -----------------------------------------
                // A[slice, slice] -> Matrix
                // -----------------------------------------
                let result =
                    matrix_ref
                        .slice(
                            row_start,
                            row_end,
                            col_start,
                            col_end,
                        )
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(ControlFlow::Value(
                    Value::Matrix(
                        Rc::new(
                            RefCell::new(result)
                        )
                    )
                ))
            }

            // =========================================================
            // Matrix[single]
            // =========================================================
            (
                Value::Matrix(_),
                IndexExpr::Single(_),
            ) => {
                Err(self.error(
                    ErrorKind::Index,
                    "Matrix indexing requires two indices: Matrix[row, col]",
                    whole,
                ))
            }

            // =========================================================
            // Matrix slicing
            //
            // =========================================================
            (
                Value::Matrix(_),
                IndexExpr::Range { .. },
            ) => {
                Err(self.error(
                    ErrorKind::Index,
                    "Matrix slicing requires tuple indexing; slicing is not implemented yet",
                    whole,
                ))
            }

            // (
            //     Value::Matrix(_),
            //     IndexExpr::Tuple(_),
            // ) => {
            //     unreachable!(
            //         "Matrix tuple indexing should have been handled above"
            //     )
            // }

            // =========================================================
            // Tuple indexing on unsupported values
            // =========================================================
            (
                Value::List(_),
                IndexExpr::Tuple(_),
            ) => {
                Err(self.error(
                    ErrorKind::Index,
                    "tuple indexing is currently supported only for Matrix",
                    whole,
                ))
            }

            (
                Value::Str(_),
                IndexExpr::Tuple(_),
            ) => {
                Err(self.error(
                    ErrorKind::Index,
                    "tuple indexing is not supported for Str",
                    whole,
                ))
            }

            (
                Value::Dict(_),
                IndexExpr::Tuple(_),
            ) => {
                Err(self.error(
                    ErrorKind::Index,
                    "tuple indexing is not supported for Dict",
                    whole,
                ))
            }

            // =========================================================
            // Everything else
            // =========================================================
            (
                other,
                _,
            ) => {
                Err(self.error(
                    ErrorKind::Type,
                    format!(
                        "invalid indexing on {}",
                        other.type_name()
                    ),
                    whole,
                ))
            }
        }
    }

    fn eval_index_int(&mut self, expr: &Expr, whole: &Expr) -> Result<usize> {
        let v = self.eval_value(expr)?;
        
        let v: i64 = self.expect(
            v,
            whole,
        )?;

        if v >= 0 { Ok(v as usize) }
        else {
            Err(self.error(
                ErrorKind::Index, 
                "negative index is not supported", 
                whole,
            ))
        }
    }

    fn resolve_matrix_range(
        &mut self,
        start: Option<&Expr>,
        end: Option<&Expr>,
        inclusive: bool,
        len: usize,
        axis: &str,
        whole: &Expr,
    ) -> Result<(usize, usize)> {
        let start_value =
            match start {
                Some(expr) => {
                    let v = self.eval_value(expr)?;

                    let v: i64 = self.expect(
                        v,
                        whole,
                    )?;

                    if v >= 0 { v as usize }
                    else {
                        return Err(
                            self.error(
                                ErrorKind::Index,
                                format!(
                                    "negative Matrix {} slice start",
                                    axis
                                ),
                                whole,
                            )
                        );
                    }
                }

                None => 0,
            };

        let end_value =
            match end {
                Some(expr) => {
                    let v = self.eval_value(expr)?;

                    let v: i64 = self.expect(
                        v,
                        whole,
                    )?;

                    if v >= 0 {
                        if inclusive {
                            (v as usize).checked_add(1)
                                .ok_or_else(|| {
                                    self.error(
                                        ErrorKind::Overflow,
                                        format!(
                                            "inclusive Matrix {} slice endpoint overflow",
                                            axis
                                        ),
                                        whole,
                                    )
                                })?
                        } else {
                            v as usize
                        }
                    } else {
                        return Err(
                            self.error(
                                ErrorKind::Index,
                                format!(
                                    "negative Matrix {} slice end",
                                    axis
                                ),
                                whole,
                            )
                        );
                    }
                }

                None => len,
            };

        if start_value > end_value {
            return Err(
                self.error(
                    ErrorKind::Index,
                    format!(
                        "invalid Matrix {} slice: {}..{}",
                        axis,
                        start_value,
                        end_value
                    ),
                    whole,
                )
            );
        }

        if end_value > len {
            return Err(
                self.error(
                    ErrorKind::Index,
                    format!(
                        "Matrix {} slice end {} exceeds dimension {}",
                        axis,
                        end_value,
                        len
                    ),
                    whole,
                )
            );
        }

        if start_value == end_value {
            return Err(
                self.error(
                    ErrorKind::Index,
                    format!(
                        "Matrix {} slice must not be empty",
                        axis
                    ),
                    whole,
                )
            );
        }

        Ok((start_value, end_value))
    }

    /// Helper for `resolve_slice()`
    fn resolve_matrix_axis(
        &mut self,
        index: &IndexExpr,
        len: usize,
        axis: &str,
        whole: &Expr,
    ) -> Result<(usize, usize)> {
        match index {
            IndexExpr::Single(expr) => {
                let value = self.eval_value(expr)?;

                let value: i64 = self.expect::<i64>(
                    value,
                    whole,
                )?;

                let value = if value >= 0 { value as usize }
                else {
                    return Err(
                        self.error(
                            ErrorKind::Index,
                            format!(
                                "negative Matrix {} index",
                                axis
                            ),
                            whole,
                        )
                    );
                };

                if value >= len {
                    return Err(
                        self.error(
                            ErrorKind::Index,
                            format!(
                                "Matrix {} index out of bounds: {}",
                                axis,
                                value
                            ),
                            whole,
                        )
                    );
                }

                // Single index → 1 element slice
                Ok((value, value + 1))
            }

            IndexExpr::Range {
                start,
                end,
                inclusive,
            } => {
                self.resolve_matrix_range(
                    start.as_deref(),
                    end.as_deref(),
                    *inclusive,
                    len,
                    axis,
                    whole,
                )
            }

            IndexExpr::Tuple(_) => {
                Err(self.error(
                    ErrorKind::Index,
                    format!(
                        "nested Matrix {} index is not supported",
                        axis
                    ),
                    whole,
                ))
            }
        }
    }

    fn resolve_slice(
        &mut self,
        start: Option<&Expr>,
        end: Option<&Expr>,
        inclusive: bool,
        len: usize,
        whole: &Expr
    ) -> Result<(usize,usize)> {
        let s = match start { 
            Some(e) => self.eval_index_int(e, whole)?, 
            None => 0 
        };

        let mut e = match end { 
            Some(e) => self.eval_index_int(e, whole)?, 
            None => len 
        };

        if inclusive {
            e = e.checked_add(1).ok_or_else(|| self.error(ErrorKind::Overflow, "slice endpoint overflow", whole))?;
        }

        if s > e || e > len { 
            return Err(self.error(ErrorKind::Index, "invalid slice range", whole)); 
        }

        Ok((s,e))
    }

    fn eval_iterable(
        &mut self,
        index: &IndexExpr,
        whole: &Expr,
    ) -> Result<IteratorRef> {
        match index {
            // =====================================================
            // Single iterable expression
            // =====================================================
            IndexExpr::Single(expr) => {
                let value =
                    self.eval_value(expr)?;

                self.make_iterator(
                    value,
                    whole,
                )
            }

            // =====================================================
            // Range
            // =====================================================
            IndexExpr::Range {
                start,
                end,
                inclusive,
            } => {
                let start =
                    match start {
                        Some(expr) => {
                            let v = self.eval_value(expr)?;

                            let v: i64 = self.expect(
                                v,
                                whole,
                            )?;

                            if v >= 0 { v }
                            else {
                                return Err(
                                    self.error(
                                        ErrorKind::Index,
                                        "negative range start",
                                        whole,
                                    )
                                );
                            }
                        }

                        None => 0,
                    };

                let mut end =
                    match end {
                        Some(expr) => {
                            let v = self.eval_value(expr)?;

                            self.expect::<i64>(
                                v,
                                whole,
                            )?
                        }

                        None => i64::MAX,
                    };

                if *inclusive {
                    end =
                        end.checked_add(1)
                            .ok_or_else(|| {
                                self.error(
                                    ErrorKind::Overflow,
                                    "inclusive range endpoint overflow",
                                    whole,
                                )
                            })?;
                }

                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Range {
                                current: start,
                                end,
                            }
                        )
                    )
                )
            }

            // =====================================================
            // Tuple index expression is not an iterable itself
            // =====================================================
            IndexExpr::Tuple(_) => {
                Err(
                    self.error(
                        ErrorKind::Type,
                        "tuple index is not iterable",
                        whole,
                    )
                )
            }
        }
    }

    /// Helper for `eval_iterable()`
    fn make_iterator(
        &mut self,
        value: Value,
        whole: &Expr,
    ) -> Result<IteratorRef> {
        match value {
            Value::Iterator(iterator) => {
                Ok(iterator)
            }

            Value::List(data) => {
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::List {
                                data,
                                index: 0,
                            }
                        )
                    )
                )
            }

            Value::Str(string) => {
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Str {
                                data: Rc::new(
                                    string
                                        .chars()
                                        .collect()
                                ),
                                index: 0,
                            }
                        )
                    )
                )
            }

            Value::Range(
                start,
                end,
                inclusive,
            ) => {
                let end =
                    if inclusive {
                        end.checked_add(1)
                            .ok_or_else(|| {
                                self.error(
                                    ErrorKind::Overflow,
                                    "inclusive range endpoint overflow",
                                    whole,
                                )
                            })?
                    } else {
                        end
                    };

                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Range {
                                current: start,
                                end,
                            }
                        )
                    )
                )
            }

            other => {
                Err(
                    self.error(
                        ErrorKind::Type,
                        format!(
                            "{} is not iterable",
                            other.type_name()
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn eval_while(
        &mut self,
        cond: &Expr,
        body: &Expr,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        self.loop_depth += 1;

        let result = (|| {
            let mut last =
                Value::Unit;

            loop {
                // -------------------------------------------------
                // Evaluate condition.
                // -------------------------------------------------

                let condition =
                    self.eval_value(cond)?;

                let b: bool = self.expect(
                    condition,
                    whole,
                )?;

                match b {
                    false => break,
                    true => (),
                }

                // -------------------------------------------------
                // Execute body.
                // -------------------------------------------------

                match self.eval(body)? {
                    ControlFlow::Value(value) => {
                        last = value;
                    }

                    ControlFlow::Break => {
                        break;
                    }

                    ControlFlow::Continue => {
                        continue;
                    }

                    ControlFlow::Return(value) => {
                        return Ok(
                            ControlFlow::Return(value)
                        );
                    }
                }
            }

            Ok(
                ControlFlow::Value(last)
            )
        })();

        self.loop_depth -= 1;

        result
    }

    fn eval_for(
        &mut self,
        pattern: &Pattern,
        iterable: &Expr,
        body: &Expr,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let value =
            self.eval_value(iterable)?;

        let iterator =
            self.make_iterator(
                value,
                whole,
            )?;

        let old_env =
            self.env.clone();

        self.env =
            self.env.child();

        self.loop_depth += 1;

        let result = (|| {
            let mut names =
                Vec::new();

            collect_pattern_names(
                pattern,
                &mut names,
            );

            for name in names {
                self.env
                    .declare(
                        name,
                        Value::Unit,
                    )
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Name,
                            message,
                            whole,
                        )
                    })?;
            }

            let mut last =
                Value::Unit;

            while let Some(value) =
                self.next_from_iterator(
                    &iterator,
                    whole,
                )?
            {
                let bindings =
                    self.match_pattern_bindings(
                        pattern,
                        &value,
                        whole,
                    )?;

                for (name, value)
                    in bindings
                {
                    self.env.assign_local(
                        &name,
                        value,
                    );
                }

                match self.eval(body)? {
                    ControlFlow::Value(value) =>
                        last = value,

                    ControlFlow::Break =>
                        break,

                    ControlFlow::Continue =>
                        continue,

                    ControlFlow::Return(value) =>
                        return Ok(
                            ControlFlow::Return(value)
                        ),
                }
            }

            Ok(
                ControlFlow::Value(last)
            )
        })();

        self.loop_depth -= 1;
        self.env = old_env;

        result
    }

    fn eval_return(
        &mut self,
        value: &Option<Box<Expr>>,
    ) -> Result<ControlFlow> {
        match value {
            Some(value) => {
                match self.eval(value)? {
                    ControlFlow::Value(value) =>
                        Ok(
                            ControlFlow::Return(value)
                        ),

                    ControlFlow::Return(value) =>
                        Ok(
                            ControlFlow::Return(value)
                        ),

                    ControlFlow::Break =>
                        Err(
                            self.error(
                                ErrorKind::Control,
                                "break cannot be used in a return expression",
                                value,
                            )
                        ),

                    ControlFlow::Continue =>
                        Err(
                            self.error(
                                ErrorKind::Control,
                                "continue cannot be used in a return expression",
                                value,
                            )
                        ),
                }
            }
            

            None => Ok(
                ControlFlow::Return(Value::Unit)
            )
        }
    }

    fn eval_match(
        &mut self,
        value_expr: &Expr,
        arms: &[MatchArm],
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let value =
            self.eval_value(value_expr)?;

        for arm in arms {
            let mut bindings 
                = HashMap::<String, Value>::new();

            let matched =
                match_pattern(
                    &arm.pattern,
                    &value,
                    &mut bindings,
                )
                .map_err(|message| {
                    self.error(
                        ErrorKind::Runtime,
                        message,
                        whole,
                    )
                })?;

            if matched {
                return self.eval_match_arm(
                    &arm.body,
                    bindings,
                );
            }
        }

        Err(
            self.error(
                ErrorKind::Runtime,
                "non-exhaustive match",
                whole,
            )
        )
    }

    fn eval_match_arm(
    &mut self,
    body: &Expr,
    bindings: HashMap<String, Value>,
) -> Result<ControlFlow> {
    // ---------------------------------------------------------
    // Create a child scope for the match arm.
    // ---------------------------------------------------------
    let new_env = self.env.child();
    let previous =
        std::mem::replace(
            &mut self.env,
            new_env,
        );

    // ---------------------------------------------------------
    // Bind pattern variables.
    // ---------------------------------------------------------
    for (name, value) in bindings {
        self.env.define(
            name,
            value,
        );
    }

    // ---------------------------------------------------------
    // Evaluate the arm as a normal expression/control flow.
    // ---------------------------------------------------------
    let result =
        self.eval(body);

    // ---------------------------------------------------------
    // Always restore the previous environment before returning.
    // ---------------------------------------------------------
    self.env = previous;

    result
}

    fn eval_try(
        &mut self,
        whole: &Expr,
        inner: &Expr,
    ) -> Result<ControlFlow> {
        let value =
            self.eval_value(inner)?;

        match value {
            Value::EnumValue(value) => {
                match (
                    value.enum_name(),
                    value.variant(),
                ) {
                    // =================================================
                    // Option.Some
                    // =================================================

                    ("Option", "Some") => {
                        if value.fields().len()
                            != 1
                        {
                            return Err(
                                self.error(
                                    ErrorKind::Runtime,
                                    "Option.Some expects exactly one value",
                                    whole,
                                )
                            );
                        }

                        Ok(
                            ControlFlow::Value(
                                value
                                    .field(0)
                                    .unwrap()
                            )
                        )
                    }

                    // =================================================
                    // Option.None
                    // =================================================

                    ("Option", "None") => {
                        Ok(
                            ControlFlow::Return(
                                Value::EnumValue(
                                    Rc::clone(&value)
                                )
                            )
                        )
                    }

                    // =================================================
                    // Result.Ok
                    // =================================================

                    ("Result", "Ok") => {
                        if value.fields().len()
                            != 1
                        {
                            return Err(
                                self.error(
                                    ErrorKind::Runtime,
                                    "Result.Ok expects exactly one value",
                                    whole,
                                )
                            );
                        }

                        Ok(
                            ControlFlow::Value(
                                value
                                    .field(0)
                                    .unwrap()
                            )
                        )
                    }

                    // =================================================
                    // Result.Err
                    // =================================================

                    ("Result", "Err") => {
                        Ok(
                            ControlFlow::Return(
                                Value::EnumValue(
                                    Rc::clone(&value)
                                )
                            )
                        )
                    }

                    // =================================================
                    // Unknown enum
                    // =================================================

                    _ => {
                        Err(
                            self.error(
                                ErrorKind::Type,
                                format!(
                                    "? cannot be applied to {}.{}",
                                    value.enum_name(),
                                    value.variant()
                                ),
                                whole,
                            )
                        )
                    }
                }
            }

            other => {
                Err(
                    self.error(
                        ErrorKind::Type,
                        format!(
                            "? expects Option or Result, got {}",
                            other.type_name()
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn eval_range(
        &mut self,
        start: &Option<Box<Expr>>,
        end: &Option<Box<Expr>>,
        inclusive: bool,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let start =
            match start {
                Some(expr) => {
                    let v = self.eval_value(expr)?;

                    self.expect::<i64>(
                        v,
                        whole,
                    )?
                }

                None => 0,
            };

        let end =
            match end {
                Some(expr) => {
                    let v = self.eval_value(expr)?;

                    self.expect::<i64>(
                        v,
                        whole,
                    )?
                }

                None => i64::MAX,
            };

        Ok(
            ControlFlow::Value(
                Value::Range(
                    start,
                    end,
                    inclusive,
                )
            )
        )
    }

    fn eval_block(
        &mut self,
        exprs: &[Expr],
        _: bool,
    ) -> Result<ControlFlow> {
        let new_env = self.env.child();
        let old_env =
            std::mem::replace(
                &mut self.env,
                new_env,
            );

        let result = (|| {
            let mut last =
                Value::Unit;

            for expr in exprs {
                match self.eval(expr)? {
                    ControlFlow::Value(value) => {
                        last = value;
                    }

                    other => {
                        return Ok(other);
                    }
                }
            }

            Ok(
                ControlFlow::Value(last)
            )
        })();

        self.env = old_env;

        result
    }

    fn eval_and(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let left =
            self.eval_value(lhs)?;

        match left {
            // =====================================================
            // Scalar Bool: preserve short-circuit semantics
            // =====================================================

            Value::Bool(false) => {
                Ok(
                    ControlFlow::Value(
                        Value::Bool(false)
                    )
                )
            }

            Value::Bool(true) => {
                let v = self.eval_value(rhs)?; 

                Ok(ControlFlow::Value(
                    Value::Bool(
                        self.expect::<bool>(
                            v, whole
                        )?
                    )
                ))
            }

            // =====================================================
            // Series Bool: element-wise AND
            // =====================================================

            Value::Series(left_series) => {
                let right =
                    self.eval_value(rhs)?;

                match right {
                    Value::Series(
                        right_series
                    ) => {
                        let result =
                            operator::apply_series_boolean_op(
                                left_series,
                                right_series,
                                false,
                            )
                            .map_err(|message| {
                                self.error(
                                    ErrorKind::Runtime,
                                    message,
                                    whole,
                                )
                            })?;

                        Ok(
                            ControlFlow::Value(
                                Value::Series(
                                    Rc::new(result)
                                )
                            )
                        )
                    }

                    other => {
                        Err(
                            self.error(
                                ErrorKind::Type,
                                format!(
                                    "'and' expects Bool or Series<Bool>, got {}",
                                    other.type_name()
                                ),
                                whole,
                            )
                        )
                    }
                }
            }

            // =====================================================
            // Everything else
            // =====================================================

            other => {
                Err(
                    self.error(
                        ErrorKind::Type,
                        format!(
                            "'and' expects Bool or Series<Bool>, got {}",
                            other.type_name()
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn eval_or(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let left =
            self.eval_value(lhs)?;

        match left {
            // =====================================================
            // Scalar Bool: short-circuit
            // =====================================================

            Value::Bool(true) => {
                Ok(
                    ControlFlow::Value(
                        Value::Bool(true)
                    )
                )
            }

            Value::Bool(false) => {
                let v = self.eval_value(rhs)?;

                Ok(ControlFlow::Value(
                    Value::Bool(
                        self.expect::<bool>(
                            v, whole,
                        )?
                    )
                ))
            }

            // =====================================================
            // Series Bool: element-wise OR
            // =====================================================

            Value::Series(left_series) => {
                let right =
                    self.eval_value(rhs)?;

                match right {
                    Value::Series(
                        right_series
                    ) => {
                        let result =
                            operator::apply_series_boolean_op(
                                left_series,
                                right_series,
                                true,
                            )
                            .map_err(|message| {
                                self.error(
                                    ErrorKind::Runtime,
                                    message,
                                    whole,
                                )
                            })?;

                        Ok(
                            ControlFlow::Value(
                                Value::Series(
                                    Rc::new(result)
                                )
                            )
                        )
                    }

                    other => {
                        Err(
                            self.error(
                                ErrorKind::Type,
                                format!(
                                    "'or' expects Bool or Series<Bool>, got {}",
                                    other.type_name()
                                ),
                                whole,
                            )
                        )
                    }
                }
            }

            // =====================================================
            // Everything else
            // =====================================================

            other => {
                Err(
                    self.error(
                        ErrorKind::Type,
                        format!(
                            "'or' expects Bool or Series<Bool>, got {}",
                            other.type_name()
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        whole: &Expr,
    ) -> Result<ControlFlow> {
        
        let callable = self.eval_value(callee)?;

        let values =
        args.iter()
            .map(|arg| self.eval_value(arg))
            .collect::<Result<Vec<_>>>()?;

        match callable {
            Value::Func(func) => self.call_function(
                func, 
                values, 
                whole
            ),

            Value::Builtin(function) => {
                function(values)
                    .map(ControlFlow::Value)
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Runtime,
                            message,
                            whole,
                        )
                    })
            }

            Value::BoundMethod(method) => {
                self.call_bound_method(
                    method,
                    values,
                    whole,
                )
            }

            Value::Struct(definition) => {
                let object = definition
                    .instantiate(values)
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Arity,
                            message,
                            whole,
                        )
                    })?;

                Ok(ControlFlow::Value(
                    Value::Object(object)
                ))
            }

            Value::EnumConstructor(constructor) => {
                let variant =
                    constructor
                        .enum_def()
                        .variant(
                            constructor.variant()
                        )
                        .ok_or_else(|| {
                            self.error(
                                ErrorKind::Name,
                                format!(
                                    "unknown enum variant '{}'",
                                    constructor.variant()
                                ),
                                whole,
                            )
                        })?;

                if values.len()
                    != variant.arity()
                {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            format!(
                                "{}.{} expects {} arguments, got {}",
                                constructor
                                    .enum_def()
                                    .name(),
                                constructor.variant(),
                                variant.arity(),
                                values.len(),
                            ),
                            whole,
                        )
                    );
                }

                let value =
                    EnumValue::new(
                        constructor
                            .enum_def()
                            .name(),
                        constructor.variant(),
                        values,
                    );

                Ok(
                    ControlFlow::Value(
                        Value::EnumValue(
                            Rc::new(value)
                        )
                    )
                )
            }

            other => Err(self.error(ErrorKind::Type,format!("{} is not callable",other.type_name()),whole)),
        }
    }

    fn eval_field(
        &mut self,
        obj: &Expr,
        name: &str,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        let value = self.eval_value(obj)?;

        match value {
            Value::Str(string) => {
                let receiver =
                    MethodReceiver::Str(
                        string
                    );

                if receiver.supports_method(name) {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    receiver,
                                    name,
                                )
                            )
                        )
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Str has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }

            Value::List(list) => {
                let receiver =
                    MethodReceiver::List(
                        list.clone()
                    );

                if receiver.supports_method(name) {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    receiver,
                                    name,
                                )
                            )
                        )
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "List has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }

            Value::Set(set) => {
                let receiver =
                    MethodReceiver::Set(
                        set.clone()
                    );

                if receiver.supports_method(name) {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    receiver,
                                    name,
                                )
                            )
                        )
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Set has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }

            Value::Dict(dict) => {
                let receiver =
                    MethodReceiver::Dict(
                        dict.clone()
                    );

                if receiver.supports_method(name) {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    receiver,
                                    name,
                                )
                            )
                        )
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Dict has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }

            Value::Object(object) => {
                if object.borrow()
                    .get_method(name)
                    .is_some() 
                {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    MethodReceiver::Object(
                                        object.clone()
                                    ),
                                    name,
                                )
                            )
                        )
                    );
                }

                if let Some(value) =
                    object.borrow().get_field(name)
                {
                    return Ok(
                        ControlFlow::Value(value)
                    );
                }

                Err(self.error(
                    ErrorKind::Runtime,
                    format!(
                        "object has no field or method '{}'",
                        name
                    ),
                    whole,
                ))
            }

            Value::Module(module) => {
                let module =
                    module.borrow();

                if let Some(value) =
                    module.get_field(name)
                {
                    return Ok(
                        ControlFlow::Value(value)
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Name,
                        format!(
                            "module '{}' has no exported member '{}'",
                            module.name(),
                            name,
                        ),
                        whole,
                    )
                )
            }

            Value::Enum(enum_def) => {
                let variant =
                    enum_def
                        .variant(name)
                        .ok_or_else(|| {
                            self.error(
                                ErrorKind::Name,
                                format!(
                                    "enum '{}' has no variant '{}'",
                                    enum_def.name(),
                                    name,
                                ),
                                whole,
                            )
                        })?;

                // ---------------------------------------------------------
                // Unit variant
                //---------------------------------------------------------
                if variant.arity() == 0 {
                    return Ok(
                        ControlFlow::Value(
                            Value::EnumValue(
                                Rc::new(
                                    EnumValue::new(
                                        enum_def.name(),
                                        name,
                                        Vec::new(),
                                    )
                                )
                            )
                        )
                    );
                }

                // ---------------------------------------------------------
                // Payload variant
                // ---------------------------------------------------------
                Ok(
                    ControlFlow::Value(
                        Value::EnumConstructor(
                            EnumConstructor::new(
                                enum_def.clone(),
                                name,
                            )
                        )
                    )
                )
            }

            Value::Vector(vector) => {
                let receiver =
                    MethodReceiver::Vector(
                        vector.clone()
                    );

                if receiver.supports_method(name) {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    receiver,
                                    name,
                                )
                            )
                        )
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Vector has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }

            Value::Matrix(matrix) => {
                let receiver =
                    MethodReceiver::Matrix(
                        matrix.clone()
                    );

                if receiver.supports_method(name) {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    receiver,
                                    name,
                                )
                            )
                        )
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Matrix has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }

            Value::Series(series) => {
                match name {
                    "name" => {
                        Ok(
                            ControlFlow::Value(
                                Value::Str(
                                    Rc::new(
                                        series.name()
                                            .to_owned()
                                    )
                                )
                            )
                        )
                    }

                    "len" => {
                        Ok(
                            ControlFlow::Value(
                                Value::Int(
                                    series.len() as i64
                                )
                            )
                        )
                    }

                    "to_list"
                    | "to_matrix"
                    | "is_null"
                    | "is_not_null"
                    | "mean"
                    | "std"
                    | "median"
                    | "quantile"
                    | "sum"
                    | "min"
                    | "max"
                    | "dropna"
                    | "unique"
                    | "value_counts" => {
                        Ok(
                            ControlFlow::Value(
                                Value::BoundMethod(
                                    BoundMethod::new(
                                        MethodReceiver::Series(
                                            series.clone()
                                        ),
                                        name,
                                    )
                                )
                            )
                        )
                    }

                    _ => {
                        Err(
                            self.error(
                                ErrorKind::Runtime,
                                format!(
                                    "Series has no field or method '{}'",
                                    name
                                ),
                                whole,
                            )
                        )
                    }
                }
            }

            Value::DataFrame(df) => {
                match name {
                    "nrows" => {
                        Ok(
                            ControlFlow::Value(
                                Value::Int(
                                    df.nrows() as i64
                                )
                            )
                        )
                    }

                    "ncols" => {
                        Ok(
                            ControlFlow::Value(
                                Value::Int(
                                    df.ncols() as i64
                                )
                            )
                        )
                    }

                    "columns" => {
                        let values =
                            df.columns()
                                .into_iter()
                                .map(|name| {
                                    Value::Str(
                                        Rc::new(name)
                                    )
                                })
                                .collect::<Vec<_>>();

                        Ok(
                            ControlFlow::Value(
                                Value::List(
                                    Rc::new(
                                        RefCell::new(values)
                                    )
                                )
                            )
                        )
                    }

                    "column"
                    | "select"
                    | "filter"
                    | "head"
                    | "drop"
                    | "rename"
                    | "group_by"
                    | "sort"
                    | "describe"
                    | "to_matrix"
                    | "crosstab" => {
                        Ok(
                            ControlFlow::Value(
                                Value::BoundMethod(
                                    BoundMethod::new(
                                        MethodReceiver::DataFrame(
                                            df.clone()
                                        ),
                                        name,
                                    )
                                )
                            )
                        )
                    }

                    _ => {
                        Err(
                            self.error(
                                ErrorKind::Runtime,
                                format!(
                                    "DataFrame has no field or method '{}'",
                                    name
                                ),
                                whole,
                            )
                        )
                    }
                }
            }

            Value::GroupedDataFrame(grouped) => {
                match name {
                    "count"
                    | "mean"
                    | "sum"
                    | "aggregate" => {
                        Ok(
                            ControlFlow::Value(
                                Value::BoundMethod(
                                    BoundMethod::new(
                                        MethodReceiver::GroupedDataFrame(
                                            grouped.clone()
                                        ),
                                        name,
                                    )
                                )
                            )
                        )
                    }

                    "group_column" => {
                        Ok(
                            ControlFlow::Value(
                                Value::Str(
                                    Rc::new(
                                        grouped
                                            .group_column()
                                            .to_owned()
                                    )
                                )
                            )
                        )
                    }

                    _ => {
                        Err(
                            self.error(
                                ErrorKind::Runtime,
                                format!(
                                    "GroupedDataFrame has no field or method '{}'",
                                    name
                                ),
                                whole,
                            )
                        )
                    }
                }
            }

            Value::Iterator(iterator) => {
                let receiver =
                    MethodReceiver::Iterator(
                        iterator
                    );

                if receiver.supports_method(name) {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    receiver,
                                    name,
                                )
                            )
                        )
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Iterator has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }

            Value::Range(
                start,
                end,
                inclusive,
            ) => {
                let receiver =
                    MethodReceiver::Range {
                        start,
                        end,
                        inclusive,
                    };

                if receiver.supports_method(name) {
                    return Ok(
                        ControlFlow::Value(
                            Value::BoundMethod(
                                BoundMethod::new(
                                    receiver,
                                    name,
                                )
                            )
                        )
                    );
                }

                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Range has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
            
            other => Err(self.error(
                ErrorKind::Runtime,
                format!(
                    "no field or method '{}' on {}",
                    name,
                    other.type_name()
                ),
                whole,
            )),
        }
    }

    fn call_predicate(
        &mut self,
        predicate: &Value,
        argument: Value,
        whole: &Expr,
    ) -> Result<Value> {
        let result =
            match predicate {
                Value::Func(function) => {
                    self.call_function(
                        function.clone(),
                        vec![argument],
                        whole,
                    )?
                }

                Value::Builtin(function) => {
                    function(vec![argument])
                        .map(ControlFlow::Value)
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?
                }

                other => {
                    return Err(
                        self.error(
                            ErrorKind::Type,
                            format!(
                                "filter predicate must be callable, got {}",
                                other.type_name()
                            ),
                            whole,
                        )
                    );
                }
            };

        match result {
            ControlFlow::Value(value) =>
                Ok(value),

            other => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "filter predicate did not return a value: {:?}",
                            other
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn call_function(
        &mut self,
        func: crate::runtime::FuncRef,
        args: Vec<Value>,
        call_site: &Expr,
    ) -> Result<ControlFlow> {
        if func.params.len() != args.len() {
            return Err(self.error(
                ErrorKind::Arity,
                format!(
                    "function expects {} arguments, got {}",
                    func.params.len(),
                    args.len(),
                ),
                call_site,
            ));
        }

        let function_name =
            func.name
                .clone()
                .unwrap_or_else(|| "<lambda>".into());

        self.stack.push(StackFrame {
            function: function_name,
            span: Some(call_site.span),
        });

        let old_env = std::mem::replace(
            &mut self.env,
            func.closure.child(),
        );

        for (name, value) in func.params.iter().zip(args) {
            self.env.define(name.clone(), value);
        }

        self.function_depth += 1;

        let result = self.eval(&func.body);

        self.function_depth -= 1;
        self.env = old_env;
        self.stack.pop();

        match result? {
            ControlFlow::Value(value)
            | ControlFlow::Return(value) => {
                Ok(ControlFlow::Value(value))
            }

            ControlFlow::Break 
            => Err(
                self.error(
                    ErrorKind::Control,
                    "break escaped function boundary",
                    call_site,
                )
            ),

            ControlFlow::Continue
            => Err(
                self.error(
                    ErrorKind::Control,
                    "continue escaped function boundary",
                    call_site,
                )
            ),
        }
    }

    fn call_bound_method(
        &mut self,
        method: BoundMethod,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match method.receiver() {
            MethodReceiver::Str(string) => {
                self.call_string_method(
                    string.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }
            
            MethodReceiver::List(list) => {
                self.call_list_method(
                    list.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::Set(set) => {
                self.call_set_method(
                    set.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::Dict(dict) => {
                self.call_dict_method(
                    dict.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::Vector(vector) => {
                self.call_vector_method(
                    vector.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::Matrix(matrix) => {
                self.call_matrix_method(
                    matrix.clone(), 
                    method.name(), 
                    args, 
                    whole
                )
            }

            MethodReceiver::Range {
                start,
                end,
                inclusive,
            } => {
                self.call_range_method(
                    *start,
                    *end,
                    *inclusive,
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::Object(object) => {
                self.call_object_method(
                    object.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::Iterator(iterator) => {
                self.call_iterator_method(
                    iterator.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::Series(series) => {
                self.call_series_method(
                    series.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::DataFrame(dataframe) => {
                self.call_dataframe_method(
                    dataframe.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }

            MethodReceiver::GroupedDataFrame(grouped) => {
                self.call_grouped_dataframe_method(
                    grouped.clone(),
                    method.name(),
                    args,
                    whole,
                )
            }
        }
    }

    fn call_string_method(
        &mut self,
        string: Rc<String>,
        name: &str,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            // =====================================================
            // chars()
            // =====================================================
            "chars" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "chars() expects no arguments",
                            whole,
                        )
                    );
                }

                let chars =
                    string
                        .chars()
                        .collect::<Vec<char>>();

                let iterator =
                    IteratorObj::Str {
                        data: Rc::new(chars),
                        index: 0,
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    iterator
                                )
                            )
                        )
                    )
                )
            }

            // =====================================================
            // len()
            // =====================================================
            "len" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "len() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Int(
                            string.chars().count()
                                as i64
                        )
                    )
                )
            }

            // =====================================================
            // trim()
            // =====================================================
            "trim" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "trim() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Str(
                            Rc::new(
                                string.trim()
                                    .to_owned()
                            )
                        )
                    )
                )
            }

            // =====================================================
            // to_upper()
            // =====================================================
            "to_upper" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "to_upper() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Str(
                            Rc::new(
                                string
                                    .to_uppercase()
                            )
                        )
                    )
                )
            }

            // =====================================================
            // to_lower()
            // =====================================================
            "to_lower" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "to_lower() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Str(
                            Rc::new(
                                string
                                    .to_lowercase()
                            )
                        )
                    )
                )
            }

            // =====================================================
            // contains()
            // =====================================================
            "contains" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "contains() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let v = args[0].clone();
                let str: StrRef = self.expect(
                    v,
                    whole,
                )?;

                let needle = str.as_ref();

                Ok(
                    ControlFlow::Value(
                        Value::Bool(
                            string.contains(
                                needle
                            )
                        )
                    )
                )
            }

            // =========================================================
            // starts_with(prefix)
            // =========================================================
            "starts_with" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "starts_with() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let v = args[0].clone();
                let str: StrRef = self.expect(
                    v,
                    whole,
                )?;

                let prefix = str.as_str();

                Ok(
                    ControlFlow::Value(
                        Value::Bool(
                            string.starts_with(prefix)
                        )
                    )
                )
            }

            // =========================================================
            // ends_with(suffix)
            // =========================================================
            "ends_with" => {
            if args.len() != 1 {
                return Err(
                    self.error(
                        ErrorKind::Arity,
                        "ends_with() takes exactly 1 argument",
                        whole,
                    )
                );
            }

            let v = args[0].clone();
            let str: StrRef = self.expect(
                v,
                whole,
            )?;

            let suffix = str.as_str();

            Ok(
                ControlFlow::Value(
                    Value::Bool(
                        string.ends_with(suffix)
                    )
                )
            )
        }

            // =========================================================
            // split(separator)
            // =========================================================
            "split" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "split() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let v = args[0].clone();
                let str: StrRef = self.expect(
                    v,
                    whole,
                )?;

                let separator = str.as_str();

                let values =
                    string
                        .split(separator)
                        .map(|part| {
                            Value::Str(
                                Rc::new(
                                    part.to_owned()
                                )
                            )
                        })
                        .collect::<Vec<_>>();

                Ok(
                    ControlFlow::Value(
                        Value::List(
                            Rc::new(
                                RefCell::new(values)
                            )
                        )
                    )
                )
            }

            // =========================================================
            // replace(Str, Str)
            // =========================================================
            "replace" => {
                if args.len() != 2 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "replace() takes exactly 2 arguments",
                            whole,
                        )
                    );
                }

                let mut args =
                    args.into_iter();

                let from =
                    match args.next().unwrap() {
                        Value::Str(value) =>
                            value,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "replace() expects Str as first argument, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let to =
                    match args.next().unwrap() {
                        Value::Str(value) =>
                            value,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "replace() expects Str as second argument, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let result =
                    string.replace(
                        from.as_str(),
                        to.as_str(),
                    );

                Ok(
                    ControlFlow::Value(
                        Value::Str(
                            Rc::new(result)
                        )
                    )
                )
            }

            // =========================================================
            // repeat(Int)
            // =========================================================
            "repeat" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "repeat() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let count =
                    match &args[0] {
                        Value::Int(value)
                            if *value >= 0 =>
                        {
                            *value as usize
                        }

                        Value::Int(_) => {
                            return Err(
                                self.error(
                                    ErrorKind::Index,
                                    "repeat() does not accept negative counts",
                                    whole,
                                )
                            );
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "repeat() expects Int, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Str(
                            Rc::new(
                                string.repeat(count)
                            )
                        )
                    )
                )
            }

            // =========================================================
            // get(Int)
            // =========================================================
            "get" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "get() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let index =
                    match &args[0] {
                        Value::Int(value)
                            if *value >= 0 =>
                        {
                            *value as usize
                        }

                        Value::Int(_) =>
                            return Ok(
                                ControlFlow::Value(
                                    option_none()
                                )
                            ),

                        other =>
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "get() expects Int, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            ),
                    };

                let value =
                    string
                        .chars()
                        .nth(index)
                        .map(|ch| {
                            Value::Str(
                                Rc::new(
                                    ch.to_string()
                                )
                            )
                        });

                Ok(
                    ControlFlow::Value(
                        match value {
                            Some(value) =>
                                option_some(value),

                            None =>
                                option_none(),
                        }
                    )
                )
            }

            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Str has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn call_list_method(
        &mut self,
        list: List,
        name: &str,
        mut args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            // =====================================================
            // push(value)
            // =====================================================

            "push" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "push() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                list.borrow_mut()
                    .push(
                        args.remove(0)
                    );

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            // =====================================================
            // pop()
            // =====================================================

            "pop" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "pop() takes no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        list.borrow_mut()
                            .pop()
                            .unwrap_or(
                                Value::Unit
                            )
                    )
                )
            }

            // =====================================================
            // remove(index)
            // =====================================================

            "remove" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "remove() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let index =
                    match args.remove(0) {
                        Value::Int(i)
                            if i >= 0 =>
                        {
                            i as usize
                        }

                        Value::Int(_) => {
                            return Err(
                                self.error(
                                    ErrorKind::Index,
                                    "remove() does not accept negative indices",
                                    whole,
                                )
                            );
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "remove() expects Int, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let mut list =
                    list.borrow_mut();

                if index >= list.len() {
                    return Err(
                        self.error(
                            ErrorKind::Index,
                            format!(
                                "index out of range: {}",
                                index
                            ),
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        list.remove(index)
                    )
                )
            }

            // =====================================================
            // len()
            // =====================================================

            "len" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "len() takes no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Int(
                            list.borrow().len() as i64
                        )
                    )
                )
            }

            // =====================================================
            // iter()
            // =====================================================

            "iter" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "iter() takes no arguments",
                            whole,
                        )
                    );
                }

                let iterator =
                    IteratorObj::List {
                        data: list.clone(),
                        index: 0,
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    iterator
                                )
                            )
                        )
                    )
                )
            }

            // =====================================================
            // get(index)
            //
            // xs.get(i)
            //   -> Some(value)
            //   -> None
            // =====================================================

            "get" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "get() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let index =
                    match args.remove(0) {
                        Value::Int(i)
                            if i >= 0 =>
                        {
                            i as usize
                        }

                        Value::Int(_) => {
                            return Ok(
                                ControlFlow::Value(
                                    option_none()
                                )
                            );
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "get() expects Int, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let value =
                    list.borrow()
                        .get(index)
                        .cloned();

                Ok(
                    ControlFlow::Value(
                        match value {
                            Some(value) =>
                                option_some(
                                    value
                                ),

                            None =>
                                option_none(),
                        }
                    )
                )
            }

            // =====================================================
            // set(index, value)
            // =====================================================

            "set" => {
                if args.len() != 2 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "set() takes exactly 2 arguments",
                            whole,
                        )
                    );
                }

                let mut args =
                    args.into_iter();

                let index =
                    match args.next().unwrap() {
                        Value::Int(i)
                            if i >= 0 =>
                        {
                            i as usize
                        }

                        Value::Int(_) => {
                            return Err(
                                self.error(
                                    ErrorKind::Index,
                                    "set() does not accept negative indices",
                                    whole,
                                )
                            );
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "set() expects Int as first argument, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let value =
                    args.next().unwrap();

                let mut list =
                    list.borrow_mut();

                if index >= list.len() {
                    return Err(
                        self.error(
                            ErrorKind::Index,
                            format!(
                                "index out of range: {}",
                                index
                            ),
                            whole,
                        )
                    );
                }

                list[index] =
                    value;

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            // =====================================================
            // insert(index, value)
            // =====================================================

            "insert" => {
                if args.len() != 2 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "insert() takes exactly 2 arguments",
                            whole,
                        )
                    );
                }

                let mut args =
                    args.into_iter();

                let index =
                    match args.next().unwrap() {
                        Value::Int(i)
                            if i >= 0 =>
                        {
                            i as usize
                        }

                        Value::Int(_) => {
                            return Err(
                                self.error(
                                    ErrorKind::Index,
                                    "insert() does not accept negative indices",
                                    whole,
                                )
                            );
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "insert() expects Int as first argument, got {}",
                                        other.type_name()
                                    ),
                                    whole
                                )
                            );
                        }
                    };

                let value =
                    args.next().unwrap();

                let mut list =
                    list.borrow_mut();

                if index > list.len() {
                    return Err(
                        self.error(
                            ErrorKind::Index,
                            format!(
                                "index out of range: {}",
                                index
                            ),
                            whole,
                        )
                    );
                }

                list.insert(
                    index,
                    value,
                );

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            // =====================================================
            // contains(value)
            // =====================================================

            "contains" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "contains() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let needle =
                    &args[0];

                let values =
                    list.borrow();

                for value in values.iter() {
                    if Value::eq_values(
                        value,
                        needle,
                    )
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Runtime,
                            message,
                            whole,
                        )
                    })?
                    {
                        return Ok(
                            ControlFlow::Value(
                                Value::Bool(true)
                            )
                        );
                    }
                }

                Ok(
                    ControlFlow::Value(
                        Value::Bool(false)
                    )
                )
            }

            // =====================================================
            // reverse()
            // =====================================================

            "reverse" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "reverse() takes no arguments",
                            whole,
                        )
                    );
                }

                list.borrow_mut()
                    .reverse();

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            // =====================================================
            // clear()
            // =====================================================

            "clear" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "clear() takes no arguments",
                            whole,
                        )
                    );
                }

                list.borrow_mut()
                    .clear();

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            // =====================================================
            // extend(other_list)
            // =====================================================

            "extend" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "extend() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let other =
                    match args.remove(0) {
                        Value::List(other) =>
                            other,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "extend() expects List, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                // Clone first so that extending a list with itself
                // does not create a borrow conflict.
                let values =
                    other.borrow().clone();

                list.borrow_mut()
                    .extend(values);

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            // =====================================================
            // join(separator)
            // =====================================================

            "join" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "join() takes exactly 1 argument",
                            whole,
                        )
                    );
                }

                let separator =
                    match args.remove(0) {
                        Value::Str(value) =>
                            value,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "join() expects Str, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let values =
                    list.borrow();

                let mut result =
                    String::new();

                for (i, value)
                    in values.iter().enumerate()
                {
                    if i > 0 {
                        result.push_str(
                            separator.as_str()
                        );
                    }

                    match value {
                        Value::Str(value) =>
                            result.push_str(
                                value.as_str()
                            ),

                        other =>
                            result.push_str(
                                &other.to_string()
                            ),
                    }
                }

                Ok(
                    ControlFlow::Value(
                        Value::Str(
                            Rc::new(result)
                        )
                    )
                )
            }

            // =====================================================
            // vector()
            // =====================================================
            "vector" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "vector() takes no arguments",
                            whole,
                        )
                    );
                }

                let values =
                    list.borrow();

                let mut data =
                    Vec::with_capacity(
                        values.len()
                    );

                for value in values.iter() {
                    match value {
                        Value::Int(n) => {
                            data.push(*n as f64);
                        }

                        Value::Float(x) => {
                            data.push(*x);
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "vector() expects numeric elements, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    }
                }

                let vector =
                    Vector::new(data);

                Ok(
                    ControlFlow::Value(
                        Value::Vector(
                            Rc::new(
                                RefCell::new(
                                    vector
                                )
                            )
                        )
                    )
                )
            }


            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "List has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn call_set_method(
        &mut self,
        set: SetRef,
        name: &str,
        mut args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            "len" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "len() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Int(
                            set.borrow().len() as i64
                        )
                    )
                )
            }

            "add" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "add() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                set.borrow_mut()
                    .add(args.remove(0))
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Runtime,
                            message,
                            whole,
                        )
                    })?;

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            "remove" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "remove() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let removed =
                    set.borrow_mut()
                        .remove(
                            &args[0]
                        )
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::Bool(
                            removed
                        )
                    )
                )
            }

            "contains" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "contains() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let contains =
                    set.borrow()
                        .contains(
                            &args[0]
                        )
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::Bool(
                            contains
                        )
                    )
                )
            }

            "clear" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "clear() expects no arguments",
                            whole,
                        )
                    );
                }

                set.borrow_mut()
                    .clear();

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            "iter" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "iter() expects no arguments",
                            whole,
                        )
                    );
                }

                let data =
                    Rc::new(
                        RefCell::new(
                            set.borrow()
                                .values()
                                .to_vec()
                        )
                    );

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    IteratorObj::List {
                                        data,
                                        index: 0,
                                    }
                                )
                            )
                        )
                    )
                )
            }

            "union"
            | "intersection"
            | "difference" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            format!(
                                "{}() expects exactly 1 argument",
                                name
                            ),
                            whole,
                        )
                    );
                }

                let other =
                    self.expect::<SetRef>(
                        args.remove(0),
                        whole,
                    )?;

                let result =
                    match name {
                        "union" =>
                            set.borrow()
                                .union(
                                    &other.borrow()
                                ),

                        "intersection" =>
                            set.borrow()
                                .intersection(
                                    &other.borrow()
                                ),

                        "difference" =>
                            set.borrow()
                                .difference(
                                    &other.borrow()
                                ),

                        _ =>
                            unreachable!(),
                    }
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Runtime,
                            message,
                            whole,
                        )
                    })?;

                Ok(
                    ControlFlow::Value(
                        Value::Set(
                            Rc::new(
                                RefCell::new(
                                    result
                                )
                            )
                        )
                    )
                )
            }

            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Set has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn call_dict_method(
        &mut self,
        dict: Dict,
        name: &str,
        mut args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            "len" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "len() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Int(
                            dict.borrow().len()
                                as i64
                        )
                    )
                )
            }

            "get" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "get() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let key =
                    self.expect::<StrRef>(
                        args.remove(0),
                        whole,
                    )?;

                let value =
                    dict.borrow()
                        .get(key.as_str())
                        .cloned();

                Ok(
                    ControlFlow::Value(
                        match value {
                            Some(value) =>
                                option_some(
                                    value
                                ),

                            None =>
                                option_none(),
                        }
                    )
                )
            }

            "set" => {
                if args.len() != 2 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "set() expects exactly 2 arguments",
                            whole,
                        )
                    );
                }

                let key =
                    self.expect::<StrRef>(
                        args.remove(0),
                        whole,
                    )?;

                let value =
                    args.remove(0);

                dict.borrow_mut()
                    .insert(
                        key.as_str().to_owned(),
                        value,
                    );

                Ok(
                    ControlFlow::Value(
                        Value::Unit
                    )
                )
            }

            "remove" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "remove() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let key =
                    self.expect::<StrRef>(
                        args.remove(0),
                        whole,
                    )?;

                let value =
                    dict.borrow_mut()
                        .remove(
                            key.as_str()
                        );

                Ok(
                    ControlFlow::Value(
                        match value {
                            Some(value) =>
                                option_some(
                                    value
                                ),

                            None =>
                                option_none(),
                        }
                    )
                )
            }

            "contains" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "contains() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let key =
                    self.expect::<StrRef>(
                        args.remove(0),
                        whole,
                    )?;

                let contains =
                    dict.borrow()
                        .contains_key(
                            key.as_str()
                        );

                Ok(
                    ControlFlow::Value(
                        Value::Bool(contains)
                    )
                )
            }

            "keys" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "keys() expects no arguments",
                            whole,
                        )
                    );
                }

                let values =
                    dict.borrow()
                        .keys()
                        .map(|key| {
                            Value::Str(
                                Rc::new(
                                    key.clone()
                                )
                            )
                        })
                        .collect::<Vec<_>>();

                Ok(
                    ControlFlow::Value(
                        Value::List(
                            Rc::new(
                                RefCell::new(
                                    values
                                )
                            )
                        )
                    )
                )
            }

            "values" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "values() expects no arguments",
                            whole,
                        )
                    );
                }

                let values =
                    dict.borrow()
                        .values()
                        .cloned()
                        .collect::<Vec<_>>();

                Ok(
                    ControlFlow::Value(
                        Value::List(
                            Rc::new(
                                RefCell::new(
                                    values
                                )
                            )
                        )
                    )
                )
            }

            "items" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "items() expects no arguments",
                            whole,
                        )
                    );
                }

                let values =
                    dict.borrow()
                        .iter()
                        .map(|(key, value)| {
                            Value::Tuple(
                                Rc::new(vec![
                                    Value::Str(
                                        Rc::new(
                                            key.clone()
                                        )
                                    ),
                                    value.clone(),
                                ])
                            )
                        })
                        .collect::<Vec<_>>();

                Ok(
                    ControlFlow::Value(
                        Value::List(
                            Rc::new(
                                RefCell::new(
                                    values
                                )
                            )
                        )
                    )
                )
            }

            "iter" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "iter() expects no arguments",
                            whole,
                        )
                    );
                }

                // We deliberately snapshot the entries.
                //
                // This avoids holding a RefCell borrow for the
                // lifetime of the iterator.
                let items =
                    dict.borrow()
                        .iter()
                        .map(|(key, value)| {
                            Value::Tuple(
                                Rc::new(vec![
                                    Value::Str(
                                        Rc::new(
                                            key.clone()
                                        )
                                    ),
                                    value.clone(),
                                ])
                            )
                        })
                        .collect::<Vec<_>>();

                let iterator =
                    IteratorObj::List {
                        data: Rc::new(
                            RefCell::new(items)
                        ),
                        index: 0,
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    iterator
                                )
                            )
                        )
                    )
                )
            }

            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Dict has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn call_vector_method(
        &mut self,
        vector: VectorRef,
        name: &str,
        mut args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            "len" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "len() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Int(
                            vector.borrow().len()
                                as i64
                        )
                    )
                )
            }

            "shape" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "shape() expects no arguments",
                            whole,
                        )
                    );
                }

                let vector =
                    vector.borrow();

                let (rows, _) =
                    vector.shape();

                Ok(
                    ControlFlow::Value(
                        Value::Tuple(
                            Rc::new(
                                vec![
                                    Value::Int(
                                        rows as i64
                                    ),
                                ]
                            )
                        )
                    )
                )
            }

            "norm" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "norm() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Float(
                            vector.borrow().norm()
                        )
                    )
                )
            }

            "dot" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "dot() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let other =
                    match args.remove(0) {
                        Value::Vector(other) =>
                            other,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "dot() expects Vector, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let result =
                    vector
                        .borrow()
                        .dot(
                            &other.borrow()
                        )
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Shape,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::Float(result)
                    )
                )
            }

            "to_matrix" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "to_matrix() expects no arguments",
                            whole,
                        )
                    );
                }

                let matrix =
                    vector
                        .borrow()
                        .to_column_matrix();

                Ok(
                    ControlFlow::Value(
                        Value::Matrix(
                            Rc::new(
                                RefCell::new(
                                    matrix
                                )
                            )
                        )
                    )
                )
            }

            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Vector has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn call_matrix_method(
        &mut self,
        matrix: MatrixRef,
        name: &str,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            "shape" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "shape() expects no arguments",
                            whole,
                        )
                    );
                }

                let matrix =
                    matrix.borrow();

                let (rows, cols) =
                    matrix.shape();

                Ok(
                    ControlFlow::Value(
                        Value::Tuple(
                            Rc::new(
                                vec![
                                    Value::Int(
                                        rows as i64
                                    ),
                                    Value::Int(
                                        cols as i64
                                    ),
                                ]
                            )
                        )
                    )
                )
            }

            "transpose" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "transpose() expects no arguments",
                            whole,
                        )
                    );
                }

                let result =
                    matrix
                        .borrow()
                        .transpose();

                Ok(
                    ControlFlow::Value(
                        Value::Matrix(
                            Rc::new(
                                RefCell::new(
                                    result
                                )
                            )
                        )
                    )
                )
            }

            "trace" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "trace() expects no arguments",
                            whole,
                        )
                    );
                }

                let result =
                    matrix
                        .borrow()
                        .trace()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Shape,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::Float(result)
                    )
                )
            }

            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Matrix has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn call_range_method(
        &mut self,
        start: i64,
        end: i64,
        inclusive: bool,
        name: &str,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            "iter" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "iter() expects no arguments",
                            whole,
                        )
                    );
                }

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            self.make_range_iterator(
                                start,
                                end,
                                inclusive,
                                whole,
                            )?
                        )
                    )
                )
            }

            "map"
            | "filter"
            | "collect"
            | "reduce"
            | "fold"
            | "any"
            | "all"
            | "enumerate"
            | "zip"
            | "take"
            | "skip" => {
                let iterator =
                    self.make_range_iterator(
                        start,
                        end,
                        inclusive,
                        whole,
                    )?;

                self.call_iterator_method(
                    iterator,
                    name,
                    args,
                    whole,
                )
            }

            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Range has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    /// Helper for `call_range_method()`
    fn make_range_iterator(
        &mut self,
        start: i64,
        end: i64,
        inclusive: bool,
        whole: &Expr,
    ) -> Result<IteratorRef> {
        let end =
            if inclusive {
                end.checked_add(1)
                    .ok_or_else(|| {
                        self.error(
                            ErrorKind::Overflow,
                            "inclusive range endpoint overflow",
                            whole,
                        )
                    })?
            } else {
                end
            };

        Ok(
            Rc::new(
                RefCell::new(
                    IteratorObj::Range {
                        current: start,
                        end,
                    }
                )
            )
        )
    }

    fn call_object_method(
        &mut self,
        object: ObjectRef,
        name: &str,
        mut args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        // ---------------------------------------------------------
        // Find the method definition.
        // ---------------------------------------------------------

        let function = {
            let object_ref = object.borrow();

            object_ref
                .get_method(name)
                .ok_or_else(|| {
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "object has no method '{}'",
                            name
                        ),
                        whole,
                    )
                })?
        };

        // ---------------------------------------------------------
        // Bind implicit self.
        // ---------------------------------------------------------

        let mut call_args =
            Vec::with_capacity(
                args.len() + 1
            );

        call_args.push(
            Value::Object(
                object.clone()
            )
        );

        call_args.append(
            &mut args
        );

        // ---------------------------------------------------------
        // Invoke the actual function.
        // ---------------------------------------------------------

        self.call_function(
            function,
            call_args,
            whole,
        )
    }

    fn call_iterator_method(
        &mut self,
        iterator: IteratorRef,
        name: &str,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            "next" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "next() expects no arguments",
                            whole,
                        )
                    );
                }

                let next =
                    self.next_from_iterator(
                        &iterator,
                        whole,
                    )?;

                Ok(
                    ControlFlow::Value(
                        match next {
                            Some(value) =>
                                option_some(value),

                            None =>
                                option_none(),
                        }
                    )
                )
            }

            "map" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "map() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let function =
                    match args
                        .into_iter()
                        .next()
                        .unwrap()
                    {
                        Value::Func(function) =>
                            function,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "map() expects Function, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let mapped =
                    IteratorObj::Map {
                        source:
                            iterator.clone(),

                        function,
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    mapped
                                )
                            )
                        )
                    )
                )
            }

            "filter" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "filter() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let predicate =
                    match args
                        .into_iter()
                        .next()
                        .unwrap()
                    {
                        Value::Func(function) =>
                            function,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "filter() expects Function, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let filtered =
                    IteratorObj::Filter {
                        source:
                            iterator.clone(),

                        predicate,
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    filtered
                                )
                            )
                        )
                    )
                )
            }

            "collect" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "collect() expects no arguments",
                            whole,
                        )
                    );
                }

                let mut values =
                    Vec::new();

                loop {
                    let value =
                        {
                            let mut iterator =
                                iterator.borrow_mut();

                            self.next_iterator_value(
                                &mut iterator,
                                whole,
                            )?
                        };

                    match value {
                        Some(value) =>
                            values.push(value),

                        None =>
                            break,
                    }
                }

                Ok(
                    ControlFlow::Value(
                        Value::List(
                            Rc::new(
                                RefCell::new(
                                    values
                                )
                            )
                        )
                    )
                )
            }

            "reduce" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "reduce() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let function =
                    match args.into_iter().next().unwrap() {
                        Value::Func(function) =>
                            function,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "reduce() expects Function, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let Some(mut accumulator) =
                    self.next_from_iterator(
                        &iterator,
                        whole,
                    )?
                else {
                    return Ok(
                        ControlFlow::Value(
                            option_none()
                        )
                    );
                };

                while let Some(value) =
                    self.next_from_iterator(
                        &iterator,
                        whole,
                    )?
                {
                    accumulator =
                        self.call_iterator_callback(
                            function.clone(),
                            vec![
                                accumulator,
                                value,
                            ],
                            whole
                        )?;
                }

                Ok(
                    ControlFlow::Value(
                        option_some(
                            accumulator
                        )
                    )
                )
            }
            
            "fold" => {
                if args.len() != 2 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "fold() expects exactly 2 arguments",
                            whole,
                        )
                    );
                }

                let mut args =
                    args.into_iter();

                let mut accumulator =
                    args.next().unwrap();

                let function =
                    match args.next().unwrap() {
                        Value::Func(function) =>
                            function,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "fold() expects Function as second argument, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                loop {
                    let next =
                        self.next_from_iterator(
                            &iterator,
                            whole,
                        )?;

                    let value =
                        match next {
                            Some(value) =>
                                value,

                            None =>
                                break,
                        };

                    accumulator =
                        self.call_iterator_callback(
                            function.clone(),
                            vec![
                                accumulator,
                                value,
                            ],
                            whole
                        )?;
                }

                Ok(
                    ControlFlow::Value(
                        accumulator
                    )
                )
            }
            
            "any" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "any() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let predicate =
                    match args.into_iter().next().unwrap() {
                        Value::Func(function) =>
                            function,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "any() expects Function, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                loop {
                    let next =
                        self.next_from_iterator(
                            &iterator,
                            whole,
                        )?;

                    let value =
                        match next {
                            Some(value) =>
                                value,

                            None => {
                                return Ok(
                                    ControlFlow::Value(
                                        Value::Bool(false)
                                    )
                                );
                            }
                        };

                    match self.call_iterator_predicate(
                        predicate.clone(),
                        value,
                        whole
                    )? {
                        true => return Ok(
                                ControlFlow::Value(
                                    Value::Bool(true)
                                )
                            ),

                        false => continue,
                    }
                }
            }
            
            "all" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "all() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let predicate =
                    match args.into_iter().next().unwrap() {
                        Value::Func(function) =>
                            function,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "all() expects Function, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                loop {
                    let next =
                        self.next_from_iterator(
                            &iterator,
                            whole,
                        )?;

                    let value =
                        match next {
                            Some(value) =>
                                value,

                            None => {
                                return Ok(
                                    ControlFlow::Value(
                                        Value::Bool(true)
                                    )
                                );
                            }
                        };

                    match self.call_iterator_predicate(
                        predicate.clone(),
                        value,
                        whole
                    )? {
                        false => return Ok(
                                ControlFlow::Value(
                                    Value::Bool(false)
                                )
                            ),

                        true => continue,
                    }
                }
            }
            
            "enumerate" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "enumerate() expects no arguments",
                            whole,
                        )
                    );
                }

                let result =
                    IteratorObj::Enumerate {
                        source: iterator.clone(),
                        index: 0,
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    result
                                )
                            )
                        )
                    )
                )
            }

            "zip" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "zip() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let other =
                    match &args[0] {
                        Value::Iterator(other) =>
                            other,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "zip() expects Iterator, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let result =
                    IteratorObj::Zip {
                        left: iterator.clone(),
                        right: other.clone(),
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    result
                                )
                            )
                        )
                    )
                )
            }

            "take" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "take() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let remaining =
                    match &args[0] {
                        Value::Int(value)
                            if *value >= 0 =>
                        {
                            *value as usize
                        }

                        Value::Int(_) => {
                            return Err(
                                self.error(
                                    ErrorKind::Range,
                                    "take() does not accept negative counts",
                                    whole,
                                )
                            );
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "take() expects Int, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let result =
                    IteratorObj::Take {
                        source: iterator.clone(),
                        remaining,
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    result
                                )
                            )
                        )
                    )
                )
            }

            "skip" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "skip() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let remaining =
                    match &args[0] {
                        Value::Int(value)
                            if *value >= 0 =>
                        {
                            *value as usize
                        }

                        Value::Int(_) => {
                            return Err(
                                self.error(
                                    ErrorKind::Range,
                                    "skip() does not accept negative counts",
                                    whole,
                                )
                            );
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "skip() expects Int, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let result =
                    IteratorObj::Skip {
                        source: iterator.clone(),
                        remaining,
                    };

                Ok(
                    ControlFlow::Value(
                        Value::Iterator(
                            Rc::new(
                                RefCell::new(
                                    result
                                )
                            )
                        )
                    )
                )
            }

            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Iterator has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    /// Helper for `call_iterator_method()`
    fn call_iterator_predicate(
        &mut self,
        function: FuncRef,
        value: Value,
        whole: &Expr,
    ) -> Result<bool> {
        match self.call_iterator_callback(
            function,
            vec![value],
            whole,
        )? {
            Value::Bool(result) =>
                Ok(result),

            other => {
                Err(
                    self.error(
                        ErrorKind::Type,
                        format!(
                            "predicate must return Bool, got {}",
                            other.type_name()
                        ),
                        whole,
                    )
                )
            }
        }
    }

    /// Helper for `call_iterator_method`
    fn call_iterator_callback(
        &mut self,
        function: FuncRef,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<Value> {
        match self.call_function(
            function,
            args,
            whole,
        )? {
            ControlFlow::Value(value) =>
                Ok(value),

            ControlFlow::Return(_) => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        "iterator callback cannot return",
                        whole,
                    )
                )
            }

            ControlFlow::Break => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        "iterator callback cannot break",
                        whole,
                    )
                )
            }

            ControlFlow::Continue => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        "iterator callback cannot continue",
                        whole,
                    )
                )
            }
        }
    }

    fn call_series_method(
        &mut self,
        series: SeriesRef,
        name: &str,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            // =====================================================
            // Existing methods
            // =====================================================

            "to_list" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "to_list() expects no arguments",
                        whole,
                    ));
                }

                Ok(
                    ControlFlow::Value(
                        Value::List(
                            Rc::new(
                                RefCell::new(
                                    series.data()
                                        .to_vec()
                                )
                            )
                        )
                    )
                )
            }

            "to_matrix" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "to_matrix() expects no arguments",
                        whole,
                    ));
                }

                let matrix =
                    series
                        .to_matrix()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::Matrix(
                            Rc::new(
                                RefCell::new(matrix)
                            )
                        )
                    )
                )
            }

            "is_null" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "is_null() expects no arguments",
                        whole,
                    ));
                }

                Ok(
                    ControlFlow::Value(
                        Value::Series(
                            Rc::new(
                                series.is_null()
                            )
                        )
                    )
                )
            }

            "is_not_null" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "is_not_null() expects no arguments",
                        whole,
                    ));
                }

                Ok(
                    ControlFlow::Value(
                        Value::Series(
                            Rc::new(
                                series.is_not_null()
                            )
                        )
                    )
                )
            }

            // =====================================================
            // Descriptive statistics
            // =====================================================

            "mean" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "mean() expects no arguments",
                        whole,
                    ));
                }

                let value =
                    series.mean()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Type,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(value)
                )
            }

            "std" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "std() expects no arguments",
                        whole,
                    ));
                }

                let value =
                    series.std()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Type,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(value)
                )
            }

            "median" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "median() expects no arguments",
                        whole,
                    ));
                }

                let value =
                    series.median()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Type,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(value)
                )
            }

            "sum" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "sum() expects no arguments",
                        whole,
                    ));
                }

                let value =
                    series.sum()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Type,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(value)
                )
            }

            "min" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "min() expects no arguments",
                        whole,
                    ));
                }

                let value =
                    series.min()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Type,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(value)
                )
            }

            "max" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "max() expects no arguments",
                        whole,
                    ));
                }

                let value =
                    series.max()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Type,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(value)
                )
            }

            "quantile" => {
                if args.len() != 1 {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "quantile() expects exactly 1 argument",
                        whole,
                    ));
                }

                let q =
                    match args[0] {
                        Value::Int(value) =>
                            value as f64,

                        Value::Float(value) =>
                            value,

                        ref other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "quantile() expects numeric q, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let value =
                    series.quantile(q)
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Value,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(value)
                )
            }

            // =====================================================
            // Missing values / categorical operations
            // =====================================================

            "dropna" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "dropna() expects no arguments",
                        whole,
                    ));
                }

                Ok(
                    ControlFlow::Value(
                        Value::Series(
                            Rc::new(
                                series.dropna()
                            )
                        )
                    )
                )
            }

            "unique" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "unique() expects no arguments",
                        whole,
                    ));
                }

                let result =
                    series.unique()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::Series(
                            Rc::new(result)
                        )
                    )
                )
            }

            "value_counts" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "value_counts() expects no arguments",
                        whole,
                    ));
                }

                let result =
                    series.value_counts()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::DataFrame(
                            Rc::new(result)
                        )
                    )
                )
            }

            _ => {
                Err(
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "Series has no method '{}'",
                            name
                        ),
                        whole,
                    )
                )
            }
        }
    }

    fn call_dataframe_method(
        &mut self,
        dataframe: DataFrameRef,
        name: &str,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            // =====================================================
            // column()
            // =====================================================
            "column" => {
                if args.len() != 1 {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "column() expects exactly 1 argument",
                        whole,
                    ));
                }

                let column_name =
                    match &args[0] {
                        Value::Str(name) =>
                            name.as_ref(),

                        other => {
                            return Err(self.error(
                                ErrorKind::Type,
                                format!(
                                    "column() expects Str, got {}",
                                    other.type_name()
                                ),
                                whole,
                            ));
                        }
                    };

                let series =
                    dataframe
                        .column(column_name)
                        .ok_or_else(|| {
                            self.error(
                                ErrorKind::Name,
                                format!(
                                    "unknown DataFrame column '{}'",
                                    column_name
                                ),
                                whole,
                            )
                        })?;

                Ok(ControlFlow::Value(
                    Value::Series(series)
                ))
            }

            // =====================================================
            // select()
            // =====================================================
            "select" => {
                if args.len() != 1 {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "select() expects exactly 1 argument",
                        whole,
                    ));
                }

                let names =
                    self.value_to_string_list(
                        &args[0],
                        whole,
                    )?;

                let selected =
                    dataframe
                        .select(&names)
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(ControlFlow::Value(
                    Value::DataFrame(
                        Rc::new(selected)
                    )
                ))
            }

            // =====================================================
            // to_matrix()
            // =====================================================
            "to_matrix" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "to_matrix() expects no arguments",
                        whole,
                    ));
                }

                let matrix =
                    dataframe
                        .to_matrix()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(ControlFlow::Value(
                    Value::Matrix(
                        Rc::new(
                            RefCell::new(matrix)
                        )
                    )
                ))
            }

            // =====================================================
            // head()
            // =====================================================
            "head" => {
                let n =
                    match args.as_slice() {
                        [] => 5,

                        [Value::Int(n)]
                            if *n >= 0 =>
                        {
                            *n as usize
                        }

                        [other] => {
                            return Err(self.error(
                                ErrorKind::Type,
                                format!(
                                    "head() expects non-negative Int, got {}",
                                    other.type_name()
                                ),
                                whole,
                            ));
                        }

                        _ => {
                            return Err(self.error(
                                ErrorKind::Arity,
                                "head() expects 0 or 1 argument",
                                whole,
                            ));
                        }
                    };

                let result =
                    dataframe
                        .head(n)
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(ControlFlow::Value(
                    Value::DataFrame(
                        Rc::new(result)
                    )
                ))
            }

            // =====================================================
            // filter()
            // =====================================================
            "filter" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "filter() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let predicate =
                    args.into_iter()
                        .next()
                        .unwrap();

                match predicate {
                    // =================================================
                    // Boolean Series mask
                    // =================================================

                    Value::Series(mask) => {
                        let mut keep =
                            Vec::with_capacity(
                                dataframe.nrows()
                            );

                        if mask.len()
                            != dataframe.nrows()
                        {
                            return Err(
                                self.error(
                                    ErrorKind::Index,
                                    format!(
                                        "filter mask length {} does not match DataFrame row count {}",
                                        mask.len(),
                                        dataframe.nrows()
                                    ),
                                    whole,
                                )
                            );
                        }

                        for value in mask.data() {
                            match value {
                                Value::Bool(value) =>
                                    keep.push(*value),

                                Value::Null => {
                                    // Missing boolean is treated as false.
                                    keep.push(false);
                                }

                                other => {
                                    return Err(
                                        self.error(
                                            ErrorKind::Type,
                                            format!(
                                                "filter mask must contain Bool or Null, got {}",
                                                other.type_name()
                                            ),
                                            whole,
                                        )
                                    );
                                }
                            }
                        }

                        let result =
                            dataframe
                                .filter_rows(&keep)
                                .map_err(|message| {
                                    self.error(
                                        ErrorKind::Runtime,
                                        message,
                                        whole,
                                    )
                                })?;

                        Ok(
                            ControlFlow::Value(
                                Value::DataFrame(
                                    Rc::new(result)
                                )
                            )
                        )
                    }

                    // =================================================
                    // Closure predicate
                    // =================================================

                    predicate => {
                        let mut keep =
                            Vec::with_capacity(
                                dataframe.nrows()
                            );

                        for index in 0..dataframe.nrows() {
                            let row =
                                dataframe
                                    .row(index)
                                    .ok_or_else(|| {
                                        self.error(
                                            ErrorKind::Index,
                                            format!(
                                                "row index out of bounds: {}",
                                                index
                                            ),
                                            whole,
                                        )
                                    })?;

                            let result =
                                self.call_predicate(
                                    &predicate,
                                    Value::Object(row),
                                    whole,
                                )?;

                            match result {
                                Value::Bool(value) =>
                                    keep.push(value),

                                Value::Null =>
                                    keep.push(false),

                                other => {
                                    return Err(
                                        self.error(
                                            ErrorKind::Type,
                                            format!(
                                                "filter predicate must return Bool or Null, got {}",
                                                other.type_name()
                                            ),
                                            whole,
                                        )
                                    );
                                }
                            }
                        }

                        let result =
                            dataframe
                                .filter_rows(&keep)
                                .map_err(|message| {
                                    self.error(
                                        ErrorKind::Runtime,
                                        message,
                                        whole,
                                    )
                                })?;

                        Ok(
                            ControlFlow::Value(
                                Value::DataFrame(
                                    Rc::new(result)
                                )
                            )
                        )
                    }
                }
            }

            // =====================================================
            // group_by()
            // =====================================================
            "group_by" => {
                if args.len() != 1 {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "group_by() expects exactly 1 argument",
                        whole,
                    ));
                }

                let column_name =
                    match &args[0] {
                        Value::Str(name) =>
                            name.as_ref(),

                        other => {
                            return Err(self.error(
                                ErrorKind::Type,
                                format!(
                                    "group_by() expects Str, got {}",
                                    other.type_name()
                                ),
                                whole,
                            ));
                        }
                    };

                let grouped =
                    GroupedDataFrame::from_column(
                        dataframe.clone(),
                        column_name,
                    )
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Runtime,
                            message,
                            whole,
                        )
                    })?;

                Ok(ControlFlow::Value(
                    Value::GroupedDataFrame(
                        Rc::new(grouped)
                    )
                ))
            }

            // =====================================================
            // drop()
            // =====================================================
            "drop" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "drop() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let names =
                    self.value_to_string_list(
                        &args[0],
                        whole,
                    )?;

                let result =
                    dataframe
                        .drop_columns(&names)
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::DataFrame(
                            Rc::new(result)
                        )
                    )
                )
            }

            // =====================================================
            // rename()
            // =====================================================
            "rename" => {
                if args.len() != 1 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "rename() expects exactly 1 argument",
                            whole,
                        )
                    );
                }

                let mapping =
                    self.value_to_string_dict(
                        &args[0],
                        whole,
                    )?;

                let result =
                    dataframe
                        .rename(&mapping)
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::DataFrame(
                            Rc::new(result)
                        )
                    )
                )
            }

            // =====================================================
            // sort()
            // =====================================================
            "sort" => {
                if args.is_empty()
                    || args.len() > 2
                {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "sort() expects 1 or 2 arguments",
                            whole,
                        )
                    );
                }

                let column =
                    match &args[0] {
                        Value::Str(name) =>
                            name.as_ref(),

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "sort() expects column name as Str, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let ascending =
                    match args.get(1) {
                        None => true,

                        Some(Value::Bool(value)) =>
                            *value,

                        Some(other) => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "sort() second argument must be Bool, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let result =
                    dataframe
                        .sort_by_column(
                            column,
                            ascending,
                        )
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::DataFrame(
                            Rc::new(result)
                        )
                    )
                )
            }

            // =====================================================
            // describe()
            // =====================================================
            "describe" => {
                if !args.is_empty() {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "describe() expects no arguments",
                            whole,
                        )
                    );
                }

                let result =
                    dataframe
                        .describe()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::DataFrame(
                            Rc::new(result)
                        )
                    )
                )
            }

            // =====================================================
            // crosstab()
            // =====================================================
            "crosstab" => {
                if args.len() != 2 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "crosstab() expects exactly 2 arguments",
                            whole,
                        )
                    );
                }

                let row_column =
                    match &args[0] {
                        Value::Str(name) =>
                            name.as_ref(),

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "crosstab() first argument must be Str, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let column_column =
                    match &args[1] {
                        Value::Str(name) =>
                            name.as_ref(),

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "crosstab() second argument must be Str, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let result =
                    dataframe
                        .crosstab(
                            row_column,
                            column_column,
                        )
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::DataFrame(
                            Rc::new(result)
                        )
                    )
                )
            }


            _ => {
                Err(self.error(
                    ErrorKind::Runtime,
                    format!(
                        "DataFrame has no method '{}'",
                        name
                    ),
                    whole,
                ))
            }
        }
    }

    /// Helper for `call_dataframe_method()`
    fn value_to_string_list(
        &self,
        value: &Value,
        whole: &Expr,
    ) -> Result<Vec<String>> {
        let list =
            match value {
                Value::List(list) =>
                    list.borrow(),

                other => {
                    return Err(
                        self.error(
                            ErrorKind::Type,
                            format!(
                                "expected List of Str, got {}",
                                other.type_name()
                            ),
                            whole,
                        )
                    );
                }
            };

        let mut result =
            Vec::with_capacity(
                list.len()
            );

        for value in list.iter() {
            match value {
                Value::Str(name) => {
                    result.push(
                        name.as_ref().clone()
                    );
                }

                other => {
                    return Err(
                        self.error(
                            ErrorKind::Type,
                            format!(
                                "expected Str in column list, got {}",
                                other.type_name()
                            ),
                            whole,
                        )
                    );
                }
            }
        }

        Ok(result)
    }

    fn value_to_string_dict(
        &self,
        value: &Value,
        whole: &Expr,
    ) -> Result<HashMap<String, String>> {
        let dict =
            match value {
                Value::Dict(dict) =>
                    dict.borrow(),

                other => {
                    return Err(
                        self.error(
                            ErrorKind::Type,
                            format!(
                                "expected Dict, got {}",
                                other.type_name()
                            ),
                            whole,
                        )
                    );
                }
            };

        let mut result =
            HashMap::new();

        for (key, value) in dict.iter() {
            let value =
                match value {
                    Value::Str(value) =>
                        value.as_ref().clone(),

                    other => {
                        return Err(
                            self.error(
                                ErrorKind::Type,
                                format!(
                                    "rename mapping value must be Str, got {}",
                                    other.type_name()
                                ),
                                whole,
                            )
                        );
                    }
                };

            result.insert(
                key.clone(),
                value,
            );
        }

        Ok(result)
    }

    fn call_grouped_dataframe_method(
        &mut self,
        grouped: GroupedDataFrameRef,
        name: &str,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match name {
            "count" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "count() expects no arguments",
                        whole,
                    ));
                }

                let result =
                    grouped
                        .count()
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(ControlFlow::Value(
                    Value::DataFrame(
                        Rc::new(result)
                    )
                ))
            }

            "mean"
            | "sum" => {
                if args.len() != 1 {
                    return Err(self.error(
                        ErrorKind::Arity,
                        format!(
                            "{}() expects exactly 1 argument",
                            name
                        ),
                        whole,
                    ));
                }

                let column =
                    match &args[0] {
                        Value::Str(name) =>
                            name.as_ref(),

                        other => {
                            return Err(self.error(
                                ErrorKind::Type,
                                format!(
                                    "{}() expects Str, got {}",
                                    name,
                                    other.type_name()
                                ),
                                whole,
                            ));
                        }
                    };

                let result =
                    match name {
                        "mean" =>
                            grouped.mean(column),

                        "sum" =>
                            grouped.sum(column),

                        _ =>
                            unreachable!(),
                    }
                    .map_err(|message| {
                        self.error(
                            ErrorKind::Runtime,
                            message,
                            whole,
                        )
                    })?;

                Ok(ControlFlow::Value(
                    Value::DataFrame(
                        Rc::new(result)
                    )
                ))
            }

            "aggregate" => {
                if args.len() != 2 {
                    return Err(
                        self.error(
                            ErrorKind::Arity,
                            "aggregate() expects 2 arguments",
                            whole,
                        )
                    );
                }

                let column =
                    match &args[0] {
                        Value::Str(name) =>
                            name.as_ref(),

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "aggregate() first argument must be Str, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                let functions =
                    self.value_to_string_list(
                        &args[1],
                        whole,
                    )?;

                let result =
                    grouped
                        .aggregate(
                            column,
                            &functions,
                        )
                        .map_err(|message| {
                            self.error(
                                ErrorKind::Runtime,
                                message,
                                whole,
                            )
                        })?;

                Ok(
                    ControlFlow::Value(
                        Value::DataFrame(
                            Rc::new(result)
                        )
                    )
                )
            }

            _ => {
                Err(self.error(
                    ErrorKind::Runtime,
                    format!(
                        "GroupedDataFrame has no method '{}'",
                        name
                    ),
                    whole,
                ))
            }
        }
    }

    fn error(
        &self,
        kind: ErrorKind,
        message: impl Into<String>,
        expr: &Expr
    ) -> Error {
        Error::new(
            kind,
            message, 
            Some(expr.span))
                .with_stack(&self.stack)
    }
}

/// Helper to wrap Value with Option
fn option_some(
    value: Value,
) -> Value {
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Option",
                "Some",
                vec![value],
            )
        )
    )
}

/// Helper to create Option.None
fn option_none() -> Value {
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Option",
                "None",
                vec![],
            )
        )
    )
}

/// Helper for `eval_match()`
fn match_pattern(
    pattern: &Pattern,
    value: &Value,
    bindings: &mut HashMap<String, Value>,
) -> std::result::Result<bool, String> {
    match pattern {
        Pattern::Wildcard => {
            Ok(true)
        }

        Pattern::Ident(name) => {
            if bindings.contains_key(name) {
                return Err(format!(
                    "duplicate binding '{}' in pattern",
                    name
                ));
            }

            bindings.insert(
                name.clone(),
                value.clone(),
            );

            Ok(true)
        }

        Pattern::Int(expected) => {
            Ok(
                matches!(
                    value,
                    Value::Int(actual)
                        if actual == expected
                )
            )
        }

        Pattern::Float(expected) => {
            Ok(
                matches!(
                    value,
                    Value::Float(actual)
                        if actual == expected
                )
            )
        }

        Pattern::Bool(expected) => {
            Ok(
                matches!(
                    value,
                    Value::Bool(actual)
                        if actual == expected
                )
            )
        }

        Pattern::Str(expected) => {
            Ok(
                matches!(
                    value,
                    Value::Str(actual)
                        if actual.as_ref() == expected
                )
            )
        }

        Pattern::Enum {
            path,
            fields,
        } => {
            match value {
                Value::EnumValue(enum_value) => {
                    match_enum_pattern(
                        path,
                        fields,
                        enum_value,
                        bindings,
                    )
                }

                _ => Ok(false),
            }
        }

        Pattern::Tuple(patterns) => {
            match value {
                Value::Tuple(tuple) => {
                    if tuple.len()
                        != patterns.len()
                    {
                        return Ok(false);
                    }

                    for (pattern, value)
                        in patterns
                            .iter()
                            .zip(tuple.iter())
                    {
                        if !match_pattern(
                            pattern,
                            value,
                            bindings,
                        )? {
                            return Ok(false);
                        }
                    }

                    Ok(true)
                }

                _ => Ok(false),
            }
        }
    
        Pattern::List(patterns) => {
            let Value::List(list) =
                value
            else {
                return Ok(false);
            };

            let values =
                list.borrow();

            if patterns.len()
                != values.len()
            {
                return Ok(false);
            }

            for (pattern, value)
                in patterns.iter()
                    .zip(values.iter())
            {
                if !match_pattern(
                    pattern,
                    value,
                    bindings,
                )? {
                    return Ok(false);
                }
            }

            Ok(true)
        }

    }
}

/// Helper for `match_pattern()`
fn match_enum_pattern(
    path: &[String],
    patterns: &[Pattern],
    value: &EnumValueRef,
    bindings: &mut HashMap<String, Value>,
) -> std::result::Result<bool, String> {
    if path.len() != 2 {
        return Err(
            "enum pattern expects Enum.Variant"
                .into()
        );
    }

    let enum_name =
        &path[0];

    let variant_name =
        &path[1];

    if value.enum_name()
        != enum_name
    {
        return Ok(false);
    }

    if value.variant()
        != variant_name
    {
        return Ok(false);
    }

    if value.fields().len()
        != patterns.len()
    {
        return Err(format!(
            "enum variant '{}' expects {} pattern arguments, got {}",
            variant_name,
            value.fields().len(),
            patterns.len()
        ));
    }

    for (pattern, field)
        in patterns
            .iter()
            .zip(value.fields())
    {
        if !match_pattern(
            pattern,
            field,
            bindings,
        )? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Helper for `eval_for()`
fn collect_pattern_names(
    pattern: &Pattern,
    names: &mut Vec<String>,
) {
    match pattern {
        Pattern::Wildcard
        | Pattern::Int(_)
        | Pattern::Float(_)
        | Pattern::Bool(_)
        | Pattern::Str(_) => {}

        Pattern::Ident(name) => {
            names.push(name.clone());
        }

        Pattern::Tuple(patterns) |
        Pattern::List(patterns) => {
            for pattern in patterns {
                collect_pattern_names(
                    pattern,
                    names,
                );
            }
        }

        Pattern::Enum {
            fields,
            ..
        } => {
            for pattern in fields {
                collect_pattern_names(
                    pattern,
                    names,
                );
            }
        }
    }
}

