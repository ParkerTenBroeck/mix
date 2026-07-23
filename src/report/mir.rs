use std::borrow::Cow;

use crate::{
	files::Span,
	report::{Report, ReportAnnotation, ReportLevel},
};

#[derive(Clone, Debug)]
pub struct DuplicateAttrError {
	pub span: Span,
	pub first: Span,
	pub name: Cow<'static, str>,
}

impl From<DuplicateAttrError> for Report {
	fn from(err: DuplicateAttrError) -> Self {
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
pub struct DuplicatePatternBindingError {
	pub span: Span,
	pub first: Span,
	pub name: Cow<'static, str>,
}

impl From<DuplicatePatternBindingError> for Report {
	fn from(err: DuplicatePatternBindingError) -> Self {
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
