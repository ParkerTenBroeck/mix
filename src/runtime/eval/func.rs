use super::*;

impl Evaluator{
	fn spill_deep_value(&mut self, value: &Value) -> Result<(), EvalError> {
		match &value {
			Value::AttrSet(attrs) => {
				if !self.deeply_evaluated.insert(attrs.id()) {
					return Ok(());
				}
				for lazy in attrs.values() {
					self.frame_stack
						.push(PotentialFrame::PotentialDeep(lazy.clone()));
				}
			}
			Value::List(list) => {
				if !self.deeply_evaluated.insert(list.id()) {
					return Ok(());
				}
				for lazy in list.iter() {
					self.frame_stack
						.push(PotentialFrame::PotentialDeep(lazy.clone()));
				}
			}
			_ => {}
		}
		Ok(())
	}

	pub(super) fn ret(&mut self, prev: CodePos) -> Result<Option<Value>, EvalError> {
		let ret = self.pop_value()?;

		// update the thunk if the current frame was evaluating a thunk
		match &self.curr_frame.kind {
			FrameKind::ThunkEval(thunk)
			| FrameKind::ThunkEvalDeep(thunk)
			| FrameKind::ThunkEvalDeepRoot(thunk) => {
				thunk.eval_end(ret.clone()).map_err(|()| {
					EvalError::Internal(
						"tried to finish a thunk that was not currently evaluating".into(),
					)
				})?;
			}
			_ => {}
		}

		// if the current frame is in a deep eval spill inner values onto evaluation stack
		match &self.curr_frame.kind {
			FrameKind::FunctionDeepRoot
			| FrameKind::ThunkEvalDeep(_)
			| FrameKind::ThunkEvalDeepRoot(_) => {
				self.frame_stack.push(PotentialFrame::DeepEval(prev));
				self.spill_deep_value(&ret)?;
			}
			_ => {}
		}

		// push value onto stack if this frame should produce a return value
		match &self.curr_frame.kind {
			FrameKind::Function
			| FrameKind::FunctionDeepRoot
			| FrameKind::ThunkEval(_)
			| FrameKind::ThunkEvalDeepRoot(_) => {
				self.push_value(ret)?;
			}
			_ => {}
		}

		while !self.frame_stack.is_empty() {
			match self.pop_frame()? {
				PotentialFrame::Realized(frame) => {
					self.curr_frame = frame;
					return Ok(None);
				}
				PotentialFrame::DeepEval(_) => {}
				PotentialFrame::PotentialDeep(thunk) => {
					let thunk = match thunk.try_get_value() {
						Ok(value) => {
							self.spill_deep_value(&value)?;
							continue;
						}
						Err(thunk) => thunk,
					};
					let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;
					self.curr_frame = Frame::new(pos, scope, FrameKind::ThunkEvalDeep(thunk));
					return Ok(None);
				}
			}
		}

		// return resulting value from evaluator
		if self.frame_stack.is_empty() {
			return Ok(Some(self.pop_value()?));
		}
		Ok(None)
	}

	pub(super) fn apply(&mut self, runtime: &Runtime, arg_pos: CodePos) -> Result<(), EvalError> {
		let lambda = self.pop_lambda()?;

		match lambda {
			Lambda::Lambda { scope, lambda } => {
				let lambda = runtime.program.get_lambda(lambda).ok_or_else(|| {
					EvalError::Internal(
						format!("invalid lambda id {} in bytecode", lambda.index()).into(),
					)
				})?;

				let thunk = Thunk::uneval_with_scope(arg_pos, self.curr_frame.scope.clone()).into();
				self.thunk_stack.push(thunk);

				let frame = Frame::new(lambda.code, scope.new_level(), FrameKind::Function);
				self.begin_frame(frame)?
			}
			Lambda::NativeLambda(native_lambda) => {
				let value = native_lambda.call()?;
				self.push_value(value)?
			}
		};
		Ok(())
	}
}
