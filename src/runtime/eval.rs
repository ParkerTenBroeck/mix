use std::{borrow::Cow, path::PathBuf};

use dumpster::unsync::Gc;

use crate::{
    bytecode::CodeLoc,
    runtime::{AttrSet, Lambda, List, Runtime, Value, scope::Scope},
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
}

impl<'a, 'b> Evaluator<'a, 'b> {
    pub fn new(runtime: &'b Runtime<'a>) -> Self {
        Self {
            runtime,
            call_stack: Default::default(),
            value_stack: Default::default(),
        }
    }

    pub fn eval_expr(&mut self, scope: Scope, expr: CodeLoc) -> Value {
        self.run(expr, scope).unwrap()
    }

    pub fn push_value(&mut self, value: Value) -> Result<(), EvalError<'a>> {
        self.value_stack.push(value);
        Ok(())
    }

    pub fn pop_value(&mut self) -> Result<Value, EvalError<'a>> {
        self.value_stack
            .pop()
            .ok_or(EvalError::ByteCode("value stack"))
    }

    pub fn push_call(&mut self, call: (CodeLoc, Scope)) -> Result<(), EvalError<'a>> {
        self.call_stack.push(call);
        Ok(())
    }

    pub fn pop_call(&mut self) -> Result<(CodeLoc, Scope), EvalError<'a>> {
        self.call_stack
            .pop()
            .ok_or(EvalError::ByteCode("call stack"))
    }

    pub fn run(&mut self, mut pos: CodeLoc, mut scope: Scope) -> Result<Value, EvalError<'a>> {
        use crate::bytecode::OpCode;
        loop {
            let op;
            (op, pos) = self.runtime.program.get(pos);
            match op {
                OpCode::Add => todo!(),
                OpCode::Sub => {}
                OpCode::Mul => todo!(),
                OpCode::Div => todo!(),
                OpCode::Rem => todo!(),
                OpCode::Eq => todo!(),
                OpCode::Ne => todo!(),
                OpCode::Lt => todo!(),
                OpCode::Lte => todo!(),
                OpCode::Gt => todo!(),
                OpCode::Gte => todo!(),
                OpCode::Not => todo!(),
                OpCode::Neg => todo!(),

                OpCode::And(rhs) => todo!(),
                OpCode::Or(rhs) => todo!(),
                OpCode::LogImp(rhs) => todo!(),

                OpCode::If(then_expr, else_expr) => {
                    let cond = self.pop_value()?;
                    match cond {
                        Value::Bool(cond) => {
                            self.push_call((pos, scope.clone()))?;
                            if cond {
                                pos = then_expr;
                            } else {
                                pos = else_expr;
                            }
                        }
                        _ => todo!(),
                    }
                }
                OpCode::CreateAttrSet => self.push_value(Value::AttrSet(AttrSet::default()))?,
                OpCode::InitAttrExpr(expr) => {
                    let path = self.pop_value()?;

                    let Value::Path(path) = &path else { todo!() };
                    let path = path.iter().next().unwrap().display().to_string();

                    let mut attr_set = self.pop_value()?;
                    match &mut attr_set {
                        Value::AttrSet(attr_set) => {
                            attr_set.get_mut();
                            // .insert(path.into(), LazyExpr::uneval(expr));
                        }
                        _ => todo!(),
                    }
                    self.push_value(attr_set)?;
                }
                OpCode::InitAttrPath => todo!(),
                OpCode::CreateList(capacity) => {
                    self.push_value(Value::List(List::with_capacity(capacity)))?
                }
                OpCode::AppendList => {}
                OpCode::Apply(loc) => {
                    self.push_call((pos, scope.clone()))?;
                    // Rc::new(super::Scope)
                }

                OpCode::LoadLambda(lambda_id) => {
                    let lambda = Lambda::Lambda {
                        scope: scope.clone(),
                        lambda: lambda_id,
                    };
                    self.push_value(Value::Lambda(lambda))?;
                }
                OpCode::LoadStr(str) => self.push_value(Value::String(Gc::new(str.into())))?,
                OpCode::LoadInt(int) => self.push_value(Value::Int(int))?,
                OpCode::LoadFloat(float) => self.push_value(Value::Float(float))?,

                OpCode::WithScope => todo!(),
                OpCode::HasAttr => todo!(),
                OpCode::GetAttr => todo!(),
                OpCode::GetAttrOr(expr_id) => todo!(),

                OpCode::Ret if self.call_stack.is_empty() => break self.pop_value(),
                OpCode::Ret => (pos, scope) = self.pop_call()?,

                OpCode::CreatePath => self.push_value(Value::Path(PathBuf::new()))?,
                OpCode::PushPathPart => {
                    let part = self.pop_value()?;
                    let path = self.pop_value()?;
                    let Value::String(part) = part else { todo!() };
                    let Value::Path(mut path) = path else { todo!() };
                    path.push(part.as_ref());
                    self.push_value(Value::Path(path))?;
                }
                OpCode::PopPathPart => todo!(),
            }
        }
    }
}
