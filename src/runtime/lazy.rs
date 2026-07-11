use std::cell::RefCell;

use dumpster::Trace;

use crate::{
	bytecode::CodePos,
	runtime::{scope::Scope, thunk::Thunk, value::Value},
};

#[derive(Clone, Debug, Trace)]
pub struct LazyValue {
	state: RefCell<LazyValueState>,
}

impl<T: Into<Value>> From<T> for LazyValue {
	fn from(value: T) -> Self {
		LazyValueState::Value(value.into()).into()
	}
}

impl From<Thunk> for LazyValue {
	fn from(value: Thunk) -> Self {
		LazyValueState::Thunk(value).into()
	}
}

impl From<LazyValueState> for LazyValue {
	fn from(value: LazyValueState) -> Self {
		Self {
			state: RefCell::new(value),
		}
	}
}

impl LazyValue {
	pub fn construct_begin(code: CodePos) -> Self {
		LazyValueState::Thunk(Thunk::construct_begin(code)).into()
	}

	pub fn construct_end(&self, scope: Scope) -> bool {
		match &*self.state.borrow() {
			LazyValueState::Thunk(thunk) => thunk.construct_end(scope),
			_ => false,
		}
	}

	pub fn try_get_value(&self) -> Result<Value, Thunk> {
		let mut myself = self.state.borrow_mut();
		match &*myself {
			LazyValueState::Thunk(thunk) => match thunk.get_value() {
				Some(value) => {
					*myself = LazyValueState::Value(value.clone());
					Ok(value)
				}
				None => Err(thunk.clone()),
			},
			LazyValueState::Value(value) => Ok(value.clone()),
		}
	}

	pub fn try_into_value(self) -> Result<Value, Thunk> {
		match self.state.into_inner() {
			LazyValueState::Thunk(thunk) => match thunk.get_value() {
				Some(value) => Ok(value),
				None => Err(thunk),
			},
			LazyValueState::Value(value) => Ok(value),
		}
	}

	pub fn uneval(code: CodePos, scope: Scope) -> Self {
		LazyValueState::Thunk(Thunk::uneval(code, scope)).into()
	}
}

#[derive(Clone, Debug, Trace)]
pub enum LazyValueState {
	Thunk(Thunk),
	Value(Value),
}

impl<T: Into<Value>> From<T> for LazyValueState {
	fn from(value: T) -> Self {
		Self::Value(value.into())
	}
}
