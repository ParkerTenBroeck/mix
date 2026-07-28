use dumpster::{Trace, unsync::Gc};

use crate::runtime::{
	LazyValue,
	value::{AttrSet, AttrSetInner, Lambda, StringKind, Value},
	value::{NativeLambda},
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

	pub fn parent(&self) -> Option<Scope> {
		self.0.prev.clone()
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

	pub fn with_builtins(mut self) -> Self {
		use super::builtin::*;
		
		let mut builtins = AttrSet::new();
		builtins.get_mut().insert(
			StringKind::String("match".into()),
			Value::Lambda(Lambda::NativeLambda(NativeLambda::new(Match::new()))).into(),
		);

		builtins.get_mut().insert(
			StringKind::String("mkList".into()),
			Value::Lambda(Lambda::NativeLambda(NativeLambda::new(MkList::new()))).into(),
		);

		self.scope.insert(
			StringKind::String("builtins".into()),
			Value::AttrSet(builtins).into(),
		);

		self
	}

	pub fn with(mut self, key: impl Into<StringKind>, value: impl Into<LazyValue>) -> Self {
		self.scope.insert(key.into(), value.into());
		self
	}

	pub fn bottom(self) -> Scope {
		Scope::new(self.scope)
	}
}
