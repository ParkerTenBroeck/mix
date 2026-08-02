use std::{
	cell::Cell,
	ops::{Deref, DerefMut},
};

use dumpster::{Trace, unsync::Gc};

use crate::{
	HashMap,
	bytecode::CodePos,
	runtime::{
		lazy::{LazyValue, LazyValueKind},
		value::{DeepState, StringKind, Value},
	},
};

#[derive(Clone, Default, Trace)]
pub struct AttrSet {
	inner: Gc<AttrSetInner>,
}

impl std::fmt::Debug for AttrSet {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		(&*self.inner).fmt(f)
	}
}

#[derive(Clone, Default)]
struct AttrSetInner {
	attrs: HashMap<StringKind, LazyValue>,
	deep: Cell<DeepState>,
	created_at: Option<CodePos>,
}

impl Deref for AttrSetInner {
	type Target = HashMap<StringKind, LazyValue>;

	fn deref(&self) -> &Self::Target {
		&self.attrs
	}
}

impl DerefMut for AttrSetInner {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.attrs
	}
}

impl std::fmt::Debug for AttrSetInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_map()
			.entries(
				self.iter()
					.map(|(name, value)| (name, AttrValueDebug(value))),
			)
			.finish()
	}
}

struct AttrValueDebug<'a>(&'a LazyValue);

impl std::fmt::Debug for AttrValueDebug<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self.0.snapshot() {
			Some(LazyValueKind::Thunk(thunk)) => thunk.fmt(f),
			Some(LazyValueKind::Value(Value::List(_) | Value::AttrSet(_))) => f.write_str("..."),
			Some(LazyValueKind::Value(Value::Lambda(_))) => f.write_str("Lambda"),
			Some(LazyValueKind::Value(value)) => value.fmt(f),
			None => f.write_str("<borrowed>"),
		}
	}
}

unsafe impl<Z: dumpster::Visitor> dumpster::TraceWith<Z> for AttrSetInner {
	fn accept(&self, visitor: &mut Z) -> Result<(), ()> {
		for value in self.attrs.values() {
			value.accept(visitor)?;
		}
		Ok(())
	}
}

impl AttrSet {
	pub fn id(&self) -> usize {
		Gc::as_ptr(&self.inner) as *const () as usize
	}

	pub fn get_mut(&mut self) -> &mut HashMap<StringKind, LazyValue> {
		let inner = Gc::make_mut(&mut self.inner);
		inner.deep.set(DeepState::Shallow);
		&mut inner.attrs
	}

	pub fn deep_state(&self) -> DeepState {
		self.inner.deep.get()
	}

	/// Only set this once the value has been deeply evaluated.
	/// Aka once all elements are also deeply evaluated
	///
	/// doing otherwise will cause incorrect (but not fatal or undefined) behavior when the VM tries to deeply evaluate it
	pub fn set_deeply_evaluated(&self) {
		self.inner.deep.set(DeepState::Deep);
	}

	pub fn begin_deeply_evaluated(&self) {
		self.inner.deep.set(DeepState::Evaluating);
	}

	pub fn new() -> Self {
		Self::default()
	}

	pub fn new_at(pos: CodePos) -> Self {
		Self {
			inner: Gc::new(AttrSetInner {
				attrs: Default::default(),
				deep: Cell::new(DeepState::Deep),
				created_at: Some(pos),
			}),
		}
	}

	pub fn from(attrs: HashMap<StringKind, LazyValue>) -> Self {
		Self {
			inner: Gc::new(AttrSetInner {
				attrs,
				deep: Cell::new(DeepState::Shallow),
				created_at: None,
			}),
		}
	}

	pub fn creation_pos(&self) -> Option<CodePos> {
		self.inner.created_at
	}
}

impl Deref for AttrSet {
	type Target = HashMap<StringKind, LazyValue>;

	fn deref(&self) -> &Self::Target {
		&*self.inner
	}
}
