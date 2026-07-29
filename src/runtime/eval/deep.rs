use crate::{
	bytecode::CodePos,
	runtime::{
		eval::{
			EvalError, Frame, FrameKind, LocalEvaluator, NativeCtx, NativeFrame, NativeLambdaAsync,
			NativePosKind, ThunkResult,
		},
		lazy::{LazyValue, LazyValueKind},
		value::Value,
	},
};

async fn deep_eval_frame(mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
	let res = ctx.eval_lazy(arg).await?;
	match &res {
		Value::List(list) => {
			for el in list.iter() {
				ctx.eval_lazy_deep(el.clone()).await?;
			}
		}
		Value::AttrSet(attr_set) => {
			for el in attr_set.values() {
				ctx.eval_lazy_deep(el.clone()).await?;
			}
		}
		_ => {}
	}
	Ok(res)
}

impl LocalEvaluator {
	pub(super) fn deep_eval_value(&mut self, value: Value) -> Result<ThunkResult, EvalError> {
		let mut not_deep_evaluated = 0;
		match &value {
			Value::AttrSet(attrs) => {
				for lazy in attrs.values() {
					if let LazyValueKind::Value(value) = lazy.try_get_value()
						&& value.deeply_evaluated()
					{
						continue;
					}
					not_deep_evaluated += 1;
				}
			}
			Value::List(list) => {
				for lazy in list.iter() {
					if let LazyValueKind::Value(value) = lazy.try_get_value()
						&& value.deeply_evaluated()
					{
						continue;
					}
					not_deep_evaluated += 1;
				}
			}
			_ => {}
		}

		value.set_deeply_evaluated();

		if not_deep_evaluated == 0 {
			Ok(ThunkResult::Value(value))
		} else {
			let pos = value
				.creation_pos()
				.map(NativePosKind::Value)
				.unwrap_or_default();
			Ok(ThunkResult::Frame(self.get_deep_frame(pos, value)))
		}
	}

	pub(super) fn get_deep_frame(&self, pos: NativePosKind, value: Value) -> Frame {
		value.set_deeply_evaluated();

		let future = deep_eval_frame(NativeCtx::get(), value.into());
		let state = Box::pin(future);
		let kind = FrameKind::Native(NativeFrame {
			state,
			name: "deep eval".into(),
			pos,
		});
		Frame {
			kind,
			thunk: None,
			deep: false,
		}
	}
}
