use crate::{runtime::Env, syntax::{Expr, Pattern}};
use std::{fmt, rc::Rc};

pub type FuncRef = Rc<Function>;

#[derive(Clone)]
pub struct Function {
    pub name: Option<String>,
    pub params: Vec<Pattern>,
    pub body: Rc<Expr>,
    pub closure: Env,
}

impl Function {
    pub fn parameters(
        &self,
    ) -> &[Pattern] {
        &self.params
    }

    pub fn body(
        &self,
    ) -> &Expr {
        &self.body
    }

    pub fn closure(
        &self,
    ) -> Env {
        self.closure.clone()
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self, f) }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "<{name}>"),
            None => write!(f, "<lambda>"),
        }
    }
}
