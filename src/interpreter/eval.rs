use crate::{
    stdlib,
    interpreter::operator, 
    Lexer, 
    Parser, 
    error::{
        Error, 
        ErrorKind, 
        Result, 
        StackFrame
    }, 
    runtime::{
        Env, 
        Value,
        Function, 
        ControlFlow, 
        IteratorObj,
        List,
        Module, 
        ModuleContext, 
        ModulePath, 
        ModuleRef,
        ObjectRef,
        StructDefinition, 
        SeriesRef, 
        DataFrameRef,
        GroupedDataFrame,
        GroupedDataFrameRef,
        BoundMethod,
        MethodReceiver,
    }, 
    syntax::{
        BinOp, 
        Expr, 
        ExprKind, 
        IndexExpr, 
        ListItem, 
        Program
    },
};
use std::{
    cell::RefCell, 
    path::PathBuf, 
    rc::Rc,
    collections::HashMap,
};

pub struct Interpreter {
    env: Env,
    stack: Vec<StackFrame>,
    loop_depth: usize,
    function_depth: usize,
    module_stack: Vec<ModuleContext>,
    project_root: PathBuf,
}

impl Default for Interpreter {
    fn default() -> Self { Self::new() }
}

impl Interpreter {
    pub fn new() -> Self {
        let env = Env::global();

        let project_root = std::env::current_dir()
            .expect(
                "failed to determine current directory"
            );

        let interpreter = Self { 
            env,
            stack: Vec::new(),
            loop_depth: 0,
            function_depth: 0,
            module_stack: Vec::new(),
            project_root,
        };

        for (name, value) 
            in stdlib::builtins() 
        {
            interpreter.env.define(name, value);
        }
        
        interpreter
    }

