use std::{borrow::Borrow, rc::Rc};

#[derive(Clone)]
pub enum StringKind {
	String(String),
	Interned(Rc<String>),
}

impl std::fmt::Debug for StringKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		(**self).fmt(f)
	}
}

impl PartialEq for StringKind {
	fn eq(&self, other: &Self) -> bool {
		**self == **other
	}
}

impl Eq for StringKind {}

unsafe impl<__V: ::dumpster::Visitor> ::dumpster::TraceWith<__V> for StringKind {
	#[inline]
	fn accept(&self, _: &mut __V) -> ::core::result::Result<(), ()> {
		Ok(())
	}
}

impl AsRef<str> for StringKind {
	fn as_ref(&self) -> &str {
		&**self
	}
}

impl Borrow<str> for StringKind {
	fn borrow(&self) -> &str {
		&**self
	}
}

impl std::ops::Deref for StringKind {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		match self {
			StringKind::String(str) => &**str,
			StringKind::Interned(gc) => &**gc,
		}
	}
}

impl std::hash::Hash for StringKind {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(**self).hash(state);
	}
}

impl PartialOrd for StringKind {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some((&**self).cmp(&**other))
	}
}

impl Ord for StringKind {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(&**self).cmp(&**other)
	}
}

impl StringKind {
	pub fn get_mut(&mut self) -> &mut String {
		match self {
			StringKind::Interned(str) => *self = StringKind::String(str.as_str().to_owned()),
			_ => {}
		}

		match self {
			StringKind::String(str) => str,
			StringKind::Interned(_) => unreachable!(),
		}
	}
}
