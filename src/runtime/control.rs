use super::Value;

#[derive(Debug, Clone)]
pub enum ControlFlow {
    Value(Value),
    Return(Value),
    Break,
    Continue,
}
