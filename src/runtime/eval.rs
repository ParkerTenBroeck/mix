mod attr;
mod binop;
mod error;
mod frame;
mod func;
mod native;

pub use error::*;
pub use frame::*;

pub use native::*;

use std::{collections::HashSet, num::NonZeroUsize};

use crate::{
	bytecode::{CodePos, OpCode},
	runtime::{
		LazyValue, Runtime, Value,
		thunk::Thunk,
		trace::ErrorTrace,
		value::{AttrSet, Lambda, List, StringKind, ValueType},
	},
};

#[derive(Default)]
pub struct Evaluator {
	pub local: LocalEvaluator,
	pub frames: Vec<Frame>,
}

pub struct Fule(Option<NonZeroUsize>);

impl Fule {
	pub fn unlimited() -> Self {
		Self(None)
	}

	pub fn limited(amount: usize) -> Self {
		Self(Some(NonZeroUsize::new(amount.saturating_add(1))).unwrap())
	}

	pub fn fule(&mut self) -> bool {
		match self.0 {
			None => true,
			Some(ammount) => {
				if let Some(fule) = NonZeroUsize::new(ammount.get() - 1) {
					self.0 = Some(fule);
					true
				} else {
					false
				}
			}
		}
	}
}

impl Evaluator {
	pub fn begin_eval(thunk: Thunk, recursive: bool) -> Result<Evaluator, ErrorTrace> {
		let (pos, scope, thunk) = match thunk.eval_begin() {
			Ok((pos, scope)) => (pos, scope, thunk),
			Err(err) => {
				return Err(ErrorTrace {
					kind: EvalError::ThunkEval(err),
					stack: Vec::new(),
				});
			}
		};
		let frame = Frame {
			kind: FrameKind::Thunk {
				eval: EvalFrame { pos, scope },
				thunk,
			},
		};

		Ok(Self {
			local: Default::default(),
			frames: vec![frame],
			// frame: Frame::new(pos, scope, meta),
		})
	}

	fn begin_frame(&mut self, frame: Frame) -> Result<(), EvalError> {
		self.frames.push(frame);
		Ok(())
	}

	fn pop_frame(&mut self) -> Result<Frame, EvalError> {
		self.frames.pop().ok_or(EvalError::ByteCode("call stack"))
	}

	pub fn run(
		&mut self,
		runtime: &mut Runtime,
		mut fule: Fule,
	) -> Result<Option<Value>, EvalError> {
		loop {
			let Some(frame) = self.frames.last_mut() else {
				todo!()
			};

			let res = match &mut frame.kind {
				FrameKind::Function { eval } => {
					self.local.run_bytecode(runtime, eval, &mut fule)?
				}
				FrameKind::Thunk { eval, thunk } => {
					let res = self.local.run_bytecode(runtime, eval, &mut fule)?;
					if matches!(res, ByteCodeStep::Ret) {
						thunk.eval_end(self.local.peek_value()?, false).unwrap();
					}
					res
				}
				FrameKind::Native { state, name } => {
					self.local.poll_native_lambda(runtime, &mut fule, state.as_mut())?
				}
				FrameKind::Deep { pos, remaining } => {
					todo!()
				}
			};

			match res {
				// might want to error on this
				ByteCodeStep::Pending if !fule.fule() => return Ok(None),
				ByteCodeStep::Pending => {}
				ByteCodeStep::Ret if self.frames.len() == 1 => {
					return Ok(Some(self.local.pop_value()?));
				}
				ByteCodeStep::Ret => _ = self.pop_frame()?,
				ByteCodeStep::BeginFrame(frame) => {
					self.begin_frame(frame)?;
				}
			}
		}
	}
}

#[derive(Default)]
pub struct LocalEvaluator {
	pub value_stack: Vec<Value>,
	pub thunk_stack: Vec<LazyValue>,
}

impl LocalEvaluator {
	fn push_value(&mut self, value: Value) -> Result<(), EvalError> {
		self.value_stack.push(value);
		Ok(())
	}

	fn peek_value(&mut self) -> Result<Value, EvalError> {
		self.value_stack
			.last()
			.cloned()
			.ok_or(EvalError::ByteCode("value stack"))
	}

	fn pop_value(&mut self) -> Result<Value, EvalError> {
		self.value_stack
			.pop()
			.ok_or(EvalError::ByteCode("value stack"))
	}

	fn pop_bool(&mut self) -> Result<bool, EvalError> {
		self.pop_value()?.expect_bool()
	}

	fn pop_string(&mut self) -> Result<StringKind, EvalError> {
		self.pop_value()?.expect_string()
	}

	fn pop_list(&mut self) -> Result<List, EvalError> {
		self.pop_value()?.expect_list()
	}

	fn pop_attrset(&mut self) -> Result<AttrSet, EvalError> {
		self.pop_value()?.expect_attrset()
	}

	fn pop_lambda(&mut self) -> Result<Lambda, EvalError> {
		self.pop_value()?.expect_lambda()
	}

