use crate::runtime::{
	Runtime,
	eval::{ByteCodeFrame, EvalError, Frame, FrameKind, LocalEvaluator, func::ApplyResult},
	lazy::{LazyValue, LazyValueKind},
	thunk::{Thunk, ThunkState},
	value::{DeepState, Lambda, Value},
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
		if !deep || !value.deep_state().shallow() {
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
		enum Action {
			EvalExpr(ByteCodeFrame),
			EvalApply(Lambda, LazyValue),
			Value(Value),
		}

		let action = {
			let mut inner = thunk.0.borrow_mut();
			match &mut *inner {
				ThunkState::Expr(code_loc, scope) => {
					let eval = ByteCodeFrame {
						pos: *code_loc,
						scope: scope.clone(),
					};
					*inner = ThunkState::Evaluating;
					Action::EvalExpr(eval)
				}
				ThunkState::Constructing(_) => {
					return Err(EvalError::ThunkEval(ThunkEvalErr::NotConstructed));
				}
				ThunkState::Evaluating => {
					return Err(EvalError::ThunkEval(ThunkEvalErr::InfiniteRec));
				}
				ThunkState::Evaluated(value) => Action::Value(value.clone()),
				ThunkState::Apply(_, _) => {
					let state = std::mem::replace(&mut *inner, ThunkState::Evaluating);
					let ThunkState::Apply(func, arg) = state else {
						unreachable!()
					};
					Action::EvalApply(func, arg)
				}
			}
		};

		match action {
			Action::EvalExpr(eval) => Ok(ThunkResult::Frame(Frame {
				kind: FrameKind::ByteCode(eval),
				thunk: Some(thunk),
				deep,
			})),
			Action::EvalApply(func, arg) => self.eval_apply(runtime, func, arg, Some(thunk), deep),
			Action::Value(value) => self.eval_value(runtime, value, deep),
		}
	}

	pub fn eval_apply(
		&mut self,
		runtime: &mut Runtime,
		func: Lambda,
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
		lazy: LazyValue,
		deep: bool,
	) -> Result<ThunkResult, EvalError> {
		match lazy.try_get_value() {
			LazyValueKind::Thunk(thunk) => self.eval_thunk(runtime, thunk, deep),
			LazyValueKind::Value(value) => self.eval_value(runtime, value, deep),
		}
	}
}
