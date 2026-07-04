use dumpster::Trace;

use crate::runtime::{AttrSet, LazyValue};

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


#[derive(Debug, Default)]
pub struct ScopeBuilder{
    scope: AttrSet
}

impl ScopeBuilder{
    pub fn new() -> Self{
        Default::default()
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<LazyValue>) -> Self {
        self.scope.get_mut().insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> Scope{
        Scope::bottom(self.scope)
    }
}