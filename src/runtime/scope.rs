use std::ops::{Deref, DerefMut};

use dumpster::Trace;

use crate::runtime::{LazyValue, value::AttrSet};

#[derive(Clone, Default, Debug, Trace)]
pub struct Scope(AttrSet);

impl Scope {
    pub fn new(scope: AttrSet) -> Self {
        Self(scope)
    }

    pub fn resolve(&self, name: &str) -> Option<&LazyValue> {
        self.0.get(name)
    }
}

impl Deref for Scope {
    type Target = AttrSet;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Scope {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Default)]
pub struct ScopeBuilder {
    scope: AttrSet,
}

impl ScopeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<LazyValue>) -> Self {
        self.scope.get_mut().insert(key.into(), value.into());
        self
    }

    pub fn bottom(self) -> Scope {
        Scope::new(self.scope)
    }
}
