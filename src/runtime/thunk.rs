use std::cell::RefCell;

use dumpster::{Trace, unsync::Gc};

use crate::{
    bytecode::CodePos,
    runtime::{scope::Scope, value::Value},
};

#[derive(Clone, Trace)]
pub struct Thunk(Gc<RefCell<ThunkState>>);

#[derive(Clone, Debug)]
pub enum ThunkSnapshot {
    Constructing(CodePos),
    Unevaluated(CodePos),
    Evaluating,
    Evaluated(Value),
}

impl std::fmt::Debug for Thunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.try_borrow().ok().as_ref() {
            Some(state) => match &**state {
                ThunkState::Constructing(_) => f.debug_tuple("Thunk::Constructing").finish(),
                ThunkState::Unevaluated(_, _) => f.debug_tuple("Thunk::Unevalated").finish(),
                ThunkState::Evaluating => f.debug_tuple("Thunk::Evaluated").finish(),
                ThunkState::Evaluated(value) => {
                    f.debug_tuple("Thunk::Evaluated").field(value).finish()
                }
            },
            None => f.debug_tuple("Thunk").finish(),
        }
    }
}

#[derive(Debug)]
pub enum ThunkEvalErr {
    InfiniteRec,
    NotConstructed,
    AlreadyEvaluated,
}

impl Thunk {
    pub fn id(&self) -> usize {
        Gc::as_ptr(&self.0) as *const () as usize
    }

    pub fn construct_begin(pos: CodePos) -> Self {
        Self(Gc::new(RefCell::new(ThunkState::Constructing(pos))))
    }

    pub fn construct_end(&self, scope: Scope) -> bool {
        let mut inner = self.0.borrow_mut();
        match &*inner {
            ThunkState::Constructing(code_loc) => {
                *inner = ThunkState::Unevaluated(*code_loc, scope);
                true
            }
            _ => false,
        }
    }

    pub fn eval_begin(&self) -> Result<(CodePos, Scope), ThunkEvalErr> {
        let mut inner = self.0.borrow_mut();
        match &*inner {
            ThunkState::Unevaluated(code_loc, scope) => {
                let ret = Ok((*code_loc, scope.clone()));
                *inner = ThunkState::Evaluating;
                ret
            }
            ThunkState::Constructing(_) => Err(ThunkEvalErr::NotConstructed),
            ThunkState::Evaluating => Err(ThunkEvalErr::InfiniteRec),
            ThunkState::Evaluated(_) => Err(ThunkEvalErr::AlreadyEvaluated),
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
        Self(Gc::new(RefCell::new(ThunkState::Unevaluated(code, scope))))
    }

    pub fn get_value(&self) -> Option<Value> {
        match &*self.0.try_borrow().ok()? {
            ThunkState::Constructing(_) => None,
            ThunkState::Unevaluated(_, _) => None,
            ThunkState::Evaluating => None,
            ThunkState::Evaluated(value) => Some(value.clone()),
        }
    }

    pub fn snapshot(&self) -> Option<ThunkSnapshot> {
        Some(match &*self.0.try_borrow().ok()? {
            ThunkState::Constructing(pos) => ThunkSnapshot::Constructing(*pos),
            ThunkState::Unevaluated(pos, _) => ThunkSnapshot::Unevaluated(*pos),
            ThunkState::Evaluating => ThunkSnapshot::Evaluating,
            ThunkState::Evaluated(value) => ThunkSnapshot::Evaluated(value.clone()),
        })
    }

    pub fn is_evaluating(&self) -> Option<bool> {
        match &*self.0.try_borrow().ok()? {
            ThunkState::Constructing(_) => Some(false),
            ThunkState::Unevaluated(_, _) => Some(false),
            ThunkState::Evaluating => Some(true),
            ThunkState::Evaluated(_) => Some(false),
        }
    }
}

#[derive(Clone, Trace)]
pub enum ThunkState {
    Constructing(CodePos),
    Unevaluated(CodePos, Scope),
    Evaluating,
    Evaluated(Value),
}

impl std::fmt::Debug for ThunkState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constructing(arg0) => f.debug_tuple("Constructing").field(arg0).finish(),
            Self::Unevaluated(arg0, _) => f.debug_tuple("Unevaluated").field(arg0).finish(),
            Self::Evaluating => write!(f, "Evaluating"),
            Self::Evaluated(arg0) => f.debug_tuple("Evaluated").field(arg0).finish(),
        }
    }
}