    pub fn eval_program(&mut self, program: &Program) -> Result<ControlFlow> {
        let mut last = Value::Unit;
        for expr in &program.statements {
            match self.eval(expr)? {
                ControlFlow::Value(v) => last = v,
                ControlFlow::Return(_) => return Err(self.error(ErrorKind::Control, "return outside function", expr)),
                ControlFlow::Break => return Err(self.error(ErrorKind::Control, "break outside loop", expr)),
            }
        }
        Ok(ControlFlow::Value(last))
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<ControlFlow> {
        use ExprKind::*;
        match &expr.kind {
            Int(n) => Ok(ControlFlow::Value(Value::Int(*n))),
            Float(n) => Ok(ControlFlow::Value(Value::Float(*n))),
            Str(s) => Ok(ControlFlow::Value(Value::Str(Rc::new(s.clone())))),
            Bool(v) => Ok(ControlFlow::Value(Value::Bool(*v))),
            Ident(name) => self.lookup(name, expr),
            List(items) => self.eval_list(items, expr),
            Dict(entries) => self.eval_dict(entries, expr),

            StructDecl {
                name,
                fields,
                methods,
            } => {
                self.eval_struct_decl(
                    name,
                    fields,
                    methods,
                    expr,
                )
            }

            Import(parts) => self.eval_import(parts, expr),

            Let(name, rhs) => {
                if self.env.contains_local(name) {
                    return Err(self.error(
                        ErrorKind::Name,
                        format!("{} is already defined in this scope", name),
                        expr,
                    ));
                }

                let mut value = self.eval_value(rhs)?;

                if let Value::Func(func) = &value {
                    if func.name.is_none() {
                        value = Value::Func(Rc::new(Function {
                            name: Some(name.clone()),
                            params: func.params.clone(),
                            body: func.body.clone(),
                            closure: func.closure.clone(),
                        }));
                    }
                }

                self.env.define(name.clone(), value.clone());

                Ok(ControlFlow::Value(value))
            }
            Assign(name, rhs) => {
                let mut value = self.eval_value(rhs)?;
                if let Value::Func(func) = &value {
                    if func.name.is_none() {
                        value = Value::Func(Rc::new(Function {
                            name: Some(name.clone()),
                            params: func.params.clone(),
                            body: func.body.clone(),
                            closure: func.closure.clone(),
                        }));
                    }
                }
                if !self.env.assign(name, value.clone()) {
                    self.env.define(name.clone(), value.clone());
                }
                Ok(ControlFlow::Value(value))
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
                        self.attach_runtime_error(
                            e,
                            expr,
                        )
                    })
            }

            Neg(e) => {
                self.eval_value(e)?.negate()
                    .map(ControlFlow::Value)
                    .map_err(|msg| self.attach(Error::new(ErrorKind::Type, msg, Some(expr.span)), expr))
            }

            Not(e) => {
                let v = self.eval_value(e)?;
                match v {
                    Value::Bool(v) => Ok(ControlFlow::Value(Value::Bool(!v))),
                    v => Err(self.error(ErrorKind::Type, format!("'not' expects Bool, got {}", v.type_name()), expr)),
                }
            }

            If(cond, then_branch, else_branch) => {
                match self.eval_value(cond)? {
                    Value::Bool(true) => self.eval(then_branch),
                    Value::Bool(false) => match else_branch { Some(e) => self.eval(e), None => Ok(ControlFlow::Value(Value::Unit)) },
                    v => Err(self.error(ErrorKind::Type, format!("'if' expects Bool, got {}", v.type_name()), expr)),
                }
            }

            While(cond, body) => self.eval_while(cond, body, expr),
            Break => {
                if self.loop_depth == 0 { Err(self.error(ErrorKind::Control, "break outside loop", expr)) } else { Ok(ControlFlow::Break) }
            }
            Return(value) => {
                if self.function_depth == 0 { return Err(self.error(ErrorKind::Control, "return outside function", expr)); }
                let v = match value { Some(v) => self.eval_value(v)?, None => Value::Unit };
                Ok(ControlFlow::Return(v))
            }
            For(name, index, body) => self.eval_for(name, index, body, expr),

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

            Null => Ok(ControlFlow::Value(Value::Null))
        }
    }

    fn eval_value(&mut self, expr: &Expr) -> Result<Value> {
        match self.eval(expr)? {
            ControlFlow::Value(v) => Ok(v),
            ControlFlow::Return(v) => Ok(v),
            ControlFlow::Break => Err(self.error(ErrorKind::Control, "break cannot appear here", expr)),
        }
    }

    fn lookup(&self, name: &str, expr: &Expr) -> Result<ControlFlow> {
        self.env.get(name)
            .map(ControlFlow::Value)
            .ok_or_else(|| self.error(ErrorKind::Name, format!("{} is undefined", name), expr))
    }

    fn eval_list(&mut self, items: &[ListItem], expr: &Expr) -> Result<ControlFlow> {
        let mut values = Vec::new();
        for item in items {
            match item {
                ListItem::Expr(e) => values.push(self.eval_value(e)?),
                ListItem::Range(range) => {
                    let mut it = self.eval_iterable(range, expr)?;
                    while let Some(v) = it.next() { values.push(v); }
                }
            }
        }
        Ok(ControlFlow::Value(Value::List(Rc::new(RefCell::new(values)))))
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
        name: &str,
        fields: &[String],
        methods: &[(String, Box<Expr>)],
        expr: &Expr,
    ) -> Result<ControlFlow> {
        if self.env.contains_local(name) {
            return Err(self.error(
                ErrorKind::Name,
                format!(
                    "struct '{}' is already defined in this scope",
                    name
                ),
                expr,
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

        let definition = StructDefinition::new(
            name.to_owned(),
            fields.to_vec(),
            method_map,
        );

        self.env.define(
            name.to_owned(),
            Value::Struct(Rc::new(definition)),
        );

        Ok(ControlFlow::Value(Value::Unit))
    }

    fn resolve_module_path(
        &self,
        requested: &ModulePath,
        whole: &Expr,
    ) -> Result<PathBuf> {
        let path = if let Some(current) =
            self.module_stack.last()
        {
            // Import from another module:
            //
            // /project/tests/modules/a.nv
            //
            // import b
            //
            // -> /project/tests/modules/b.nv

            let parent =
                current
                    .file_path
                    .parent()
                    .ok_or_else(|| {
                        self.error(
                            ErrorKind::Runtime,
                            "module file has no parent directory",
                            whole,
                        )
                    })?;

            let mut path =
                parent.to_path_buf();

            for part in requested.parts() {
                path.push(part);
            }

            path.set_extension("nv");

            path
        } else {
            // Import from main program:
            //
            // import tests.modules.a
            //
            // -> project/tests/modules/a.nv

            let mut path =
                self.project_root.clone();

            for part in requested.parts() {
                path.push(part);
            }

            path.set_extension("nv");

            path
        };

        if !path.is_file() {
            return Err(
                self.error(
                    ErrorKind::Name,
                    format!(
                        "module '{}' not found at '{}'",
                        requested.name(),
                        path.display()
                    ),
                    whole,
                )
            );
        }

        Ok(path)
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
                    ErrorKind::Runtime,
                    "cannot bind empty module path",
                    whole,
                )
            );
        }

        // ---------------------------------------------------------
        // Root namespace
        // ---------------------------------------------------------

        let root_name =
            &parts[0];

        let mut current =
            match self.env.get(root_name) {
                Some(Value::Module(module)) => {
                    module
                }

                Some(other) => {
                    return Err(
                        self.error(
                            ErrorKind::Name,
                            format!(
                                "cannot create module namespace '{}': name already refers to {}",
                                root_name,
                                other.type_name()
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
                                    root_name.clone()
                                )
                            )
                        );

                    self.env.define(
                        root_name.clone(),
                        Value::Module(
                            module.clone()
                        ),
                    );

                    module
                }
            };

        // ---------------------------------------------------------
        // Intermediate namespaces
        // ---------------------------------------------------------

        for part in
            &parts[1..parts.len() - 1]
        {
            let next =
                current.borrow().get(part);

            current =
                match next {
                    Some(Value::Module(module)) =>
                        module,

                    Some(other) => {
                        return Err(
                            self.error(
                                ErrorKind::Name,
                                format!(
                                    "module namespace '{}' is already occupied by {}",
                                    part,
                                    other.type_name()
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
                                        part.clone()
                                    )
                                )
                            );

                        current
                            .borrow_mut()
                            .set(
                                part.clone(),
                                Value::Module(
                                    module.clone()
                                ),
                            );

                        module
                    }
                };
        }

        // ---------------------------------------------------------
        // Final component
        // ---------------------------------------------------------

        let final_name =
            parts.last()
                .unwrap();

        current.borrow_mut().set(
            final_name.clone(),
            Value::Module(module),
        );

        Ok(())
    }

    fn eval_import(
        &mut self,
        parts: &[String],
        whole: &Expr,
    ) -> Result<ControlFlow> {
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
        // Standard library module
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
        // Resolve physical file
        // =========================================================

        let path =
            self.resolve_module_path(
                &requested,
                whole,
            )?;

        let canonical =
            std::fs::canonicalize(&path)
                .map_err(|error| {
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "failed to resolve module '{}': {}",
                            path.display(),
                            error
                        ),
                        whole,
                    )
                })?;

        // =========================================================
        // Cyclic import detection
        // =========================================================

        if self
            .module_stack
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
        // Read source
        // =========================================================

        let source =
            std::fs::read_to_string(&canonical)
                .map_err(|error| {
                    self.error(
                        ErrorKind::Runtime,
                        format!(
                            "failed to read module '{}': {}",
                            canonical.display(),
                            error
                        ),
                        whole,
                    )
                })?;

        // =========================================================
        // Create module environment
        // =========================================================

        let module_env =
            self.env.child();

        let previous_env =
            std::mem::replace(
                &mut self.env,
                module_env,
            );

        // =========================================================
        // Push module context
        // =========================================================

        self.module_stack.push(
            ModuleContext::new(
                requested.clone(),
                canonical.clone(),
            )
        );

        // =========================================================
        // Evaluate module
        // =========================================================

        let result =
            self.execute_source(&source);

        // Always restore interpreter state.
        self.module_stack.pop();

        let module_env =
            std::mem::replace(
                &mut self.env,
                previous_env,
            );

        result?;

        // =========================================================
        // Build runtime Module
        // =========================================================

        let mut module =
            Module::new(
                requested.name()
            );

        for (name, value)
            in module_env.local_values()
        {
            module.set(
                name,
                value,
            );
        }

        let module =
            Rc::new(
                RefCell::new(module)
            );

        // =========================================================
        // Insert into nested module namespace
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
                let key =
                    match self.eval_value(index)? {
                        Value::Str(s) =>
                            s.as_ref().clone(),

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "dictionary index expects Str, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

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
                    match value {
                        Value::Int(v) =>
                            v as f64,

                        Value::Float(v) =>
                            v,

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "Matrix value must be numeric, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

                matrix
                    .borrow_mut()
                    .set(
                        row,
                        col,
                        numeric,
                    )
                    .map_err(|message| {
                        self.attach(
                            Error::new(
                                ErrorKind::Runtime,
                                message,
                                None,
                            ),
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
                match self.eval_value(expr)? {
                    Value::Int(v) if v >= 0 => {
                        Ok(v as usize)
                    }

                    Value::Int(_) => {
                        Err(self.error(
                            ErrorKind::Index,
                            format!(
                                "negative Matrix {} index",
                                axis
                            ),
                            whole,
                        ))
                    }

                    other => {
                        Err(self.error(
                            ErrorKind::Type,
                            format!(
                                "Matrix {} index must be Int, got {}",
                                axis,
                                other.type_name()
                            ),
                            whole,
                        ))
                    }
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
                let key =
                    match self.eval_value(index)? {
                        Value::Str(s) =>
                            s.as_ref().clone(),

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "dictionary index expects Str, got {}",
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    };

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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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

            (
                Value::Matrix(_),
                IndexExpr::Tuple(_),
            ) => {
                unreachable!(
                    "Matrix tuple indexing should have been handled above"
                )
            }

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
        match self.eval_value(expr)? {
            Value::Int(i) if i >= 0 => Ok(i as usize),
            Value::Int(_) => Err(self.error(ErrorKind::Index, "negative index is not supported", whole)),
            other => Err(self.error(ErrorKind::Type, format!("index expects Int, got {}", other.type_name()), whole)),
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
                    match self.eval_value(expr)? {
                        Value::Int(v) if v >= 0 => {
                            v as usize
                        }

                        Value::Int(_) => {
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

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "Matrix {} slice start must be Int, got {}",
                                        axis,
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
                    }
                }

                None => 0,
            };

        let end_value =
            match end {
                Some(expr) => {
                    match self.eval_value(expr)? {
                        Value::Int(v) if v >= 0 => {
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
                        }

                        Value::Int(_) => {
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

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "Matrix {} slice end must be Int, got {}",
                                        axis,
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
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
                let value =
                    match self.eval_value(expr)? {
                        Value::Int(v) if v >= 0 => {
                            v as usize
                        }

                        Value::Int(_) => {
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
                        }

                        other => {
                            return Err(
                                self.error(
                                    ErrorKind::Type,
                                    format!(
                                        "Matrix {} index must be Int, got {}",
                                        axis,
                                        other.type_name()
                                    ),
                                    whole,
                                )
                            );
                        }
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

    fn resolve_slice(&mut self, start: Option<&Expr>, end: Option<&Expr>, inclusive: bool, len: usize, whole: &Expr) -> Result<(usize,usize)> {
        let s = match start { Some(e) => self.eval_index_int(e, whole)?, None => 0 };
        let mut e = match end { Some(e) => self.eval_index_int(e, whole)?, None => len };
        if inclusive {
            e = e.checked_add(1).ok_or_else(|| self.error(ErrorKind::Overflow, "slice endpoint overflow", whole))?;
        }
        if s > e || e > len { return Err(self.error(ErrorKind::Index, "invalid slice range", whole)); }
        Ok((s,e))
    }

    fn eval_iterable(
        &mut self,
        index: &IndexExpr,
        whole: &Expr,
    ) -> Result<IteratorObj> {
        match index {
            IndexExpr::Single(expr) => {
                match self.eval_value(expr)? {
                    Value::Iterator(it) => Ok(it),

                    Value::List(data) => {
                        Ok(IteratorObj::List {
                            data,
                            index: 0,
                        })
                    }

                    Value::Str(s) => {
                        Ok(IteratorObj::Str {
                            data: Rc::new(
                                s.chars().collect()
                            ),
                            index: 0,
                        })
                    }

                    other => Err(self.error(
                        ErrorKind::Type,
                        format!(
                            "{} is not iterable",
                            other.type_name()
                        ),
                        whole,
                    )),
                }
            }

            IndexExpr::Range {
                start,
                end,
                inclusive,
            } => {
                let start_value =
                    match start {
                        Some(expr) => {
                            match self.eval_value(expr)? {
                                Value::Int(v) if v >= 0 => v,

                                Value::Int(_) => {
                                    return Err(
                                        self.error(
                                            ErrorKind::Index,
                                            "negative range start",
                                            whole,
                                        )
                                    );
                                }

                                other => {
                                    return Err(
                                        self.error(
                                            ErrorKind::Type,
                                            format!(
                                                "range expects Int, got {}",
                                                other.type_name()
                                            ),
                                            whole,
                                        )
                                    );
                                }
                            }
                        }

                        None => 0,
                    };

                let mut end_value =
                    match end {
                        Some(expr) => {
                            match self.eval_value(expr)? {
                                Value::Int(v) => v,

                                other => {
                                    return Err(
                                        self.error(
                                            ErrorKind::Type,
                                            format!(
                                                "range expects Int, got {}",
                                                other.type_name()
                                            ),
                                            whole,
                                        )
                                    );
                                }
                            }
                        }

                        None => i64::MAX,
                    };

                if *inclusive {
                    end_value =
                        end_value
                            .checked_add(1)
                            .ok_or_else(|| {
                                self.error(
                                    ErrorKind::Overflow,
                                    "inclusive range endpoint overflow",
                                    whole,
                                )
                            })?;
                }

                Ok(IteratorObj::Range {
                    current: start_value,
                    end: end_value,
                })
            }

            IndexExpr::Tuple(_) => {
                Err(self.error(
                    ErrorKind::Type,
                    "tuple index is not iterable",
                    whole,
                ))
            }
        }
    }

    fn eval_while(&mut self, cond: &Expr, body: &Expr, whole: &Expr) -> Result<ControlFlow> {
        let mut last = Value::Bool(false);
        self.loop_depth += 1;
        let result = (|| {
            loop {
                match self.eval_value(cond)? {
                    Value::Bool(true) => match self.eval(body)? {
                        ControlFlow::Value(v) => last=v,
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                    }
                    Value::Bool(false) => break,
                    other => return Err(self.error(ErrorKind::Type, format!("'while' expects Bool, got {}",other.type_name()),whole)),
                }
            }
            Ok(ControlFlow::Value(last))
        })();
        self.loop_depth -= 1;
        result
    }

    fn eval_for(&mut self, name: &str, iterable: &IndexExpr, body: &Expr, whole: &Expr) -> Result<ControlFlow> {
        let mut iterator = self.eval_iterable(iterable, whole)?;
        let old_env = self.env.clone();
        self.env = self.env.child();
        self.loop_depth += 1;
        let result = (|| {
            let mut last = Value::Unit;
            while let Some(value) = iterator.next() {
                self.env.define(name, value);
                match self.eval(body)? {
                    ControlFlow::Value(v) => last=v,
                    ControlFlow::Break => break,
                    ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                }
            }
            Ok(ControlFlow::Value(last))
        })();
        self.loop_depth -= 1;
        self.env = old_env;
        result
    }

    fn eval_block(&mut self, exprs: &[Expr], scoped: bool) -> Result<ControlFlow> {
        let old_env = self.env.clone();
        if scoped { self.env = self.env.child(); }
        let result = (|| {
            let mut last = Value::Unit;
            for expr in exprs {
                match self.eval(expr)? {
                    ControlFlow::Value(v) => last=v,
                    other => return Ok(other),
                }
            }
            Ok(ControlFlow::Value(last))
        })();
        if scoped { self.env = old_env; }
        result
    }

    fn eval_and(&mut self, lhs: &Expr, rhs: &Expr, whole: &Expr) -> Result<ControlFlow> {
        match self.eval_value(lhs)? {
            Value::Bool(false) => Ok(ControlFlow::Value(Value::Bool(false))),
            Value::Bool(true) => match self.eval_value(rhs)? { Value::Bool(v)=>Ok(ControlFlow::Value(Value::Bool(v))), other=>Err(self.error(ErrorKind::Type,format!("'and' expects Bool, got {}",other.type_name()),whole)) },
            other => Err(self.error(ErrorKind::Type,format!("'and' expects Bool, got {}",other.type_name()),whole)),
        }
    }

    fn eval_or(&mut self, lhs: &Expr, rhs: &Expr, whole: &Expr) -> Result<ControlFlow> {
        match self.eval_value(lhs)? {
            Value::Bool(true) => Ok(ControlFlow::Value(Value::Bool(true))),
            Value::Bool(false) => match self.eval_value(rhs)? { Value::Bool(v)=>Ok(ControlFlow::Value(Value::Bool(v))), other=>Err(self.error(ErrorKind::Type,format!("'or' expects Bool, got {}",other.type_name()),whole)) },
            other => Err(self.error(ErrorKind::Type,format!("'or' expects Bool, got {}",other.type_name()),whole)),
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
                        self.attach(
                            Error::new(
                                ErrorKind::Runtime,
                                message,
                                None,
                            ),
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
            Value::List(list) => {
                Ok(ControlFlow::Value(
                    Value::BoundMethod(
                        BoundMethod::new(
                            MethodReceiver::List(
                                list.clone()
                            ),
                            name,
                        )
                    )
                ))
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

                module
                    .get(name)
                    .map(ControlFlow::Value)
                    .ok_or_else(|| {
                        self.error(
                            ErrorKind::Runtime,
                            format!(
                                "module '{}' has no member '{}'",
                                module.name(),
                                name
                            ),
                            whole,
                        )
                    })
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
                    | "to_matrix" => {
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
                    | "to_matrix" => {
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
        call_site: &Expr
    ) -> Result<ControlFlow> {
        if func.params.len() != args.len() {
            return Err(self.error(ErrorKind::Arity, format!("function expects {} arguments, got {}", func.params.len(), args.len()), call_site));
        }
        self.stack.push(StackFrame { function: func.name.clone().unwrap_or_else(|| "<lambda>".into()), span: Some(call_site.span) });
        let old_env = self.env.clone();
        let call_env = func.closure.child();
        for (name,value) in func.params.iter().zip(args) { call_env.define(name.clone(), value); }
        self.env = call_env;
        self.function_depth += 1;
        let result = self.eval(&func.body);
        self.function_depth -= 1;
        self.env = old_env;
        self.stack.pop();
        match result? {
            ControlFlow::Value(v) | ControlFlow::Return(v) => Ok(ControlFlow::Value(v)),
            ControlFlow::Break => Err(self.error(ErrorKind::Control,"break outside loop",call_site)),
        }
    }

    fn call_bound_method(
        &mut self,
        method: BoundMethod,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        match method.receiver() {
            MethodReceiver::List(list) => {
                self.call_list_method(
                    list.clone(),
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
                    return Err(self.error(
                        ErrorKind::Arity,
                        "push() takes exactly 1 argument",
                        whole,
                    ));
                }

                let value =
                    args.pop().unwrap();

                list.borrow_mut().push(value);

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
                    return Err(self.error(
                        ErrorKind::Arity,
                        "pop() takes no arguments",
                        whole,
                    ));
                }

                let value =
                    list.borrow_mut()
                        .pop()
                        .unwrap_or(Value::Unit);

                Ok(
                    ControlFlow::Value(value)
                )
            }

            // =====================================================
            // remove(index)
            // =====================================================

            "remove" => {
                if args.len() != 1 {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "remove() takes exactly 1 argument",
                        whole,
                    ));
                }

                let index =
                    match args.pop().unwrap() {
                        Value::Int(index)
                            if index >= 0 =>
                        {
                            index as usize
                        }

                        Value::Int(_) => {
                            return Err(self.error(
                                ErrorKind::Index,
                                "remove() does not accept negative indices",
                                whole,
                            ));
                        }

                        other => {
                            return Err(self.error(
                                ErrorKind::Type,
                                format!(
                                    "remove() expects Int, got {}",
                                    other.type_name()
                                ),
                                whole,
                            ));
                        }
                    };

                let mut list =
                    list.borrow_mut();

                if index >= list.len() {
                    return Err(self.error(
                        ErrorKind::Index,
                        format!(
                            "index out of range: {}",
                            index
                        ),
                        whole,
                    ));
                }

                let value =
                    list.remove(index);

                Ok(
                    ControlFlow::Value(value)
                )
            }

            // =====================================================
            // len()
            // =====================================================

            "len" => {
                if !args.is_empty() {
                    return Err(self.error(
                        ErrorKind::Arity,
                        "len() takes no arguments",
                        whole,
                    ));
                }

                let len =
                    list.borrow().len();

                Ok(
                    ControlFlow::Value(
                        Value::Int(
                            len as i64
                        )
                    )
                )
            }

            // =====================================================
            // Unknown method
            // =====================================================

            _ => {
                Err(self.error(
                    ErrorKind::Runtime,
                    format!(
                        "unknown list method '{}'",
                        name
                    ),
                    whole,
                ))
            }
        }
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

    fn call_series_method(
        &mut self,
        series: SeriesRef,
        name: &str,
        args: Vec<Value>,
        whole: &Expr,
    ) -> Result<ControlFlow> {
        if !args.is_empty() {
            return Err(
                self.error(
                    ErrorKind::Arity,
                    format!(
                        "{}() expects no arguments",
                        name
                    ),
                    whole,
                )
            );
        }

        match name {
            "to_list" => {
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
                let matrix =
                    series.to_matrix()
                        .map_err(|message| {
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                    return Err(self.error(
                        ErrorKind::Arity,
                        "filter() expects exactly 1 argument",
                        whole,
                    ));
                }

                let predicate =
                    args.into_iter()
                        .next()
                        .unwrap();

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
                        Value::Bool(true) =>
                            keep.push(true),

                        Value::Bool(false) =>
                            keep.push(false),

                        other => {
                            return Err(self.error(
                                ErrorKind::Type,
                                format!(
                                    "DataFrame filter predicate must return Bool, got {}",
                                    other.type_name()
                                ),
                                whole,
                            ));
                        }
                    }
                }

                let result =
                    dataframe
                        .filter_rows(&keep)
                        .map_err(|message| {
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                        self.attach(
                            Error::new(
                                ErrorKind::Runtime,
                                message,
                                None,
                            ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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
                        self.attach(
                            Error::new(
                                ErrorKind::Runtime,
                                message,
                                None,
                            ),
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
                            self.attach(
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                ),
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

    fn error(&self, kind: ErrorKind, message: impl Into<String>, expr: &Expr) -> Error {
        Error::new(kind, message, Some(expr.span)).with_stack(&self.stack)
    }

    fn attach(&self, mut error: Error, expr: &Expr) -> Error {
        if error.span.is_none() { error.span = Some(expr.span); }
        if error.stack.is_empty() { error.stack = self.stack.clone(); }
        error
    }

    /// Helper to convert String to Error
    fn attach_runtime_error(
        &self,
        message: impl Into<String>,
        expr: &Expr,
    ) -> Error {
        self.attach(
            Error::new(
                ErrorKind::Runtime,
                message,
                None,
            ),
            expr,
        )
    }

    fn execute_source(
        &mut self,
        source: &str,
    ) -> Result<ControlFlow> {
        let mut lexer =
            Lexer::new(source);

        let tokens =
            lexer.lex()
                .map_err(|e| e)?;

        let mut parser =
            Parser::new(tokens);

        let program =
            parser.parse()?;

        self.eval_program(&program)
    }

}
