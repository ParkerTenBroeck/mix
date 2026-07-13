mod expr;
mod pattern;
mod token;
mod ty;
pub use token::*;

use std::ops::Not;

use crate::{
	files::{FileId, Node, Span},
	lex::{Lexer, Token},
	report::{Reports, parser::*},
};

use super::ast;

pub struct Parser<'a> {
	fid: FileId,
	lex: Lexer<'a>,
	reports: Reports<'a>,
	last: Node<Token<'a>>,
	curr: Node<Token<'a>>,
}

struct State<'a> {
	lex: Lexer<'a>,
	reports: usize,
	last: Node<Token<'a>>,
	curr: Node<Token<'a>>,
}

pub type ParserResult<'a> = (Result<Node<ast::Expr<'a>>, ()>, Reports<'a>);

impl<'a> Parser<'a> {
	pub fn parse(str: &'a str, fid: FileId) -> ParserResult<'a> {
		let mut parser = Self {
			fid,
			lex: Lexer::new(str),
			reports: Default::default(),
			curr: Node(Token::Eof, Span::new(Default::default(), fid)),
			last: Node(Token::Eof, Span::new(Default::default(), fid)),
		};
		_ = parser.next();

		let expr = parser.parse_expr();
		if parser.curr.0 != Token::Eof {
			parser.reports.emit(ExpectedEofError {
				span: parser.curr.1,
				token: parser.curr.0,
			});
		}

		(
			parser.reports.has_errors().not().then_some(expr).ok_or(()),
			parser.reports,
		)
	}

	fn state(&self) -> State<'a> {
		State {
			lex: self.lex,
			reports: self.reports.state(),
			last: self.last,
			curr: self.curr,
		}
	}

	fn restore(&mut self, state: State<'a>) {
		self.curr = state.curr;
		self.last = state.last;
		self.lex = state.lex;
		self.reports.restore(state.reports);
	}

	fn next(&mut self) -> Token<'a> {
		self.last = self.curr;
		loop {
			let (tok, range) = self.lex.next_tok();
			let span = Span::new(range, self.fid);
			match tok {
				Err(e) => self.reports.emit(Node(e, span)),
				Ok(Token::Comment(_)) => {}
				Ok(tok) => {
					self.curr = Node(tok, span);
					break;
				}
			}
		}
		self.last.0
	}

	fn close_delim(&mut self, opening: Node<Delim>) {
		let mut level = 0usize;
		let mut error = false;
		loop {
			match self.next() {
				Token::LBrace | Token::LBrack | Token::LParen => level += 1,
				Token::RBrace | Token::RBrack | Token::RParen => {
					if level == 0 {
						let closing = Node(
							match self.last.0 {
								Token::RParen => Delim::Paren,
								Token::RBrack => Delim::Brack,
								Token::RBrace => Delim::Brace,
								_ => unreachable!(),
							},
							self.last.1,
						);
						if opening.0 != closing.0 {
							self.reports.emit(MismatchedDelimError {
								span: self.last.1,
								opening,
								closing,
							})
						}
						return;
					} else {
						level -= 1;
					}
				}
				Token::Eof => {
					let closing = Node(opening.0, self.last.1);
					self.reports.emit(UnclosedDelimError {
						span: self.last.1,
						opening,
						closing,
					});
					break;
				}
				token => {
					if !error {
						self.reports.emit(ExpectedClosingDelimError {
							span: self.last.1,
							delim: opening.0,
							token,
						});
						error = true;
					}
				}
			}
		}
	}

	fn consume_if(&mut self, token: Token) -> bool {
		if self.curr.0 == token {
			self.next();
			true
		} else {
			false
		}
	}

	fn try_parse_ident(&mut self) -> Option<Node<&'a str>> {
		match self.curr.0 {
			Token::Ident(ident) => {
				self.next();
				Some(Node(ident, self.last.1))
			}
			_ => None,
		}
	}

	fn parse_ident(&mut self) -> Node<&'a str> {
		match self.curr.0 {
			Token::Ident(ident) => {
				self.next();
				Node(ident, self.last.1)
			}
			_ => {
				self.reports.emit(ExpectedIdent {
					span: self.curr.1,
					got: self.curr.0,
				});
				Node("<ERROR>", self.last.1)
			}
		}
	}
}
