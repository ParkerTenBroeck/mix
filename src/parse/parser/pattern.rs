use crate::{
	files::Node,
	lex::Token,
	parse::{Delim, Parser},
	report::parser::*,
};

use super::ast;

impl<'a> Parser<'a> {
	fn parse_attr_destruct(&mut self) -> ast::PatternDestructKind<'a> {
		let start = self.curr.1;
		if !self.consume_if(Token::LBrace) {
			todo!()
		}
		let mut fields = Vec::new();
		let mut strict = true;
		let mut strict_span = None;
		loop {
			if matches!(
				self.curr.0,
				Token::RParen | Token::RBrace | Token::RBrack | Token::Eof
			) {
				break;
			}
			if self.consume_if(Token::DotDot) {
				let span = self.last.1;
				if let Some(first) = strict_span {
					self.reports.emit(DuplicatePatternRest { span, first });
				} else {
					strict_span = Some(span);
				}
				strict = false;
				_ = self.consume_if(Token::Comma) || self.consume_if(Token::Semicolon);
				if !matches!(
					self.curr.0,
					Token::RParen | Token::RBrace | Token::RBrack | Token::Eof
				) {
					self.reports.emit(NonTrailingPatternRestWarning { span });
				}
				continue;
			}
			let start = self.curr.1;
			let attr = self.parse_ident();
			let pattern = if self.consume_if(Token::Assign) {
				self.parse_pattern()
			} else {
				let pattern = ast::Pattern {
					binding: Some(attr),
					ty: self
						.consume_if(Token::ColonColon)
						.then(|| self.parse_type()),
					destruct: None,
				};
				Node(pattern, start.merge(self.last.1))
			};
			let field = ast::AttrPattern { attr, pattern };
			fields.push(Node(field, start.merge(self.last.1)));
			if !(self.consume_if(Token::Comma) || self.consume_if(Token::Semicolon)) {
				break;
			}
		}

		self.close_delim(Node(Delim::Brace, start));
		ast::PatternDestructKind::AttrSet { fields, strict }
	}

	fn parse_list_destruct(&mut self) -> ast::PatternDestructKind<'a> {
		let start = self.curr.1;
		if !self.consume_if(Token::LBrack) {
			todo!()
		}
		let mut elements = Vec::new();
		let mut kind = ast::PatternListKind::Strict;
		let mut trail_span = None;

		loop {
			if matches!(
				self.curr.0,
				Token::RParen | Token::RBrace | Token::RBrack | Token::Eof
			) {
				break;
			}
			if self.consume_if(Token::DotDot) {
				let span = self.last.1;
				if let Some(first) = trail_span {
					self.reports.emit(DuplicatePatternRest { span, first });
				} else {
					trail_span = Some(span);
					kind = if elements.is_empty() {
						ast::PatternListKind::TrailLeft
					} else {
						ast::PatternListKind::TrailRight
					};
				}
			} else {
				if kind == ast::PatternListKind::TrailRight {
					self.reports.emit(NonTrailingListTrail {
						span: trail_span.unwrap(),
					});
				}
				elements.push(self.parse_pattern());
			}
			if !(self.consume_if(Token::Comma)) {
				break;
			}
		}

		self.close_delim(Node(Delim::Brack, start));
		ast::PatternDestructKind::List { elements, kind }
	}

	fn parse_pattern_kind(&mut self) -> Node<ast::PatternDestructKind<'a>> {
		let start = self.curr.1;

		let kind = match self.curr.0 {
			Token::LBrace => self.parse_attr_destruct(),
			Token::LBrack => self.parse_list_destruct(),
			got => {
				self.reports.emit(UnexpectedTokenPattern {
					span: self.curr.1,
					got,
				});
				ast::PatternDestructKind::AttrSet {
					fields: Default::default(),
					strict: false,
				}
			}
		};

		let end = self.last.1;

		Node(kind, start.merge(end))
	}

	pub(super) fn parse_pattern(&mut self) -> Node<ast::Pattern<'a>> {
		let start = self.curr.1;

		let binding = self.try_parse_ident();

		let kind =
			(binding.is_none() || self.consume_if(Token::At)).then(|| self.parse_pattern_kind());
		let ty = self
			.consume_if(Token::ColonColon)
			.then(|| self.parse_type());

		let pattern = ast::Pattern {
			binding,
			destruct: kind,
			ty,
		};

		let end = self.last.1;

		Node(pattern, start.merge(end))
	}
}
