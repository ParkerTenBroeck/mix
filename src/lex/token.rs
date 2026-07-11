#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token<'a> {
	Ident(&'a str),
	Num(&'a str),
	String(&'a str),

	Comment(&'a str),

	LParen,
	RParen,
	LBrace,
	RBrace,
	LBrack,
	RBrack,

	If,
	Then,
	Else,

	Bang,
	Percent,
	SmallRArrow,
	Eq,
	Ne,
	Gt,
	Gte,
	Lt,
	Lte,
	Assign,
	Comma,
	Semicolon,
	Colon,
	ColonColon,
	Dot,
	DotDotDot,
	Question,
	At,
	PipeR,
	PipeL,

	Plus,
	Minus,
	Star,
	Slash,

	Or,
	And,
	Eof,
	Dollar,
}

impl<'a> std::fmt::Display for Token<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Token::LParen => write!(f, "'('"),
			Token::RParen => write!(f, "')'"),
			Token::LBrace => write!(f, "'{{'"),
			Token::RBrace => write!(f, "'}}'"),
			Token::LBrack => write!(f, "'['"),
			Token::RBrack => write!(f, "']'"),
			Token::Plus => write!(f, "'+'"),
			Token::Minus => write!(f, "'-'"),
			Token::Star => write!(f, "'*'"),
			Token::Slash => write!(f, "'/'"),
			Token::Semicolon => write!(f, "';'"),
			Token::And => write!(f, "'&&'"),
			Token::Or => write!(f, "'||'"),
			Token::Bang => write!(f, "!"),
			Token::Comma => write!(f, "','"),
			Token::Lt => write!(f, "'<'"),
			Token::Lte => write!(f, "'<='"),
			Token::Gt => write!(f, "'>'"),
			Token::Gte => write!(f, "'>='"),
			Token::Eq => write!(f, "'=='"),
			Token::Ne => write!(f, "'!='"),
			Token::Assign => write!(f, "'='"),
			Token::Percent => write!(f, "'%'"),
			Token::SmallRArrow => write!(f, "'->'"),
			Token::Ident(ident) => {
				if f.alternate() {
					write!(f, "'{ident}'")
				} else {
					write!(f, "ident")
				}
			}
			Token::Num(num) => {
				if f.alternate() {
					write!(f, "'{num}'")
				} else {
					write!(f, "number")
				}
			}
			Token::String(_) => write!(f, "string"),
			Token::Comment(_) => write!(f, "comment"),
			Token::If => write!(f, "if"),
			Token::Then => write!(f, "then"),
			Token::Else => write!(f, "else"),
			Token::Colon => write!(f, ":"),
			Token::ColonColon => write!(f, "::"),
			Token::Dot => write!(f, "."),
			Token::DotDotDot => write!(f, "..."),
			Token::Question => write!(f, "?"),
			Token::At => write!(f, "@"),
			Token::Dollar => write!(f, "$"),
			Token::PipeR => write!(f, "|>"),
			Token::PipeL => write!(f, "<|"),
			Token::Eof => write!(f, "eof"),
		}
	}
}
