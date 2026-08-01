use std::borrow::Cow;

use dumpster::Trace;

use crate::runtime::{
	LoadError,
	eval::{EvalError, NativeCtx, NativeLambdaAsync},
	lazy::LazyValue,
	value::Value,
};

#[derive(Trace, Clone)]
pub struct Import;

impl NativeLambdaAsync for Import {
	fn identifier(&self) -> Cow<'static, str> {
		"import".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let path = ctx.eval_lazy(arg).await?.expect_string()?;
		let result = ctx.runtime(|runtime| runtime.load(&path)).await;
		match result {
			Ok(value) => ctx.eval_lazy(value).await,
			Err(LoadError::Io(error)) => Err(EvalError::Custom(error)),
			Err(LoadError::Reports(reports)) => Err(EvalError::Reports(reports)),
		}
	}
}
