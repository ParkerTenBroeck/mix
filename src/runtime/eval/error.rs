use std::borrow::Cow;

use crate::{
	report::Reports,
	runtime::{eval::ThunkEvalErr, value::ValueType},
};

#[derive(Debug)]
pub enum EvalError {
	Custom(Cow<'static, str>),
	Reports(Reports),
	LimitExceeded {
		resource: &'static str,
		limit: usize,
	},
	TypeMismatch {
		expected: ValueType,
		got: ValueType,
	},
	BinOpTypeMismatch {
		details: Cow<'static, str>,
	},
	Arithmetic(Cow<'static, str>),
	MissingAttr(Cow<'static, str>),
	MissingBinding(Cow<'static, str>),
	Internal(Cow<'static, str>),
	ThunkEval(ThunkEvalErr),
	ByteCode(&'static str),
}
