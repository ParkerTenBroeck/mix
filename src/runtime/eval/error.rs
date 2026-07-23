use std::borrow::Cow;

use crate::runtime::{thunk::ThunkEvalErr, value::ValueType};

#[derive(Debug)]
pub enum EvalError {
	TypeMismatch { expected: ValueType, got: ValueType },
	BinOpTypeMismatch { details: Cow<'static, str> },
	Arithmetic(Cow<'static, str>),
	MissingAttr(Cow<'static, str>),
	MissingBinding(Cow<'static, str>),
	Internal(Cow<'static, str>),
	ThunkEval(ThunkEvalErr),
	ByteCode(&'static str),
}