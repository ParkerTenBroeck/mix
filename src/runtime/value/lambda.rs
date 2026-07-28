use dumpster::Trace;

use crate::{
	bytecode::LambdaId,
	runtime::{scope::Scope, value::NativeLambda},
};

#[derive(Clone, Debug, Trace)]
pub enum Lambda {
	Lambda { scope: Scope, lambda: LambdaId },
	NativeLambda(NativeLambda),
}

impl From<NativeLambda> for Lambda {
	fn from(value: NativeLambda) -> Self {
		Self::NativeLambda(value)
	}
}
