use crate::{
	files::Node,
	parse::{Parser, ast},
};

impl<'a> Parser<'a> {
	pub(super) fn parse_type(&mut self) -> Node<ast::Type<'a>> {
		let start = self.curr.1;

		let name = self.parse_ident();

		let ty = ast::Type { name };

		let end = self.last.1;
		Node(ty, start.merge(end))
	}
}
