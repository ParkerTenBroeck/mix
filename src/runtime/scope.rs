use dumpster::{Trace, unsync::Gc};

use crate::{
	HashMap,
	runtime::{
		LazyValue,
		builtin::BuiltinsBuilder,
		value::{StringKind, Value},
	},
};

#[derive(Clone, Default, Debug, Trace)]
pub struct Scope(Gc<ScopeInner>);

#[derive(Clone, Default, Debug)]
struct ScopeLevel {
	attrs: HashMap<StringKind, LazyValue>,
	// created_at: Option<CodePos>,
}

unsafe impl<Z: dumpster::Visitor> dumpster::TraceWith<Z> for ScopeLevel {
	fn accept(&self, visitor: &mut Z) -> Result<(), ()> {
		for value in self.attrs.values() {
			value.accept(visitor)?;
		}
		Ok(())
	}
}

#[derive(Clone, Default, Debug, Trace)]
struct ScopeInner {
	curr: ScopeLevel,
	prev: Option<Scope>,
}

impl Scope {
	fn new(curr: ScopeLevel) -> Self {
		Self(Gc::new(ScopeInner { curr, prev: None }))
	}

	fn new_with(curr: ScopeLevel, prev: Scope) -> Self {
		Self(Gc::new(ScopeInner {
			curr,
			prev: Some(prev),
		}))
	}

	pub fn resolve(&self, name: &str) -> Option<&LazyValue> {
		if let Some(resolved) = self.0.curr.attrs.get(name) {
			return Some(resolved);
		}
		if let Some(prev) = &self.0.prev {
			return prev.resolve(name);
		}
		None
	}

	pub fn new_level(&self) -> Scope {
		Self::new_with(ScopeLevel::default(), self.clone())
	}

	pub fn parent(&self) -> Option<Scope> {
		self.0.prev.clone()
	}

	pub fn bind(&mut self, ident: StringKind, value: LazyValue) -> Option<LazyValue> {
		Gc::make_mut(&mut self.0).curr.attrs.insert(ident, value)
	}
}

#[derive(Debug, Default)]
pub struct ScopeBuilder {
	scope: ScopeLevel,
}

impl ScopeBuilder {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn with_builtins(mut self) -> Self {
		self.add_builtins(true);
		self
	}

	/// Add the standard builtins, optionally exposing the filesystem-backed
	/// `builtins.import` function.
	pub fn with_builtins_and_imports(mut self, allow_imports: bool) -> Self {
		self.add_builtins(allow_imports);
		self
	}

	fn add_builtins(&mut self, allow_imports: bool) {
		let builtins = BuiltinsBuilder::new().allow_imports(allow_imports).build();

		self.scope.attrs.insert(
			StringKind::String("builtins".into()),
			Value::AttrSet(builtins).into(),
		);
	}

	pub fn with(mut self, key: impl Into<StringKind>, value: impl Into<LazyValue>) -> Self {
		self.scope.attrs.insert(key.into(), value.into());
		self
	}

	pub fn bottom(self) -> Scope {
		Scope::new(self.scope)
	}
}
