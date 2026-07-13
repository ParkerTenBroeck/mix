use std::range::Range;

use super::Token;

#[derive(Clone, Copy, Debug)]
pub struct Lexer<'a> {
	str: &'a str,
	pos: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexError {
	UnexpectedChar(char),
	UnclosedComment,
	UnclosedString,
	NumberError,
}

pub type Node<T> = (T, Range<usize>);

fn ident_start(c: char) -> bool {
	matches!(c, 'a'..='z'|'A'..='Z'|'_')
}

fn ident_continue(c: char) -> bool {
	matches!(c, 'a'..='z'|'A'..='Z'|'_'|'0'..='9'|'\'')
}

impl<'a> Lexer<'a> {
	pub fn new(str: &'a str) -> Self {
		Self { str, pos: 0 }
	}

	fn peek_char(&self) -> Option<char> {
		self.str.get(self.pos..)?.chars().next()
	}

	fn next_char(&mut self) -> Option<char> {
		let char = self.peek_char()?;
		self.pos += char.len_utf8();
		Some(char)
	}

	pub fn next_tok(&mut self) -> Node<Result<Token<'a>, LexError>> {
		self.pos += if let Some(str) = self.str.get(self.pos..) {
			str.len() - str.trim_start().len()
		} else {
			0
		};
		let start = self.pos;

		let token = match self.next_char() {
			None => Ok(Token::Eof),

			Some('(') => Ok(Token::LParen),
			Some(')') => Ok(Token::RParen),
			Some('{') => Ok(Token::LBrace),
			Some('}') => Ok(Token::RBrace),
			Some('[') => Ok(Token::LBrack),
			Some(']') => Ok(Token::RBrack),
			Some('<') => match self.peek_char() {
				Some('|') => {
					self.next_char();
					Ok(Token::PipeL)
				}
				Some('=') => {
					self.next_char();
					Ok(Token::Lte)
				}
				_ => Ok(Token::Lt),
			},
			Some('>') => match self.peek_char() {
				Some('=') => {
					self.next_char();
					Ok(Token::Gte)
				}
				_ => Ok(Token::Gt),
			},

			Some('!') => match self.peek_char() {
				Some('=') => {
					self.next_char();
					Ok(Token::Ne)
				}
				_ => Ok(Token::Bang),
			},
			Some('=') => match self.peek_char() {
				Some('=') => {
					self.next_char();
					Ok(Token::Eq)
				}
				_ => Ok(Token::Assign),
			},
			Some('+') => Ok(Token::Plus),
			Some('-') => match self.peek_char() {
				Some('>') => {
					self.next_char();
					Ok(Token::SmallRArrow)
				}
				_ => Ok(Token::Minus),
			},
			Some('*') => Ok(Token::Star),
			Some('/') => match self.peek_char() {
				Some('*') => {
					self.next_char();
					let mut star = false;
					loop {
						match self.next_char() {
							Some('*') => star = true,
							Some('/') if star => {
								break Ok(Token::Comment(&self.str[start + 2..self.pos - 2]));
							}
							None => break Err(LexError::UnclosedComment),
							_ => star = false,
						}
					}
				}
				_ => Ok(Token::Slash),
			},
			Some('%') => Ok(Token::Percent),
			Some(';') => Ok(Token::Semicolon),
			Some(':') => match self.peek_char() {
				Some(':') => {
					self.next_char();
					Ok(Token::ColonColon)
				}
				_ => Ok(Token::Colon),
			},
			Some('?') => Ok(Token::Question),
			Some('@') => Ok(Token::At),
			Some('$') => Ok(Token::Dollar),
			Some(',') => Ok(Token::Comma),

			Some('.') => match self.peek_char() {
				Some('.') => {
					self.next_char();
					Ok(Token::DotDot)
				}
				_ => Ok(Token::Dot),
			},
			Some('|') => match self.peek_char() {
				Some('|') => {
					self.next_char();
					Ok(Token::Or)
				}
				Some('>') => {
					self.next_char();
					Ok(Token::PipeR)
				}
				_ => Err(LexError::UnexpectedChar('|')),
			},
			Some('&') => match self.peek_char() {
				Some('&') => {
					self.next_char();
					Ok(Token::And)
				}
				_ => Err(LexError::UnexpectedChar('&')),
			},

			Some('"') => {
				loop {
					match self.next_char() {
						None | Some('"') => break,
						Some('\\') => _ = self.next_char(),
						_ => {}
					}
				}
				Ok(Token::String(&self.str[start + 1..self.pos - 1]))
			}

			Some(c) if ident_start(c) => {
				while self.peek_char().map(ident_continue).unwrap_or(false) {
					self.next_char();
				}
				let str = &self.str[start..self.pos];
				Ok(match str {
					"if" => Token::If,
					"then" => Token::Then,
					"else" => Token::Else,
					_ => Token::Ident(str),
				})
			}
			Some('#') => {
				while self.peek_char().map(|c| c != '\n').unwrap_or(false) {
					self.next_char();
				}
				let comment = &self.str[start + 1..self.pos];
				Ok(Token::Comment(comment))
			}
			Some('0'..='9') => {
				while let Some('0'..='9' | '_') = self.peek_char() {
					_ = self.next_tok()
				}
				Ok(Token::Num(&self.str[start..self.pos]))
			}

			Some(char) => Err(LexError::UnexpectedChar(char)),
		};
		let end = self.pos;
		(token, Range { start, end })
	}
}
