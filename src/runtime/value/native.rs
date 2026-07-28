use std::ops::Deref;

use dumpster::{Trace, unsync::Gc};

use crate::runtime::eval::NativeLambdaDyn;

#[derive(Clone, Trace)]
pub struct NativeLambda {
	inner: Gc<Box<dyn NativeLambdaDyn>>, // silly rust
}

impl Deref for NativeLambda {
	type Target = dyn NativeLambdaDyn;

	fn deref(&self) -> &Self::Target {
		&**self.inner
	}
}

impl NativeLambda {
	pub fn new<T: NativeLambdaDyn>(lambda: T) -> Self {
		Self {
			inner: Gc::new(Box::new(lambda)),
		}
	}
}

impl std::fmt::Debug for NativeLambda {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("NativeLambda")
			.field("identifer", &self.inner.identifier())
			.finish()
	}
}
