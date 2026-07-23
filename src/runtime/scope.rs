use dumpster::{Trace, unsync::Gc};

use crate::runtime::{
	LazyValue,
	value::{AttrSetInner, StringKind},
};

#[derive(Clone, Default, Debug, Trace)]
pub struct Scope(Gc<ScopeInner>);

#[derive(Clone, Default, Debug, Trace)]
struct ScopeInner {
	curr: AttrSetInner,
	prev: Option<Scope>,
}

impl Scope {
	pub fn new(curr: AttrSetInner) -> Self {
		Self(Gc::new(ScopeInner { curr, prev: None }))
	}

	pub fn new_with(curr: AttrSetInner, prev: Scope) -> Self {
		Self(Gc::new(ScopeInner {
			curr,
			prev: Some(prev),
		}))
	}

	pub fn resolve(&self, name: &str) -> Option<&LazyValue> {
		if let Some(resolved) = self.0.curr.get(name) {
			return Some(resolved);
		}
		if let Some(prev) = &self.0.prev {
			return prev.resolve(name);
		}
		None
	}

	pub fn new_level(&self) -> Scope {
		Self::new_with(AttrSetInner::default(), self.clone())
	}

	pub fn bind(&mut self, ident: StringKind, value: LazyValue) -> Option<LazyValue> {
		Gc::make_mut(&mut self.0).curr.insert(ident, value)
	}
}

#[derive(Debug, Default)]
pub struct ScopeBuilder {
	scope: AttrSetInner,
}

impl ScopeBuilder {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn with(mut self, key: impl Into<StringKind>, value: impl Into<LazyValue>) -> Self {
		self.scope.insert(key.into(), value.into());
		self
	}

	pub fn bottom(self) -> Scope {
		Scope::new(self.scope)
	}
}
