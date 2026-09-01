use super::{EnumDef, Value};

use std::rc::Rc;

pub fn option() -> Value {
    let mut def = EnumDef::new("Option");

    def.add_variant("Some", 1).expect("valid Option");

    def.add_variant("None", 0).expect("valid Option");

    Value::Enum(Rc::new(def))
}

pub fn result() -> Value {
    let mut def = EnumDef::new("Result");

    def.add_variant("Ok", 1).expect("valid Result");

    def.add_variant("Err", 1).expect("valid Result");

    Value::Enum(Rc::new(def))
}
