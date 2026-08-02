use std::fmt::Write;

use super::{AttrSeparatorStyle, FormatOptions, IndentStyle, layout::*};

use crate::{
	files::Node,
	parse::ast::{
		Attr, AttrPath, AttrPathPart, Expr, Num, Pattern, PatternDestructKind, PatternListKind,
		Type, UnOp,
	},
};

pub fn format_ast(expr: &Node<Expr<'_>>, options: &FormatOptions) -> String {
	let mut formatter = Formatter {
		options,
		output: String::new(),
	};
	formatter.expr(&expr.0, 0, 0);
	if options.final_newline {
		formatter.output.push('\n');
	}
	formatter.output
}

struct Formatter<'a> {
	options: &'a FormatOptions,
	output: String,
}

impl Formatter<'_> {
	fn indent(&mut self, depth: usize) {
		match self.options.indent_style {
			IndentStyle::Spaces => self
				.output
				.push_str(&" ".repeat(depth * self.options.indent_width)),
			IndentStyle::Tabs => self.output.push_str(&"\t".repeat(depth)),
		}
	}

	fn expr(&mut self, expr: &Expr<'_>, depth: usize, parent_precedence: u32) {
		match expr {
			Expr::Lambda(lambda) => {
				self.pattern(&lambda.arg.0);
				self.output.push(':');
				let mut body = &lambda.body.0;
				while let Expr::Lambda(next) = body {
					self.output.push(' ');
					self.pattern(&next.arg.0);
					self.output.push(':');
					body = &next.body.0;
				}
				self.nested_or_inline(body, depth);
			}
			Expr::FuncApp { func, arg } => {
				self.expr_wrapped(&func.0, depth, is_application_atom(&func.0));
				self.output.push(' ');
				self.expr_wrapped(&arg.0, depth, is_argument_atom(&arg.0));
			}
			Expr::IfThenElse {
				cond,
				then_expr,
				else_expr,
			} => {
				self.output.push_str("if ");
				self.expr(&cond.0, depth, 0);
				self.output.push_str(" then");
				self.conditional_branch(&then_expr.0, depth);
				self.output.push('\n');
				self.indent(depth);
				self.output.push_str("else");
				if matches!(&else_expr.0, Expr::IfThenElse { .. }) {
					self.output.push(' ');
					self.expr(&else_expr.0, depth, 0);
				} else {
					self.conditional_branch(&else_expr.0, depth);
				}
			}
			Expr::BinOp { lhs, op, rhs } => {
				let precedence = op.0.precedence();
				let wrapped = precedence < parent_precedence;
				if wrapped {
					self.output.push('(');
				}
				self.expr(&lhs.0, depth, precedence);
				write!(self.output, " {} ", binop(op.0)).ok();
				self.expr(&rhs.0, depth, precedence + 1);
				if wrapped {
					self.output.push(')');
				}
			}
			Expr::UnOp { expr, op } => {
				self.output.push_str(match op.0 {
					UnOp::Neg => "-",
					UnOp::Not => "!",
				});
				self.expr_wrapped(&expr.0, depth, is_unary_atom(&expr.0));
			}
			Expr::Let { bindings, expr } => {
				self.output.push_str("let\n");
				for (index, binding) in bindings.iter().enumerate() {
					self.indent(depth + 1);
					self.pattern(&binding.id.0);
					self.output.push_str(" = ");
					self.expr(&binding.value.0, depth + 1, 0);
					if self.options.trailing_separator || index + 1 < bindings.len() {
						self.output.push(self.options.let_separator.token());
					}
					self.output.push('\n');
					if index + 1 < bindings.len()
						&& (is_block_expr(&binding.value.0)
							|| is_block_expr(&bindings[index + 1].value.0))
					{
						self.output.push('\n');
					}
				}
				self.indent(depth);
				self.output.push_str("in");
				self.nested_or_inline(&expr.0, depth);
			}
			Expr::AttrSet { attrs } => self.attrset(attrs, depth),
			Expr::List { elements } => {
				if elements.is_empty() {
					self.output.push_str("[]");
					return;
				}
				self.output.push_str("[\n");
				for (index, element) in elements.iter().enumerate() {
					self.indent(depth + 1);
					self.expr(&element.0, depth + 1, 0);
					if self.options.trailing_separator || index + 1 < elements.len() {
						self.output.push(',');
					}
					self.output.push('\n');
				}
				self.indent(depth);
				self.output.push(']');
			}
			Expr::AccessAttr { expr, path, or } => {
				self.expr_wrapped(&expr.0, depth, is_access_atom(&expr.0));
				self.output.push('.');
				self.attr_path(&path.0, depth);
				if let Some(or) = or {
					self.output.push_str(" ? ");
					self.expr(&or.0, depth, 0);
				}
			}
			Expr::HasAttr { expr, path } => {
				self.expr_wrapped(&expr.0, depth, is_access_atom(&expr.0));
				self.output.push_str(" ? ");
				self.attr_path(&path.0, depth);
			}
			Expr::Paren(expr) => {
				self.output.push('(');
				self.expr(&expr.0, depth, 0);
				self.output.push(')');
			}
			Expr::Ident(value) => self.output.push_str(value),
			Expr::Num(Num::Int(value)) => _ = write!(self.output, "{value}"),
			Expr::Num(Num::Float(value)) => {
				if value.fract() == 0.0 {
					_ = write!(self.output, "{value:.1}");
				} else {
					_ = write!(self.output, "{value}");
				}
			}
			Expr::Str(value) => self.raw_string(value),
		};
	}

	fn expr_wrapped(&mut self, expr: &Expr<'_>, depth: usize, atom: bool) {
		if !atom {
			self.output.push('(');
		}
		self.expr(expr, depth, 0);
		if !atom {
			self.output.push(')');
		}
	}

	fn nested_or_inline(&mut self, expr: &Expr<'_>, depth: usize) {
		if is_block_expr(expr) && !matches!(expr, Expr::Lambda(_)) {
			self.output.push('\n');
			self.indent(depth + 1);
			self.expr(expr, depth + 1, 0);
		} else {
			self.output.push(' ');
			self.expr(expr, depth, 0);
		}
	}

	fn conditional_branch(&mut self, expr: &Expr<'_>, depth: usize) {
		if matches!(expr, Expr::FuncApp { .. }) || is_block_expr(expr) {
			self.output.push('\n');
			self.indent(depth + 1);
			self.expr(expr, depth + 1, 0);
		} else {
			self.output.push(' ');
			self.expr(expr, depth, 0);
		}
	}

	fn attrset(&mut self, attrs: &[Node<Attr<'_>>], depth: usize) {
		if attrs.is_empty() {
			self.output.push_str("{}");
			return;
		}
		let all_inherited = attrs.iter().all(|attr| attr.0.value.is_none());
		if self.options.attr_separator == AttrSeparatorStyle::Smart
			&& let Some(inline) = self.inline_attrset(attrs)
			&& depth * self.options.indent_width + inline.len() <= self.options.max_inline_width
		{
			self.output.push_str(&inline);
			return;
		}
		let separator = match self.options.attr_separator {
			AttrSeparatorStyle::Smart if all_inherited => ',',
			AttrSeparatorStyle::Smart | AttrSeparatorStyle::Semicolon => ';',
			AttrSeparatorStyle::Comma => ',',
		};
		self.output.push_str("{\n");
		for (index, attr) in attrs.iter().enumerate() {
			self.indent(depth + 1);
			self.attr_path(&attr.0.path.0, depth + 1);
			if let Some(value) = &attr.0.value {
				self.output.push_str(" = ");
				self.expr(&value.0, depth + 1, 0);
			}
			if self.options.trailing_separator || index + 1 < attrs.len() {
				self.output.push(separator);
			}
			self.output.push('\n');
		}
		self.indent(depth);
		self.output.push('}');
	}

	fn inline_attrset(&self, attrs: &[Node<Attr<'_>>]) -> Option<String> {
		let mut output = String::from("{ ");
		for (index, attr) in attrs.iter().enumerate() {
			if index > 0 {
				output.push_str(", ");
			}
			output.push_str(&inline_attr_path(&attr.0.path.0)?);
			if let Some(value) = &attr.0.value {
				output.push_str(" = ");
				output.push_str(&inline_expr(&value.0)?);
			}
		}
		output.push_str(" }");
		Some(output)
	}

	fn attr_path(&mut self, path: &AttrPath<'_>, depth: usize) {
		for (index, part) in path.parts.iter().enumerate() {
			if index > 0 {
				self.output.push('.');
			}
			match &part.0 {
				AttrPathPart::Ident(value) => self.output.push_str(value),
				AttrPathPart::Str(value) => self.raw_string(value),
				AttrPathPart::Num(value) => _ = write!(self.output, "{value}"),
				AttrPathPart::Expr(expr) => {
					self.output.push_str("${");
					self.expr(expr, depth, 0);
					self.output.push('}');
				}
			};
		}
	}

	fn pattern(&mut self, pattern: &Pattern<'_>) {
		if let Some(binding) = pattern.binding {
			self.output.push_str(binding.0);
			if pattern.destruct.is_some() {
				self.output.push('@');
			}
		}
		if let Some(destruct) = &pattern.destruct {
			self.pattern_destruct(&destruct.0);
		}
		if let Some(ty) = &pattern.ty {
			self.output.push_str(" :: ");
			self.ty(&ty.0, 0);
		}
	}

	fn raw_string(&mut self, value: &str) {
		self.output.push('"');
		self.output.push_str(value);
		self.output.push('"');
	}

	fn pattern_destruct(&mut self, destruct: &PatternDestructKind<'_>) {
		match destruct {
			PatternDestructKind::AttrSet { fields, strict } => {
				self.output.push('{');
				for (index, field) in fields.iter().enumerate() {
					if index > 0 {
						self.output.push(self.options.pattern_separator.token());
						self.output.push(' ');
					}
					self.output.push_str(field.0.attr.0);
					if !is_same_binding(&field.0.pattern.0, field.0.attr.0) {
						self.output.push_str(" = ");
						self.pattern(&field.0.pattern.0);
					}
					if let Some(default) = &field.0.default {
						self.output.push_str(" ? ");
						self.expr(&default.0, 0, 0);
					}
				}
				if !strict {
					if !fields.is_empty() {
						self.output.push(self.options.pattern_separator.token());
						self.output.push(' ');
					}
					self.output.push_str("..");
				}
				self.output.push('}');
			}
			PatternDestructKind::List { elements, kind } => {
				self.output.push('[');
				if *kind == PatternListKind::TrailLeft {
					self.output.push_str(".., ");
				}
				for (index, element) in elements.iter().enumerate() {
					if index > 0 {
						self.output.push_str(", ");
					}
					self.pattern(&element.0);
				}
				if *kind == PatternListKind::TrailRight {
					if !elements.is_empty() {
						self.output.push_str(", ");
					}
					self.output.push_str("..");
				}
				self.output.push(']');
			}
		}
	}

	fn ty(&mut self, ty: &Type<'_>, precedence: u8) {
		match ty {
			Type::Named(name) => self.output.push_str(name.0),
			Type::Lambda { arg, ret } => {
				let wrapped = precedence > 0;
				if wrapped {
					self.output.push('(');
				}
				self.ty(&arg.0, 1);
				self.output.push_str(" -> ");
				self.ty(&ret.0, 0);
				if wrapped {
					self.output.push(')');
				}
			}
			Type::List(element) => {
				self.output.push('[');
				self.ty(&element.0, 0);
				self.output.push(']');
			}
			Type::Tuple(elements) => {
				self.output.push('[');
				for element in elements {
					self.ty(&element.0, 0);
					self.output.push_str(", ");
				}
				self.output.push(']');
			}
			Type::AttrSet { name, fields } => {
				if let Some(name) = name {
					self.output.push_str(name.0);
					self.output.push(' ');
				}
				self.output.push('{');
				for (index, field) in fields.iter().enumerate() {
					if index > 0 {
						self.output.push_str(", ");
					}
					self.output.push_str(field.0.name.0);
					self.output.push_str(" :: ");
					self.ty(&field.0.ty.0, 0);
				}
				self.output.push('}');
			}
			Type::Union(variants) => {
				let wrapped = precedence > 1;
				if wrapped {
					self.output.push('(');
				}
				for (index, variant) in variants.iter().enumerate() {
					if index > 0 {
						self.output.push_str(" | ");
					}
					self.ty(&variant.0, 2);
				}
				if wrapped {
					self.output.push(')');
				}
			}
		}
	}
}
