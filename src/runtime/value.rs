use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    ops::Deref,
    path::PathBuf,
};

use dumpster::{Trace, TraceWith, unsync::Gc};

use crate::{
    bytecode::{CodeLoc, LambdaId},
    runtime::scope::Scope,
};

#[derive(Clone, Debug, Trace)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Gc<String>),
    Path(PathBuf),
    List(List),
    AttrSet(AttrSet),
    Lambda(Lambda),
}

#[derive(Clone, Trace)]
pub struct NativeLambda {
    inner: Gc<NativeLambdaInner>,
}

#[derive(Trace)]
pub struct NativeLambdaInner {
    pub identifer: Gc<String>,
    // pub func: dyn Fn(&mut super::Runtime<'static>, Value) -> Value,
}

impl std::fmt::Debug for NativeLambda {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeLambda")
            .field("identifer", &self.inner.identifer)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub enum Lambda {
    Lambda { scope: Scope, lambda: LambdaId },
    NativeLambda(NativeLambda),
}

unsafe impl<V: dumpster::Visitor> TraceWith<V> for Lambda {
    fn accept(&self, visitor: &mut V) -> Result<(), ()> {
        match self {
            Lambda::Lambda { scope, .. } => {
                ::dumpster::TraceWith::accept(scope, visitor)?;
                ::core::result::Result::Ok(())
            }
            Lambda::NativeLambda(field0) => {
                ::dumpster::TraceWith::accept(field0, visitor)?;
                ::core::result::Result::Ok(())
            }
        }
    }
}

#[derive(Clone, Trace)]
pub enum LazyExpr {
    Unevaluated(Gc<RefCell<LazyExprState>>),
    Evaluated(Value),
}

impl LazyExpr {
    pub fn uneval(code: CodeLoc, scope: Scope) -> Self {
        Self::Unevaluated(Gc::new(RefCell::new(LazyExprState::Unevaluated(
            code, scope,
        ))))
    }
}

impl std::fmt::Debug for LazyExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LazyExpr")
    }
}

#[derive(Clone, Debug)]
pub enum LazyExprState {
    Unevaluated(CodeLoc, Scope),
    Evaluating,
    Evaluated(Value),
}

unsafe impl<V: dumpster::Visitor> TraceWith<V> for LazyExprState {
    #[inline]
    fn accept(&self, visitor: &mut V) -> ::core::result::Result<(), ()> {
        match self {
            LazyExprState::Unevaluated(_, scope) => {
                ::dumpster::TraceWith::accept(scope, visitor)?;
                ::core::result::Result::Ok(())
            }
            LazyExprState::Evaluating => ::core::result::Result::Ok(()),
            LazyExprState::Evaluated(value) => {
                ::dumpster::TraceWith::accept(value, visitor)?;
                ::core::result::Result::Ok(())
            }
        }
    }
}

#[derive(Clone, Default, Debug, Trace)]
pub struct List {
    inner: Gc<VecDeque<LazyExpr>>,
}

impl List {
    pub fn with_capacity(capacity: usize) -> List {
        Self {
            inner: Gc::new(VecDeque::with_capacity(capacity)),
        }
    }

    pub fn get_mut(&mut self) -> &mut VecDeque<LazyExpr> {
        Gc::make_mut(&mut self.inner)
    }
}

impl Deref for List {
    type Target = VecDeque<LazyExpr>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone, Default, Debug, Trace)]
pub struct AttrSet {
    inner: Gc<HashMap<Gc<String>, LazyExpr>>,
}

impl AttrSet {
    pub fn get_mut(&mut self) -> &mut HashMap<Gc<String>, LazyExpr> {
        Gc::make_mut(&mut self.inner)
    }
}

impl Deref for AttrSet {
    type Target = HashMap<Gc<String>, LazyExpr>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
