use super::*;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CorrectionType {
    SplitCombined = 0,
    SplitDrefRegular = 1,
    SplitDrefComparison = 2,
}

#[derive(Debug, Clone, Default)]
pub struct CorrectionBinding {
    pub corrections: Vec<CorrectionType>,
}

#[derive(Debug, Clone, Default)]
pub struct CorrectionSet {
    pub bindings: HashMap<u32, CorrectionBinding>,
}

#[derive(Debug, Clone, Default)]
pub struct CorrectionMap {
    pub sets: HashMap<u32, CorrectionSet>,
}
