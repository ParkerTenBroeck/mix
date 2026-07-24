use std::ops::Deref;

use dumpster::{Trace, unsync::Gc};

use crate::runtime::{
	Runtime,
	eval::{EvalError, Evaluator, NativeCtx},
	lazy::LazyValue,
	value::{List, Value},
};

#[derive(Clone, Trace)]
pub struct NativeLambda {
	inner: Gc<Box<dyn NativeLambdaTrait>>, // silly rust
}

impl Deref for NativeLambda {
	type Target = dyn NativeLambdaTrait;

	fn deref(&self) -> &Self::Target {
		&**self.inner
	}
}

impl NativeLambda {
	pub fn new<T: NativeLambdaTrait>(lambda: T) -> Self {
		Self {
			inner: Gc::new(Box::new(lambda)),
		}
	}
}

pub type NativeLambdaState = std::pin::Pin<Box<dyn Future<Output = Result<Value, EvalError>>>>;

pub enum NativeLambdaResult {
	Value(Value),
	Future(NativeLambdaState),
	Err(EvalError),
}

pub trait NativeLambdaTrait: Trace + 'static {
	fn identifier(&self) -> &str;
	fn begin(
		&self,
		evaluator: &mut Evaluator,
		runtime: &mut Runtime,
		arg: LazyValue,
	) -> NativeLambdaResult;
}

impl std::fmt::Debug for NativeLambda {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("NativeLambda")
			.field("identifer", &self.inner.identifier())
			.finish()
	}
}

#[derive(Trace)]
pub struct Match;

impl NativeLambdaTrait for Match {
	fn identifier(&self) -> &str {
		"match"
	}

	fn begin(&self, _: &mut Evaluator, _: &mut Runtime, arg: LazyValue) -> NativeLambdaResult {
		NativeLambdaResult::Future(Box::pin(async {
			let lazy = NativeCtx::eval_lazy(arg).await.expect_string()?;
			let regex = regex::Regex::new(&lazy)
				.map_err(|err| EvalError::Internal(err.to_string().into()))?;

			Ok(Value::Lambda(super::value::Lambda::NativeLambda(
				NativeLambda::new(Matcher(regex)),
			)))
		}))
	}
}

pub struct Matcher(regex::Regex);

unsafe impl<__V: ::dumpster::Visitor> ::dumpster::TraceWith<__V> for Matcher {
	#[inline]
	fn accept(&self, visitor: &mut __V) -> ::core::result::Result<(), ()> {
		::core::result::Result::Ok(())
	}
}

impl NativeLambdaTrait for Matcher {
	fn identifier(&self) -> &str {
		"matcher"
	}

	fn begin(&self, _: &mut Evaluator, _: &mut Runtime, arg: LazyValue) -> NativeLambdaResult {
		let reg = self.0.clone();
		NativeLambdaResult::Future(Box::pin(async move {
			let lazy = NativeCtx::eval_lazy(arg).await.expect_string()?;

			let mut list = List::default();
			if let Some(captures) = reg.captures(&lazy) {
				for capture in captures.iter().flatten() {
					list.get_mut().push_back(capture.as_str().to_owned().into());
				}
			}

			Ok(Value::List(list))
		}))
	}
}
