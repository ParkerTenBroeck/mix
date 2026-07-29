use crate::runtime::{
	Runtime,
	eval::{ByteCodeFrame, EvalError, Frame, FrameKind, LocalEvaluator, func::ApplyResult},
	lazy::{LazyValue, LazyValueKind},
	thunk::{Thunk, ThunkState},
	value::{NativeLambda, Value},
};

#[derive(Debug)]
pub enum ThunkEvalErr {
	InfiniteRec,
	NotConstructed,
	AlreadyEvaluated,
}

pub enum ThunkResult {
	Value(Value),
	Frame(Frame),
}

impl LocalEvaluator {
	pub fn eval_value(
		&mut self,
		_: &mut Runtime,
		value: Value,
		deep: bool,
	) -> Result<ThunkResult, EvalError> {
		if !deep || value.deeply_evaluated() {
			return Ok(ThunkResult::Value(value));
		}

		self.deep_eval_value(value)
	}

	pub fn eval_thunk(
		&mut self,
		runtime: &mut Runtime,
		thunk: Thunk,
		deep: bool,
	) -> Result<ThunkResult, EvalError> {
		let inner = &mut *thunk.0.borrow_mut();
		match inner {
			ThunkState::Expr(code_loc, scope) => {
				let (pos, scope) = (*code_loc, scope.clone());
				*inner = ThunkState::Evaluating;

				let eval = ByteCodeFrame { pos, scope };
				Ok(ThunkResult::Frame(Frame {
					kind: FrameKind::ByteCode(eval),
					thunk: Some(thunk.clone()),
					deep,
				}))
			}
			ThunkState::Constructing(_) => Err(EvalError::ThunkEval(ThunkEvalErr::NotConstructed)),
			ThunkState::Evaluating => Err(EvalError::ThunkEval(ThunkEvalErr::InfiniteRec)),
			ThunkState::Evaluated(value) => self.eval_value(runtime, value.clone(), deep),
			ThunkState::Apply(func, arg) => {
				let func = std::mem::replace(func, Value::Bool(false));
				let arg = std::mem::replace(arg, Value::Bool(false).into());
				*inner = ThunkState::Evaluating;
				self.eval_apply(runtime, func, arg, Some(thunk.clone()), deep)
			}
		}
	}

	pub fn eval_apply(
		&mut self,
		runtime: &mut Runtime,
		func: Value,
		arg: LazyValue,
		thunk: Option<Thunk>,
		deep: bool,
	) -> Result<ThunkResult, EvalError> {
		let res = self.apply(runtime, func, arg)?;
		match res {
			ApplyResult::Value(value) => self.eval_value(runtime, value, deep),
			ApplyResult::Frame(kind) => Ok(ThunkResult::Frame(Frame { kind, thunk, deep })),
		}
	}

	pub fn eval_lazy(
		&mut self,
		runtime: &mut Runtime,
		thunk: LazyValue,
		deep: bool,
	) -> Result<ThunkResult, EvalError> {
		match thunk.try_get_value() {
			LazyValueKind::Thunk(thunk) => self.eval_thunk(runtime, thunk, deep),
			LazyValueKind::Value(value) => self.eval_value(runtime, value, deep),
		}
	}
}
