mod attrset;
mod lambda;
mod list;
mod native;
mod string;
mod ty;

pub use attrset::*;
pub use lambda::*;
pub use list::*;
pub use native::*;
pub use string::*;
pub use ty::*;

use std::{fmt::Debug, path::PathBuf};

use crate::{bytecode::CodePos, runtime::eval::EvalError};

use dumpster::Trace;

#[derive(Clone, Debug, Trace)]
pub enum Value {
	Bool(bool),
	Int(i64),
	Float(f64),
	String(StringKind),
	Path(PathBuf),
	List(List),
	AttrSet(AttrSet),
	Lambda(Lambda),
}

impl Value {
	pub fn ty(&self) -> ValueType {
		match self {
			Value::Bool(_) => ValueType::Bool,
			Value::Int(_) => ValueType::Int,
			Value::Float(_) => ValueType::Float,
			Value::String(_) => ValueType::String,
			Value::Path(_) => ValueType::Path,
			Value::List(_) => ValueType::List,
			Value::AttrSet(_) => ValueType::AttrSet,
			Value::Lambda(_) => ValueType::Lambda,
		}
	}

	/// Only set this once the value has been deeply evaluated.
	/// Aka once all elements are also deeply evaluated
	///
	/// doing otherwise will cause incorrect (but not fatal or undefined) behavior when the VM tries to deeply evaluate it
	pub fn set_deeply_evaluated(&self) {
		match self {
			Value::List(list) => list.set_deeply_evaluated(),
			Value::AttrSet(attr_set) => attr_set.set_deeply_evaluated(),
			_ => {}
		}
	}

	pub fn deeply_evaluated(&self) -> bool {
		match self {
			Value::List(list) => list.deeply_evaluated(),
			Value::AttrSet(attr_set) => attr_set.deeply_evaluated(),
			_ => true,
		}
	}

	pub fn creation_pos(&self) -> Option<CodePos> {
		match self {
			Value::List(list) => list.creation_pos(),
			Value::AttrSet(attr_set) => attr_set.creation_pos(),
			_ => None,
		}
	}

	pub fn expect_bool(self) -> Result<bool, EvalError> {
		match self {
			Self::Bool(value) => Ok(value),
			other => Err({
				let expected = ValueType::Bool;
				EvalError::TypeMismatch {
					expected,
					got: other.ty(),
				}
			}),
		}
	}

	pub fn expect_int(self) -> Result<i64, EvalError> {
		match self {
			Self::Int(value) => Ok(value),
			other => Err({
				let expected = ValueType::Int;
				EvalError::TypeMismatch {
					expected,
					got: other.ty(),
				}
			}),
		}
	}

	pub fn expect_float(self) -> Result<f64, EvalError> {
		match self {
			Self::Float(value) => Ok(value),
			other => Err({
				let expected = ValueType::Float;
				EvalError::TypeMismatch {
					expected,
					got: other.ty(),
				}
			}),
		}
	}

	pub fn expect_string(self) -> Result<StringKind, EvalError> {
		match self {
			Self::String(value) => Ok(value),
			other => Err({
				let expected = ValueType::String;
				EvalError::TypeMismatch {
					expected,
					got: other.ty(),
				}
			}),
		}
	}

	pub fn expect_path(self) -> Result<PathBuf, EvalError> {
		match self {
			Self::Path(value) => Ok(value),
			other => Err({
				let expected = ValueType::Path;
				EvalError::TypeMismatch {
					expected,
					got: other.ty(),
				}
			}),
		}
	}

	pub fn expect_list(self) -> Result<List, EvalError> {
		match self {
			Self::List(value) => Ok(value),
			other => Err({
				let expected = ValueType::List;
				EvalError::TypeMismatch {
					expected,
					got: other.ty(),
				}
			}),
		}
	}

	pub fn expect_attrset(self) -> Result<AttrSet, EvalError> {
		match self {
			Self::AttrSet(value) => Ok(value),
			other => Err({
				let expected = ValueType::AttrSet;
				EvalError::TypeMismatch {
					expected,
					got: other.ty(),
				}
			}),
		}
	}

	pub fn expect_lambda(self) -> Result<Lambda, EvalError> {
		match self {
			Self::Lambda(value) => Ok(value),
			other => Err({
				let expected = ValueType::Lambda;
				EvalError::TypeMismatch {
					expected,
					got: other.ty(),
				}
			}),
		}
	}
}

impl From<i64> for Value {
	fn from(value: i64) -> Self {
		Self::Int(value)
	}
}

impl From<f64> for Value {
	fn from(value: f64) -> Self {
		Self::Float(value)
	}
}

impl From<bool> for Value {
	fn from(value: bool) -> Self {
		Self::Bool(value)
	}
}

impl From<String> for Value {
	fn from(value: String) -> Self {
		Self::String(StringKind::String(value))
	}
}

impl From<std::rc::Rc<String>> for Value {
	fn from(value: std::rc::Rc<String>) -> Self {
		Self::String(StringKind::Interned(value))
	}
}

impl From<StringKind> for Value {
	fn from(value: StringKind) -> Self {
		Self::String(value)
	}
}

impl From<Lambda> for Value {
	fn from(value: Lambda) -> Self {
		Self::Lambda(value)
	}
}

impl From<NativeLambda> for Value {
	fn from(value: NativeLambda) -> Self {
		Self::Lambda(Lambda::NativeLambda(value))
	}
}

impl From<AttrSet> for Value {
	fn from(value: AttrSet) -> Self {
		Self::AttrSet(value)
	}
}

impl From<List> for Value {
	fn from(value: List) -> Self {
		Self::List(value)
	}
}
