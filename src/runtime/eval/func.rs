use crate::runtime::{
	native::{NativeLambda, NativeLambdaState},
	thunk,
};

use super::*;

impl Evaluator {
	fn spill_deep_value(&mut self, value: &Value) -> Result<(), EvalError> {
		match &value {
			Value::AttrSet(attrs) => {
				if !self.deeply_evaluated.insert(attrs.id()) {
					return Ok(());
				}
				for lazy in attrs.values() {
					self.frame_stack
						.push(PotentialFrame::PotentialDeep(lazy.clone()));
				}
			}
			Value::List(list) => {
				if !self.deeply_evaluated.insert(list.id()) {
					return Ok(());
				}
				for lazy in list.iter() {
					self.frame_stack
						.push(PotentialFrame::PotentialDeep(lazy.clone()));
				}
			}
			_ => {}
		}
		Ok(())
	}

	pub(super) fn ret(
		&mut self,
		runtime: &mut Runtime,
		prev: CodePos,
	) -> Result<Option<Value>, EvalError> {
		let ret = self.pop_value()?;

		// update the thunk if the current frame was evaluating a thunk
		match &self.curr_frame.kind {
			FrameKind::ThunkEval(thunk)
			| FrameKind::ThunkEvalDeep(thunk)
			| FrameKind::ThunkEvalDeepRoot(thunk) => {
				thunk.eval_end(ret.clone()).map_err(|()| {
					EvalError::Internal(
						"tried to finish a thunk that was not currently evaluating".into(),
					)
				})?;
			}
			_ => {}
		}

		// if the current frame is in a deep eval spill inner values onto evaluation stack
		match &self.curr_frame.kind {
			FrameKind::FunctionDeepRoot
			| FrameKind::ThunkEvalDeep(_)
			| FrameKind::ThunkEvalDeepRoot(_) => {
				self.frame_stack.push(PotentialFrame::DeepEval(prev));
				self.spill_deep_value(&ret)?;
			}
			_ => {}
		}

		// push value onto stack if this frame should produce a return value
		match &self.curr_frame.kind {
			FrameKind::Function
			| FrameKind::FunctionDeepRoot
			| FrameKind::ThunkEval(_)
			| FrameKind::ThunkEvalDeepRoot(_) => {
				self.push_value(ret)?;
			}
			_ => {}
		}

		while !self.frame_stack.is_empty() {
			match self.pop_frame()? {
				PotentialFrame::Realized(frame) => {
					self.curr_frame = frame;
					return Ok(None);
				}
				PotentialFrame::DeepEval(_) => {}
				PotentialFrame::PotentialDeep(thunk) => {
					let thunk = match thunk.try_get_value() {
						Ok(value) => {
							self.spill_deep_value(&value)?;
							continue;
						}
						Err(thunk) => thunk,
					};
					let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;
					self.curr_frame = Frame::new(pos, scope, FrameKind::ThunkEvalDeep(thunk));
					return Ok(None);
				}
				PotentialFrame::NativeLambda(future) => {
					let cont = self.pop_value()?;
					self.continue_native_lambda(runtime, future, Some(cont))?;
				}
			}
		}

		// return resulting value from evaluator
		if self.frame_stack.is_empty() {
			return Ok(Some(self.pop_value()?));
		}
		Ok(None)
	}

	pub(super) fn apply(
		&mut self,
		runtime: &mut Runtime,
		arg_pos: CodePos,
	) -> Result<(), EvalError> {
		let lambda = self.pop_lambda()?;

		let arg = Thunk::uneval_with_scope(arg_pos, self.curr_frame.scope.clone()).into();

		match lambda {
			Lambda::Lambda { scope, lambda } => {
				let lambda = runtime.program.get_lambda(lambda).ok_or_else(|| {
					EvalError::Internal(
						format!("invalid lambda id {} in bytecode", lambda.index()).into(),
					)
				})?;

				self.thunk_stack.push(arg);

				let frame = Frame::new(lambda.code, scope.new_level(), FrameKind::Function);
				self.begin_frame(frame)?
			}
			Lambda::NativeLambda(native_lambda) => {
				self.apply_native_lambda(runtime, native_lambda, arg)?;
			}
		};
		Ok(())
	}

	fn apply_native_lambda(
		&mut self,
		runtime: &mut Runtime,
		lambda: NativeLambda,
		arg: LazyValue,
	) -> Result<(), EvalError> {
		use crate::runtime::native::NativeLambdaResult;

		let future = match lambda.begin(self, runtime, arg) {
			NativeLambdaResult::Value(value) => return self.push_value(value),
			NativeLambdaResult::Err(err) => return Err(err),

			NativeLambdaResult::Future(future) => future,
		};

		self.continue_native_lambda(runtime, future, None)
	}

	fn continue_native_lambda(
		&mut self,
		runtime: &mut Runtime,
		mut future: NativeLambdaState,
		cont: Option<Value>,
	) -> Result<(), EvalError> {
		let data = NativeCtx {
			evaluator: self,
			runtime,
			to_eval: None,
			evaluated: cont,
		};
		let waker = unsafe { Waker::new((&raw const data).cast(), &VTABLE) };
		let mut ctx = Context::from_waker(&waker);

		match future.as_mut().poll(&mut ctx) {
			Poll::Ready(result) => return self.push_value(result?),
			Poll::Pending => {}
		}

		let to_eval = data.to_eval.unwrap();

		// self.eval()
		let (pos, scope) = to_eval.eval_begin().map_err(EvalError::ThunkEval)?;
		self.begin_frame(Frame::new(pos, scope, FrameKind::ThunkEval(to_eval)))?;
		self.frame_stack.push(PotentialFrame::NativeLambda(future));

		Ok(())
	}
}

pub fn pending_once() -> PendingOnce {
	PendingOnce { is_ready: false }
}

pub struct PendingOnce {
	is_ready: bool,
}

impl Future for PendingOnce {
	type Output = ();
	fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		if self.is_ready {
			Poll::Ready(())
		} else {
			self.is_ready = true;
			Poll::Pending
		}
	}
}

use std::{pin::Pin, task::*};
static VTABLE: RawWakerVTable = RawWakerVTable::new(|_| panic!(), |_| {}, |_| {}, |_| {});

pub struct NativeCtx<'a> {
	evaluator: &'a mut Evaluator,
	runtime: &'a mut Runtime,

	to_eval: Option<Thunk>,
	evaluated: Option<Value>,
}

impl<'a> NativeCtx<'a> {
	pub async fn with<T>(func: impl FnOnce(&mut NativeCtx<'_>) -> T) -> T {
		let (data, vtable) = std::future::poll_fn(|cx: &mut Context<'_>| {
			Poll::Ready((cx.waker().data(), cx.waker().vtable()))
		})
		.await;
		assert_eq!(vtable, &VTABLE);

		func(unsafe { data.cast_mut().cast::<NativeCtx>().as_mut().unwrap() })
	}

	pub fn evaluator(&mut self) -> &mut Evaluator {
		self.evaluator
	}

	pub fn runtime(&mut self) -> &mut Runtime {
		self.runtime
	}

	pub async fn eval_lazy(arg: LazyValue) -> Value {
		match arg.try_get_value() {
			Ok(value) => value,
			Err(thunk) => Self::eval(thunk).await,
		}
	}

	pub async fn eval(thunk: Thunk) -> Value {
		Self::with(|ctx| {
			assert!(ctx.to_eval.is_none());
			ctx.to_eval = Some(thunk);
		})
		.await;
		pending_once().await;
		Self::with(|ctx| ctx.evaluated.take().unwrap()).await
	}
}
