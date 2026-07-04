use std::borrow::Cow;

use crate::{
    bytecode::{CodeLoc, CodeLocOffset, OpCode},
    runtime::{AttrSet, Lambda, LazyExpr, List, Runtime, Value, scope::Scope},
};

#[derive(Debug)]
pub enum EvalError<'a> {
    Custom(Cow<'a, str>),
    ByteCode(&'static str),
}

pub struct Evaluator<'a, 'b> {
    runtime: &'b Runtime<'a>,

    value_stack: Vec<Value>,
    call_stack: Vec<(CodeLoc, Scope)>,

    pos: CodeLoc,
    scope: Scope,
}

impl<'a, 'b> Evaluator<'a, 'b> {
    pub fn new(runtime: &'b Runtime<'a>, start: CodeLoc, scope: Scope) -> Self {
        Self {
            runtime,
            call_stack: Default::default(),
            value_stack: Default::default(),
            pos: start,
            scope,
        }
    }

    pub fn eval(&mut self) -> Value {
        self.run_loop().unwrap()
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

    fn push_call(&mut self, call: (CodeLoc, Scope)) -> Result<(), EvalError<'a>> {
        self.call_stack.push(call);
        Ok(())
    }

    fn pop_call(&mut self) -> Result<(CodeLoc, Scope), EvalError<'a>> {
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

    fn goto(&mut self, pos: CodeLoc) {
        self.pos = pos;
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
                        .insert(name, LazyExpr::construct_begin(expr));
                    self.push_value(Value::AttrSet(attrset))?;
                }
                OpCode::FinalizeAttrSet(recursive) => {
                    let Value::AttrSet(attrset) = self.pop_value()? else {
                        todo!()
                    };
                    let scope = if recursive {
                        Scope::new(attrset.clone(), self.scope.clone())
                    } else {
                        self.scope.clone()
                    };

                    for element in attrset.values() {
                        element.construct_end(scope.clone());
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
                        .push_back(LazyExpr::uneval(expr, self.scope.clone()));
                    self.push_value(Value::List(list))?;
                }
                OpCode::Apply(loc) => {
                    self.push_call((self.pos, self.scope.clone()))?;
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
                OpCode::GetAttr => todo!(),
                OpCode::GetAttrOr(expr_id) => todo!(),

                OpCode::Ret if self.call_stack.is_empty() => break self.pop_value(),
                OpCode::Ret => {
                    let (pos, scope) = self.pop_call()?;
                    self.goto(pos);
                    self.scope = scope;
                }
            }
        }
    }
}
