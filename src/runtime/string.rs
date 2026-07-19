use std::{borrow::Borrow, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringKind{
	String(String),
	Interned(Rc<String>),
}

unsafe impl<__V: ::dumpster::Visitor> ::dumpster::TraceWith<__V> for StringKind {
	#[inline]
	fn accept(&self, visitor: &mut __V) -> ::core::result::Result<(), ()> {
		Ok(())
	}
}

impl AsRef<str> for StringKind{
	fn as_ref(&self) -> &str {
		&**self
	}
}

impl Borrow<str> for StringKind{
	fn borrow(&self) -> &str {
		&**self
	}
}

impl std::ops::Deref for StringKind{
	type Target = str;

	fn deref(&self) -> &Self::Target {
		match self{
			StringKind::String(str) => &**str,
			StringKind::Interned(gc) => &**gc,
		}
	}
}

impl std::hash::Hash for StringKind{
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(**self).hash(state);
	}
}

impl PartialOrd for StringKind{
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some((&**self).cmp(&**other))
	}
}

impl Ord for StringKind{
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(&**self).cmp(&**other)
	}
}
