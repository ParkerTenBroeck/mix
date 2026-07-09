use super::trace::*;
use std::{borrow::Cow, usize};

use crate::{
    bytecode::{CodeLocOffset, CodePos, OpCode},
    runtime::{
        LazyValue, Runtime, Value,
        scope::Scope,
        thunk::{Thunk, ThunkEvalErr},
        value::{AttrSet, Lambda, List, ValueType},
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
    TypeMismatch { expected: ValueType, got: ValueType },
    BinOpTypeMismatch { details: Cow<'a, str> },
    Arithmetic(Cow<'a, str>),
    MissingAttr(Cow<'a, str>),
    MissingBinding(Cow<'a, str>),
    Internal(Cow<'a, str>),
    ThunkEval(ThunkEvalErr),
    ByteCode(&'static str),
}

pub enum PotentialFrame {
    Realized(Frame),
    PotentialDeep(Thunk),
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

macro_rules! checked_numeric_op {
    (
        $lhs:expr,
        $rhs:expr,
        type_error($bad_lhs:ident, $bad_rhs:ident) = $type_error:block,
        int($int_lhs:ident, $int_rhs:ident) = $int_eval:block,
        float($float_lhs:ident, $float_rhs:ident) = $float_eval:block $(,)?
    ) => {{
        match ($lhs, $rhs) {
            (Value::Int($int_lhs), Value::Int($int_rhs)) => $int_eval,
            (Value::Float($float_lhs), Value::Int($float_rhs)) => {
                let $float_rhs = $float_rhs as f64;
                $float_eval
            }
            (Value::Int($float_lhs), Value::Float($float_rhs)) => {
                let $float_lhs = $float_lhs as f64;
                $float_eval
            }
            (Value::Float($float_lhs), Value::Float($float_rhs)) => $float_eval,
            ($bad_lhs, $bad_rhs) => Err($type_error),
        }
    }};
}

macro_rules! checked_numeric_method {
    (
        $name:ident,
        $this:ident,
        op_name = $op_name:literal,
        symbol = $symbol:literal,
        type_error($bad_lhs:ident, $bad_rhs:ident) = $type_error:block,
        int = $int_method:ident,
        float($float_lhs:ident, $float_rhs:ident) = $float_eval:block $(,)?
    ) => {
        fn $name(&self, lhs: Value, rhs: Value) -> Result<Value, EvalError<'a>> {
            let $this = self;
            checked_numeric_op!(
                lhs,
                rhs,
                type_error($bad_lhs, $bad_rhs) = $type_error,
                int(lhs, rhs) = {
                    self.checked_int_result(
                        $op_name,
                        format!("{lhs} {} {rhs}", $symbol),
                        lhs.$int_method(rhs),
                    )
                },
                float($float_lhs, $float_rhs) = $float_eval,
            )
        }
    };
}

macro_rules! checked_zero_numeric_method {
    (
        $name:ident,
        $this:ident,
        op_name = $op_name:literal,
        symbol = $symbol:literal,
        type_error($bad_lhs:ident, $bad_rhs:ident) = $type_error:block,
        int = $int_method:ident,
        float($float_lhs:ident, $float_rhs:ident) = $float_eval:block,
        zero = $zero_message:literal $(,)?
    ) => {
        fn $name(&self, lhs: Value, rhs: Value) -> Result<Value, EvalError<'a>> {
            let $this = self;
            checked_numeric_op!(
                lhs,
                rhs,
                type_error($bad_lhs, $bad_rhs) = $type_error,
                int(lhs, rhs) = {
                    if rhs == 0 {
                        return Err(EvalError::Arithmetic(format!($zero_message, lhs).into()));
                    }

                    self.checked_int_result(
                        $op_name,
                        format!("{lhs} {} {rhs}", $symbol),
                        lhs.$int_method(rhs),
                    )
                },
                float($float_lhs, $float_rhs) = {
                    if $float_rhs == 0.0 {
                        return Err(EvalError::Arithmetic(
                            format!($zero_message, $float_lhs).into(),
                        ));
                    }

                    $float_eval
                },
            )
        }
    };
}

impl<'a, 'b> Evaluator<'a, 'b> {
    fn checked_float_result(
        &self,
        op_name: &'static str,
        lhs: f64,
        rhs: f64,
        eval: impl FnOnce(f64, f64) -> f64,
    ) -> Result<Value, EvalError<'a>> {
        let value = eval(lhs, rhs);

        if value.is_finite() {
            Ok(Value::Float(value))
        } else {
            Err(EvalError::Arithmetic(
                format!("{op_name} overflowed or produced a non-finite float for {lhs} and {rhs}")
                    .into(),
            ))
        }
    }

    fn checked_int_result(
        &self,
        op_name: &'static str,
        display: String,
        value: Option<i64>,
    ) -> Result<Value, EvalError<'a>> {
        value.map(Value::Int).ok_or_else(|| {
            EvalError::Arithmetic(format!("{op_name} overflowed for {display}").into())
        })
    }

    fn checked_add(&self, lhs: Value, rhs: Value) -> Result<Value, EvalError<'a>> {
        match (lhs, rhs) {
            (Value::String(mut lhs), Value::String(rhs)) => {
                lhs.push_str(&rhs);
                Ok(Value::String(lhs))
            }
            (lhs, rhs) => checked_numeric_op!(
                lhs,
                rhs,
                type_error(lhs, rhs) = {
                    EvalError::BinOpTypeMismatch {
                        details: format!("cannot add {} to {}", rhs.ty(), lhs.ty()).into(),
                    }
                },
                int(lhs, rhs) = {
                    self.checked_int_result(
                        "addition",
                        format!("{lhs} + {rhs}"),
                        lhs.checked_add(rhs),
                    )
                },
                float(lhs, rhs) =
                    { self.checked_float_result("addition", lhs, rhs, |lhs, rhs| lhs + rhs) },
            ),
        }
    }

    checked_numeric_method!(
        checked_sub,
        this,
        op_name = "subtraction",
        symbol = "-",
        type_error(lhs, rhs) = {
            EvalError::BinOpTypeMismatch {
                details: format!("cannot subtract {} from {}", rhs.ty(), lhs.ty()).into(),
            }
        },
        int = checked_sub,
        float(lhs, rhs) =
            { this.checked_float_result("subtraction", lhs, rhs, |lhs, rhs| lhs - rhs) },
    );

    checked_numeric_method!(
        checked_mul,
        this,
        op_name = "multiplication",
        symbol = "*",
        type_error(lhs, rhs) = {
            EvalError::BinOpTypeMismatch {
                details: format!("cannot multiply {} by {}", lhs.ty(), rhs.ty()).into(),
            }
        },
        int = checked_mul,
        float(lhs, rhs) =
            { this.checked_float_result("multiplication", lhs, rhs, |lhs, rhs| lhs * rhs) },
    );

    checked_zero_numeric_method!(
        checked_div,
        this,
        op_name = "division",
        symbol = "/",
        type_error(lhs, rhs) = {
            EvalError::BinOpTypeMismatch {
                details: format!("cannot divide {} by {}", lhs.ty(), rhs.ty()).into(),
            }
        },
        int = checked_div,
        float(lhs, rhs) = { this.checked_float_result("division", lhs, rhs, |lhs, rhs| lhs / rhs) },
        zero = "cannot divide {} by zero",
    );

    checked_zero_numeric_method!(
        checked_rem,
        this,
        op_name = "remainder",
        symbol = "%",
        type_error(lhs, rhs) = {
            EvalError::BinOpTypeMismatch {
                details: format!("cannot take {} % {}", lhs.ty(), rhs.ty()).into(),
            }
        },
        int = checked_rem,
        float(lhs, rhs) =
            { this.checked_float_result("remainder", lhs, rhs, |lhs, rhs| lhs % rhs) },
        zero = "cannot calculate {} % 0",
    );

    pub fn eval(
        runtime: &'b Runtime<'a>,
        lazy: LazyValue,
        recursive: bool,
    ) -> Result<Value, ErrorTrace<'a>> {
        let (pos, scope, thunk) = match lazy.try_into_value() {
            Ok(value) => return Ok(value),
            Err(thunk) => match thunk.eval_begin() {
                Ok((pos, scope)) => (pos, scope, thunk),
                Err(err) => {
                    return Err(ErrorTrace {
                        kind: EvalError::ThunkEval(err),
                        stack: Vec::new(),
                    });
                }
            },
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

    fn pop_bool(&mut self) -> Result<bool, EvalError<'a>> {
        let value = self.pop_value()?;
        match value {
            Value::Bool(value) => Ok(value),
            other => Err(EvalError::TypeMismatch {
                expected: ValueType::Bool,
                got: other.ty(),
            }),
        }
    }

    fn pop_string(&mut self) -> Result<String, EvalError<'a>> {
        let value = self.pop_value()?;
        match value {
            Value::String(value) => Ok(value),
            other => Err(EvalError::TypeMismatch {
                expected: ValueType::String,
                got: other.ty(),
            }),
        }
    }

    fn pop_list(&mut self) -> Result<List, EvalError<'a>> {
        let value = self.pop_value()?;
        match value {
            Value::List(value) => Ok(value),
            other => Err(EvalError::TypeMismatch {
                expected: ValueType::List,
                got: other.ty(),
            }),
        }
    }

    fn pop_attrset(&mut self) -> Result<AttrSet, EvalError<'a>> {
        let value = self.pop_value()?;
        match value {
            Value::AttrSet(value) => Ok(value),
            other => Err(EvalError::TypeMismatch {
                expected: ValueType::AttrSet,
                got: other.ty(),
            }),
        }
    }

    fn pop_lambda(&mut self) -> Result<Lambda, EvalError<'a>> {
        let value = self.pop_value()?;
        match value {
            Value::Lambda(value) => Ok(value),
            other => Err(EvalError::TypeMismatch {
                expected: ValueType::Lambda,
                got: other.ty(),
            }),
        }
    }

    fn begin_frame(&mut self, mut frame: Frame) -> Result<(), EvalError<'a>> {
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
            return Err(EvalError::ByteCode("instruction pointer overran bytecode"));
        };
        self.curr_frame.pos = pos;
        Ok(op)
    }

    fn branch(&mut self, off: CodeLocOffset) {
        self.curr_frame.pos = self.curr_frame.pos + off;
    }

    fn get_attr(&mut self) -> Result<LazyValue, EvalError<'a>> {
        match self.pop_value()? {
            Value::String(name) => match self.pop_value()? {
                Value::AttrSet(attrset) => {
                    let Some(lazy) = attrset.get(&name) else {
                        return Err(EvalError::MissingAttr(
                            format!("attribute {name:?} was not found in attrset").into(),
                        ));
                    };
                    Ok(lazy.clone())
                }
                Value::List(list) => {
                    if name != "len" {
                        Err(EvalError::MissingAttr(
                            format!("attribute {name:?} was not found in list").into(),
                        ))
                    } else {
                        Ok(LazyValue::Value(Value::Int(list.len() as i64)))
                    }
                }
                value => Err(EvalError::TypeMismatch {
                    expected: ValueType::AttrSet,
                    got: value.ty(),
                }),
            },
            Value::Int(index) => {
                let list = self.pop_list()?;
                let Some(lazy) = list.get(index.try_into().unwrap_or(usize::MAX)) else {
                    return Err(EvalError::MissingAttr(
                        format!("index {index} is not in list").into(),
                    ));
                };
                Ok(lazy.clone())
            }
            value => Err(EvalError::TypeMismatch {
                expected: ValueType::AttrSet,
                got: value.ty(),
            }),
        }
    }

    fn run_loop(&mut self) -> Result<Value, EvalError<'a>> {
        use crate::bytecode::OpCode;

        macro_rules! binop_cmp {
            ($op: literal, $lhs: ident, $rhs: ident, $expr: expr) => {{
                let rhs = self.pop_value()?;
                let lhs = self.pop_value()?;
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
                    (lhs, rhs) => {
                        return Err(EvalError::BinOpTypeMismatch {
                            details: format!(
                                "cannot compare {} and {} with {}",
                                lhs.ty(),
                                rhs.ty(),
                                $op
                            )
                            .into(),
                        });
                    }
                };
                self.push_value(result)?;
            }};
        }
        'main_loop: loop {
            match self.next_op()? {
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
                OpCode::Eq => binop_cmp!("==", lhs, rhs, Value::Bool(lhs == rhs)),
                OpCode::Ne => binop_cmp!("!=", lhs, rhs, Value::Bool(lhs != rhs)),
                OpCode::Lt => binop_cmp!("<", lhs, rhs, Value::Bool(lhs < rhs)),
                OpCode::Lte => binop_cmp!("<=", lhs, rhs, Value::Bool(lhs <= rhs)),
                OpCode::Gt => binop_cmp!(">", lhs, rhs, Value::Bool(lhs > rhs)),
                OpCode::Gte => binop_cmp!(">=", lhs, rhs, Value::Bool(lhs >= rhs)),
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
                        let scope_mut = scope.get_mut();
                        for (name, value) in attrset.iter() {
                            scope_mut.insert(name.into(), value.clone());
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
                OpCode::Apply(arg_pos) => {
                    let lambda = self.pop_lambda()?;

                    let frame = match lambda {
                        Lambda::Lambda { mut scope, lambda } => {
                            let lambda =
                                self.runtime.program.get_lambda(lambda).ok_or_else(|| {
                                    EvalError::Internal(
                                        format!("invalid lambda id {} in bytecode", lambda.index())
                                            .into(),
                                    )
                                })?;
                            let lambda_pos = lambda.code;

                            let arg_name = lambda
                                .arg_name
                                .map(|id| self.runtime.program.get_str(id))
                                .unwrap_or("");
                            let thunk = LazyValue::Thunk(Thunk::uneval_with_scope(
                                arg_pos,
                                self.curr_frame.scope.clone(),
                            ));

                            let scope_mut = scope.get_mut();
                            scope_mut.insert(arg_name.into(), thunk);

                            Frame::new(lambda_pos, scope, FrameKind::Function)
                        }
                    };
                    self.begin_frame(frame)?;
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

                OpCode::HasAttr => {
                    let name = self.pop_string()?;
                    let attrset = self.pop_attrset()?;
                    self.push_value(Value::Bool(attrset.get(&name).is_some()))?;
                }
                OpCode::GetAttr => {
                    let lazy = self.get_attr()?;
                    match lazy.try_get_value() {
                        Ok(ok) => self.push_value(ok)?,
                        Err(thunk) => {
                            let (pos, scope) = thunk.eval_begin().map_err(EvalError::ThunkEval)?;
                            self.begin_frame(Frame::new(pos, scope, FrameKind::ThunkEval(thunk)))?;
                        }
                    }
                }
                OpCode::GetAttrOr(_expr_id) => todo!(),

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

                OpCode::Pop => _ = self.pop_value()?,

                OpCode::Ret => {
                    let ret = self.pop_value()?;

                    match &self.curr_frame.kind {
                        FrameKind::ThunkEval(thunk)
                        | FrameKind::ThunkEvalDeep(thunk)
                        | FrameKind::ThunkEvalDeepRoot(thunk) => {
                            thunk.eval_end(ret.clone()).map_err(|()| {
                                EvalError::Internal(
                                    "tried to finish a thunk that was not currently evaluating"
                                        .into(),
                                )
                            })?;
                        }
                        _ => {}
                    }

                    match &self.curr_frame.kind {
                        FrameKind::ThunkEvalDeep(_) | FrameKind::ThunkEvalDeepRoot(_) => match &ret
                        {
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
                        },
                        _ => {}
                    }

                    match &self.curr_frame.kind {
                        FrameKind::Function
                        | FrameKind::ThunkEval(_)
                        | FrameKind::ThunkEvalDeepRoot(_) => {
                            self.push_value(ret)?;
                        }
                        _ => {}
                    }

                    while !self.frame_stack.is_empty() {
                        match self.pop_frame()? {
                            PotentialFrame::Realized(frame) => {
                                self.curr_frame = frame;
                                continue 'main_loop;
                            }
                            PotentialFrame::PotentialDeep(thunk) => {
                                if thunk.get_value().is_some() {
                                    continue;
                                }
                                let (pos, scope) =
                                    thunk.eval_begin().map_err(EvalError::ThunkEval)?;
                                self.curr_frame =
                                    Frame::new(pos, scope, FrameKind::ThunkEvalDeep(thunk));
                                continue 'main_loop;
                            }
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
