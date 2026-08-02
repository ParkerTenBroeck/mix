use std::fmt::Write;

use crate::parse::ast::{AttrPath, AttrPathPart, BinOp, Expr, Num, Pattern, UnOp};

pub(super) fn is_same_binding(pattern: &Pattern<'_>, name: &str) -> bool {
	pattern.binding.is_some_and(|binding| binding.0 == name)
		&& pattern.ty.is_none()
		&& pattern.destruct.is_none()
}

pub(super) fn is_application_atom(expr: &Expr<'_>) -> bool {
	matches!(
		expr,
		Expr::Ident(_)
			| Expr::Paren(_)
			| Expr::AccessAttr { .. }
			| Expr::HasAttr { .. }
			| Expr::FuncApp { .. }
	)
}

pub(super) fn is_argument_atom(expr: &Expr<'_>) -> bool {
	matches!(
		expr,
		Expr::Ident(_)
			| Expr::Num(_)
			| Expr::Str(_)
			| Expr::Paren(_)
			| Expr::AttrSet { .. }
			| Expr::List { .. }
			| Expr::AccessAttr { .. }
			| Expr::HasAttr { .. }
	)
}

pub(super) fn is_unary_atom(expr: &Expr<'_>) -> bool {
	matches!(
		expr,
		Expr::Ident(_) | Expr::Num(_) | Expr::AccessAttr { .. } | Expr::Paren(_)
	)
}

pub(super) fn is_access_atom(expr: &Expr<'_>) -> bool {
	matches!(
		expr,
		Expr::Ident(_)
			| Expr::Paren(_)
			| Expr::AttrSet { .. }
			| Expr::List { .. }
			| Expr::AccessAttr { .. }
	)
}

pub(super) fn inline_attr_path(path: &AttrPath<'_>) -> Option<String> {
	let mut output = String::new();
	for (index, part) in path.parts.iter().enumerate() {
		if index > 0 {
			output.push('.');
		}
		match &part.0 {
			AttrPathPart::Ident(value) => output.push_str(value),
			AttrPathPart::Str(value) => {
				output.push('"');
				output.push_str(value);
				output.push('"');
			}
			AttrPathPart::Num(value) => _ = write!(output, "{value}"),
			AttrPathPart::Expr(expr) => {
				output.push_str("${");
				output.push_str(&inline_expr(expr)?);
				output.push('}');
			}
		}
	}
	Some(output)
}

pub(super) fn inline_expr(expr: &Expr<'_>) -> Option<String> {
	inline_expr_prec(expr, 0)
}

fn inline_expr_prec(expr: &Expr<'_>, parent_precedence: u32) -> Option<String> {
	Some(match expr {
		Expr::Ident(value) => (*value).into(),
		Expr::Num(Num::Int(value)) => value.to_string(),
		Expr::Num(Num::Float(value)) if value.fract() == 0.0 => format!("{value:.1}"),
		Expr::Num(Num::Float(value)) => value.to_string(),
		Expr::Str(value) => format!("\"{value}\""),
		Expr::Paren(expr) => format!("({})", inline_expr(&expr.0)?),
		Expr::UnOp { expr, op } => {
			let op = match op.0 {
				UnOp::Neg => "-",
				UnOp::Not => "!",
			};
			format!("{op}{}", inline_expr(&expr.0)?)
		}
		Expr::BinOp { lhs, op, rhs } => {
			let precedence = op.0.precedence();
			let value = format!(
				"{} {} {}",
				inline_expr_prec(&lhs.0, precedence)?,
				binop(op.0),
				inline_expr_prec(&rhs.0, precedence + 1)?
			);
			if precedence < parent_precedence {
				format!("({value})")
			} else {
				value
			}
		}
		Expr::FuncApp { func, arg } => {
			let formatted_func = inline_expr(&func.0)?;
			let formatted_arg = inline_expr(&arg.0)?;
			let formatted_func = if is_application_atom(&func.0) {
				formatted_func
			} else {
				format!("({formatted_func})")
			};
			let formatted_arg = if is_argument_atom(&arg.0) {
				formatted_arg
			} else {
				format!("({formatted_arg})")
			};
			format!("{formatted_func} {formatted_arg}")
		}
		Expr::AccessAttr { expr, path, or } => {
			let formatted_expr = inline_expr(&expr.0)?;
			let formatted_expr = if is_access_atom(&expr.0) {
				formatted_expr
			} else {
				format!("({formatted_expr})")
			};
			let mut value = format!("{formatted_expr}.{}", inline_attr_path(&path.0)?);
			if let Some(or) = or {
				write!(value, " ? {}", inline_expr(&or.0)?).ok()?;
			}
			value
		}
		Expr::HasAttr { expr, path } => {
			format!("{} ? {}", inline_expr(&expr.0)?, inline_attr_path(&path.0)?)
		}
		Expr::AttrSet { .. } => return None,
		Expr::List { elements } => {
			let values = elements
				.iter()
				.map(|element| inline_expr(&element.0))
				.collect::<Option<Vec<_>>>()?;
			format!("[{}]", values.join(", "))
		}
		Expr::Lambda(_) | Expr::IfThenElse { .. } | Expr::Let { .. } => return None,
	})
}

pub(super) fn is_block_expr(expr: &Expr<'_>) -> bool {
	match expr {
		Expr::Let { .. } | Expr::IfThenElse { .. } => true,
		Expr::Lambda(lambda) => is_block_expr(&lambda.body.0),
		Expr::AttrSet { attrs } => {
			attrs.len() > 1
				|| attrs.iter().any(|attr| {
					attr.0
						.value
						.as_ref()
						.is_some_and(|value| is_block_expr(&value.0))
				})
		}
		Expr::List { elements } => elements.len() > 3,
		_ => false,
	}
}

pub(super) fn binop(op: BinOp) -> &'static str {
	match op {
		BinOp::Rem => "%",
		BinOp::Div => "/",
		BinOp::Mul => "*",
		BinOp::Sub => "-",
		BinOp::Add => "+",
		BinOp::Lt => "<",
		BinOp::Lte => "<=",
		BinOp::Gt => ">",
		BinOp::Gte => ">=",
		BinOp::Eq => "==",
		BinOp::Ne => "!=",
		BinOp::And => "&&",
		BinOp::Or => "||",
		BinOp::PipeL => "<|",
		BinOp::PipeR => "|>",
		BinOp::LogImp => "->",
	}
}
