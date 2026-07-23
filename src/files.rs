use std::{
	borrow::Cow, cell::RefCell, ops::Deref, path::{Path, PathBuf}, rc::Rc,
};

use std::range::Range;

use crate::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
	#[serde(with = "span_range")]
	pub range: Range<usize>,
	pub fid: FileId,
}

// stupid
mod span_range {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};
	use std::range::Range;

	pub fn serialize<S>(range: &Range<usize>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		(range.start, range.end).serialize(serializer)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Range<usize>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let (start, end) = <(usize, usize)>::deserialize(deserializer)?;
		Ok((start..end).into())
	}
}

impl Span {
	pub fn new(range: Range<usize>, fid: FileId) -> Self {
		Self { range, fid }
	}

	pub fn merge(self, other: Span) -> Self {
		let start = self.range.start.min(other.range.start);
		let end = self.range.end.max(other.range.end);
		assert_eq!(self.fid, other.fid);
		Self {
			range: (start..end).into(),
			fid: other.fid,
		}
	}

	pub fn before(self) -> Self {
		Self {
			range: (self.range.start..self.range.start).into(),
			fid: self.fid,
		}
	}

	pub fn after(self) -> Self {
		Self {
			range: (self.range.end..self.range.end).into(),
			fid: self.fid,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node<T>(pub T, pub Span);

impl<T> Node<T> {
	pub fn map<R>(self, map: impl FnOnce(T) -> R) -> Node<R> {
		Node(map(self.0), self.1)
	}
}

type Error = Cow<'static, str>;
type Storage = Result<(Rc<String>, FileId), Error>;
type LoaderResult = Result<Rc<String>, Error>;
type Func = dyn FnMut(&Path) -> LoaderResult;
type Return = Result<(Rc<String>, FileId), Error>;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileId(u32);

#[derive(Clone)]
pub struct FileLoader {
	inner: Rc<RefCell<Inner>>,
}

pub struct Files<'a>(std::cell::Ref<'a, Inner>);

impl<'a> Deref for Files<'a>{
	type Target = Inner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::fmt::Debug for FileLoader {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Files").finish()
	}
}

pub struct Inner {
	func: Box<Func>,
	loaded: HashMap<PathBuf, Storage>,
	fid_mapping: Vec<PathBuf>,
}

impl Inner {
	pub fn load(&mut self, path: &Path) -> Return{
		if !self.loaded.contains_key(path) {
			let fid = FileId(self.fid_mapping.len() as u32);
			let result = (self.func)(path);
			self.fid_mapping.push(path.to_path_buf());
			self
				.loaded
				.insert(path.to_path_buf(), result.map(|cow| (cow, fid)));
		}

		match &self.loaded[path] {
			Ok((ok, fid)) => Ok((ok.clone(), *fid)),
			Err(e) => Err(e.clone()),
		}
	}

	pub fn file(&self, fid: FileId) -> (&Path, &Rc<String>) {
		let path = &self.fid_mapping[fid.0 as usize];
		let (contents, _) = self.loaded[path]
			.as_ref()
			.expect("requested file contents for a file that failed to load");

		(&path, &contents)
	}

	pub fn exists(&mut self, path: &Path) -> bool {
		self.load(path).is_ok()
	}
}

impl FileLoader {
	pub fn new(func: impl FnMut(&Path) -> LoaderResult + 'static) -> Self {
		Self {
			inner: Rc::new(RefCell::new(Inner {
				func: Box::new(func),
				loaded: Default::default(),
				fid_mapping: Default::default(),
			})),
		}
	}
	
	pub fn load(&self, path: &Path) -> Return {
		self.inner.borrow_mut().load(path)
	}

	pub fn file(&self, fid: FileId) -> (PathBuf, Rc<String>) {
		let inner = self.inner.borrow();
		let (path, file) = inner.file(fid);
		(path.to_owned(), file.clone())
	}

	pub fn files(&self) -> Files<'_>{
		Files(self.inner.borrow())
	}

	pub fn exists(&self, path: &Path) -> bool {
		self.load(path).is_ok()
	}
}
