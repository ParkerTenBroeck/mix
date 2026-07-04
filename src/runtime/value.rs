use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    ops::Deref,
    path::PathBuf,
};

use dumpster::{Trace, unsync::Gc};

use crate::{
    bytecode::{CodePos, LambdaId},
    runtime::scope::Scope,
};

#[derive(Clone, Debug, Trace)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Path(PathBuf),
    List(List),
    AttrSet(AttrSet),
    Lambda(Lambda),
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
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

#[derive(Clone, Debug, Trace)]
pub enum Lambda {
    Lambda { scope: Scope, lambda: LambdaId },
    // NativeLambda(NativeLambda),
}

#[derive(Clone, Trace)]
pub enum LazyValue {
    Unevaluated(Gc<RefCell<LazyExprState>>),
    Evaluated(Value),
}

impl<T: Into<Value>> From<T> for LazyValue {
    fn from(value: T) -> Self {
        Self::Evaluated(value.into())
    }
}

impl std::fmt::Debug for LazyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unevaluated(arg0) => {
                if let Ok(borrow) = arg0.try_borrow() {
                    f.debug_tuple("Unevaluated").field(&*borrow).finish()
                } else {
                    f.debug_tuple("Unevaluated").finish()
                }
            }
            Self::Evaluated(arg0) => f.debug_tuple("Evaluated").field(arg0).finish(),
        }
    }
}

impl LazyValue {
    pub fn construct_begin(code: CodePos) -> Self {
        Self::Unevaluated(Gc::new(RefCell::new(LazyExprState::Constructing(code))))
    }

    pub fn construct_end(&self, scope: Scope) -> bool {
        match self {
            LazyValue::Unevaluated(gc) => {
                let mut inner = gc.borrow_mut();
                match &*inner {
                    LazyExprState::Constructing(code_loc) => {
                        *inner = LazyExprState::Unevaluated(*code_loc, scope);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn uneval(code: CodePos, scope: Scope) -> Self {
        Self::Unevaluated(Gc::new(RefCell::new(LazyExprState::Unevaluated(
            code, scope,
        ))))
    }
}

#[derive(Clone, Trace)]
pub enum LazyExprState {
    Constructing(CodePos),
    Unevaluated(CodePos, Scope),
    Evaluating,
    Evaluated(Value),
}

impl std::fmt::Debug for LazyExprState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constructing(arg0) => f.debug_tuple("Constructing").field(arg0).finish(),
            Self::Unevaluated(arg0, _) => f.debug_tuple("Unevaluated").field(arg0).finish(),
            Self::Evaluating => write!(f, "Evaluating"),
            Self::Evaluated(arg0) => f.debug_tuple("Evaluated").field(arg0).finish(),
        }
    }
}

#[derive(Clone, Default, Debug, Trace)]
pub struct List {
    inner: Gc<VecDeque<LazyValue>>,
}

impl List {
    pub fn with_capacity(capacity: usize) -> List {
        Self {
            inner: Gc::new(VecDeque::with_capacity(capacity)),
        }
    }

    pub fn get_mut(&mut self) -> &mut VecDeque<LazyValue> {
        Gc::make_mut(&mut self.inner)
    }
}

impl Deref for List {
    type Target = VecDeque<LazyValue>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone, Default, Trace)]
pub struct AttrSet {
    inner: Gc<HashMap<String, LazyValue>>,
}

impl AttrSet {
    pub fn get_mut(&mut self) -> &mut HashMap<String, LazyValue> {
        Gc::make_mut(&mut self.inner)
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(map: HashMap<String, LazyValue>) -> Self {
        Self {
            inner: Gc::new(map),
        }
    }
}

impl std::fmt::Debug for AttrSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AttrSet").field(&*self.inner).finish()
    }
}

impl Deref for AttrSet {
    type Target = HashMap<String, LazyValue>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
