use std::{borrow::Cow, cell::RefCell};

use dumpster::{Trace, unsync::Gc};

use crate::{
    bytecode::{CodeLocOffset, CodePos, OpCode},
    runtime::{AttrSet, Lambda, LazyExprState, LazyValue, List, Runtime, Value, scope::Scope},
};

#[derive(Trace)]
enum LazyUpdate {
    None,
    Eval(Gc<RefCell<LazyExprState>>),
    Rec(Gc<RefCell<LazyExprState>>),
}

#[derive(Debug)]
pub enum EvalError<'a> {
    Custom(Cow<'a, str>),
    ByteCode(&'static str),
}

pub struct Evaluator<'a, 'b> {
    runtime: &'b Runtime<'a>,

    value_stack: Vec<Value>,
    call_stack: Vec<(CodePos, Scope, LazyUpdate)>,

    pos: CodePos,
    scope: Scope,
}

impl<'a, 'b> Evaluator<'a, 'b> {
    pub fn eval(runtime: &'b Runtime<'a>, lazy: LazyValue, recursive: bool) -> Result<Value, EvalError<'a>>{
        let mut eval = Self {
            runtime,
            call_stack: Default::default(),
            value_stack: Default::default(),
            pos: CodePos::default(),
            scope: Default::default(),
        };
        eval.eval_lazy(lazy, recursive)?;
        eval.run_loop()
    }

    fn push_value(&mut self, value: Value) -> Result<(), EvalError<'a>> {
        self.value_stack.push(value);
        Ok(())
    }

    fn pop_value(&mut self) -> Result<Value, EvalError<'a>> {
        self.value_stack
            .pop()
            .ok_or(EvalError::ByteCode("value stack"))
    }

    fn push_call_stack(&mut self, pos: CodePos, scope: Scope, lazy: LazyUpdate) -> Result<(), EvalError<'a>> {
        self.call_stack.push((self.pos, self.scope.clone(), lazy));
        self.pos = pos;
        self.scope = scope;
        Ok(())
    }

    fn pop_call(&mut self) -> Result<(CodePos, Scope, LazyUpdate), EvalError<'a>> {
        self.call_stack
            .pop()
            .ok_or(EvalError::ByteCode("call stack"))
    }

    fn next_op(&mut self) -> OpCode {
        let (op, pos) = self.runtime.program.get(self.pos);
        self.pos = pos;
        op
    }

    fn branch(&mut self, off: CodeLocOffset) {
        self.pos = self.pos + off;
    }

    fn goto(&mut self, pos: CodePos) {
        self.pos = pos;
    }

    fn eval_lazy(&mut self, lazy: LazyValue, rec: bool) -> Result<(), EvalError<'a>> {
        match lazy{
            LazyValue::Unevaluated(gc) => {
                let mut state = gc.borrow_mut();
                match &*state{
                    super::LazyExprState::Constructing(_) => todo!(),
                    super::LazyExprState::Evaluating => todo!(),

                    super::LazyExprState::Evaluated(value) => self.push_value(value.clone()),

                    super::LazyExprState::Unevaluated(code_loc, scope) => {
                        let kind = if rec{
                            LazyUpdate::Rec(gc.clone())
                        }else{
                            LazyUpdate::Eval(gc.clone())
                        };
                        self.push_call_stack(*code_loc, scope.clone(), kind)?;
                        *state = super::LazyExprState::Evaluating;
                        Ok(())
                    },
                }
            },
            LazyValue::Evaluated(value) => self.push_value(value),
        }
    }

    fn run_loop(&mut self) -> Result<Value, EvalError<'a>> {
        use crate::bytecode::OpCode;

        macro_rules! binop_num {
            ($lhs: ident, $rhs: ident, $expr: expr) => {{
                let lhs = self.pop_value()?;
                let rhs = self.pop_value()?;
                let result = match (lhs, rhs) {
                    (Value::Int($lhs), Value::Int($rhs)) => $expr,
                    (Value::Float($lhs), Value::Int($rhs)) => {
                        let $lhs = $lhs as f64;
                        let $rhs = $rhs as f64;
                        $expr
                    }
                    (Value::Int($lhs), Value::Float($rhs)) => {
                        let $lhs = $lhs as f64;
                        let $rhs = $rhs as f64;
                        $expr
                    }
                    (Value::Float($lhs), Value::Float($rhs)) => $expr,
                    _ => todo!(),
                };
                self.push_value(result)?;
            }};
        }
        macro_rules! binop_cmp {
            ($lhs: ident, $rhs: ident, $expr: expr) => {{
                let lhs = self.pop_value()?;
                let rhs = self.pop_value()?;
                let result = match (lhs, rhs) {
                    (Value::Int($lhs), Value::Int($rhs)) => $expr,
                    (Value::Float($lhs), Value::Int($rhs)) => {
                        let $lhs = $lhs as f64;
                        let $rhs = $rhs as f64;
                        $expr
                    }
                    (Value::Int($lhs), Value::Float($rhs)) => {
                        let $lhs = $lhs as f64;
                        let $rhs = $rhs as f64;
                        $expr
                    }
                    (Value::Float($lhs), Value::Float($rhs)) => $expr,
                    #[allow(clippy::bool_comparison)]
                    (Value::Bool($lhs), Value::Bool($rhs)) => $expr,
                    (Value::String($lhs), Value::String($rhs)) => $expr,
                    _ => todo!(),
                };
                self.push_value(result)?;
            }};
        }
        loop {
            match self.next_op() {
                OpCode::Add => {
                    let lhs = self.pop_value()?;
                    let rhs = self.pop_value()?;
                    let result = match (lhs, rhs) {
                        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs + rhs),
                        (Value::Float(lhs), Value::Int(rhs)) => Value::Float(lhs + rhs as f64),
                        (Value::Int(lhs), Value::Float(rhs)) => Value::Float(lhs as f64 + rhs),
                        (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs + rhs),
                        (Value::String(mut lhs), Value::String(rhs)) => {
                            lhs.push_str(&rhs);
                            Value::String(lhs)
                        }
                        _ => todo!(),
                    };
                    self.push_value(result)?;
                }
                OpCode::Sub => binop_num!(lhs, rhs, (lhs - rhs).into()),
                OpCode::Mul => binop_num!(lhs, rhs, (lhs * rhs).into()),
                OpCode::Div => binop_num!(lhs, rhs, (lhs / rhs).into()), // TODO div by zero
                OpCode::Rem => binop_num!(lhs, rhs, (lhs % rhs).into()), // TODO div by zero
                OpCode::Eq => binop_cmp!(lhs, rhs, Value::Bool(lhs == rhs)),
                OpCode::Ne => binop_cmp!(lhs, rhs, Value::Bool(lhs != rhs)),
                OpCode::Lt => binop_cmp!(lhs, rhs, Value::Bool(lhs < rhs)),
                OpCode::Lte => binop_cmp!(lhs, rhs, Value::Bool(lhs <= rhs)),
                OpCode::Gt => binop_cmp!(lhs, rhs, Value::Bool(lhs > rhs)),
                OpCode::Gte => binop_cmp!(lhs, rhs, Value::Bool(lhs >= rhs)),
                OpCode::Not => {
                    let result = match self.pop_value()? {
                        Value::Bool(bool) => Value::Bool(!bool),
                        _ => todo!(),
                    };
                    self.push_value(result)?;
                }
                OpCode::Neg => {
                    let result = match self.pop_value()? {
                        Value::Int(int) => Value::Int(-int),
                        Value::Float(float) => Value::Float(-float),
                        _ => todo!(),
                    };
                    self.push_value(result)?;
                }

                op @ (OpCode::And(rhs) | OpCode::Or(rhs) | OpCode::LogImp(rhs)) => {
                    let Value::Bool(lhs) = self.pop_value()? else {
                        todo!()
                    };
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
                    let Value::Bool(cond) = self.pop_value()? else {
                        todo!()
                    };
                    if !cond {
                        self.branch(else_off);
                    }
                }
                OpCode::Branch(offset) => self.branch(offset),

                OpCode::CreateAttrSet => {
                    self.value_stack.push(Value::AttrSet(AttrSet::default()));
                }
                OpCode::InitAttrExpr(expr) => {
                    let Value::String(name) = self.pop_value()? else {
                        todo!()
                    };
                    let Value::AttrSet(mut attrset) = self.pop_value()? else {
                        todo!()
                    };
                    attrset
                        .get_mut()
                        .insert(name, LazyValue::construct_begin(expr));
                    self.push_value(Value::AttrSet(attrset))?;
                }
                op @ (OpCode::FinalizeAttrSetRec | OpCode::FinalizeAttrSet) => {
                    let Value::AttrSet(attrset) = self.pop_value()? else {
                        todo!()
                    };
                    let scope = if op == OpCode::FinalizeAttrSetRec {
                        Scope::new(attrset.clone(), self.scope.clone())
                    } else {
                        self.scope.clone()
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
                    let Value::List(mut list) = self.pop_value()? else {
                        todo!()
                    };
                    list.get_mut()
                        .push_back(LazyValue::uneval(expr, self.scope.clone()));
                    self.push_value(Value::List(list))?;
                }
                OpCode::Apply(loc) => {
                    // self.push_call_stack((self.pos, self.scope.clone()))?;
                }

                OpCode::LoadLambda(lambda_id) => {
                    let lambda = Lambda::Lambda {
                        scope: self.scope.clone(),
                        lambda: lambda_id,
                    };
                    self.push_value(Value::Lambda(lambda))?;
                }
                OpCode::LoadStr(str) => {
                    self.push_value(Value::String(self.runtime.program.get_str(str).into()))?
                }
                OpCode::LoadInt(int) => self.push_value(Value::Int(int))?,
                OpCode::LoadFloat(float) => self.push_value(Value::Float(float))?,
                OpCode::LoadBool(bool) => self.push_value(Value::Bool(bool))?,

                OpCode::WithScope => {
                    let Value::AttrSet(attrset) = self.pop_value()? else {
                        todo!()
                    };
                    self.scope = Scope::new(attrset, self.scope.clone());
                }
                OpCode::LastScope => {
                    if let Some(previous) = self.scope.prev.clone() {
                        self.scope = *previous;
                    } else {
                        self.scope = Scope::bottom(AttrSet::default());
                    }
                }
                OpCode::HasAttr => todo!(),
                OpCode::GetAttr => {
                    let Value::String(name) = self.pop_value()? else {
                        todo!()
                    };
                    let Value::AttrSet(attrset) = self.pop_value()? else {
                        todo!()
                    };
                    let Some(lazy) = attrset.get(&name) else {
                        todo!()
                    };
                    self.eval_lazy(lazy.clone(), false)?;
                },
                OpCode::GetAttrOr(expr_id) => todo!(),

                OpCode::LoadScope => {
                    let Value::String(name) = self.pop_value()? else {
                        todo!()
                    };
                    let Some(lazy) = self.scope.resolve(&name) else {
                        todo!()
                    };
                    self.eval_lazy(lazy.clone(), false)?;
                }

                OpCode::Ret => {
                    let (pos, scope, lazy) = self.pop_call()?;
                    match lazy {
                        LazyUpdate::None => {}
                        LazyUpdate::Eval(state) => {
                            let res = self.pop_value()?;
                            let mut state = state.borrow_mut();
                            match &*state{
                                LazyExprState::Evaluating => {},
                                _ => todo!(),
                            }
                            *state = LazyExprState::Evaluated(res.clone());
                            self.push_value(res)?;
                        }
                        LazyUpdate::Rec(state) => {
                            let res = self.pop_value()?;
                            let mut state = state.borrow_mut();
                            match &*state{
                                LazyExprState::Evaluating => {},
                                _ => todo!(),
                            }
                            *state = LazyExprState::Evaluated(res.clone());

                            match &res{
                                Value::AttrSet(attrs) => {
                                    for lazy in attrs.values(){
                                        self.eval_lazy(lazy.clone(), true)?;
                                    }
                                }
                                Value::List(list) => {
                                    for lazy in list.iter(){
                                        self.eval_lazy(lazy.clone(), true)?;
                                    }
                                }
                                _ => {}
                            }
                        },
                    }
                    if self.call_stack.is_empty(){
                        break self.pop_value()
                    }else{
                        self.goto(pos);
                        self.scope = scope;
                    }
                }
            }
        }
    }
}
