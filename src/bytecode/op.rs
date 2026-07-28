use crate::bytecode::{CodeLocOffset, ExprLoc, LambdaId, StrId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

	CreateThunk(ExprLoc),
	BeginThunk(ExprLoc),
	FinalizeThunk,

	And(CodeLocOffset),
	Or(CodeLocOffset),
	LogImp(CodeLocOffset),

	If(CodeLocOffset),

	CreateAttrSet,
	SetAttr,
	FinalizeAttrSetRec,

	CreateList(usize),
	AppendList,

	Apply,

	LoadLambda(LambdaId),
	LoadStr(StrId),
	LoadInt(i64),
	LoadFloat(f64),
	LoadBool(bool),

	LoadScope,

	HasAttr,
	GetAttr,
	GetAttrOr(CodeLocOffset),

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
