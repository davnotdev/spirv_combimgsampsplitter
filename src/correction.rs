#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CorrectionType {
    SplitCombined = 0,
    SplitDrefRegular = 1,
    SplitDrefComparison = 2,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SetBinding(pub u32, pub u32);

#[derive(Debug, Clone)]
pub struct CorrectionBinding {
    pub binding: u32,
    pub corrections: Vec<CorrectionType>,
}

#[derive(Debug, Clone)]
pub struct CorrectionSet {
    pub set: u32,
    pub bindings: Vec<CorrectionBinding>,
}

#[derive(Debug, Clone, Default)]
pub struct CorrectionMap {
    pub sets: Vec<CorrectionSet>,
}
