use std::borrow::Cow;

use crate::{
	files::{Node, Span},
	mir::{self, lowerer::MirLowerer},
	parse::ast,
	report::mir::DuplicateAttrError,
};

#[derive(Clone, Debug)]
pub(crate) struct StaticAttrBuilder<'a> {
	pub(crate) name: Node<&'a str>,
	pub(crate) full_span: Span,
	pub(crate) value: Option<Node<mir::Expr<'a>>>,
	pub(crate) children: Vec<StaticAttrBuilder<'a>>,
}

impl MirLowerer {
	pub(crate) fn static_attr_parts<'a>(
		&self,
		path: &Node<ast::AttrPath<'a>>,
	) -> Option<Vec<Node<&'a str>>> {
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

	pub(crate) fn insert_static_attr<'a>(
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
}
