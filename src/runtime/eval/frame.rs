use crate::{
	bytecode::CodePos, runtime::{eval::NativeLambdaStateBox, scope::Scope, thunk::Thunk},
};

#[derive(Clone, Copy, Debug)]
pub enum DeepKind {
	No,
	Root,
	RootChild,
	RemainingChildren(u32),
}


#[derive(Debug)]
pub struct EvalFrame {
	pub pos: CodePos,
	pub scope: Scope,
}

pub enum FrameKind {
	Function {
		eval: EvalFrame,
	},
	Thunk {
		eval: EvalFrame,
		thunk: Thunk,
	},
	Deep {
		pos: Option<CodePos>,
		remaining: usize,
	},
	Native {
		state: NativeLambdaStateBox,
		name: std::borrow::Cow<'static, str>,
	},
}

impl std::fmt::Debug for FrameKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Function { eval } => f.debug_struct("Function").field("eval", eval).finish(),
			Self::Thunk { eval, thunk } => f
				.debug_struct("Thunk")
				.field("eval", eval)
				.field("thunk", thunk)
				.finish(),
			Self::Deep { pos, remaining } => f
				.debug_struct("Deep")
				.field("pos", pos)
				.field("remaining", remaining)
				.finish(),
			Self::Native { state, name } => f.debug_struct("Native").field("name", name).finish(),
		}
	}
}

#[derive(Debug)]
pub struct Frame {
	pub kind: FrameKind,
}
