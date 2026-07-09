use crate::files::Node;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern<'a> {
    pub binding: Option<Node<&'a str>>,
    pub destruct: Vec<()>,
    pub strict_destruct: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Rem,
    Div,
    Mul,

    Sub,
    Add,

    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Ne,

    And,
    Or,
    LogImp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub struct Lambda<'a> {
    pub arg: Node<Pattern<'a>>,
    pub body: Box<Node<Expr<'a>>>,
}

#[derive(Clone, Copy, Debug)]
pub enum Num {
    Float(f64),
    Int(i64),
}

#[derive(Clone, Debug)]
pub enum Expr<'a> {
    Lambda(Lambda<'a>),
    FuncApp {
        func: Box<Node<Expr<'a>>>,
        arg: Box<Node<Expr<'a>>>,
    },
    IfThenElse {
        cond: Box<Node<Expr<'a>>>,
        then_expr: Box<Node<Expr<'a>>>,
        else_expr: Box<Node<Expr<'a>>>,
    },
    BinOp {
        lhs: Box<Node<Expr<'a>>>,
        op: Node<BinOp>,
        rhs: Box<Node<Expr<'a>>>,
    },
    UnOp {
        expr: Box<Node<Expr<'a>>>,
        op: Node<UnOp>,
    },
    Let {
        bindings: Vec<LetBinding<'a>>,
    },
    AttrSet(AttrSet<'a>),
    List {
        elements: Vec<Node<Expr<'a>>>,
    },
    AccessAttr {
        expr: Box<Node<Expr<'a>>>,
        path: Node<AttrPath<'a>>,
        or: Option<Box<Node<Expr<'a>>>>,
    },
    HasAttr {
        expr: Box<Node<Expr<'a>>>,
        path: Node<AttrPath<'a>>,
    },
    Ident(&'a str),
    Num(Num),
    Str(&'a str),
}

#[derive(Clone, Debug)]
pub struct LetBinding<'a> {
    pub id: Node<Pattern<'a>>,
    pub value: Node<Expr<'a>>,
}

#[derive(Clone, Debug, Default)]
pub struct AttrSet<'a> {
    pub static_attrs: Vec<Node<StaticAttr<'a>>>,
    pub dynamic_attrs: Vec<Node<DynamicAttr<'a>>>,
}

#[derive(Clone, Debug)]
pub struct StaticAttr<'a> {
    pub name: Node<&'a str>,
    pub value: Option<Node<Expr<'a>>>,
}

#[derive(Clone, Debug)]
pub struct DynamicAttr<'a> {
    pub part: Node<AttrPathPart<'a>>,
    pub value: Option<Node<Expr<'a>>>,
}

#[derive(Clone, Debug)]
pub struct AttrPath<'a> {
    pub parts: Vec<Node<AttrPathPart<'a>>>,
}

#[derive(Clone, Debug)]
pub enum AttrPathPart<'a> {
    Ident(&'a str),
    Num(i64),
    Expr(Expr<'a>),
}
