use super::*;

impl LocalEvaluator {
	fn spill_deep_value(&mut self, value: &Value) -> Result<u32, EvalError> {
		let start = self.thunk_stack.len();
		match &value {
			Value::AttrSet(attrs) => {
				for lazy in attrs.values() {
					self.push_thunk(lazy.clone())?;
				}
			}
			Value::List(list) => {
				for lazy in list.iter() {
					self.push_thunk(lazy.clone())?;
				}
			}
			_ => {}
		}
		Ok((self.thunk_stack.len() - start) as u32)
	}

	pub(super) fn apply(
		&mut self,
		runtime: &mut Runtime,
		func: Value,
		arg: LazyValue,
	) -> Result<ByteCodeStep, EvalError> {
		let lambda = func.expect_lambda()?;

		match lambda {
			Lambda::Lambda { scope, lambda } => {
				let lambda = runtime.program.get_lambda(lambda).ok_or_else(|| {
					EvalError::Internal(
						format!("invalid lambda id {} in bytecode", lambda.index()).into(),
					)
				})?;

				self.thunk_stack.push(arg);

				Ok(ByteCodeStep::BeginFrame(Frame {
					kind: FrameKind::Function {
						eval: EvalFrame {
							pos: lambda.code,
							scope: scope.new_level(),
						},
					},
				}))
			}
			Lambda::NativeLambda(native_lambda) => {
				self.apply_native_lambda(runtime, native_lambda, arg)
			}
		}
	}
}
