mod attr;
mod binop;
mod bytecode;
mod deep;
mod error;
mod frame;
mod fule;
mod func;
mod native;
mod thunk;

pub use error::*;
pub use frame::*;
pub use fule::*;
pub use native::*;
pub use thunk::*;

use crate::{
	bytecode::OpCode,
	runtime::{
		LazyValue, Runtime, Value,
		trace::ErrorTrace,
		value::{AttrSet, Lambda, List, StringKind, ValueType},
	},
};

pub struct Evaluator {
	pub local: LocalEvaluator,
	pub frames: Vec<Frame>,
	max_frames: usize,
}

enum EvalStep {
	Pending,
	Ret,
	BeginFrame(Frame),
}

impl Evaluator {
	fn new(runtime: &Runtime) -> Self {
		Self {
			local: LocalEvaluator::new(runtime),
			frames: vec![],
			max_frames: runtime.limits.max_frames,
		}
	}

	pub fn begin_eval(
		runtime: &mut Runtime,
		lazy: LazyValue,
		deep: bool,
	) -> Result<Evaluator, ErrorTrace> {
		let mut myself = Self::new(runtime);
		let initial = myself
			.local
			.eval_lazy(runtime, lazy, deep)
			.map_err(|error| ErrorTrace::build(runtime, &myself, error))?;
		match initial {
			ThunkResult::Value(value) => myself.local.push_value(value),
			ThunkResult::Frame(frame) => myself.begin_frame(frame),
		}
		.map_err(|error| ErrorTrace::build(runtime, &myself, error))?;
		Ok(myself)
	}

	pub fn begin_apply(
		runtime: &mut Runtime,
		lambda: Lambda,
		arg: LazyValue,
	) -> Result<Evaluator, ErrorTrace> {
		let mut myself = Self::new(runtime);
		let result = myself
			.local
			.eval_apply(runtime, lambda, arg, None, false)
			.map_err(|error| ErrorTrace::build(runtime, &myself, error))?;

		match result {
			ThunkResult::Value(value) => myself.local.push_value(value),
			ThunkResult::Frame(frame) => myself.begin_frame(frame),
		}
		.map_err(|error| ErrorTrace::build(runtime, &myself, error))?;

		Ok(myself)
	}

	fn begin_frame(&mut self, frame: Frame) -> Result<(), EvalError> {
		if self.frames.len() >= self.max_frames {
			return Err(EvalError::LimitExceeded {
				resource: "frame stack",
				limit: self.max_frames,
			});
		}
		self.frames.push(frame);
		Ok(())
	}

	fn pop_frame(&mut self) -> Result<Frame, EvalError> {
		self.frames.pop().ok_or(EvalError::ByteCode("call stack"))
	}

	pub fn run(
		&mut self,
		runtime: &mut Runtime,
		fule: &mut Fule,
	) -> Result<Option<Value>, EvalError> {
		loop {
			let Some(frame) = self.frames.last_mut() else {
				// return Ok(self.local.value_stack.pop());
				todo!()
			};

			let res = match &mut frame.kind {
				FrameKind::ByteCode(frame) => self.local.run_bytecode(runtime, frame, fule)?,
				FrameKind::Native(frame) => {
					self.local
						.poll_native_lambda(runtime, fule, frame.state.as_mut())?
				}
			};

			if let Some(thunk) = &frame.thunk
				&& matches!(res, EvalStep::Ret)
			{
				thunk
					.eval_end(self.local.peek_value()?.clone())
					.map_err(|_| EvalError::ThunkEval(ThunkEvalErr::AlreadyEvaluated))?;
			}

			match res {
				EvalStep::Pending if !fule.fule() => return Ok(None),
				EvalStep::Pending => {}

				EvalStep::Ret if frame.deep && self.local.peek_value()?.deep_state().shallow() => {
					let frame = self.pop_frame()?;

					let pos = match frame.kind {
						FrameKind::ByteCode(frame) => NativePosKind::Expr(frame.pos),
						FrameKind::Native(frame) => frame.pos,
					};
					let value = self.local.pop_value()?;
					self.begin_frame(self.local.get_deep_frame(pos, value))?;
				}

				EvalStep::Ret if self.frames.len() == 1 => {
					return Ok(Some(self.local.pop_value()?));
				}
				EvalStep::Ret => _ = self.pop_frame()?,
				EvalStep::BeginFrame(frame) => {
					self.begin_frame(frame)?;
				}
			}
		}
	}
}

pub struct LocalEvaluator {
	pub value_stack: Vec<Value>,
	pub lazy_stack: Vec<LazyValue>,
	max_values: usize,
	max_thunks: usize,
}

impl LocalEvaluator {
	fn new(runtime: &Runtime) -> Self {
		Self {
			value_stack: vec![],
			lazy_stack: vec![],
			max_values: runtime.limits.max_values,
			max_thunks: runtime.limits.max_thunks,
		}
	}

	fn push_value(&mut self, value: Value) -> Result<(), EvalError> {
		if self.value_stack.len() >= self.max_values {
			return Err(EvalError::LimitExceeded {
				resource: "value stack",
				limit: self.max_values,
			});
		}
		self.value_stack.push(value);
		Ok(())
	}

	fn peek_value(&mut self) -> Result<&Value, EvalError> {
		self.value_stack
			.last()
			.ok_or(EvalError::ByteCode("value stack"))
	}

	fn peek_lazy(&mut self) -> Result<&LazyValue, EvalError> {
		self.lazy_stack
			.last()
			.ok_or(EvalError::ByteCode("value stack"))
	}

	fn pop_value(&mut self) -> Result<Value, EvalError> {
		self.value_stack
			.pop()
			.ok_or(EvalError::ByteCode("value stack"))
	}

	fn pop_bool(&mut self) -> Result<bool, EvalError> {
		self.pop_value()?.expect_bool()
	}

	fn pop_string(&mut self) -> Result<StringKind, EvalError> {
		self.pop_value()?.expect_string()
	}

	fn pop_list(&mut self) -> Result<List, EvalError> {
		self.pop_value()?.expect_list()
	}

	fn pop_attrset(&mut self) -> Result<AttrSet, EvalError> {
		self.pop_value()?.expect_attrset()
	}

	fn pop_lambda(&mut self) -> Result<Lambda, EvalError> {
		self.pop_value()?.expect_lambda()
	}

	fn push_lazy(&mut self, value: LazyValue) -> Result<(), EvalError> {
		if self.lazy_stack.len() >= self.max_thunks {
			return Err(EvalError::LimitExceeded {
				resource: "thunk stack",
				limit: self.max_thunks,
			});
		}
		self.lazy_stack.push(value);
		Ok(())
	}

	fn pop_lazy(&mut self) -> Result<LazyValue, EvalError> {
		self.lazy_stack
			.pop()
			.ok_or(EvalError::ByteCode("lazy stack"))
	}
}
