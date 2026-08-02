use crate::runtime::{
	Runtime,
	eval::{ByteCodeFrame, EvalError, EvalStep, Fule, LocalEvaluator, ThunkResult},
	thunk::Thunk,
	value::{AttrSet, Lambda, List, Value, ValueType},
};

impl LocalEvaluator {
	pub(super) fn run_bytecode(
		&mut self,
		runtime: &mut Runtime,
		frame: &mut ByteCodeFrame,
		fule: &mut Fule,
	) -> Result<EvalStep, EvalError> {
		while fule.fule() {
			let res = self.step_bytecode(runtime, frame)?;
			if !matches!(res, EvalStep::Pending) {
				return Ok(res);
			}
		}
		Ok(EvalStep::Pending)
	}

	fn step_bytecode(
		&mut self,
		runtime: &mut Runtime,
		frame: &mut ByteCodeFrame,
	) -> Result<EvalStep, EvalError> {
		use crate::bytecode::OpCode;

		let Some((op, mut next_pos)) = runtime.program.get(frame.pos) else {
			return Err(EvalError::ByteCode("instruction pointer overran bytecode")).into();
		};

		match op {
			OpCode::Add => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = Self::checked_add(lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Sub => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = Self::checked_sub(lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Mul => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = Self::checked_mul(lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Div => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = Self::checked_div(lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Rem => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = Self::checked_rem(lhs, rhs)?;
				self.push_value(result)?;
			}
			op @ (OpCode::Eq | OpCode::Ne) => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = Self::binop_eq(op, lhs, rhs)?;
				self.push_value(result)?;
			}
			op @ (OpCode::Lt | OpCode::Lte | OpCode::Gt | OpCode::Gte) => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = Self::binop_cmp(op, lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Not => {
				let result = match self.pop_value()? {
					Value::Bool(bool) => Value::Bool(!bool),
					other => {
						return Err(EvalError::TypeMismatch {
							expected: ValueType::Bool,
							got: other.ty(),
						});
					}
				};
				self.push_value(result)?;
			}
			OpCode::Neg => {
				let result = match self.pop_value()? {
					Value::Int(int) => Value::Int(-int),
					Value::Float(float) => Value::Float(-float),
					other => {
						return Err(EvalError::TypeMismatch {
							expected: ValueType::Number,
							got: other.ty(),
						});
					}
				};
				self.push_value(result)?;
			}

			op @ (OpCode::And(rhs) | OpCode::Or(rhs) | OpCode::LogImp(rhs)) => {
				let lhs = self.pop_bool()?;
				let result = match op {
					OpCode::And(_) if !lhs => Some(false),
					OpCode::Or(_) if lhs => Some(true),
					OpCode::LogImp(_) if !lhs => Some(true),
					_ => None,
				};
				if let Some(result) = result {
					next_pos = next_pos + rhs;
					self.push_value(Value::Bool(result))?;
				}
			}

			OpCode::If(else_off) => {
				let cond = self.pop_bool()?;
				if !cond {
					next_pos = next_pos + else_off;
				}
			}
			OpCode::Branch(offset) => next_pos = next_pos + offset,

			OpCode::CreateAttrSet => {
				self.value_stack
					.push(Value::AttrSet(AttrSet::new_at(frame.pos)));
			}
			OpCode::SetAttr => {
				let name = self.pop_string()?;
				let mut attrset = self.pop_attrset()?;
				let value = self.pop_lazy()?;

				attrset.get_mut().insert(name, value);
				self.push_value(Value::AttrSet(attrset))?;
			}
			OpCode::CreateList(capacity) => {
				self.push_value(Value::List(List::with_capacity_at(capacity, frame.pos)))?
			}
			OpCode::AppendList => {
				let mut list = self.pop_list()?;
				let value = self.pop_lazy()?;
				list.get_mut().push_back(value);
				self.push_value(Value::List(list))?;
			}
			OpCode::Apply => {
				let func = self.pop_lambda()?;
				let arg = self.pop_lazy()?;

				match self.eval_apply(runtime, func, arg, None, false)? {
					ThunkResult::Value(value) => self.push_value(value)?,
					ThunkResult::Frame(next_frame) => {
						frame.pos = next_pos;
						return Ok(EvalStep::BeginFrame(next_frame));
					}
				}
			}
			OpCode::LoadLambda(lambda_id) => {
				let lambda = Lambda::Lambda {
					scope: frame.scope.clone(),
					lambda: lambda_id,
				};
				self.push_value(Value::Lambda(lambda))?;
			}
			OpCode::LoadStr(str) => self.push_value(Value::String(
				runtime
					.program
					.get_str(str)
					.ok_or(EvalError::ByteCode("invalid string id"))?,
			))?,
			OpCode::LoadInt(int) => self.push_value(Value::Int(int))?,
			OpCode::LoadFloat(float) => self.push_value(Value::Float(float))?,
			OpCode::LoadBool(bool) => self.push_value(Value::Bool(bool))?,

			OpCode::HasAttr => {
				let name = self.pop_string()?;
				let attrset = self.pop_attrset()?;
				self.push_value(Value::Bool(attrset.get(&name).is_some()))?;
			}
			OpCode::GetAttr => {
				let index = self.pop_value()?;
				let indexing = self.pop_value()?;
				let lazy = Self::get_attr(&indexing, &index)?;

				if let Some(lazy) = lazy {
					self.push_lazy(lazy)?;
				} else {
					let idx = match index {
						Value::Bool(bool) => format!("{bool}"),
						Value::Int(int) => format!("{int}"),
						Value::Float(float) => format!("{float}"),
						Value::String(str) => format!("{str:?}"),
						Value::Path(path_buf) => path_buf.display().to_string(),
						other => other.ty().to_string(),
					};
					return Err(EvalError::MissingAttr(
						format!("attr {idx} not found for {}", indexing.ty()).into(),
					));
				}
			}
			OpCode::GetAttrOr(else_off) => {
				let index = self.pop_value()?;
				let indexing = self.pop_value()?;
				let lazy = Self::get_attr(&indexing, &index).ok().flatten();
				if let Some(lazy) = lazy {
					self.push_lazy(lazy)?;
				} else {
					next_pos = next_pos + else_off;
				}
			}
			OpCode::EvalThunk => {
				let lazy = self.pop_lazy()?;

				match self.eval_lazy(runtime, lazy, false)? {
					ThunkResult::Value(value) => self.push_value(value)?,
					ThunkResult::Frame(next_frame) => {
						frame.pos = next_pos;
						return Ok(EvalStep::BeginFrame(next_frame));
					}
				}
			}
			OpCode::UnEvalValue => {
				let value = self.pop_value()?;
				self.push_lazy(value.into())?;
			}
			OpCode::BindThunkScope => {
				let attr = self.pop_string()?;
				let lazy = self.pop_lazy()?;
				frame.scope.bind(attr, lazy);
			}
			OpCode::BindValueScope => {
				let attr = self.pop_string()?;
				let value = self.pop_value()?;
				frame.scope.bind(attr, value.into());
			}

			OpCode::CreateThunk(code_pos) => {
				self.push_lazy(Thunk::uneval(code_pos, frame.scope.clone()).into())?
			}
			OpCode::BeginThunk(code_pos) => {
				self.push_lazy(Thunk::construct_begin(code_pos).into())?
			}
			OpCode::FinalizeThunk => {
				let succ = self
					.peek_lazy()?
					.thunk()
					.map_or(false, |t| t.construct_end(frame.scope.clone()));
				if !succ {
					return Err(EvalError::ByteCode(
						"attempted to finalize thunk which has already been finalized",
					));
				}
			}

			OpCode::LoadScope => {
				let name = self.pop_string()?;
				let Some(lazy) = frame.scope.resolve(&name) else {
					return Err(EvalError::MissingBinding(
						format!("failed to resolve {name:?}").into(),
					));
				};
				self.push_lazy(lazy.clone())?;
			}
			OpCode::EnterScope => frame.scope = frame.scope.new_level(),
			OpCode::LeaveScope => {
				frame.scope = frame
					.scope
					.parent()
					.ok_or(EvalError::ByteCode("attempted to leave the root scope"))?;
			}

			OpCode::PopV => _ = self.pop_value()?,
			OpCode::DupV => {
				let value = self.pop_value()?;
				self.push_value(value.clone())?;
				self.push_value(value)?;
			}

			OpCode::PopT => _ = self.pop_lazy()?,
			OpCode::DupT => {
				let lazy = self.pop_lazy()?;
				self.push_lazy(lazy.clone())?;
				self.push_lazy(lazy)?;
			}

			OpCode::Ret => {
				return Ok(EvalStep::Ret);
			}
		}
		frame.pos = next_pos;

		Ok(EvalStep::Pending)
	}
}
