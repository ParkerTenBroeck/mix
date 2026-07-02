use crate::bytecode::{ExprLoc, LambdaId, StrId};

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

    And(ExprLoc),
    Or(ExprLoc),
    LogImp(ExprLoc),

    If(ExprLoc, ExprLoc),

    CreateAttrSet,
    InitAttrExpr(ExprLoc),
    InitAttrPath,

    CreateList(usize),
    AppendList,

    CreatePath,
    PushPathPart,
    PopPathPart,

    Apply(ExprLoc),

    LoadLambda(LambdaId),
    LoadStr(StrId),
    LoadInt(i64),
    LoadFloat(f64),
    WithScope,

    HasAttr,
    GetAttr,
    GetAttrOr(ExprLoc),

    Ret,
}
