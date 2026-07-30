use std::{cell::RefCell, fmt};

use dumpster::{Trace, unsync::Gc};

use crate::{
	bytecode::CodePos,
	runtime::{
		lazy::LazyValue,
		scope::Scope,
		value::{Lambda, Value},
	},
};

#[derive(Clone, Trace)]
pub struct Thunk(pub(super) Gc<RefCell<ThunkState>>);

#[derive(Clone, Debug)]
pub enum ThunkSnapshot {
	Constructing(CodePos),
	Expr(CodePos),
	Apply(Lambda, LazyValue),
	Evaluating,
	Evaluated(Value),
}

impl fmt::Debug for Thunk {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.0.try_borrow() {
			Ok(state) => f.debug_tuple("Thunk").field(&*state).finish(),
			Err(_) => f.debug_tuple("Thunk").field(&"<borrowed>").finish(),
		}
	}
}

impl Thunk {
	pub fn id(&self) -> usize {
		Gc::as_ptr(&self.0) as *const () as usize
	}

	pub fn construct_begin(pos: CodePos) -> Self {
		Self(Gc::new(RefCell::new(ThunkState::Constructing(pos))))
	}

	pub fn uneval_with_scope(pos: CodePos, scope: Scope) -> Self {
		Self(Gc::new(RefCell::new(ThunkState::Expr(pos, scope))))
	}

	pub fn application(func: Lambda, arg: LazyValue) -> Self {
		Self(Gc::new(RefCell::new(ThunkState::Apply(func, arg))))
	}

	pub fn construct_end(&self, scope: Scope) -> bool {
		let mut inner = self.0.borrow_mut();
		match &*inner {
			ThunkState::Constructing(code_loc) => {
				*inner = ThunkState::Expr(*code_loc, scope);
				true
			}
			_ => false,
		}
	}

	pub fn eval_end(&self, value: Value) -> Result<(), ()> {
		let mut inner = self.0.borrow_mut();
		match &*inner {
			ThunkState::Evaluating => {
				*inner = ThunkState::Evaluated(value);
				Ok(())
			}
			_ => Err(()),
		}
	}

	pub fn uneval(code: CodePos, scope: Scope) -> Self {
		Self(Gc::new(RefCell::new(ThunkState::Expr(code, scope))))
	}

	pub fn get_value(&self) -> Option<Value> {
		match &*self.0.try_borrow().ok()? {
			ThunkState::Evaluated(value) => Some(value.clone()),
			_ => None,
		}
	}

	pub fn snapshot(&self) -> Option<ThunkSnapshot> {
		Some(match &*self.0.try_borrow().ok()? {
			ThunkState::Constructing(pos) => ThunkSnapshot::Constructing(*pos),
			ThunkState::Expr(pos, _) => ThunkSnapshot::Expr(*pos),
			ThunkState::Apply(func, arg) => ThunkSnapshot::Apply(func.clone(), arg.clone()),
			ThunkState::Evaluating => ThunkSnapshot::Evaluating,
			ThunkState::Evaluated(value) => ThunkSnapshot::Evaluated(value.clone()),
		})
	}

	pub fn is_evaluating(&self) -> Option<bool> {
		match &*self.0.try_borrow().ok()? {
			ThunkState::Constructing(_) => Some(false),
			ThunkState::Expr(_, _) => Some(false),
			ThunkState::Apply(_, _) => Some(false),
			ThunkState::Evaluating => Some(true),
			ThunkState::Evaluated(_) => Some(false),
		}
	}
}

#[derive(Clone, Trace)]
pub enum ThunkState {
	Constructing(CodePos),
	Expr(CodePos, Scope),

	Apply(Lambda, LazyValue),

	Evaluating,

	Evaluated(Value),
}

impl fmt::Debug for ThunkState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Constructing(pos) => f.debug_tuple("Constructing").field(pos).finish(),
			Self::Expr(pos, _) => f.debug_tuple("Expr").field(pos).finish(),
			Self::Apply(_, _) => f.write_str("Apply"),
			Self::Evaluating => f.write_str("Evaluating"),
			Self::Evaluated(value) => f.debug_tuple("Evaluated").field(value).finish(),
		}
	}
}
