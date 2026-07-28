use dumpster::Trace;

use crate::runtime::{
	Runtime,
	eval::{EvalError, EvalStep, FrameKind, Fule, LocalEvaluator, NativeFrame, func::ApplyResult},
	lazy::{LazyValue, LazyValueKind},
	thunk::Thunk,
	value::NativeLambda,
	value::Value,
};

use std::{borrow::Cow, marker::PhantomData, pin::Pin, task::*};

impl LocalEvaluator {
	pub(super) fn apply_native_lambda(
		runtime: &mut Runtime,
		lambda: NativeLambda,
		arg: LazyValue,
	) -> Result<ApplyResult, EvalError> {
		match lambda.begin(runtime, arg) {
			NativeLambdaResult::Value(value) => Ok(ApplyResult::Value(value)),
			NativeLambdaResult::Err(err) => Err(err),

			NativeLambdaResult::Future(future) => {
				Ok(ApplyResult::Frame(FrameKind::Native(NativeFrame {
					state: future,
					name: lambda.identifier(),
				})))
			}
		}
	}

	pub(super) fn poll_native_lambda(
		&mut self,
		runtime: &mut Runtime,
		fule: &mut Fule,
		mut future: NativeLambdaStateRef<'_>,
	) -> Result<EvalStep, EvalError> {
		let mut data = NativeCtxData {
			evaluator: self,
			runtime,
			to_eval: ToEval::None,
			fule,
		};

		// MUST borrow data as mut
		let waker = unsafe { Waker::new((&raw mut data).cast(), &VTABLE) };
		let mut ctx = Context::from_waker(&waker);

		match future.as_mut().poll(&mut ctx) {
			Poll::Ready(result) => {
				self.push_value(result?)?;
				return Ok(EvalStep::Ret);
			}
			Poll::Pending => {}
		}

		let res = match data.to_eval {
			ToEval::Thunk(thunk, deep) => self.eval_thunk(runtime, thunk, deep)?,
			ToEval::Func(func, arg, deep) => self.eval_apply(runtime, func, arg, None, deep)?,
			ToEval::None => return Ok(EvalStep::Pending),
		};
		match res {
			super::ThunkResult::Value(value) => {
				self.push_value(value)?;
				Ok(EvalStep::Ret)
			}
			super::ThunkResult::Frame(frame) => Ok(EvalStep::BeginFrame(frame)),
		}
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

static VTABLE: RawWakerVTable = RawWakerVTable::new(|_| panic!(), |_| {}, |_| {}, |_| {});

enum ToEval {
	Thunk(Thunk, bool),
	Func(Value, LazyValue, bool),
	None,
}

struct NativeCtxData<'a> {
	evaluator: &'a mut LocalEvaluator,
	runtime: &'a mut Runtime,
	fule: &'a mut Fule,

	to_eval: ToEval,
}

pub struct NativeCtx(PhantomData<*mut ()>);

impl NativeCtx {
	async fn with<'a, T>(func: impl FnOnce(&'a mut NativeCtxData<'a>) -> T) -> T {
		let (data, vtable) = std::future::poll_fn(|cx: &mut Context<'_>| {
			Poll::Ready((cx.waker().data(), cx.waker().vtable()))
		})
		.await;
		assert_eq!(vtable, &VTABLE);

		func(unsafe { data.cast_mut().cast::<NativeCtxData>().as_mut().unwrap() })
	}

	pub async fn runtime<T>(&mut self, func: impl FnOnce(&mut Runtime) -> T) -> T {
		Self::with(|ctx| func(ctx.runtime)).await
	}

	pub async fn fule(&mut self) {
		let fule = Self::with(|cx| cx.fule.fule()).await;
		if !fule {
			pending_once().await;
		}
	}

	pub async fn eval_lazy(&mut self, arg: LazyValue) -> Result<Value, EvalError> {
		match arg.try_get_value() {
			LazyValueKind::Value(value) => Ok(value),
			LazyValueKind::Thunk(thunk) => self.eval(thunk).await,
		}
	}

	pub async fn eval(&mut self, thunk: Thunk) -> Result<Value, EvalError> {
		Self::with(|ctx| {
			debug_assert!(matches!(ctx.to_eval, ToEval::None));
			ctx.to_eval = ToEval::Thunk(thunk, false);
		})
		.await;
		pending_once().await;
		Self::with(|ctx| ctx.evaluator.pop_value()).await
	}

	pub async fn eval_deep(&mut self, thunk: Thunk) -> Result<Value, EvalError> {
		Self::with(|ctx| {
			debug_assert!(matches!(ctx.to_eval, ToEval::None));
			ctx.to_eval = ToEval::Thunk(thunk, true);
		})
		.await;
		pending_once().await;
		Self::with(|ctx| ctx.evaluator.pop_value()).await
	}

	pub async fn eval_apply(
		&mut self,
		func: impl Into<Value>,
		arg: impl Into<LazyValue>,
	) -> Result<Value, EvalError> {
		Self::with(|ctx| {
			debug_assert!(matches!(ctx.to_eval, ToEval::None));
			ctx.to_eval = ToEval::Func(func.into(), arg.into(), false);
		})
		.await;
		pending_once().await;
		Self::with(|ctx| ctx.evaluator.pop_value()).await
	}
}

pub struct NativeLambdaFrame {
	name: Cow<'static, str>,
	state: NativeLambdaState,
}
pub type NativeLambdaState = dyn Future<Output = Result<Value, EvalError>>;
pub type NativeLambdaStateBox = std::pin::Pin<Box<NativeLambdaState>>;
pub type NativeLambdaStateRef<'a> = std::pin::Pin<&'a mut NativeLambdaState>;

pub enum NativeLambdaResult {
	Value(Value),
	Future(NativeLambdaStateBox),
	Err(EvalError),
}

pub trait NativeLambdaDyn: Trace + 'static {
	fn identifier(&self) -> Cow<'static, str>;
	fn begin(&self, runtime: &mut Runtime, arg: LazyValue) -> NativeLambdaResult;
}

pub trait NativeLambdaAsync: Trace + Clone + 'static {
	fn identifier(&self) -> Cow<'static, str>;
	#[allow(async_fn_in_trait)]
	async fn apply(self, ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError>;
}

impl<T: NativeLambdaAsync> NativeLambdaDyn for T {
	fn identifier(&self) -> Cow<'static, str> {
		NativeLambdaAsync::identifier(self)
	}

	fn begin(&self, _: &mut Runtime, arg: LazyValue) -> NativeLambdaResult {
		let fut = NativeLambdaAsync::apply(self.clone(), NativeCtx(PhantomData), arg);
		NativeLambdaResult::Future(Box::pin(fut))
	}
}

// pub trait NativeLambdaImm: Trace + 'static {
// 	fn identifier(&self) -> Cow<'static, str>;
// 	fn apply(&self, runtime: &mut Runtime, arg: LazyValue) -> Result<Value, EvalError>;
// }

// impl<T: NativeLambdaImm> NativeLambdaDyn for T {
// 	fn identifier(&self) -> Cow<'static, str> {
// 		NativeLambdaImm::identifier(self)
// 	}

// 	fn begin(&self, runtime: &mut Runtime, arg: LazyValue) -> NativeLambdaResult {
// 		match NativeLambdaImm::apply(self, runtime, arg){
// 			Ok(ok) => NativeLambdaResult::Value(ok),
// 			Err(err) => NativeLambdaResult::Err(err),
// 		}
// 	}
// }
