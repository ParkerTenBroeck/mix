use crate::{
	bytecode::CodePos,
	runtime::{eval::NativeLambdaStateBox, scope::Scope, thunk::Thunk},
};

#[derive(Clone, Copy, Debug)]
pub enum DeepKind {
	No,
	Root,
	RootChild,
	RemainingChildren(u32),
}

#[derive(Debug)]
pub struct ByteCodeFrame {
	pub pos: CodePos,
	pub scope: Scope,
}

pub struct NativeFrame {
	pub state: NativeLambdaStateBox,
	pub name: std::borrow::Cow<'static, str>,
}

impl std::fmt::Debug for NativeFrame {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("NativeFrame").field(&self.name).finish()
	}
}

#[derive(Debug)]
pub enum FrameKind {
	ByteCode(ByteCodeFrame),
	Native(NativeFrame),
}

#[derive(Debug)]
pub struct Frame {
	pub kind: FrameKind,
	pub thunk: Option<Thunk>,
	pub deep: bool,
}
impl Frame {
	pub fn new(kind: FrameKind) -> Self {
		Self {
			kind,
			thunk: None,
			deep: false,
		}
	}
}
