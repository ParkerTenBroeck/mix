use std::borrow::Cow;

use dumpster::Trace;

use crate::runtime::{
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
		let result = ctx
			.runtime(|runtime| {
				runtime
					.loader
					.load(std::path::Path::new(&*path))
					.map_err(|error| error.into_owned())?;
				runtime
					.load(&path)
					.map_err(|reports| reports.render(&runtime.loader.files()).join("\n"))
			})
			.await;
		match result {
			Ok(value) => ctx.eval_lazy(value).await,
			Err(error) => Err(EvalError::Internal(error.into())),
		}
	}
}
