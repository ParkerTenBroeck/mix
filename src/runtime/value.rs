use std::{
    collections::{HashMap, VecDeque},
    ops::Deref,
    path::PathBuf,
};

use dumpster::{Trace, unsync::Gc};

use crate::{
    bytecode::{CodePos, LambdaId},
    runtime::{scope::Scope, thunk::Thunk},
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

#[derive(Clone, Debug, Trace)]
pub enum LazyValue {
    Thunk(Thunk),
    Value(Value),
}

impl<T: Into<Value>> From<T> for LazyValue {
    fn from(value: T) -> Self {
        Self::Value(value.into())
    }
}

impl LazyValue {
    pub fn construct_begin(code: CodePos) -> Self {
        Self::Thunk(Thunk::construct_begin(code))
    }

    pub fn construct_end(&self, scope: Scope) -> bool {
        match self {
            LazyValue::Thunk(thunk) => thunk.construct_end(scope),
            _ => false,
        }
    }

    pub fn try_get_value_mut(&mut self) -> Result<Value, Thunk> {
        match self {
            LazyValue::Thunk(thunk) => match thunk.get_value() {
                Some(value) => {
                    *self = LazyValue::Value(value.clone());
                    Ok(value)
                }
                None => Err(thunk.clone()),
            },
            LazyValue::Value(value) => Ok(value.clone()),
        }
    }

    pub fn try_get_value(&self) -> Result<Value, Thunk> {
        match self {
            LazyValue::Thunk(thunk) => match thunk.get_value() {
                Some(value) => Ok(value),
                None => Err(thunk.clone()),
            },
            LazyValue::Value(value) => Ok(value.clone()),
        }
    }

    pub fn try_into_value(self) -> Result<Value, Thunk> {
        match self {
            LazyValue::Thunk(thunk) => match thunk.get_value() {
                Some(value) => Ok(value),
                None => Err(thunk),
            },
            LazyValue::Value(value) => Ok(value),
        }
    }

    pub fn uneval(code: CodePos, scope: Scope) -> Self {
        Self::Thunk(Thunk::uneval(code, scope))
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

    pub fn id(&self) -> usize {
        Gc::as_ptr(&self.inner) as *const () as usize
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
    pub fn id(&self) -> usize {
        Gc::as_ptr(&self.inner) as *const () as usize
    }

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
