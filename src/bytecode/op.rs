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

	CreateAttrSet,
	InitAttrExpr(ExprLoc),
	FinalizeAttrSetRec,
	FinalizeAttrSet,

	CreateList(usize),
	AppendList(ExprLoc),

	Apply(ExprLoc),

	LoadLambda(LambdaId),
	LoadStr(StrId),
	LoadInt(i64),
	LoadFloat(f64),
	LoadBool(bool),

	LoadScope,

	HasAttr,
	GetAttr,
	GetAttrOr(ExprLoc),

	EvalThunk,

	BindThunkScope,
	BindValueScope,

	Branch(CodeLocOffset),

	PopV,
	DupV,

	PopT,
	DupT,

	Ret,
}
