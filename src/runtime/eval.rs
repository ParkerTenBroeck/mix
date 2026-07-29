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

#[derive(Default)]
pub struct Evaluator {
	pub local: LocalEvaluator,
	pub frames: Vec<Frame>,
}

enum EvalStep {
	Pending,
	Ret,
	BeginFrame(Frame),
}

impl Evaluator {
	pub fn begin_eval(
		runtime: &mut Runtime,
		lazy: LazyValue,
		deep: bool,
	) -> Result<Evaluator, ErrorTrace> {
		let mut myself = Self {
			local: Default::default(),
			frames: vec![],
		};
		match myself.local.eval_lazy(runtime, lazy, deep).unwrap() {
			ThunkResult::Value(value) => myself.local.value_stack.push(value),
			ThunkResult::Frame(frame) => myself.frames.push(frame),
		}
		Ok(myself)
	}

	pub fn begin_apply(
		runtime: &mut Runtime,
		lambda: Lambda,
		arg: LazyValue,
	) -> Result<Evaluator, ErrorTrace> {
		let mut myself = Self::default();
		let result = myself
			.local
			.eval_apply(runtime, lambda, arg, None, false)
			.map_err(|error| ErrorTrace::build(runtime, &myself, error))?;

		match result {
			ThunkResult::Value(value) => myself.local.value_stack.push(value),
			ThunkResult::Frame(frame) => myself.frames.push(frame),
		}

		Ok(myself)
	}

	fn begin_frame(&mut self, frame: Frame) -> Result<(), EvalError> {
		self.frames.push(frame);
		Ok(())
	}

	fn pop_frame(&mut self) -> Result<Frame, EvalError> {
		self.frames.pop().ok_or(EvalError::ByteCode("call stack"))
	}

	pub fn run(
		&mut self,
		runtime: &mut Runtime,
		mut fule: Fule,
	) -> Result<Option<Value>, EvalError> {
		loop {
			let Some(frame) = self.frames.last_mut() else {
				return Ok(self.local.value_stack.pop());
			};

			let res = match &mut frame.kind {
				FrameKind::ByteCode(frame) => self.local.run_bytecode(runtime, frame, &mut fule)?,
				FrameKind::Native(frame) => {
					self.local
						.poll_native_lambda(runtime, &mut fule, frame.state.as_mut())?
				}
			};

			if let Some(thunk) = &frame.thunk
				&& matches!(res, EvalStep::Ret)
			{
				thunk.eval_end(self.local.peek_value()?.clone()).unwrap();
			}

			match res {
				EvalStep::Pending if !fule.fule() => return Ok(None),
				EvalStep::Pending => {}

				EvalStep::Ret if frame.deep && !self.local.peek_value()?.deeply_evaluated() => {
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

#[derive(Default)]
pub struct LocalEvaluator {
	pub value_stack: Vec<Value>,
	pub lazy_stack: Vec<LazyValue>,
}

impl LocalEvaluator {
	fn push_value(&mut self, value: Value) -> Result<(), EvalError> {
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
		self.lazy_stack.push(value);
		Ok(())
	}

	fn pop_lazy(&mut self) -> Result<LazyValue, EvalError> {
		self.lazy_stack
			.pop()
			.ok_or(EvalError::ByteCode("lazy stack"))
	}
}
