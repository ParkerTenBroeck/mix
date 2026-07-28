use std::ops::{Deref, DerefMut};

use dumpster::{Trace, unsync::Gc};

use crate::{
	HashMap,
	runtime::{lazy::LazyValue, value::StringKind},
};

#[derive(Clone, Debug, Default, Trace)]
pub struct AttrSet {
	inner: Gc<AttrSetInner>,
}

#[derive(Clone, Default)]
pub struct AttrSetInner(HashMap<StringKind, LazyValue>);

impl Deref for AttrSetInner {
	type Target = HashMap<StringKind, LazyValue>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for AttrSetInner {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl std::fmt::Debug for AttrSetInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("AttrSet").field(&self.0).finish()
	}
}

unsafe impl<Z: dumpster::Visitor> dumpster::TraceWith<Z> for AttrSetInner {
	fn accept(&self, visitor: &mut Z) -> Result<(), ()> {
		for (k, v) in &self.0 {
			k.accept(visitor)?;
			v.accept(visitor)?;
		}
		Ok(())
	}
}

impl AttrSet {
	pub fn id(&self) -> usize {
		Gc::as_ptr(&self.inner) as *const () as usize
	}

	pub fn get_mut(&mut self) -> &mut HashMap<StringKind, LazyValue> {
		&mut Gc::make_mut(&mut self.inner).0
	}

	pub fn new() -> Self {
		Self::default()
	}

	pub fn from(map: HashMap<StringKind, LazyValue>) -> Self {
		Self {
			inner: Gc::new(AttrSetInner(map)),
		}
	}
}

impl Deref for AttrSet {
	type Target = HashMap<StringKind, LazyValue>;

	fn deref(&self) -> &Self::Target {
		&*self.inner
	}
}
