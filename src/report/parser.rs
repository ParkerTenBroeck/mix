use std::borrow::Cow;

use crate::{
	files::{Node, Span},
	lex::Token,
	parse::Delim,
	report::ReportPatch,
};

use super::{Report, ReportAnnotation, ReportHelp, ReportLevel};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnclosedDelimError {
	pub span: Span,
	pub opening: Node<Delim>,
	pub closing: Node<Delim>,
}
impl From<UnclosedDelimError> for Report {
	fn from(err: UnclosedDelimError) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Borrowed("unclosed delimiter"),
			annotations: vec![
				ReportAnnotation::primary(err.closing.1),
				ReportAnnotation::context(err.opening.1, "opened here"),
			],
			helps: vec![
				ReportHelp::new("consider closing here")
					.with_patch(err.closing.1, err.opening.0.closing()),
			],
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MismatchedDelimError {
	pub span: Span,
	pub opening: Node<Delim>,
	pub closing: Node<Delim>,
}
impl From<MismatchedDelimError> for Report {
	fn from(err: MismatchedDelimError) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Borrowed("mismatched delimiter"),
			annotations: vec![
				ReportAnnotation::primary(err.closing.1),
				ReportAnnotation::context(err.opening.1, "opened here"),
			],
			helps: vec![
				ReportHelp::new("use correct delimiter")
					.with_patch(err.closing.1, err.opening.0.closing()),
			],
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedClosingDelimError<'a> {
	pub span: Span,
	pub delim: Delim,
	pub token: Token<'a>,
}
impl<'a> From<ExpectedClosingDelimError<'a>> for Report {
	fn from(err: ExpectedClosingDelimError<'a>) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Owned(format!(
				"expected closing delim '{}' but got {:#}",
				err.delim.closing(),
				err.token
			)),
			annotations: vec![ReportAnnotation::primary(err.span.before())],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnexpectedTokenExprError<'a> {
	pub span: Span,
	pub token: Token<'a>,
	pub expected: Option<Token<'a>>,
}
impl<'a> From<UnexpectedTokenExprError<'a>> for Report {
	fn from(err: UnexpectedTokenExprError<'a>) -> Self {
		if let Some(expected) = err.expected {
			Self {
				level: ReportLevel::Error,
				span: err.span,
				title: Cow::Owned(format!(
					"unexpected token in expr {:#} expected {expected:#}",
					err.token
				)),
				annotations: vec![ReportAnnotation::primary(err.span)],
				helps: vec![ReportHelp {
					title: "consider adding here".into(),
					patches: vec![ReportPatch {
						span: err.span.before(),
						replacement: format!(" {expected:#} ").into(),
					}],
				}],
			}
		} else {
			Self {
				level: ReportLevel::Error,
				span: err.span,
				title: Cow::Owned(format!("unexpected token in expr {:#}", err.token)),
				annotations: vec![ReportAnnotation::primary(err.span)],
				helps: vec![],
			}
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnexpectedTokenAttrPathError<'a> {
	pub span: Span,
	pub token: Token<'a>,
}
impl<'a> From<UnexpectedTokenAttrPathError<'a>> for Report {
	fn from(err: UnexpectedTokenAttrPathError<'a>) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Owned(format!(
				"expected token in attr path but got {:#}",
				err.token
			)),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncAppInListError {
	pub span: Span,
	pub func: Span,
}
impl From<FuncAppInListError> for Report {
	fn from(err: FuncAppInListError) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Borrowed("function application in list"),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![
				ReportHelp::new("consider wrapping in parenthesis if intended")
					.with_patch(err.span.before(), "(")
					.with_patch(err.span.after(), ")"),
				ReportHelp::new("or add comma if not").with_patch(err.func.after(), ","),
			],
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncDefInListError {
	pub span: Span,
}
impl From<FuncDefInListError> for Report {
	fn from(err: FuncDefInListError) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Borrowed("function declaration in list"),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![
				ReportHelp::new("consider wrapping in parenthesis")
					.with_patch(err.span.before(), "(")
					.with_patch(err.span.after(), ")"),
			],
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedEofError<'a> {
	pub span: Span,
	pub token: Token<'a>,
}
impl<'a> From<ExpectedEofError<'a>> for Report {
	fn from(err: ExpectedEofError<'a>) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Owned(format!("expected eof but got token {:#}", err.token)),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug)]
pub struct FloatError {
	pub span: Span,
	pub err: std::num::ParseFloatError,
}
impl From<FloatError> for Report {
	fn from(err: FloatError) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Owned(err.err.to_string()),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug)]
pub struct IntError {
	pub span: Span,
	pub err: std::num::ParseIntError,
}
impl From<IntError> for Report {
	fn from(err: IntError) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Owned(err.err.to_string()),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug)]
pub struct UnexpectedTokenPattern<'a> {
	pub span: Span,
	pub got: Token<'a>,
}
impl<'a> From<UnexpectedTokenPattern<'a>> for Report {
	fn from(err: UnexpectedTokenPattern) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: format!("expected '{{' or '[' in pattern got {:#}", err.got).into(),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug)]
pub struct ExpectedIdent<'a> {
	pub span: Span,
	pub got: Token<'a>,
}
impl<'a> From<ExpectedIdent<'a>> for Report {
	fn from(err: ExpectedIdent) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: format!("expected ident got {:#}", err.got).into(),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug)]
pub struct DuplicatePatternRest {
	pub span: Span,
	pub first: Span,
}
impl From<DuplicatePatternRest> for Report {
	fn from(err: DuplicatePatternRest) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Borrowed("duplicate .. in pattern"),
			annotations: vec![
				ReportAnnotation::primary(err.span),
				ReportAnnotation::context(err.first, "first used here"),
			],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug)]
pub struct NonTrailingPatternRestWarning {
	pub span: Span,
}
impl From<NonTrailingPatternRestWarning> for Report {
	fn from(err: NonTrailingPatternRestWarning) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Borrowed(".. in pattern should be trailing"),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![],
		}
	}
}

#[derive(Clone, Debug)]
pub struct NonTrailingListTrail {
	pub span: Span,
}
impl From<NonTrailingListTrail> for Report {
	fn from(err: NonTrailingListTrail) -> Self {
		Self {
			level: ReportLevel::Error,
			span: err.span,
			title: Cow::Borrowed(".. cannot appear in middle of list"),
			annotations: vec![ReportAnnotation::primary(err.span)],
			helps: vec![],
		}
	}
}
