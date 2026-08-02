#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentStyle {
	Spaces,
	Tabs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeparatorStyle {
	Comma,
	Semicolon,
}

impl SeparatorStyle {
	pub(super) fn token(self) -> char {
		match self {
			Self::Comma => ',',
			Self::Semicolon => ';',
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttrSeparatorStyle {
	Smart,
	Comma,
	Semicolon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatOptions {
	pub indent_style: IndentStyle,
	pub indent_width: usize,
	pub attr_separator: AttrSeparatorStyle,
	pub let_separator: SeparatorStyle,
	pub pattern_separator: SeparatorStyle,
	pub trailing_separator: bool,
	pub final_newline: bool,
	pub max_inline_width: usize,
}

impl Default for FormatOptions {
	fn default() -> Self {
		Self {
			indent_style: IndentStyle::Spaces,
			indent_width: 2,
			attr_separator: AttrSeparatorStyle::Smart,
			let_separator: SeparatorStyle::Semicolon,
			pattern_separator: SeparatorStyle::Comma,
			trailing_separator: true,
			final_newline: true,
			max_inline_width: 80,
		}
	}
}
