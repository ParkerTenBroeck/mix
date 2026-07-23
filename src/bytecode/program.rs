use std::{num::NonZeroUsize, rc::Rc};

use dumpster::Trace;
use serde::{Deserialize, Serialize};

use super::*;

use crate::{
	files::{Node, Span},
	mir,
};

#[derive(
	Clone, Copy, Debug, PartialEq, Eq, Hash, Trace, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct CodePos(usize);

impl CodePos {
	pub fn index(self) -> usize {
		self.0
	}

	pub fn from_index(index: usize) -> Self {
		Self(index)
	}
}

impl std::ops::Add<CodeLocOffset> for CodePos {
	type Output = CodePos;

	fn add(self, rhs: CodeLocOffset) -> Self::Output {
		CodePos(self.0 + rhs.0)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CodeLocOffset(pub(super) usize);

impl CodeLocOffset {
	pub fn offset(self) -> usize {
		self.0
	}
}

pub type ExprLoc = CodePos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StrId(NonZeroUsize);

impl StrId {
	pub fn index(self) -> usize {
		self.0.get()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Trace, Serialize, Deserialize)]
pub struct LambdaId(NonZeroUsize);

impl LambdaId {
	pub fn index(self) -> usize {
		self.0.get()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExprId(NonZeroUsize);

#[derive(Debug, Serialize, Deserialize)]
pub struct Lambda {
	pub code: CodePos,
	pub span: Span,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Expr {
	pub start: CodePos,
	pub end: CodePos,
	pub span: Span,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Program {
	code: Vec<OpCode>,
	lambdas: Vec<Lambda>,
	expressions: Vec<Expr>,
	strings: Vec<Rc<String>>,
}

impl Program {
	pub fn compile(&mut self, expr: &Node<mir::Expr>) -> CodePos {
		let compiler = crate::compiler::Compiler::new();
		compiler.compile_top_level(self, expr)
	}

	pub fn get(&self, loc: CodePos) -> Option<(OpCode, CodePos)> {
		Some((*self.code.get(loc.0)?, CodePos(loc.0 + 1)))
	}

	pub fn get_str(&self, str: StrId) -> crate::runtime::value::StringKind {
		crate::runtime::value::StringKind::Interned(
			self.strings.get(str.0.get() - 1).unwrap().clone(),
		)
	}

	pub fn get_lambda(&self, lambda: LambdaId) -> Option<&Lambda> {
		self.lambdas.get(lambda.0.get() - 1)
	}

	pub fn ops(&self) -> &[OpCode] {
		&self.code
	}

	pub fn lambdas(&self) -> &[Lambda] {
		&self.lambdas
	}

	pub fn expressions(&self) -> &[Expr] {
		&self.expressions
	}

	pub fn find_pos(&self, pos: CodePos) -> Span {
		self.expressions
			.iter()
			.filter(|expr| (expr.start..expr.end).contains(&pos))
			.min_by_key(|expr| expr.end.0 - expr.start.0)
			.unwrap_or(self.expressions.last().unwrap())
			.span
	}
}

impl ProgramBuilder for Program {
	fn emit_str(&mut self, str: &str) -> StrId {
		self.strings.push(Rc::new(str.into()));
		StrId(NonZeroUsize::new(self.strings.len()).unwrap())
	}

	fn emit_expr(
		&mut self,
		span: Span,
		expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> (ExprId, CodePos) {
		let mut builder = ByteCodeBuilder::new(self);
		expr(&mut builder);
		builder.emit(OpCode::Ret);

		let built_code = builder.finish();

		let start = CodePos(self.code.len());
		let end = CodePos(self.code.len() + built_code.len());
		self.expressions.push(Expr { start, end, span });
		let expr_id = ExprId(NonZeroUsize::new(self.expressions.len()).unwrap());

		for op in built_code {
			self.code.push(op);
		}

		(expr_id, start)
	}

	fn emit_lambda(
		&mut self,
		span: Span,
		expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> (LambdaId, CodePos) {
		let (_, code) = self.emit_expr(span, expr);
		self.lambdas.push(Lambda { code, span });
		(
			LambdaId(NonZeroUsize::new(self.lambdas.len()).unwrap()),
			code,
		)
	}
}
