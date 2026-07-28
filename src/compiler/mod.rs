use crate::{
	bytecode::{ByteCodeBuilder, CodePos, ExprLoc, OpCode, ProgramBuilder},
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
		&self,
		mut builder: impl ProgramBuilder,
		expr: &Node<mir::Expr<'a>>,
	) -> CodePos {
		let (_, loc) = builder.emit_expr(expr.1, |eb| {
			self.compile_expr(eb, expr);
		});
		loc
	}

	fn compile_lambda_pattern_rec<'a, 'b>(
		&self,
		builder: &'b mut ByteCodeBuilder<'a>,
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
		&self,
		builder: &'b mut ByteCodeBuilder<'a>,
		pattern: &Node<mir::Pattern<'_>>,
	) {
		self.compile_lambda_pattern_rec(builder, pattern);
	}

	fn compile_maybe_thunk<'a, 'b>(
		&self,
		builder: &'b mut ByteCodeBuilder<'a>,
		expr: &Node<mir::Expr<'_>>,
	) -> Option<ExprLoc> {
		let Node(ast_expr, span) = expr;
		match ast_expr {
			mir::Expr::Ident(ident) => {
				builder.emit_load_str(ident).emit(OpCode::LoadScope);
				None
			}
			// mir::Expr::Lambda(_)
			mir::Expr::Num(_) | mir::Expr::Str(_) | mir::Expr::List { .. } => {
				self.compile_expr(builder, expr).emit(OpCode::UnEvalValue);
				None
			}
			mir::Expr::AttrSet(attr_set)
				if attr_set.dynamic_attrs.is_empty() && attr_set.dynamic_inherit.is_empty() =>
			{
				self.compile_expr(builder, expr).emit(OpCode::UnEvalValue);
				None
			}

			_ => Some(
				builder
					.emit_expr(*span, |builder| _ = self.compile_expr(builder, expr))
					.1,
			),
		}
	}

	fn compile_expr<'a, 'b>(
		&self,
		builder: &'b mut ByteCodeBuilder<'a>,
		expr: &Node<mir::Expr<'_>>,
	) -> &'b mut ByteCodeBuilder<'a> {
		let Node(ast_expr, span) = expr;

		match ast_expr {
			mir::Expr::Lambda(lambda) => {
				builder.emit_load_lambda(*span, |builder| {
					self.compile_lambda_pattern(builder, &lambda.arg);
					self.compile_expr(builder, &lambda.body);
				});
			}
			mir::Expr::FuncApp { func, arg } => {
				self.compile_expr(builder, func)
					.maybe_emit_create_thunk(|builder| self.compile_maybe_thunk(builder, arg))
					.emit_apply();
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
			mir::Expr::Let { bindings, expr } => {
				let mut to_finalize = 0;
				builder.emit(OpCode::EnterScope);
				for binding in bindings {
					builder.maybe_emit_begin_thunk(
						|builder| self.compile_maybe_thunk(builder, &binding.value),
						|builder| {
							to_finalize += 1;
							builder.emit(OpCode::DupT);
						},
						|_| {},
					);
					let name =
						binding.id.0.binding.expect(
							"non-identifier let bindings must be rejected during MIR analysis",
						);
					builder.emit_load_str(name.0);
					builder.emit(OpCode::BindThunkScope);
				}

				for _ in 0..to_finalize {
					builder.emit(OpCode::FinalizeThunk).emit(OpCode::PopT);
				}

				self.compile_expr(builder, expr);
				builder.emit(OpCode::LeaveScope);
			}
			mir::Expr::AttrSet(attrs) => {
				builder.emit(OpCode::CreateAttrSet);

				for attr in &attrs.static_attrs {
					builder.emit_load_str(attr.0.name.0);

					builder
						.maybe_emit_create_thunk(|builder| {
							self.compile_maybe_thunk(builder, &attr.0.value)
						})
						.emit(OpCode::SetAttr);
				}

				for attr in &attrs.dynamic_attrs {
					self.compile_attr_part(builder, &attr.0.part);
					builder
						.maybe_emit_create_thunk(|builder| {
							self.compile_maybe_thunk(builder, &attr.0.value)
						})
						.emit(OpCode::SetAttr);
				}

				for attr in &attrs.static_inherit {
					builder
						.emit_load_str(attr.0)
						.emit(OpCode::DupV)
						.emit(OpCode::LoadScope)
						.emit(OpCode::SetAttr);
				}

				for attr in &attrs.dynamic_attrs {
					self.compile_attr_part(builder, &attr.0.part)
						.emit(OpCode::DupV)
						.emit(OpCode::LoadScope)
						.emit(OpCode::SetAttr);
				}
			}
			mir::Expr::List { elements } => {
				builder.emit_create_list(elements.len());
				for element in elements {
					builder
						.maybe_emit_create_thunk(|builder| {
							self.compile_maybe_thunk(builder, element)
						})
						.emit(OpCode::AppendList);
				}
			}
			mir::Expr::AccessAttr { expr, path, or } => {
				self.compile_expr(builder, expr);
				if let Some(or) = or {
					builder.emit_get_attr_or(
						path.0.parts.iter().map(|part| {
							|builder: &mut ByteCodeBuilder<'_>| {
								self.compile_attr_part(builder, part);
							}
						}),
						|builder| _ = builder.emit(OpCode::EvalThunk),
						|builder| _ = self.compile_expr(builder, or),
					);
				} else {
					for part in &path.0.parts {
						self.compile_attr_part(builder, part);
						builder.emit(OpCode::GetAttr).emit(OpCode::EvalThunk);
					}
				}
			}
			mir::Expr::HasAttr { expr, path } => {
				self.compile_expr(builder, expr);
				builder.emit_get_attr_or(
					path.0.parts.iter().map(|part| {
						|builder: &mut ByteCodeBuilder<'_>| {
							_ = self.compile_attr_part(builder, part)
						}
					}),
					|builder| _ = builder.emit(OpCode::PopT).emit_load_bool(true),
					|builder| _ = builder.emit_load_bool(false),
				);
			}
			mir::Expr::Ident("true") => _ = builder.emit_load_bool(true),
			mir::Expr::Ident("false") => _ = builder.emit_load_bool(false),
			mir::Expr::Ident(ident) => {
				_ = builder
					.emit_load_str(ident)
					.emit(OpCode::LoadScope)
					.emit(OpCode::EvalThunk)
			}
			mir::Expr::Num(mir::Num::Float(float)) => _ = builder.emit_load_float(*float),
			mir::Expr::Num(mir::Num::Int(int)) => _ = builder.emit_load_int(*int),
			mir::Expr::Str(str) => _ = builder.emit_load_str(str),
		};

		builder
	}

	fn compile_attr_part<'a, 'b>(
		&self,
		builder: &'b mut ByteCodeBuilder<'a>,
		part: &Node<mir::AttrPathPart>,
	) -> &'b mut ByteCodeBuilder<'a> {
		match &part.0 {
			mir::AttrPathPart::Ident(ident) => builder.emit_load_str(ident),
			mir::AttrPathPart::Expr(expr) => {
				self.compile_expr(builder, &Node(expr.clone(), part.1))
			}
			mir::AttrPathPart::Num(i64) => builder.emit_load_int(*i64),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::rc::Rc;

	use crate::{
		files::FileLoader,
		runtime::{Runtime, scope::ScopeBuilder},
	};

	fn eval(source: &str) -> Result<crate::runtime::value::Value, String> {
		let source: Rc<String> = Rc::new(source.to_owned());
		let loader = FileLoader::new(move |_| Ok(source.clone()));
		let scope = ScopeBuilder::new()
			.with("false", false)
			.with("true", true)
			.bottom();
		let mut runtime = Runtime::new(loader, scope);
		let lazy = runtime
			.load("test.mix")
			.map_err(|_| "source failed to compile".to_owned())?;
		match runtime.eval(lazy, true) {
			Ok(value) => Ok(value),
			Err(error) => Err(error.render(&runtime)),
		}
	}

	#[test]
	fn let_bindings_share_a_recursive_scope() {
		let value = eval("let x = 1; y = x + 1 in y").unwrap();
		assert_eq!(value.expect_int().unwrap(), 2);
	}

	#[test]
	fn attrsets_do_not_bind_their_keys() {
		assert!(eval("{ x = 1; y = x; }.y").is_err());
	}

	#[test]
	fn duplicate_inherited_attr_is_rejected() {
		assert!(eval("let x = 1 in { x; x = 2; }").is_err());
	}

	#[test]
	fn non_identifier_let_bindings_are_rejected() {
		assert!(eval("let { x } = { x = 1; } in x").is_err());
		assert!(eval("let x :: int = 1 in x").is_err());
	}
}
