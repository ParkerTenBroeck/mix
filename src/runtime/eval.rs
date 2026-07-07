use super::trace::*;
use dumpster::Trace;
use std::borrow::Cow;

use crate::{
    bytecode::{CodeLocOffset, CodePos, OpCode},
    runtime::{
        LazyValue, Runtime, Value,
        scope::Scope,
        thunk::{Thunk, ThunkEvalErr},
        value::{AttrSet, Lambda, List},
    },
};

#[derive(Debug, Clone)]
pub enum FrameKind {
    Function,
    ThunkEval(Thunk),
    ThunkEvalDeep(Thunk),
    ThunkEvalDeepRoot(Thunk),
}

#[derive(Debug)]
pub enum EvalError<'a> {
    Custom(Cow<'a, str>),
    ThunkEval(ThunkEvalErr),
    ByteCode(&'static str),
}

pub enum PotentialFrame {
    Realized(Frame),
    PotentialDeep(Thunk)
}

#[derive(Clone)]
pub struct Frame {
    pub pos: CodePos,
    pub scope: Scope,
    pub kind: FrameKind,
}

impl Frame {
    pub fn new(pos: CodePos, scope: Scope, kind: FrameKind) -> Self {
        Self { pos, scope, kind }
    }
}

pub struct Evaluator<'a, 'b> {
    pub runtime: &'b Runtime<'a>,

    pub value_stack: Vec<Value>,

    pub frame_stack: Vec<PotentialFrame>,
    pub curr_frame: Frame,
}

impl<'a, 'b> Evaluator<'a, 'b> {
    pub fn eval(
        runtime: &'b Runtime<'a>,
        lazy: LazyValue,
        recursive: bool,
    ) -> Result<Value, ErrorTrace<'a>> {
        let (pos, scope, thunk) = match lazy.try_into_value() {
            Ok(value) => return Ok(value),
            Err(thunk) => {
                let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval).unwrap();
                (pos, scope, thunk)
            }
        };
        let frame_kind = if recursive {
            FrameKind::ThunkEvalDeepRoot(thunk)
        } else {
            FrameKind::ThunkEval(thunk)
        };

        let mut eval = Self {
            runtime,
            frame_stack: Default::default(),
            value_stack: Default::default(),
            curr_frame: Frame::new(pos, scope, frame_kind),
        };
        let res = eval.run_loop();
        res.map_err(|kind| ErrorTrace::build(&eval, kind))
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

    fn push_frame(&mut self, mut frame: Frame) -> Result<(), EvalError<'a>> {
        std::mem::swap(&mut self.curr_frame, &mut frame);
        self.frame_stack.push(PotentialFrame::Realized(frame));
        Ok(())
    }

    fn pop_frame(&mut self) -> Result<PotentialFrame, EvalError<'a>> {
        self.frame_stack
            .pop()
            .ok_or(EvalError::ByteCode("call stack"))
    }

    fn next_op(&mut self) -> Result<OpCode, EvalError<'a>> {
        let Some((op, pos)) = self.runtime.program.get(self.curr_frame.pos) else {
            return Err(EvalError::Custom("overran".into()));
        };
        self.curr_frame.pos = pos;
        Ok(op)
    }

    fn branch(&mut self, off: CodeLocOffset) {
        self.curr_frame.pos = self.curr_frame.pos + off;
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
        'main_loop: loop {
            match self.next_op()? {
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
                        Scope::new(attrset.clone(), self.curr_frame.scope.clone())
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
                    let Value::List(mut list) = self.pop_value()? else {
                        todo!()
                    };
                    list.get_mut()
                        .push_back(LazyValue::uneval(expr, self.curr_frame.scope.clone()));
                    self.push_value(Value::List(list))?;
                }
                OpCode::Apply(loc) => {
                    // self.push_frame(loc, self.curr_frame.scope.clone(), FrameKind::Function)?;
                }

                OpCode::LoadLambda(lambda_id) => {
                    let lambda = Lambda::Lambda {
                        scope: self.curr_frame.scope.clone(),
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
                    self.curr_frame.scope = Scope::new(attrset, self.curr_frame.scope.clone());
                }
                OpCode::LastScope => {
                    if let Some(previous) = self.curr_frame.scope.prev.clone() {
                        self.curr_frame.scope = *previous;
                    } else {
                        self.curr_frame.scope = Scope::bottom(AttrSet::default());
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
                        break Err(EvalError::Custom("meoew".into()));
                    };
                    match lazy.try_get_value(){
                        Ok(ok) => self.push_value(ok)?,
                        Err(thunk) => {
                            let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;
                            self.push_frame(self.curr_frame.clone())?;
                            self.curr_frame = Frame::new(pos, scope, FrameKind::ThunkEval(thunk));
                        },
                    }
                }
                OpCode::GetAttrOr(expr_id) => todo!(),

                OpCode::LoadScope => {
                    let Value::String(name) = self.pop_value()? else {
                        todo!()
                    };
                    let Some(lazy) = self.curr_frame.scope.resolve(&name) else {
                        return Err(EvalError::Custom(
                            format!("failed to resolve {name:?}").into(),
                        ));
                    };
                    match lazy.try_get_value(){
                        Ok(ok) => self.push_value(ok)?,
                        Err(thunk) => {
                            let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;
                            self.push_frame(self.curr_frame.clone())?;
                            self.curr_frame = Frame::new(pos, scope, FrameKind::ThunkEval(thunk));
                        },
                    }
                }

                OpCode::Ret => {
                    let ret = self.pop_value()?;

                    match &self.curr_frame.kind {
                        FrameKind::ThunkEval(thunk) 
                        | FrameKind::ThunkEvalDeep(thunk)
                        | FrameKind::ThunkEvalDeepRoot(thunk) => {
                            thunk.eval_end(ret.clone()).unwrap();
                        },
                        _ => {}
                    }

                    match &self.curr_frame.kind {
                        | FrameKind::ThunkEvalDeep(_)
                        | FrameKind::ThunkEvalDeepRoot(_) => {
                            match &ret {
                                Value::AttrSet(attrs) => {
                                    for lazy in attrs.values() {
                                        if let Err(thunk) = lazy.try_get_value() {
                                            self.frame_stack.push(PotentialFrame::PotentialDeep(thunk));
                                        }
                                    }
                                }
                                Value::List(list) => {
                                    for lazy in list.iter() {
                                        if let Err(thunk) = lazy.try_get_value() {
                                            self.frame_stack.push(PotentialFrame::PotentialDeep(thunk));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        },
                        _ => {}
                    }

                    match &self.curr_frame.kind {
                        FrameKind::ThunkEval(_) 
                        | FrameKind::ThunkEvalDeepRoot(_) => {
                            self.push_value(ret)?;
                        },
                        _ => {}
                    }

                    while !self.frame_stack.is_empty() {
                        match self.pop_frame()? {
                            PotentialFrame::Realized(frame) => {
                                self.curr_frame = frame;
                                break;
                            },
                            PotentialFrame::PotentialDeep(thunk) => {
                                if thunk.get_value().is_some(){
                                    continue;
                                }
                                let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;
                                self.curr_frame = Frame::new(pos, scope, FrameKind::ThunkEvalDeep(thunk));
                                continue 'main_loop;
                            },
                        }
                    }

                    if self.frame_stack.is_empty() {
                        break self.pop_value();
                    }
                }
            }
        }
    }
}
