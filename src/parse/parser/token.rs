use crate::lex::Token;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delim {
	Paren,
	Brack,
	Brace,
}

impl Delim {
	pub fn closing(&self) -> &'static str {
		match self {
			Delim::Paren => ")",
			Delim::Brack => "]",
			Delim::Brace => "}",
		}
	}
}

impl<'a> Token<'a> {
	fn starts_expr(&self) -> bool {
		match self {
			Token::Ident(_) => true,
			Token::Num(_) => true,
			Token::String(_) => true,
			Token::LParen => true,
			Token::LBrace => true,
			Token::LBrack => true,
			Token::Bang => true,
			Token::Minus => true,
			Token::If => true,

			Token::Comment(_) => false,
			Token::RParen => false,
			Token::RBrace => false,
			Token::RBrack => false,
			Token::Percent => false,
			Token::SmallRArrow => false,
			Token::Eq => false,
			Token::Ne => false,
			Token::Gt => false,
			Token::Gte => false,
			Token::Lt => false,
			Token::Lte => false,
			Token::Assign => false,
			Token::Comma => false,
			Token::Semicolon => false,
			Token::Colon => false,
			Token::Dot => false,
			Token::DotDot => false,
			Token::Question => false,
			Token::At => false,
			Token::PipeR => false,
			Token::PipeL => false,
			Token::Plus => false,
			Token::Star => false,
			Token::Slash => false,
			Token::Or => false,
			Token::And => false,
			Token::Eof => false,
			Token::Then => false,
			Token::Else => false,
			Token::Dollar => false,
			Token::ColonColon => false,
		}
	}

	pub(super) fn start_fn_arg(&self) -> bool {
		match self {
			Token::Ident(_) => true,
			Token::Num(_) => true,
			Token::String(_) => true,
			Token::LParen => true,
			Token::LBrace => true,
			Token::LBrack => true,
			Token::If => true,

			Token::Comment(_) => false,
			Token::RParen => false,
			Token::RBrace => false,
			Token::RBrack => false,
			Token::Percent => false,
			Token::SmallRArrow => false,
			Token::Eq => false,
			Token::Ne => false,
			Token::Gt => false,
			Token::Gte => false,
			Token::Lt => false,
			Token::Lte => false,
			Token::Assign => false,
			Token::Comma => false,
			Token::Semicolon => false,
			Token::Colon => false,
			Token::Dot => false,
			Token::DotDot => false,
			Token::Question => false,
			Token::At => false,
			Token::PipeR => false,
			Token::PipeL => false,
			Token::Plus => false,
			Token::Star => false,
			Token::Slash => false,
			Token::Or => false,
			Token::And => false,
			Token::Eof => false,
			Token::Then => false,
			Token::Else => false,
			Token::Bang => false,
			Token::Minus => false,
			Token::Dollar => false,
			Token::ColonColon => false,
		}
	}
}