	fn push_thunk(&mut self, value: LazyValue) -> Result<(), EvalError> {
		self.thunk_stack.push(value);
		Ok(())
	}

	fn pop_thunk(&mut self) -> Result<LazyValue, EvalError> {
		self.thunk_stack
			.pop()
			.ok_or(EvalError::ByteCode("thunk stack"))
	}
}

enum ByteCodeStep {
	Pending,
	Ret,
	BeginFrame(Frame),
}

impl LocalEvaluator {
	fn run_bytecode(
		&mut self,
		runtime: &mut Runtime,
		frame: &mut EvalFrame,
		fule: &mut Fule,
	) -> Result<ByteCodeStep, EvalError> {
		while fule.fule() {
			let res = self.step_bytecode(runtime, frame)?;
			if !matches!(res, ByteCodeStep::Pending) {
				return Ok(res);
			}
		}
		Ok(ByteCodeStep::Pending)
	}

	fn step_bytecode(
		&mut self,
		runtime: &mut Runtime,
		frame: &mut EvalFrame,
	) -> Result<ByteCodeStep, EvalError> {
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
				self.value_stack.push(Value::AttrSet(AttrSet::default()));
			}
			OpCode::InitAttrExpr(expr) => {
				let name = self.pop_string()?;
				let mut attrset = self.pop_attrset()?;

				attrset
					.get_mut()
					.insert(name, LazyValue::construct_begin(expr));
				self.push_value(Value::AttrSet(attrset))?;
			}
			op @ (OpCode::FinalizeAttrSetRec | OpCode::FinalizeAttrSet) => {
				let attrset = self.pop_attrset()?;
				let scope = if op == OpCode::FinalizeAttrSetRec {
					let mut scope = frame.scope.clone();
					for (name, value) in attrset.iter() {
						scope.bind(name.clone(), value.clone());
					}
					scope
				} else {
					frame.scope.clone()
				};

				for element in attrset.values() {
					// ignore result as some values might have already been finalized (inherited from elsewhere)
					_ = element.construct_end(scope.clone());
				}
				self.push_value(Value::AttrSet(attrset))?;
			}
			OpCode::CreateList(capacity) => {
				self.push_value(Value::List(List::with_capacity(capacity)))?
			}
			OpCode::AppendList(expr) => {
				let mut list = self.pop_list()?;
				list.get_mut()
					.push_back(LazyValue::uneval(expr, frame.scope.clone()));
				self.push_value(Value::List(list))?;
			}
			OpCode::ApplyWith(arg_pos) => {
				let arg = Thunk::uneval_with_scope(arg_pos, frame.scope.clone()).into();
				frame.pos = next_pos;
				return self.apply(runtime, arg);
			}
			OpCode::Apply => {
				let arg = self.pop_thunk()?;
				frame.pos = next_pos;
				return self.apply(runtime, arg);
			}

			OpCode::LoadLambda(lambda_id) => {
				let lambda = Lambda::Lambda {
					scope: frame.scope.clone(),
					lambda: lambda_id,
				};
				self.push_value(Value::Lambda(lambda))?;
			}
			OpCode::LoadStr(str) => self.push_value(Value::String(runtime.program.get_str(str)))?,
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
					self.push_thunk(lazy)?;
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
					self.thunk_stack.push(lazy);
				} else {
					next_pos = next_pos + else_off;
				}
			}
			OpCode::EvalThunk => {
				let thunk = self.pop_thunk()?;
				match thunk.try_get_value() {
					Ok(value) => self.push_value(value)?,
					Err(thunk) => {
						let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;

						frame.pos = next_pos;

						return Ok(ByteCodeStep::BeginFrame(Frame {
							kind: FrameKind::Thunk {
								eval: EvalFrame { pos, scope },
								thunk,
							},
						}));
					}
				}
			}
			OpCode::BindThunkScope => {
				let attr = self.pop_string()?;
				let thunk = self.pop_thunk()?;
				frame.scope.bind(attr, thunk);
			}
			OpCode::BindValueScope => {
				let attr = self.pop_string()?;
				let value = self.pop_value()?;
				frame.scope.bind(attr, value.into());
			}

			OpCode::LoadScope => {
				let name = self.pop_string()?;
				let Some(lazy) = frame.scope.resolve(&name) else {
					return Err(EvalError::MissingBinding(
						format!("failed to resolve {name:?}").into(),
					));
				};
				self.push_thunk(lazy.clone())?;
			}

			OpCode::PopV => _ = self.pop_value()?,
			OpCode::DupV => {
				let value = self.pop_value()?;
				self.push_value(value.clone())?;
				self.push_value(value)?;
			}

			OpCode::PopT => _ = self.pop_thunk()?,
			OpCode::DupT => {
				let thunk = self.pop_thunk()?;
				self.push_thunk(thunk.clone())?;
				self.push_thunk(thunk)?;
			}

			OpCode::Ret => {
				return Ok(ByteCodeStep::Ret);
			}
		}
		frame.pos = next_pos;

		Ok(ByteCodeStep::Pending)
	}
}
