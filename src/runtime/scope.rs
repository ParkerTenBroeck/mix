use dumpster::Trace;

use crate::runtime::AttrSet;

#[derive(Clone, Default, Debug, Trace)]
pub struct Scope(AttrSet);

impl Scope {
    pub fn new_regular(prev: Option<Scope>) -> Self {
        // let inner = ScopeInner{
        //     prev,
        //     scope: Default::default(),
        // };
        todo!()
        // Self(Rc::new(inner))
    }

    // pub fn bottom(scope: HashMap<Cow<'a, str>, LazyExpr<'a>> ) -> Self{
    //     let inner = ScopeInner{
    //         prev: None,
    //         scope
    //     };
    //     todo!()
    //     // Self(Rc::new(inner))
    // }
}
