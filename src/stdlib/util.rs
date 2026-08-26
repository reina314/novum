use crate::{
    syntax::Pattern,
};

#[inline]
pub fn is_self_pattern(
    pattern: &Pattern,
) -> bool {
    matches!(
        pattern,
        Pattern::Ident(name)
            if name == "self"
    )
}

pub fn encode_class_counts(
    fields: usize,
    methods: usize,
) -> u32 {
    ((fields as u32) << 16)
        | methods as u32
}

pub fn decode_class_counts(
    operand: u32,
) -> (usize, usize) {
    (
        (operand >> 16) as usize,
        (operand & 0xffff) as usize,
    )
}