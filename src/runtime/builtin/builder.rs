use crate::runtime::{
	builtin::Trace, eval::NativeLambdaDyn, value::{AttrSet, Lambda, NativeLambda, StringKind, Value},
};

use super::*;

#[derive(Clone, Copy, Debug)]
pub struct BuiltinsBuilder {
	allow_imports: bool,
}

impl Default for BuiltinsBuilder {
	fn default() -> Self {
		Self::new()
	}
}

impl BuiltinsBuilder {
	pub const fn new() -> Self {
		Self {
			allow_imports: true,
		}
	}

	pub const fn allow_imports(mut self, allow: bool) -> Self {
		self.allow_imports = allow;
		self
	}

	pub fn build(self) -> AttrSet {
		let mut builtins = AttrSet::new();

		macro_rules! builtin {
			($expr:expr) => {{
				let builtin = $expr;
				builtins.get_mut().insert(
					StringKind::String(NativeLambdaDyn::identifier(&builtin).into()),
					Value::Lambda(Lambda::NativeLambda(NativeLambda::new(builtin))).into(),
				);
			}};
		}

		builtin!(Match::new());
		builtin!(MkList::new());
		builtin!(Map::new());
		builtin!(ToJson);
		builtin!(Trace);
		if self.allow_imports {
			builtin!(Import);
		}

		builtins
	}
}
