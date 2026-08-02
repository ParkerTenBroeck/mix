use super::*;

pub enum ApplyResult {
	Value(Value),
	Frame(FrameKind),
}

impl LocalEvaluator {
	pub fn apply(
		&mut self,
		runtime: &mut Runtime,
		lambda: Lambda,
		arg: LazyValue,
	) -> Result<ApplyResult, EvalError> {
		match lambda {
			Lambda::Lambda { scope, lambda } => {
				let lambda = runtime.program.get_lambda(lambda).ok_or_else(|| {
					EvalError::Internal(
						format!("invalid lambda id {} in bytecode", lambda.index()).into(),
					)
				})?;

				self.push_lazy(arg)?;

				Ok(ApplyResult::Frame(FrameKind::ByteCode(ByteCodeFrame {
					pos: lambda.code,
					scope: scope.new_level(),
				})))
			}
			Lambda::NativeLambda(native_lambda) => {
				Self::apply_native_lambda(runtime, native_lambda, arg)
			}
		}
	}
}
