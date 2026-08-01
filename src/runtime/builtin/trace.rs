use std::borrow::Cow;

use crate::runtime::{
	eval::{EvalError, NativeCtx, NativeLambdaAsync},
	lazy::LazyValue,
	pretty::PrettyValue,
	value::{NativeLambda, Value},
};

#[derive(dumpster::Trace, Clone)]
pub struct Trace;

impl NativeLambdaAsync for Trace {
	fn identifier(&self) -> Cow<'static, str> {
		"trace".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let value = ctx.eval_lazy_deep(arg).await?;

		ctx.runtime(|runtime| {
			println!("{}", PrettyValue::new(runtime, &value));
		})
		.await;

		Ok(NativeLambda::new(Trace2).into())
	}
}

#[derive(dumpster::Trace, Clone)]
struct Trace2;

impl NativeLambdaAsync for Trace2 {
	fn identifier(&self) -> Cow<'static, str> {
		"trace".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		ctx.eval_lazy(arg).await
	}
}
