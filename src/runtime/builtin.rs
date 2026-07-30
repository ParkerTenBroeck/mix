use std::borrow::Cow;

use dumpster::Trace;

use crate::runtime::{
	Runtime,
	eval::{EvalError, NativeCtx, NativeLambdaAsync, NativeLambdaDyn, NativeLambdaResult},
	lazy::LazyValue,
	thunk::Thunk,
	value::{List, NativeLambda, Value},
};

#[derive(Clone)]
pub struct Match<T>(T);

unsafe impl<__V: ::dumpster::Visitor, T> ::dumpster::TraceWith<__V> for Match<T> {
	#[inline]
	fn accept(&self, _: &mut __V) -> ::core::result::Result<(), ()> {
		::core::result::Result::Ok(())
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
		let lazy = ctx.eval_lazy(arg).await?.expect_string()?;

		let regex =
			regex::Regex::new(&lazy).map_err(|err| EvalError::Internal(err.to_string().into()))?;

		Ok(NativeLambda::new(Match(regex)).into())
	}
}

impl NativeLambdaAsync for Match<regex::Regex> {
	fn identifier(&self) -> Cow<'static, str> {
		"match".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let lazy = ctx.eval_lazy(arg).await?.expect_string()?;

		let mut list = List::default();
		if let Some(captures) = self.0.captures(&lazy) {
			for capture in captures.iter().flatten() {
				list.get_mut().push_back(capture.as_str().to_owned().into());
			}
		}

		Ok(Value::List(list))
	}
}

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

			let element = Thunk::application(func.clone(), i.into()); // ctx.eval_call_func(func.clone(), i).await?;
			list.get_mut().push_back(element.into());
		}

		Ok(Value::List(list))
	}
}

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

		for el in list.get_mut() {
			*el = Thunk::application(lambda.clone(), std::mem::replace(el, false.into())).into();
		}
		Ok(list.into())
	}
}

#[derive(Trace, Clone)]
pub struct Import;

impl NativeLambdaAsync for Import {
	fn identifier(&self) -> Cow<'static, str> {
		"import".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let path = ctx.eval_lazy(arg).await?.expect_string()?;
		let res = ctx
			.runtime(|runtime| {
				runtime
					.load(&path)
					.map_err(|err| err.render(&runtime.loader.files()).join("\n"))
			})
			.await;
		match res {
			Ok(ok) => ctx.eval_lazy(ok).await,
			Err(err) => Err(EvalError::Internal(err.into())),
		}
	}
}
