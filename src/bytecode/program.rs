use std::{num::NonZeroUsize, range::Range};

use super::*;

use crate::{
    parse::ast,
    runtime::{files::FileId},
};

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct Loc {
    range: Range<usize>,
    file: FileId,
}

impl Loc {
    pub fn new(range: Range<usize>, file: FileId) -> Self {
        Self { range, file }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CodeLoc(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CodeLocOffset(usize);

pub type ExprLoc = CodeLoc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StrId(NonZeroUsize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LambdaId(NonZeroUsize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExprId(NonZeroUsize);

#[derive(Debug)]
pub struct Lambda {
    pub code: CodeLoc,
    pub loc: Loc,
}

#[derive(Debug)]
pub struct Expr {
    pub code: CodeLoc,
    pub loc: Loc,
}

#[derive(Default, Debug)]
pub struct Program {
    code: Vec<OpCode>,
    lambdas: Vec<Lambda>,
    expressions: Vec<Expr>,
    strings: Vec<String>,
}

impl Program {
    pub fn compile(&mut self, fid: FileId, expr: &ast::Node<ast::Expr>) -> CodeLoc {
        let mut compiler = crate::compiler::FileCompiler::new(fid);
        compiler.compile_expr(self, expr)
    }

    pub fn get(&self, loc: CodeLoc) -> (OpCode, CodeLoc) {
        (self.code[loc.0], CodeLoc(loc.0 + 1))
    }
}

impl ProgramBuilder for Program {
    fn emit_str(&mut self, str: &str) -> StrId {
        self.strings.push(str.into());
        StrId(NonZeroUsize::new(self.strings.len()).unwrap())
    }

    fn emit_expr(&mut self, loc: Loc, expr: impl FnOnce(&mut ExprBuilder)) -> (ExprId, CodeLoc) {
        let mut builder = ExprBuilder::new(self);
        expr(&mut builder);

        self.expressions.push();
    }

    fn emit_lambda(&mut self, loc: Loc, expr: impl FnOnce(&mut ExprBuilder)) -> (LambdaId, CodeLoc) {
        todo!()
    }
}