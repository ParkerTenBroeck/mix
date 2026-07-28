use std::{collections::VecDeque, ops::Deref};

use dumpster::{Trace, unsync::Gc};

use crate::runtime::lazy::LazyValue;

#[derive(Clone, Default, Debug, Trace)]
pub struct List {
	inner: Gc<VecDeque<LazyValue>>,
}

impl List {
	pub fn with_capacity(capacity: usize) -> List {
		Self {
			inner: Gc::new(VecDeque::with_capacity(capacity)),
		}
	}

	pub fn id(&self) -> usize {
		Gc::as_ptr(&self.inner) as *const () as usize
	}

	pub fn get_mut(&mut self) -> &mut VecDeque<LazyValue> {
		Gc::make_mut(&mut self.inner)
	}
}

impl Deref for List {
	type Target = VecDeque<LazyValue>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
