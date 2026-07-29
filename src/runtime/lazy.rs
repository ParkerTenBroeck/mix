use std::{cell::RefCell, fmt};

use dumpster::Trace;

use crate::{
	bytecode::CodePos,
	runtime::{scope::Scope, thunk::Thunk, value::Value},
};

#[derive(Clone, Trace)]
pub struct LazyValue {
	state: RefCell<LazyValueKind>,
}

impl fmt::Debug for LazyValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.snapshot() {
			Some(LazyValueKind::Thunk(thunk)) => thunk.fmt(f),
			Some(LazyValueKind::Value(Value::List(list))) => list.fmt(f),
			Some(LazyValueKind::Value(Value::AttrSet(attrs))) => attrs.fmt(f),
			Some(LazyValueKind::Value(value)) => value.fmt(f),
			None => f.write_str("<borrowed>"),
		}
	}
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

	pub fn snapshot(&self) -> Option<LazyValueKind> {
		self.state.try_borrow().ok().map(|state| state.clone())
	}

	pub fn thunk(&self) -> Option<Thunk> {
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

#[cfg(test)]
mod tests {
	use crate::runtime::value::{AttrSet, List};

	use super::*;

	#[test]
	fn debug_output_hides_storage_wrappers() {
		let attrset: LazyValue = Value::AttrSet(AttrSet::default()).into();
		let list: LazyValue = Value::List(List::default()).into();

		assert_eq!(format!("{attrset:?}"), "{}");
		assert_eq!(format!("{list:?}"), "[]");
	}
}
