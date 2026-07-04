use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

type Error<'a> = Cow<'a, str>;
type Storage = (Result<Cow<'static, str>, Error<'static>>, FileId);
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
}

impl Files {
    pub fn new(func: impl FnMut(&Path) -> LoaderResult + 'static) -> Self {
        Self {
            inner: RefCell::new(Inner {
                func: Box::new(func),
                loaded: Default::default(),
            }),
        }
    }
    pub fn load<'a>(&'a self, path: &Path) -> Return<'a> {
        let mut myself = self.inner.borrow_mut();
        if !myself.loaded.contains_key(path) {
            let result = (myself.func)(path);
            let storage = (result, FileId(myself.loaded.len() as u32));
            myself.loaded.insert(path.to_path_buf(), storage);
        }

        match &myself.loaded[path] {
            (Ok(ok), fid) => {
                // We know this is safe so long as the backing owned string does not drop before the lifetime of this struct ends
                let str = unsafe { std::mem::transmute::<&str, &'a str>(&*ok) };
                Ok((str, *fid))
            }
            (Err(e), _) => Err(e.clone()),
        }
    }

    pub fn exists(&self, path: &Path) -> bool {
        self.load(path).is_ok()
    }
}
