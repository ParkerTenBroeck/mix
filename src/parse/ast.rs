use std::range::Range;

use crate::runtime::files::FileId;

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
        Self{ range: (start..end).into(), fid: other.fid }
    }
    
    pub fn before(self) -> Self {
        Self { range: (self.range.start..self.range.start).into(), fid: self.fid }
    }

    pub fn after(self) -> Self {
        Self { range: (self.range.end..self.range.end).into(), fid: self.fid }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node<T>(pub T, pub Span);

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

    PipeL,
    PipeR,
    LogImp,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Associativity {
    Left,
    None,
    Right,
}

impl BinOp {
    pub fn precedence(&self) -> u32 {
        match self {
            BinOp::Add => 20 - 4,
            BinOp::Sub => 20 - 4,

            BinOp::Mul => 20 - 3,
            BinOp::Div => 20 - 3,
            BinOp::Rem => 20 - 3,

            BinOp::Lt => 20 - 6,
            BinOp::Lte => 20 - 6,
            BinOp::Gt => 20 - 6,
            BinOp::Gte => 20 - 6,

            BinOp::Eq => 20 - 7,
            BinOp::Ne => 20 - 7,

            BinOp::And => 20 - 8,
            BinOp::Or => 20 - 10,

            BinOp::LogImp => 20 - 14,

            BinOp::PipeL => 20 - 15,
            BinOp::PipeR => 20 - 15,
        }
    }

    pub fn associativity(&self) -> Associativity {
        match self {
            BinOp::Add => Associativity::Left,
            BinOp::Sub => Associativity::Left,

            BinOp::Mul => Associativity::Left,
            BinOp::Div => Associativity::Left,
            BinOp::Rem => Associativity::Left,

            BinOp::Lt => Associativity::None,
            BinOp::Lte => Associativity::None,
            BinOp::Gt => Associativity::None,
            BinOp::Gte => Associativity::None,

            BinOp::Eq => Associativity::None,
            BinOp::Ne => Associativity::None,

            BinOp::And => Associativity::Left,
            BinOp::Or => Associativity::Left,

            BinOp::LogImp => Associativity::Right,

            BinOp::PipeL => Associativity::Right,
            BinOp::PipeR => Associativity::Left,
        }
    }
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
    AttrSet {
        attrs: Vec<Node<Attr<'a>>>,
    },
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
    Paren(Box<Node<Expr<'a>>>),
    Ident(&'a str),
    Num(Num),
    Str(&'a str),
}

#[derive(Clone, Debug)]
pub struct LetBinding<'a> {
    pub id: Node<Pattern<'a>>,
    pub value: Node<Expr<'a>>,
}

#[derive(Clone, Debug)]
pub struct AttrPath<'a> {
    pub parts: Vec<Node<AttrPathPart<'a>>>,
}

#[derive(Clone, Debug)]
pub enum AttrPathPart<'a> {
    Ident(&'a str),
    Str(&'a str),
    Expr(Expr<'a>),
}

#[derive(Clone, Debug)]
pub struct Attr<'a> {
    pub path: Node<AttrPath<'a>>,
    pub value: Option<Node<Expr<'a>>>,
}
