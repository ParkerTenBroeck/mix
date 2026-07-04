use dumpster::Trace;

use crate::runtime::AttrSet;

#[derive(Clone, Default, Debug, Trace)]
pub struct Scope {
    pub curr: AttrSet,
    pub prev: Option<Box<Scope>>,
}

impl Scope {
    pub fn new(curr: AttrSet, prev: Scope) -> Self {
        Self {
            curr,
            prev: Some(Box::new(prev)),
        }
    }

    pub fn bottom(curr: AttrSet) -> Self {
        Scope { curr, prev: None }
    }
}
