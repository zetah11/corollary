use std::ops::Range;

use super::Register;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockId(pub(super) usize);

#[derive(Clone, Debug)]
pub struct Block {
    pub param: Option<Register>,
    pub insts: Range<usize>,
    pub branch: usize,
}
