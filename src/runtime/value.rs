pub use super::string::*;

use std::{
	collections::VecDeque,
	fmt::Debug,
	ops::{Deref, DerefMut},
	path::PathBuf,
};

use crate::{
	HashMap,
	runtime::{eval::EvalError, native::NativeLambda},
};

use dumpster::{Trace, unsync::Gc};

use crate::{
	bytecode::LambdaId,
	runtime::{lazy::LazyValue, scope::Scope},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
	Number,
	Bool,
	Int,
	Float,
	String,
	Path,
	List,
	AttrSet,
	Lambda,
}

impl std::fmt::Display for ValueType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let name = match self {
			ValueType::Number => "number",
			ValueType::Bool => "bool",
			ValueType::Int => "int",
			ValueType::Float => "float",
			ValueType::String => "string",
			ValueType::Path => "path",
			ValueType::List => "list",
			ValueType::AttrSet => "attrset",
			ValueType::Lambda => "lambda",
		};
		f.write_str(name)
	}
}

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

	pub fn expect_bool(self) -> Result<bool, EvalError> {
		match self {
			Self::Bool(value) => Ok(value),
			other => Err(type_mismatch(ValueType::Bool, other)),
		}
	}

	pub fn expect_int(self) -> Result<i64, EvalError> {
		match self {
			Self::Int(value) => Ok(value),
			other => Err(type_mismatch(ValueType::Int, other)),
		}
	}

	pub fn expect_float(self) -> Result<f64, EvalError> {
		match self {
			Self::Float(value) => Ok(value),
			other => Err(type_mismatch(ValueType::Float, other)),
		}
	}

	pub fn expect_string(self) -> Result<StringKind, EvalError> {
		match self {
			Self::String(value) => Ok(value),
			other => Err(type_mismatch(ValueType::String, other)),
		}
	}

	pub fn expect_path(self) -> Result<PathBuf, EvalError> {
		match self {
			Self::Path(value) => Ok(value),
			other => Err(type_mismatch(ValueType::Path, other)),
		}
	}

	pub fn expect_list(self) -> Result<List, EvalError> {
		match self {
			Self::List(value) => Ok(value),
			other => Err(type_mismatch(ValueType::List, other)),
		}
	}

	pub fn expect_attrset(self) -> Result<AttrSet, EvalError> {
		match self {
			Self::AttrSet(value) => Ok(value),
			other => Err(type_mismatch(ValueType::AttrSet, other)),
		}
	}

	pub fn expect_lambda(self) -> Result<Lambda, EvalError> {
		match self {
			Self::Lambda(value) => Ok(value),
			other => Err(type_mismatch(ValueType::Lambda, other)),
		}
	}
}

fn type_mismatch(expected: ValueType, value: Value) -> EvalError {
	EvalError::TypeMismatch {
		expected,
		got: value.ty(),
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

#[derive(Clone, Debug, Trace)]
pub enum Lambda {
	Lambda { scope: Scope, lambda: LambdaId },
	NativeLambda(NativeLambda),
}

impl From<NativeLambda> for Lambda {
	fn from(value: NativeLambda) -> Self {
		Self::NativeLambda(value)
	}
}

#[derive(Clone, Default, Debug, Trace)]
pub struct List {
	inner: Gc<VecDeque<LazyValue>>,
}

impl List {
	pub fn with_capacity(capacity: usize) -> List {
		Self {
			inner: Gc::new(VecDeque::with_capacity(capacity)),
		}
	}

	pub fn id(&self) -> usize {
		Gc::as_ptr(&self.inner) as *const () as usize
	}

	pub fn get_mut(&mut self) -> &mut VecDeque<LazyValue> {
		Gc::make_mut(&mut self.inner)
	}
}

impl Deref for List {
	type Target = VecDeque<LazyValue>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

#[derive(Clone, Debug, Default, Trace)]
pub struct AttrSet {
	inner: Gc<AttrSetInner>,
}

#[derive(Clone, Default)]
pub struct AttrSetInner(HashMap<StringKind, LazyValue>);

impl Deref for AttrSetInner {
	type Target = HashMap<StringKind, LazyValue>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for AttrSetInner {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl std::fmt::Debug for AttrSetInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("AttrSet").field(&self.0).finish()
	}
}

unsafe impl<Z: dumpster::Visitor> dumpster::TraceWith<Z> for AttrSetInner {
	fn accept(&self, visitor: &mut Z) -> Result<(), ()> {
		for (k, v) in &self.0 {
			k.accept(visitor)?;
			v.accept(visitor)?;
		}
		Ok(())
	}
}

impl AttrSet {
	pub fn id(&self) -> usize {
		Gc::as_ptr(&self.inner) as *const () as usize
	}

	pub fn get_mut(&mut self) -> &mut HashMap<StringKind, LazyValue> {
		&mut Gc::make_mut(&mut self.inner).0
	}

	pub fn new() -> Self {
		Self::default()
	}

	pub fn from(map: HashMap<StringKind, LazyValue>) -> Self {
		Self {
			inner: Gc::new(AttrSetInner(map)),
		}
	}
}

impl Deref for AttrSet {
	type Target = HashMap<StringKind, LazyValue>;

	fn deref(&self) -> &Self::Target {
		&*self.inner
	}
}
