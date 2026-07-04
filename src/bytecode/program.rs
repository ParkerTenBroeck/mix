use std::num::NonZeroUsize;

use dumpster::Trace;

use super::*;

use crate::{
    files::{Node, Span},
    mir::ast,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Trace)]
pub struct CodePos(usize);
impl CodePos {
    pub(crate) fn default() -> CodePos {
        CodePos(0)
    }
}

impl std::ops::Add<CodeLocOffset> for CodePos {
    type Output = CodePos;

    fn add(self, rhs: CodeLocOffset) -> Self::Output {
        CodePos(self.0 + rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CodeLocOffset(pub(super) usize);

pub type ExprLoc = CodePos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StrId(NonZeroUsize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Trace)]
pub struct LambdaId(NonZeroUsize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExprId(NonZeroUsize);

#[derive(Debug)]
pub struct Lambda {
    pub code: CodePos,
    pub span: Span,
}

#[derive(Debug)]
pub struct Expr {
    pub code: CodePos,
    pub span: Span,
}

#[derive(Default, Debug)]
pub struct Program {
    code: Vec<OpCode>,
    lambdas: Vec<Lambda>,
    expressions: Vec<Expr>,
    strings: Vec<String>,
}

impl Program {
    pub fn compile(&mut self, expr: &Node<ast::Expr>) -> CodePos {
        let mut compiler = crate::compiler::Compiler::new();
        compiler.compile_top_level(self, expr)
    }

    pub fn get(&self, loc: CodePos) -> (OpCode, CodePos) {
        (self.code[loc.0], CodePos(loc.0 + 1))
    }

    pub fn get_str(&self, str: StrId) -> &str {
        self.strings.get(str.0.get() - 1).unwrap()
    }
}

impl ProgramBuilder for Program {
    fn emit_str(&mut self, str: &str) -> StrId {
        self.strings.push(str.into());
        StrId(NonZeroUsize::new(self.strings.len()).unwrap())
    }

    fn emit_expr(&mut self, span: Span, expr: impl FnOnce(&mut ExprBuilder)) -> (ExprId, CodePos) {
        let mut builder = ExprBuilder::new(self);
        expr(&mut builder);
        builder.emit(OpCode::Ret);

        let built_code = builder.finish();

        let code = CodePos(self.code.len());
        self.expressions.push(Expr { code, span });
        let expr_id = ExprId(NonZeroUsize::new(self.expressions.len()).unwrap());

        for op in built_code {
            self.code.push(op);
        }

        (expr_id, code)
    }

    fn emit_lambda(
        &mut self,
        span: Span,
        expr: impl FnOnce(&mut ExprBuilder),
    ) -> (LambdaId, CodePos) {
        let (_, code) = self.emit_expr(span, expr);
        self.lambdas.push(Lambda { code, span });
        (
            LambdaId(NonZeroUsize::new(self.lambdas.len()).unwrap()),
            code,
        )
    }
}
