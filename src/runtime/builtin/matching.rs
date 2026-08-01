use std::borrow::Cow;

use crate::runtime::{
	eval::{EvalError, NativeCtx, NativeLambdaAsync},
	lazy::LazyValue,
	value::{List, NativeLambda, Value},
};

#[derive(Clone)]
pub struct Match<T>(T);

unsafe impl<Visitor: dumpster::Visitor, T> dumpster::TraceWith<Visitor> for Match<T> {
	#[inline]
	fn accept(&self, _: &mut Visitor) -> Result<(), ()> {
		Ok(())
	}
}

impl Match<()> {
	pub fn new() -> Self {
		Self(())
	}
}

impl NativeLambdaAsync for Match<()> {
	fn identifier(&self) -> Cow<'static, str> {
		"match".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let pattern = ctx.eval_lazy(arg).await?.expect_string()?;
		let regex = regex::Regex::new(&pattern)
			.map_err(|error| EvalError::Internal(error.to_string().into()))?;
		Ok(NativeLambda::new(Match(regex)).into())
	}
}

impl NativeLambdaAsync for Match<regex::Regex> {
	fn identifier(&self) -> Cow<'static, str> {
		"match".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let input = ctx.eval_lazy(arg).await?.expect_string()?;
		let mut list = List::default();
		if let Some(captures) = self.0.captures(&input) {
			for capture in captures.iter().flatten() {
				list.get_mut().push_back(capture.as_str().to_owned().into());
			}
		}
		Ok(Value::List(list))
	}
}
