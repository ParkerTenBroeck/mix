use crate::{
	bytecode::CodePos,
	runtime::{lazy::LazyValue, native::NativeLambdaState, scope::Scope, thunk::Thunk},
};

#[derive(Debug, Clone)]
pub enum FrameKind {
	Function,
	FunctionDeepRoot,
	ThunkEval(Thunk),
	ThunkEvalDeep(Thunk),
	ThunkEvalDeepRoot(Thunk),
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

pub enum PotentialFrame {
	Realized(Frame),
	DeepEval(CodePos),
	PotentialDeep(LazyValue),
	NativeLambda(NativeLambdaState),
}
