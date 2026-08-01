use std::borrow::Cow;

use dumpster::Trace;

use crate::runtime::{
	Runtime,
	eval::{EvalError, NativeCtx, NativeLambdaAsync, NativeLambdaDyn, NativeLambdaResult},
	lazy::LazyValue,
	thunk::Thunk,
	value::{NativeLambda, Value},
};

#[derive(Trace, Clone)]
pub struct Map<T>(T);

impl Map<()> {
	pub fn new() -> Self {
		Self(())
	}
}

impl NativeLambdaDyn for Map<()> {
	fn identifier(&self) -> Cow<'static, str> {
		"map".into()
	}

	fn begin(&self, _: &mut Runtime, arg: LazyValue) -> NativeLambdaResult {
		NativeLambdaResult::Value(NativeLambda::new(Map(arg)).into())
	}
}

impl NativeLambdaAsync for Map<LazyValue> {
	fn identifier(&self) -> Cow<'static, str> {
		"map".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let lambda = ctx.eval_lazy(self.0).await?.expect_lambda()?;
		let mut list = ctx.eval_lazy(arg).await?.expect_list()?;
		for element in list.get_mut() {
			*element =
				Thunk::application(lambda.clone(), std::mem::replace(element, false.into())).into();
		}
		Ok(list.into())
	}
}
