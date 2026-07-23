use crate::files::Span;

use super::*;

pub trait ProgramBuilder {
	fn emit_str(&mut self, str: &str) -> StrId;
	fn emit_expr(
		&mut self,
		span: Span,
		expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> (ExprId, CodePos);
	fn emit_lambda(
		&mut self,
		span: Span,
		expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> (LambdaId, CodePos);
}

impl<T: ProgramBuilder> ProgramBuilder for &mut T {
	fn emit_str(&mut self, str: &str) -> StrId {
		(*self).emit_str(str)
	}

	fn emit_expr(
		&mut self,
		span: Span,
		expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> (ExprId, CodePos) {
		(*self).emit_expr(span, expr)
	}

	fn emit_lambda(
		&mut self,
		span: Span,
		expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> (LambdaId, CodePos) {
		(*self).emit_lambda(span, expr)
	}
}

#[derive(Debug)]
pub struct ByteCodeBuilder<'a> {
	code: Vec<OpCode>,
	program: &'a mut Program,
}

impl<'a> ProgramBuilder for ByteCodeBuilder<'a> {
	fn emit_str(&mut self, str: &str) -> StrId {
		self.program.emit_str(str)
	}

	fn emit_expr(
		&mut self,
		span: Span,
		expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> (ExprId, CodePos) {
		self.program.emit_expr(span, expr)
	}

	fn emit_lambda(
		&mut self,
		span: Span,
		expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> (LambdaId, CodePos) {
		self.program.emit_lambda(span, expr)
	}
}

impl<'a> ByteCodeBuilder<'a> {
	pub fn new(program: &'a mut Program) -> Self {
		Self {
			code: Default::default(),
			program,
		}
	}

	pub fn finish(self) -> Vec<OpCode> {
		self.code
	}
	
	pub fn emit(&mut self, op: OpCode) -> &mut Self {
		self.code.push(op);
		self
	}

	pub fn clone(&mut self) -> ByteCodeBuilder<'_> {
		ByteCodeBuilder {
			code: Default::default(),
			program: self.program,
		}
	}
}

impl<'a> ByteCodeBuilder<'a> {
	pub fn emit_add(&mut self) -> &mut Self {
		self.emit(OpCode::Add)
	}
	pub fn emit_sub(&mut self) -> &mut Self {
		self.emit(OpCode::Sub)
	}
	pub fn emit_mul(&mut self) -> &mut Self {
		self.emit(OpCode::Mul)
	}
	pub fn emit_div(&mut self) -> &mut Self {
		self.emit(OpCode::Div)
	}
	pub fn emit_rem(&mut self) -> &mut Self {
		self.emit(OpCode::Rem)
	}

	pub fn emit_eq(&mut self) -> &mut Self {
		self.emit(OpCode::Eq)
	}
	pub fn emit_ne(&mut self) -> &mut Self {
		self.emit(OpCode::Ne)
	}
	pub fn emit_lt(&mut self) -> &mut Self {
		self.emit(OpCode::Lt)
	}
	pub fn emit_gt(&mut self) -> &mut Self {
		self.emit(OpCode::Gt)
	}
	pub fn emit_lte(&mut self) -> &mut Self {
		self.emit(OpCode::Lte)
	}
	pub fn emit_gte(&mut self) -> &mut Self {
		self.emit(OpCode::Gte)
	}

	pub fn emit_not(&mut self) -> &mut Self {
		self.emit(OpCode::Not)
	}
	pub fn emit_neg(&mut self) -> &mut Self {
		self.emit(OpCode::Neg)
	}

	pub fn emit_and(&mut self, second_expr: impl FnOnce(&mut ByteCodeBuilder)) -> &mut Self {
		let mut second_code = self.clone();
		second_expr(&mut second_code);
		let second_code = second_code.finish();

		self.emit(OpCode::And(CodeLocOffset(second_code.len())));

		for code in second_code {
			self.emit(code);
		}

		self
	}
	pub fn emit_or(&mut self, second_expr: impl FnOnce(&mut ByteCodeBuilder)) -> &mut Self {
		let mut second_code = self.clone();
		second_expr(&mut second_code);
		let second_code = second_code.finish();

		self.emit(OpCode::Or(CodeLocOffset(second_code.len())));

		for code in second_code {
			self.emit(code);
		}

		self
	}
	pub fn emit_log_imp(&mut self, second_expr: impl FnOnce(&mut ByteCodeBuilder)) -> &mut Self {
		let mut second_code = self.clone();
		second_expr(&mut second_code);
		let second_code = second_code.finish();

		self.emit(OpCode::LogImp(CodeLocOffset(second_code.len())));

		for code in second_code {
			self.emit(code);
		}

		self
	}

	pub fn emit_get_attr_or<P: FnOnce(&mut ByteCodeBuilder<'_>)>(
		&mut self,
		parts: impl Iterator<Item = P>,
		success: impl FnOnce(&mut ByteCodeBuilder<'_>),
		fallback: impl FnOnce(&mut ByteCodeBuilder<'_>),
	) -> &mut Self {
		let parts = parts
			.map(|builder| {
				let mut clone = self.clone();
				builder(&mut clone);
				clone.finish()
			})
			.collect::<Vec<_>>();

		let mut success_builder = self.clone();
		success(&mut success_builder);
		let success_code = success_builder.finish();

		let mut fallback_builder = self.clone();
		fallback(&mut fallback_builder);
		let fallback_code = fallback_builder.finish();

		let mut to_update = Vec::new();

		for (i, part) in parts.iter().enumerate() {
			for op in part {
				self.emit(*op);
			}

			self.emit(OpCode::GetAttrOr(CodeLocOffset(0)));
			to_update.push(self.code.len() - 1);

			if i != parts.len() - 1 {
				self.emit(OpCode::EvalThunk);
			}
		}

		for op in success_code {
			self.emit(op);
		}
		self.emit(OpCode::Branch(CodeLocOffset(fallback_code.len())));

		for pos in to_update {
			let off = CodeLocOffset(self.code.len() - pos - 1);
			self.code[pos] = OpCode::GetAttrOr(off)
		}
		for op in fallback_code {
			self.emit(op);
		}

		self
	}

	pub fn emit_get_attr_or_jump(&mut self) -> &mut Self {
		self.emit(OpCode::GetAttrOr(CodeLocOffset(0)))
	}

	pub fn emit_if_then(
		&mut self,
		then_expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> ThenBuilder<'_, 'a> {
		let mut then_builder = self.clone();
		then_expr(&mut then_builder);
		ThenBuilder {
			code: then_builder.finish(),
			builder: &mut *self,
		}
	}

	pub fn emit_fn_app(
		&mut self,
		span: Span,
		arg: impl FnMut(&mut ByteCodeBuilder<'_>),
	) -> &mut Self {
		let arg = self.emit_expr(span, arg).1;
		self.emit(OpCode::Apply(arg))
	}

	pub fn emit_create_list(&mut self, len: usize) -> &mut Self {
		self.emit(OpCode::CreateList(len))
	}
	pub fn emit_append_list(
		&mut self,
		span: Span,
		arg: impl FnMut(&mut ByteCodeBuilder<'_>),
	) -> &mut Self {
		let arg = self.emit_expr(span, arg).1;
		self.emit(OpCode::AppendList(arg))
	}

	pub fn emit_load_str(&mut self, str: &str) -> &mut Self {
		let id = self.emit_str(str);
		self.emit(OpCode::LoadStr(id))
	}
	pub fn emit_load_int(&mut self, int: i64) -> &mut Self {
		self.emit(OpCode::LoadInt(int))
	}
	pub fn emit_load_float(&mut self, float: f64) -> &mut Self {
		self.emit(OpCode::LoadFloat(float))
	}
	pub fn emit_load_bool(&mut self, bool: bool) -> &mut Self {
		self.emit(OpCode::LoadBool(bool))
	}
	pub fn emit_load_lambda(
		&mut self,
		span: Span,
		body: impl FnMut(&mut ByteCodeBuilder<'_>),
	) -> &mut Self {
		let lambda = self.emit_lambda(span, body).0;
		self.emit(OpCode::LoadLambda(lambda))
	}
}

#[must_use]
pub struct ThenBuilder<'a, 'p> {
	code: Vec<OpCode>,
	builder: &'a mut ByteCodeBuilder<'p>,
}

impl<'a, 'p> ThenBuilder<'a, 'p> {
	pub fn emit_else(
		self,
		else_expr: impl FnOnce(&mut ByteCodeBuilder),
	) -> &'a mut ByteCodeBuilder<'p> {
		let mut else_builder = self.builder.clone();
		else_expr(&mut else_builder);
		let mut then_code = self.code;
		let else_code = else_builder.finish();

		then_code.push(OpCode::Branch(CodeLocOffset(else_code.len())));

		self.builder
			.emit(OpCode::If(CodeLocOffset(then_code.len())));
		for op in then_code {
			self.builder.emit(op);
		}
		for op in else_code {
			self.builder.emit(op);
		}

		self.builder
	}
}
