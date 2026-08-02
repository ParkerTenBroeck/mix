use std::{cell::Cell, collections::VecDeque, fmt, ops::Deref};

use dumpster::{Trace, unsync::Gc};

use crate::{
	bytecode::CodePos,
	runtime::{
		lazy::{LazyValue, LazyValueKind},
		value::{DeepState, Value},
	},
};

#[derive(Clone, Default, Trace)]
pub struct List {
	inner: Gc<ListInner>,
}

#[derive(Clone, Default, Trace)]
pub struct ListInner {
	deep: Cell<DeepState>,
	list: VecDeque<LazyValue>,
	created_at: Option<CodePos>,
}

impl fmt::Debug for List {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_list()
			.entries(self.inner.list.iter().map(ListValueDebug))
			.finish()
	}
}

struct ListValueDebug<'a>(&'a LazyValue);

impl fmt::Debug for ListValueDebug<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.0.snapshot() {
			Some(LazyValueKind::Thunk(thunk)) => thunk.fmt(f),
			Some(LazyValueKind::Value(Value::List(_) | Value::AttrSet(_))) => f.write_str("..."),
			Some(LazyValueKind::Value(Value::Lambda(_))) => f.write_str("Lambda"),
			Some(LazyValueKind::Value(value)) => value.fmt(f),
			None => f.write_str("<borrowed>"),
		}
	}
}

impl List {
	pub fn with_capacity(capacity: usize) -> List {
		Self {
			inner: Gc::new(ListInner {
				deep: Cell::new(DeepState::Deep),
				list: VecDeque::with_capacity(capacity),
				created_at: None,
			}),
		}
	}

	pub fn with_capacity_at(capacity: usize, pos: CodePos) -> List {
		Self {
			inner: Gc::new(ListInner {
				deep: Cell::new(DeepState::Deep),
				list: VecDeque::with_capacity(capacity),
				created_at: Some(pos),
			}),
		}
	}

	pub fn id(&self) -> usize {
		Gc::as_ptr(&self.inner) as *const () as usize
	}

	pub fn deep_state(&self) -> DeepState {
		self.inner.deep.get()
	}

	pub fn creation_pos(&self) -> Option<CodePos> {
		self.inner.created_at
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

	pub fn get_mut(&mut self) -> &mut VecDeque<LazyValue> {
		let inner = Gc::make_mut(&mut self.inner);
		inner.deep.set(DeepState::Shallow);
		&mut inner.list
	}
}

impl Deref for List {
	type Target = VecDeque<LazyValue>;

	fn deref(&self) -> &Self::Target {
		&self.inner.list
	}
}
