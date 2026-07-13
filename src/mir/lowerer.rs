use std::{borrow::Cow, ops::Not};

use crate::{
	files::{Node, Span},
	mir,
	parse::ast,
	report::{Reports, mir::DuplicateAttrError},
};

pub type MirLowerResult<'a> = (Result<Node<mir::Expr<'a>>, ()>, Reports<'a>);

pub struct MirLowerer<'a> {
	reports: Reports<'a>,
}

#[derive(Clone, Debug)]
struct StaticAttrBuilder<'a> {
	name: Node<&'a str>,
	full_span: Span,
	value: Option<Node<mir::Expr<'a>>>,
	children: Vec<StaticAttrBuilder<'a>>,
}

impl<'a> MirLowerer<'a> {
	pub fn new(reports: Reports<'a>) -> Self {
		Self { reports }
	}

	pub fn lower(mut self, expr: Node<ast::Expr<'a>>) -> MirLowerResult<'a> {
		let expr = self.lower_expr(expr);
		let reports = self.reports;
		(
			reports.has_errors().not().then_some(expr).ok_or(()),
			reports,
		)
	}

	fn lower_expr(&mut self, Node(expr, span): Node<ast::Expr<'a>>) -> Node<mir::Expr<'a>> {
		let expr = match expr {
			ast::Expr::Lambda(lambda) => mir::Expr::Lambda(mir::Lambda {
				arg: self.lower_pattern(lambda.arg),
				body: Box::new(self.lower_expr(*lambda.body)),
			}),
			ast::Expr::FuncApp { func, arg } => mir::Expr::FuncApp {
				func: Box::new(self.lower_expr(*func)),
				arg: Box::new(self.lower_expr(*arg)),
			},
			ast::Expr::IfThenElse {
				cond,
				then_expr,
				else_expr,
			} => mir::Expr::IfThenElse {
				cond: Box::new(self.lower_expr(*cond)),
				then_expr: Box::new(self.lower_expr(*then_expr)),
				else_expr: Box::new(self.lower_expr(*else_expr)),
			},
			ast::Expr::BinOp { lhs, op, rhs } => self.lower_binop(*lhs, op, *rhs),
			ast::Expr::UnOp { expr, op } => mir::Expr::UnOp {
				expr: Box::new(self.lower_expr(*expr)),
				op: Node(self.lower_unop(op.0), op.1),
			},
			ast::Expr::Let { bindings } => mir::Expr::Let {
				bindings: bindings
					.into_iter()
					.map(|binding| mir::LetBinding {
						id: self.lower_pattern(binding.id),
						value: self.lower_expr(binding.value),
					})
					.collect(),
			},
			ast::Expr::AttrSet { attrs } => mir::Expr::AttrSet(self.lower_attr_set(attrs)),
			ast::Expr::List { elements } => mir::Expr::List {
				elements: elements
					.into_iter()
					.map(|element| self.lower_expr(element))
					.collect(),
			},
			ast::Expr::AccessAttr { expr, path, or } => mir::Expr::AccessAttr {
				expr: Box::new(self.lower_expr(*expr)),
				path: self.lower_attr_path(path),
				or: or.map(|or| Box::new(self.lower_expr(*or))),
			},
			ast::Expr::HasAttr { expr, path } => mir::Expr::HasAttr {
				expr: Box::new(self.lower_expr(*expr)),
				path: self.lower_attr_path(path),
			},
			ast::Expr::Paren(expr) => self.lower_expr(*expr).0,
			ast::Expr::Ident(ident) => mir::Expr::Ident(ident),
			ast::Expr::Num(ast::Num::Float(float)) => mir::Expr::Num(mir::Num::Float(float)),
			ast::Expr::Num(ast::Num::Int(int)) => mir::Expr::Num(mir::Num::Int(int)),
			ast::Expr::Str(str) => mir::Expr::Str(str),
		};

		Node(expr, span)
	}

	fn lower_pattern(&mut self, pattern: Node<ast::Pattern<'a>>) -> Node<mir::Pattern<'a>> {
		pattern.map(|pattern| mir::Pattern {
			binding: pattern.binding,
		})
	}

	fn lower_binop(
		&mut self,
		lhs: Node<ast::Expr<'a>>,
		op: Node<ast::BinOp>,
		rhs: Node<ast::Expr<'a>>,
	) -> mir::Expr<'a> {
		match op.0 {
			ast::BinOp::PipeL => mir::Expr::FuncApp {
				func: Box::new(self.lower_expr(lhs)),
				arg: Box::new(self.lower_expr(rhs)),
			},
			ast::BinOp::PipeR => mir::Expr::FuncApp {
				func: Box::new(self.lower_expr(rhs)),
				arg: Box::new(self.lower_expr(lhs)),
			},
			op_kind => mir::Expr::BinOp {
				lhs: Box::new(self.lower_expr(lhs)),
				op: Node(self.map_binop(op_kind), op.1),
				rhs: Box::new(self.lower_expr(rhs)),
			},
		}
	}

	fn lower_attr_set(&mut self, attrs: Vec<Node<ast::Attr<'a>>>) -> mir::AttrSet<'a> {
		let mut static_attrs = Vec::new();
		let mut dynamic_attrs = Vec::new();

		for attr in attrs {
			let lowered_value = attr.0.value.map(|value| self.lower_expr(value));
			if let Some(parts) = self.static_attr_parts(&attr.0.path) {
				self.insert_static_attr(
					&mut static_attrs,
					&parts,
					lowered_value,
					attr.1,
					String::new(),
				);
			} else {
				dynamic_attrs.push(self.lower_dynamic_attr(attr.0.path, lowered_value, attr.1));
			}
		}

		mir::AttrSet {
			static_attrs: static_attrs
				.into_iter()
				.map(|attr| self.finish_static_attr(attr))
				.collect(),
			dynamic_attrs,
		}
	}

	fn static_attr_parts(&self, path: &Node<ast::AttrPath<'a>>) -> Option<Vec<Node<&'a str>>> {
		path.0
			.parts
			.iter()
			.map(|part| match part.0 {
				ast::AttrPathPart::Ident(name) | ast::AttrPathPart::Str(name) => {
					Some(Node(name, part.1))
				}
				ast::AttrPathPart::Expr(_) => None,
				_ => todo!(),
			})
			.collect()
	}

	fn insert_static_attr(
		&mut self,
		attrs: &mut Vec<StaticAttrBuilder<'a>>,
		parts: &[Node<&'a str>],
		value: Option<Node<mir::Expr<'a>>>,
		attr_span: Span,
		prefix: String,
	) {
		let Node(name, name_span) = parts[0];
		let path_name = if prefix.is_empty() {
			name.to_string()
		} else {
			format!("{prefix}.{name}")
		};

		if parts.len() == 1 {
			if let Some(existing) = attrs.iter().find(|existing| existing.name.0 == name) {
				self.reports.emit(DuplicateAttrError {
					span: name_span,
					first: existing.full_span,
					name: Cow::Owned(path_name),
				});
				return;
			}

			attrs.push(StaticAttrBuilder {
				name: Node(name, name_span),
				full_span: attr_span,
				value,
				children: vec![],
			});
			return;
		}

		if let Some(existing) = attrs.iter_mut().find(|existing| existing.name.0 == name) {
			if existing.value.is_some() {
				self.reports.emit(DuplicateAttrError {
					span: name_span,
					first: existing.full_span,
					name: Cow::Owned(path_name),
				});
				return;
			}

			self.insert_static_attr(
				&mut existing.children,
				&parts[1..],
				value,
				attr_span,
				path_name,
			);
			return;
		}

		let mut child = StaticAttrBuilder {
			name: Node(name, name_span),
			full_span: attr_span,
			value: None,
			children: vec![],
		};
		self.insert_static_attr(
			&mut child.children,
			&parts[1..],
			value,
			attr_span,
			path_name,
		);
		attrs.push(child);
	}

	fn finish_static_attr(&mut self, attr: StaticAttrBuilder<'a>) -> Node<mir::StaticAttr<'a>> {
		let value = if !attr.children.is_empty() {
			let span = attr.full_span;
			Some(Node(
				mir::Expr::AttrSet(mir::AttrSet {
					static_attrs: attr
						.children
						.into_iter()
						.map(|child| self.finish_static_attr(child))
						.collect(),
					dynamic_attrs: vec![],
				}),
				span,
			))
		} else {
			attr.value
		};

		Node(
			mir::StaticAttr {
				name: attr.name,
				value,
			},
			attr.full_span,
		)
	}

	fn lower_dynamic_attr(
		&mut self,
		path: Node<ast::AttrPath<'a>>,
		value: Option<Node<mir::Expr<'a>>>,
		span: Span,
	) -> Node<mir::DynamicAttr<'a>> {
		let mut parts = self.lower_attr_path_parts(path);
		let part = parts.remove(0);
		let value = if parts.is_empty() {
			value
		} else {
			Some(Node(
				mir::Expr::AttrSet(mir::AttrSet {
					static_attrs: vec![],
					dynamic_attrs: vec![self.build_dynamic_attr(parts, value, span)],
				}),
				span,
			))
		};

		Node(mir::DynamicAttr { part, value }, span)
	}

	fn build_dynamic_attr(
		&mut self,
		mut parts: Vec<Node<mir::AttrPathPart<'a>>>,
		value: Option<Node<mir::Expr<'a>>>,
		span: Span,
	) -> Node<mir::DynamicAttr<'a>> {
		let part = parts.remove(0);
		let value = if parts.is_empty() {
			value
		} else {
			Some(Node(
				mir::Expr::AttrSet(mir::AttrSet {
					static_attrs: vec![],
					dynamic_attrs: vec![self.build_dynamic_attr(parts, value, span)],
				}),
				span,
			))
		};

		Node(mir::DynamicAttr { part, value }, span)
	}

	fn lower_attr_path(&mut self, path: Node<ast::AttrPath<'a>>) -> Node<mir::AttrPath<'a>> {
		let span = path.1;
		Node(
			mir::AttrPath {
				parts: self.lower_attr_path_parts(path),
			},
			span,
		)
	}

	fn lower_attr_path_parts(
		&mut self,
		Node(path, _span): Node<ast::AttrPath<'a>>,
	) -> Vec<Node<mir::AttrPathPart<'a>>> {
		path.parts
			.into_iter()
			.map(|Node(part, part_span)| {
				Node(
					match part {
						ast::AttrPathPart::Ident(ident) | ast::AttrPathPart::Str(ident) => {
							mir::AttrPathPart::Ident(ident)
						}
						ast::AttrPathPart::Num(num) => mir::AttrPathPart::Num(num),
						ast::AttrPathPart::Expr(expr) => {
							mir::AttrPathPart::Expr(self.lower_expr(Node(expr, part_span)).0)
						}
					},
					part_span,
				)
			})
			.collect()
	}

	fn map_binop(&self, op: ast::BinOp) -> mir::BinOp {
		match op {
			ast::BinOp::Rem => mir::BinOp::Rem,
			ast::BinOp::Div => mir::BinOp::Div,
			ast::BinOp::Mul => mir::BinOp::Mul,
			ast::BinOp::Sub => mir::BinOp::Sub,
			ast::BinOp::Add => mir::BinOp::Add,
			ast::BinOp::Lt => mir::BinOp::Lt,
			ast::BinOp::Lte => mir::BinOp::Lte,
			ast::BinOp::Gt => mir::BinOp::Gt,
			ast::BinOp::Gte => mir::BinOp::Gte,
			ast::BinOp::Eq => mir::BinOp::Eq,
			ast::BinOp::Ne => mir::BinOp::Ne,
			ast::BinOp::And => mir::BinOp::And,
			ast::BinOp::Or => mir::BinOp::Or,
			ast::BinOp::LogImp => mir::BinOp::LogImp,
			ast::BinOp::PipeL | ast::BinOp::PipeR => unreachable!(),
		}
	}

	fn lower_unop(&self, op: ast::UnOp) -> mir::UnOp {
		match op {
			ast::UnOp::Neg => mir::UnOp::Neg,
			ast::UnOp::Not => mir::UnOp::Not,
		}
	}
}
