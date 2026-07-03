use crate::bytecode::{CodeLocOffset, ExprLoc, LambdaId, StrId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpCode {
    Add,
    Sub,
    Mul,
    Div,
    Rem,

    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,

    Not,
    Neg,

    And(CodeLocOffset),
    Or(CodeLocOffset),
    LogImp(CodeLocOffset),

    If(CodeLocOffset),

    CreateAttrSet(usize),
    InitAttrExpr(ExprLoc),

    CreateList(usize),
    AppendList(ExprLoc),

    CreatePath,
    PushPathPart,
    PopPathPart,

    Apply(ExprLoc),

    LoadLambda(LambdaId),
    LoadStr(StrId),
    LoadInt(i64),
    LoadFloat(f64),
    LoadBool(bool),
    WithScope,

    HasAttr,
    GetAttr,
    GetAttrOr(ExprLoc),

    Branch(CodeLocOffset),

    Ret,
}
