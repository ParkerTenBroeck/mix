use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use std::range::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub fid: FileId,
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

type Error<'a> = Cow<'a, str>;
type Storage = Result<(Cow<'static, str>, FileId), Error<'static>>;
type LoaderResult = Result<Cow<'static, str>, Error<'static>>;
type Func = dyn FnMut(&Path) -> LoaderResult;
type Return<'a> = Result<(&'a str, FileId), Error<'a>>;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FileId(u32);

pub struct Files {
    inner: RefCell<Inner>,
}

impl std::fmt::Debug for Files {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Files").finish()
    }
}

struct Inner {
    func: Box<Func>,
    loaded: HashMap<PathBuf, Storage>,
    fid_mapping: Vec<PathBuf>,
}

impl Files {
    pub fn new(func: impl FnMut(&Path) -> LoaderResult + 'static) -> Self {
        Self {
            inner: RefCell::new(Inner {
                func: Box::new(func),
                loaded: Default::default(),
                fid_mapping: Default::default(),
            }),
        }
    }
    pub fn load<'a>(&'a self, path: &Path) -> Return<'a> {
        let mut myself = self.inner.borrow_mut();
        if !myself.loaded.contains_key(path) {
            let fid = FileId(myself.fid_mapping.len() as u32);
            let result = (myself.func)(path);
            myself.fid_mapping.push(path.to_path_buf());
            myself
                .loaded
                .insert(path.to_path_buf(), result.map(|cow| (cow, fid)));
        }

        match &myself.loaded[path] {
            Ok((ok, fid)) => {
                // We know this is safe so long as the backing owned string does not drop before the lifetime of this struct ends
                let str = unsafe { std::mem::transmute::<&str, &'a str>(ok) };
                Ok((str, *fid))
            }
            Err(e) => Err(e.clone()),
        }
    }

    pub fn file<'a>(&'a self, fid: FileId) -> (&'a Path, &'a str) {
        let myself = self.inner.borrow();
        let path = &myself.fid_mapping[fid.0 as usize];
        let (contents, _) = myself.loaded[path]
            .as_ref()
            .expect("requested file contents for a file that failed to load");

        // The backing storage lives inside `self`, so extending these references to `'a`
        // follows the same safety contract as `load`.
        let path = unsafe { std::mem::transmute::<&Path, &'a Path>(path.as_path()) };
        let contents = unsafe { std::mem::transmute::<&str, &'a str>(contents) };
        (path, contents)
    }

    pub fn exists(&self, path: &Path) -> bool {
        self.load(path).is_ok()
    }
}
