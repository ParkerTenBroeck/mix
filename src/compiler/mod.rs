use std::path;

use crate::{
	bytecode::{ByteCodeBuilder, CodePos, ExprBuilder, OpCode, ProgramBuilder},
	files::Node,
	mir,
};

#[derive(Default)]
pub struct Compiler {}

impl Compiler {
	pub fn new() -> Self {
		Self {}
	}

	pub fn compile_top_level<'a>(
		&mut self,
		mut builder: impl ProgramBuilder,
		expr: &Node<mir::Expr<'a>>,
	) -> CodePos {
		let (_, loc) = builder.emit_expr(expr.1, |eb| {
			self.compile_expr(eb, expr);
		});
		loc
	}

	fn compile_lambda_pattern_rec<'a, 'b>(
		&mut self,
		builder: &'b mut ExprBuilder<'a>,
		pattern: &Node<mir::Pattern<'_>>,
	) {
		let eval = pattern.0.destruct.is_some() || pattern.0.ty.is_some();

		if eval {
			builder.emit(OpCode::EvalThunk);
		}

		if let Some(binding) = &pattern.0.binding
			&& binding.0 != "_"
		{
			if eval {
				builder.emit(OpCode::DupV);
			}
			builder.emit_load_str(binding.0);
			if eval {
				builder.emit(OpCode::BindValueScope);
			} else {
				builder.emit(OpCode::BindThunkScope);
			}
		}

		if let Some(destruct) = &pattern.0.destruct {
			match &destruct.0 {
				mir::PatternDestructKind::AttrSet { fields, strict } => {
					for (i, field) in fields.iter().enumerate() {
						if i != fields.len() - 1 {
							builder.emit(OpCode::DupV);
						}

						builder.emit_load_str(field.0.attr.0).emit(OpCode::GetAttr);
						self.compile_lambda_pattern_rec(builder, &field.0.pattern);
					}
				}
				mir::PatternDestructKind::List { elements, kind } => {
					// builder.emit(OpCode::PopV);
				}
			}
		}

		if let Some(_) = &pattern.0.ty {
			builder.emit(OpCode::PopV);
		}
	}

	fn compile_lambda_pattern<'a, 'b>(
		&mut self,
		builder: &'b mut ExprBuilder<'a>,
		pattern: &Node<mir::Pattern<'_>>,
	) {
		self.compile_lambda_pattern_rec(builder, pattern);
	}

	fn compile_expr<'a, 'b>(
		&mut self,
		builder: &'b mut ExprBuilder<'a>,
		expr: &Node<mir::Expr<'_>>,
	) -> &'b mut ExprBuilder<'a> {
		let Node(ast_expr, span) = expr;

		match ast_expr {
			mir::Expr::Lambda(lambda) => {
				let arg_name = lambda.arg.0.binding.map(|name| name.0);
				builder.emit_load_lambda(*span, arg_name, |builder| {
					self.compile_lambda_pattern(builder, &lambda.arg);
					self.compile_expr(builder, &lambda.body);
				});
			}
			mir::Expr::FuncApp { func, arg } => {
				self.compile_expr(builder, func)
					.emit_fn_app(arg.1, |builder| _ = self.compile_expr(builder, arg));
			}
			mir::Expr::IfThenElse {
				cond,
				then_expr,
				else_expr,
			} => {
				self.compile_expr(builder, cond)
					.emit_if_then(|builder| _ = self.compile_expr(builder, then_expr))
					.emit_else(|builder| _ = self.compile_expr(builder, else_expr));
			}
			mir::Expr::BinOp {
				lhs,
				op: op @ Node(mir::BinOp::Or | mir::BinOp::And | mir::BinOp::LogImp, _),
				rhs,
			} => {
				self.compile_expr(builder, lhs);

				match op.0 {
					mir::BinOp::And => {
						builder.emit_and(|builder| _ = self.compile_expr(builder, rhs))
					}
					mir::BinOp::Or => {
						builder.emit_or(|builder| _ = self.compile_expr(builder, rhs))
					}
					mir::BinOp::LogImp => {
						builder.emit_log_imp(|builder| _ = self.compile_expr(builder, rhs))
					}
					_ => unreachable!(),
				};
			}
			mir::Expr::BinOp { lhs, op, rhs } => {
				self.compile_expr(builder, lhs);
				self.compile_expr(builder, rhs);

				match op.0 {
					mir::BinOp::Rem => builder.emit_rem(),
					mir::BinOp::Div => builder.emit_div(),
					mir::BinOp::Mul => builder.emit_mul(),
					mir::BinOp::Sub => builder.emit_sub(),
					mir::BinOp::Add => builder.emit_add(),
					mir::BinOp::Lt => builder.emit_lt(),
					mir::BinOp::Lte => builder.emit_lte(),
					mir::BinOp::Gt => builder.emit_gt(),
					mir::BinOp::Gte => builder.emit_gte(),
					mir::BinOp::Eq => builder.emit_eq(),
					mir::BinOp::Ne => builder.emit_ne(),
					_ => unreachable!(),
				};
			}
			mir::Expr::UnOp { expr, op } => {
				self.compile_expr(builder, expr);
				match op.0 {
					mir::UnOp::Neg => builder.emit_neg(),
					mir::UnOp::Not => builder.emit_not(),
				};
			}
			mir::Expr::Let { bindings } => todo!(),
			mir::Expr::AttrSet(attrs) => {
				builder.emit(OpCode::CreateAttrSet);

				for attr in &attrs.static_attrs {
					if let Some(value) = &attr.0.value {
						builder.emit_load_str(attr.0.name.0);
						let expr = builder
							.emit_expr(value.1, |builder| _ = self.compile_expr(builder, value));
						builder.emit(OpCode::InitAttrExpr(expr.1));
					} else {
						todo!()
					}
				}

				for attr in &attrs.dynamic_attrs {
					if let Some(value) = &attr.0.value {
						self.compile_attr_part(builder, &attr.0.part);
						let expr = builder
							.emit_expr(value.1, |builder| _ = self.compile_expr(builder, value));
						builder.emit(OpCode::InitAttrExpr(expr.1));
					} else {
						todo!()
					}
				}
				builder.emit(OpCode::FinalizeAttrSetRec);
			}
			mir::Expr::List { elements } => {
				builder.emit_create_list(elements.len());
				for element in elements {
					builder.emit_append_list(element.1, |builder| {
						_ = self.compile_expr(builder, element)
					});
				}
			}
			mir::Expr::AccessAttr { expr, path, or } => {
				self.compile_expr(builder, expr);
				for part in &path.0.parts {
					self.compile_attr_part(builder, part);
					builder.emit(OpCode::GetAttr).emit(OpCode::EvalThunk);
				}
			}
			mir::Expr::HasAttr { expr, path } => {}
			mir::Expr::Ident("true") => _ = builder.emit_load_bool(true),
			mir::Expr::Ident("false") => _ = builder.emit_load_bool(false),
			mir::Expr::Ident(ident) => _ = builder.emit_load_str(ident).emit(OpCode::LoadScope),
			mir::Expr::Num(mir::Num::Float(float)) => _ = builder.emit_load_float(*float),
			mir::Expr::Num(mir::Num::Int(int)) => _ = builder.emit_load_int(*int),
			mir::Expr::Str(str) => _ = builder.emit_load_str(str),
		};

		builder
	}

	fn compile_attr_part(&mut self, builder: &mut ExprBuilder, part: &Node<mir::AttrPathPart>) {
		match &part.0 {
			mir::AttrPathPart::Ident(ident) => {
				builder.emit_load_str(ident);
			}
			mir::AttrPathPart::Expr(expr) => {
				_ = self.compile_expr(builder, &Node(expr.clone(), part.1));
			}
			mir::AttrPathPart::Num(i64) => _ = builder.emit_load_int(*i64),
		}
	}
}
