use std::borrow::Cow;

use dumpster::Trace;

use crate::runtime::{
	Runtime,
	eval::{EvalError, NativeCtx, NativeLambdaAsync, NativeLambdaDyn, NativeLambdaResult},
	lazy::LazyValue,
	thunk::Thunk,
	value::{List, NativeLambda, Value},
};

#[derive(Trace, Clone)]
pub struct MkList<T>(T);

impl MkList<()> {
	pub fn new() -> Self {
		Self(())
	}
}

impl NativeLambdaDyn for MkList<()> {
	fn identifier(&self) -> Cow<'static, str> {
		"mkList".into()
	}

	fn begin(&self, _: &mut Runtime, arg: LazyValue) -> NativeLambdaResult {
		NativeLambdaResult::Value(NativeLambda::new(MkList(arg)).into())
	}
}

impl NativeLambdaAsync for MkList<LazyValue> {
	fn identifier(&self) -> Cow<'static, str> {
		"mkList".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let func = ctx.eval_lazy(self.0).await?.expect_lambda()?;
		let len = ctx.eval_lazy(arg).await?.expect_int()?;
		let mut list = List::default();
		for i in 0..len {
			ctx.fule().await;
			list.get_mut()
				.push_back(Thunk::application(func.clone(), i.into()).into());
		}
		Ok(Value::List(list))
	}
}
