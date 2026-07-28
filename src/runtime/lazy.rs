use std::cell::RefCell;

use dumpster::Trace;

use crate::{
	bytecode::CodePos,
	runtime::{scope::Scope, thunk::Thunk, value::Value},
};

#[derive(Clone, Debug, Trace)]
pub struct LazyValue {
	state: RefCell<LazyValueKind>,
}

impl<T: Into<Value>> From<T> for LazyValue {
	fn from(value: T) -> Self {
		LazyValueKind::Value(value.into()).into()
	}
}

impl From<Thunk> for LazyValue {
	fn from(value: Thunk) -> Self {
		LazyValueKind::Thunk(value).into()
	}
}

impl From<LazyValueKind> for LazyValue {
	fn from(value: LazyValueKind) -> Self {
		Self {
			state: RefCell::new(value),
		}
	}
}

impl LazyValue {
	pub fn construct_end(&self, scope: Scope) -> bool {
		match &*self.state.borrow() {
			LazyValueKind::Thunk(thunk) => thunk.construct_end(scope),
			_ => false,
		}
	}

	pub fn try_get_value(&self) -> LazyValueKind {
		let mut myself = self.state.borrow_mut();
		match &*myself {
			LazyValueKind::Thunk(thunk) => match thunk.get_value() {
				Some(value) => {
					*myself = LazyValueKind::Value(value.clone());
					LazyValueKind::Value(value)
				}
				None => myself.clone(),
			},
			other => other.clone(),
		}
	}

	pub fn thunk(self) -> Option<Thunk> {
		match &*self.state.borrow() {
			LazyValueKind::Thunk(thunk) => Some(thunk.clone()),
			_ => None,
		}
	}

	pub fn uneval(code: CodePos, scope: Scope) -> Self {
		LazyValueKind::Thunk(Thunk::uneval(code, scope)).into()
	}
}

#[derive(Clone, Debug, Trace)]
pub enum LazyValueKind {
	Thunk(Thunk),
	Value(Value),
}

impl<T: Into<Value>> From<T> for LazyValueKind {
	fn from(value: T) -> Self {
		Self::Value(value.into())
	}
}
