mod frame;
mod error;
mod binop;
mod func;
mod attr;

pub use frame::*;
pub use error::*;

use std::{collections::HashSet};

use crate::{
	bytecode::{CodeLocOffset, CodePos, OpCode}, runtime::{
		LazyValue, Runtime, Value, thunk::{Thunk}, trace::ErrorTrace, value::{AttrSet, Lambda, List, StringKind, ValueType},
	},
};


pub struct Evaluator {
	pub value_stack: Vec<Value>,
	pub thunk_stack: Vec<LazyValue>,

	pub frame_stack: Vec<PotentialFrame>,
	pub curr_frame: Frame,

	pub deeply_evaluated: HashSet<usize>,
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
		let frame_kind = if recursive {
			FrameKind::ThunkEvalDeepRoot(thunk)
		} else {
			FrameKind::ThunkEval(thunk)
		};

		Ok(Self {
			value_stack: Default::default(),
			thunk_stack: Default::default(),
			frame_stack: Default::default(),
			curr_frame: Frame::new(pos, scope, frame_kind),
			deeply_evaluated: Default::default(),
		})
	}

	pub fn begin_call(
		runtime: &Runtime,
		func: Value,
		arg: LazyValue,
		recursive: bool,
	) -> Result<Evaluator, ErrorTrace> {
		let lambda = match func {
			Value::Lambda(Lambda::Lambda { scope, lambda }) => {
				let Some(lambda) = runtime.program.get_lambda(lambda) else {
					return Err(ErrorTrace {
						kind: EvalError::Internal(
							format!("invalid lambda id {} in bytecode", lambda.index()).into(),
						),
						stack: Vec::new(),
					});
				};
				let frame_kind = if recursive {
					FrameKind::FunctionDeepRoot
				} else {
					FrameKind::Function
				};
				Frame::new(lambda.code, scope.new_level(), frame_kind)
			}
			Value::Lambda(Lambda::NativeLambda(_)) => {
				return Err(ErrorTrace {
					kind: EvalError::Internal(
						"cannot begin bytecode evaluation for a native function".into(),
					),
					stack: Vec::new(),
				});
			}
			other => {
				return Err(ErrorTrace {
					kind: EvalError::TypeMismatch {
						expected: ValueType::Lambda,
						got: other.ty(),
					},
					stack: Vec::new(),
				});
			}
		};

		Ok(Self {
			value_stack: Default::default(),
			thunk_stack: vec![arg],
			frame_stack: Default::default(),
			curr_frame: lambda,
			deeply_evaluated: Default::default(),
		})
	}

	fn push_value(&mut self, value: Value) -> Result<(), EvalError> {
		self.value_stack.push(value);
		Ok(())
	}

	fn pop_value(&mut self) -> Result<Value, EvalError> {
		self.value_stack
			.pop()
			.ok_or(EvalError::ByteCode("value stack"))
	}

	fn pop_bool(&mut self) -> Result<bool, EvalError> {
		let value = self.pop_value()?;
		match value {
			Value::Bool(value) => Ok(value),
			other => Err(EvalError::TypeMismatch {
				expected: ValueType::Bool,
				got: other.ty(),
			}),
		}
	}

	fn pop_string(&mut self) -> Result<StringKind, EvalError> {
		let value = self.pop_value()?;
		match value {
			Value::String(value) => Ok(value),
			other => Err(EvalError::TypeMismatch {
				expected: ValueType::String,
				got: other.ty(),
			}),
		}
	}

	fn pop_list(&mut self) -> Result<List, EvalError> {
		let value = self.pop_value()?;
		match value {
			Value::List(value) => Ok(value),
			other => Err(EvalError::TypeMismatch {
				expected: ValueType::List,
				got: other.ty(),
			}),
		}
	}

	fn pop_attrset(&mut self) -> Result<AttrSet, EvalError> {
		let value = self.pop_value()?;
		match value {
			Value::AttrSet(value) => Ok(value),
			other => Err(EvalError::TypeMismatch {
				expected: ValueType::AttrSet,
				got: other.ty(),
			}),
		}
	}

	fn pop_lambda(&mut self) -> Result<Lambda, EvalError> {
		let value = self.pop_value()?;
		match value {
			Value::Lambda(value) => Ok(value),
			other => Err(EvalError::TypeMismatch {
				expected: ValueType::Lambda,
				got: other.ty(),
			}),
		}
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

	fn begin_frame(&mut self, mut frame: Frame) -> Result<(), EvalError> {
		std::mem::swap(&mut self.curr_frame, &mut frame);
		self.frame_stack.push(PotentialFrame::Realized(frame));
		Ok(())
	}

	fn pop_frame(&mut self) -> Result<PotentialFrame, EvalError> {
		self.frame_stack
			.pop()
			.ok_or(EvalError::ByteCode("call stack"))
	}

	fn next_op(&mut self, runtime: &Runtime) -> Result<OpCode, EvalError> {
		let Some((op, pos)) = runtime.program.get(self.curr_frame.pos) else {
			return Err(EvalError::ByteCode("instruction pointer overran bytecode"));
		};
		self.curr_frame.pos = pos;
		Ok(op)
	}

	fn branch(&mut self, off: CodeLocOffset) {
		self.curr_frame.pos = self.curr_frame.pos + off;
	}


	pub fn run(&mut self, runtime: &Runtime) -> Result<Value, ErrorTrace> {
		loop {
			match self.do_step(runtime){
				Ok(Some(ret)) => return Ok(ret),
				Ok(None) => {},
				Err(err) => return Err(ErrorTrace::build(runtime, self, err)),
			}
		}
	}

	pub fn run_for(&mut self, runtime: &Runtime, steps: usize) -> Result<Option<Value>, ErrorTrace> {
		for _ in 0..steps {
			match self.do_step(runtime) {
				Ok(Some(ret)) => return Ok(Some(ret)),
				Ok(None) => {},
				Err(err) => return Err(ErrorTrace::build(runtime, self, err)),
			}
		}
		Ok(None)
	}

	fn do_step(&mut self, runtime: &Runtime) -> Result<Option<Value>, EvalError> {
		use crate::bytecode::OpCode;

		let prev = self.curr_frame.pos;
		match self.next_op(runtime)? {
			OpCode::Add => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = self.checked_add(lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Sub => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = self.checked_sub(lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Mul => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = self.checked_mul(lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Div => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = self.checked_div(lhs, rhs)?;
				self.push_value(result)?;
			}
			OpCode::Rem => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = self.checked_rem(lhs, rhs)?;
				self.push_value(result)?;
			}
			op @ (OpCode::Eq | OpCode::Ne) => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = self.binop_eq(op, lhs, rhs)?;
				self.push_value(result)?;
			}
			op @ (OpCode::Lt | OpCode::Lte | OpCode::Gt | OpCode::Gte) => {
				let rhs = self.pop_value()?;
				let lhs = self.pop_value()?;
				let result = self.binop_cmp(op, lhs, rhs)?;
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
					self.branch(rhs);
					self.push_value(Value::Bool(result))?;
				}
			}

			OpCode::If(else_off) => {
				let cond = self.pop_bool()?;
				if !cond {
					self.branch(else_off);
				}
			}
			OpCode::Branch(offset) => self.branch(offset),

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
					let mut scope = self.curr_frame.scope.clone();
					for (name, value) in attrset.iter() {
						scope.bind(name.clone(), value.clone());
					}
					scope
				} else {
					self.curr_frame.scope.clone()
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
					.push_back(LazyValue::uneval(expr, self.curr_frame.scope.clone()));
				self.push_value(Value::List(list))?;
			}
			OpCode::Apply(arg_pos) => self.apply(runtime, arg_pos)?,

			OpCode::LoadLambda(lambda_id) => {
				let lambda = Lambda::Lambda {
					scope: self.curr_frame.scope.clone(),
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
				let lazy = self.get_attr(&indexing, &index)?;

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
				let lazy = self.get_attr(&indexing, &index).ok().flatten();
				if let Some(lazy) = lazy {
					self.thunk_stack.push(lazy);
				} else {
					self.branch(else_off);
				}
			}
			OpCode::EvalThunk => {
				let thunk = self.pop_thunk()?;
				match thunk.try_get_value() {
					Ok(value) => self.push_value(value)?,
					Err(thunk) => {
						let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;
						self.begin_frame(Frame::new(pos, scope, FrameKind::ThunkEval(thunk)))?;
					}
				}
			}
			OpCode::BindThunkScope => {
				let attr = self.pop_string()?;
				let thunk = self.pop_thunk()?;
				self.curr_frame.scope.bind(attr, thunk);
			}
			OpCode::BindValueScope => {
				let attr = self.pop_string()?;
				let value = self.pop_value()?;
				self.curr_frame.scope.bind(attr, value.into());
			}

			OpCode::LoadScope => {
				let name = self.pop_string()?;
				let Some(lazy) = self.curr_frame.scope.resolve(&name) else {
					return Err(EvalError::MissingBinding(
						format!("failed to resolve {name:?}").into(),
					));
				};
				match lazy.try_get_value() {
					Ok(ok) => self.push_value(ok)?,
					Err(thunk) => {
						let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;
						self.begin_frame(Frame::new(pos, scope, FrameKind::ThunkEval(thunk)))?;
					}
				}
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
				if let Some(value) = self.ret(prev)? {
					return Ok(Some(value));
				}
			}
		}
		Ok(None)
	}
}
