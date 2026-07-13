use std::borrow::Cow;

use crate::{
	files::Span,
	report::{Report, ReportAnnotation, ReportLevel},
};

#[derive(Clone, Debug)]
pub struct DuplicateAttrError<'a> {
	pub span: Span,
	pub first: Span,
	pub name: Cow<'a, str>,
}

impl<'a> From<DuplicateAttrError<'a>> for Report<'a> {
	fn from(err: DuplicateAttrError<'a>) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Owned(format!("duplicate attribute {}", err.name)),
			annotations: vec![
				ReportAnnotation::primary(err.span),
				ReportAnnotation::context(err.first, "first defined here"),
			],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug)]
pub struct DuplicatePatternBindingError<'a> {
	pub span: Span,
	pub first: Span,
	pub name: Cow<'a, str>,
}

impl<'a> From<DuplicatePatternBindingError<'a>> for Report<'a> {
	fn from(err: DuplicatePatternBindingError<'a>) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Owned(format!("duplicate bound attribute {}", err.name)),
			annotations: vec![
				ReportAnnotation::primary(err.span),
				ReportAnnotation::context(err.first, "first bound here"),
			],
			helps: vec![],
		}
	}
}
