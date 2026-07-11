use std::borrow::Cow;

use crate::{
	files::Node,
	lex::LexError,
	report::{Report, ReportAnnotation, ReportLevel},
};

impl From<Node<LexError>> for Report<'_> {
	fn from(Node(err, span): Node<LexError>) -> Self {
		let title = match err {
			LexError::UnexpectedChar(char) => format!("unexpected char {char:?}"),
			LexError::UnclosedComment => "unclosed comment".to_string(),
			LexError::UnclosedString => "unclosed string literal".to_string(),
			LexError::NumberError => "number cannot contain more than one decimal".to_string(),
		};

		Self {
			level: ReportLevel::Error,
			span,
			title: Cow::Owned(title),
			annotations: vec![ReportAnnotation::primary(span)],
			helps: vec![],
		}
	}
}
