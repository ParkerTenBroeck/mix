use crate::HashMap;
use std::borrow::Cow;

use crate::{
	files::{Node, Span},
	mir::{self, lowerer::MirLowerer},
	report::mir::DuplicatePatternBindingError,
};

impl MirLowerer {
	pub(crate) fn verify_lambda_pattern_bindings<'a>(&mut self, pattern: &Node<mir::Pattern<'a>>) {
		let mut seen = HashMap::default();
		self.verify_pattern_bindings(pattern, &mut seen);
	}

	pub(crate) fn verify_let_pattern_bindings<'a>(&mut self, bindings: &[mir::LetBinding<'a>]) {
		let mut seen = HashMap::default();
		for binding in bindings {
			self.verify_pattern_bindings(&binding.id, &mut seen);
		}
	}

	fn verify_pattern_bindings<'a>(
		&mut self,
		pattern: &Node<mir::Pattern<'a>>,
		seen: &mut HashMap<&'a str, Span>,
	) {
		let Node(pattern, _) = pattern;

		if let Some(Node(name, span)) = pattern.binding
			&& name != "_"
			&& let Some(first) = seen.insert(name, span)
		{
			seen.insert(name, first);
			self.reports.emit(DuplicatePatternBindingError {
				span,
				first,
				name: name.to_owned().into(),
			});
		}

		let Some(Node(destruct, _)) = &pattern.destruct else {
			return;
		};

		match destruct {
			mir::PatternDestructKind::AttrSet { fields, .. } => {
				for Node(field, _) in fields {
					self.verify_pattern_bindings(&field.pattern, seen);
				}
			}
			mir::PatternDestructKind::List { elements, .. } => {
				for element in elements {
					self.verify_pattern_bindings(element, seen);
				}
			}
		}
	}
}
