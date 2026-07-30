use crate::{
	files::Node,
	lex::Token,
	parse::{Delim, Parser, ast},
	report::parser::UnexpectedTokenExprError,
};

impl<'a> Parser<'a> {
	pub(super) fn parse_type(&mut self) -> Node<ast::Type<'a>> {
		let arg = self.parse_union_type();

		if !self.consume_if(Token::SmallRArrow) {
			return arg;
		}

		let ret = self.parse_type();
		let span = arg.1.merge(ret.1);
		Node(
			ast::Type::Lambda {
				arg: Box::new(arg),
				ret: Box::new(ret),
			},
			span,
		)
	}

	fn parse_union_type(&mut self) -> Node<ast::Type<'a>> {
		let first = self.parse_type_atom();
		if !self.consume_if(Token::Bar) {
			return first;
		}

		let start = first.1;
		let mut variants = vec![first];
		loop {
			variants.push(self.parse_type_atom());
			if !self.consume_if(Token::Bar) {
				break;
			}
		}
		Node(ast::Type::Union(variants), start.merge(self.last.1))
	}

	fn parse_type_atom(&mut self) -> Node<ast::Type<'a>> {
		let start = self.curr.1;

		let ty = if self.consume_if(Token::LParen) {
			let opening = Node(Delim::Paren, self.last.1);
			let ty = self.parse_type();
			self.close_delim(opening);
			ty.0
		} else if self.consume_if(Token::LBrack) {
			self.parse_bracket_type(Node(Delim::Brack, self.last.1))
		} else if self.consume_if(Token::LBrace) {
			self.parse_attrset_type(Node(Delim::Brace, self.last.1), None)
		} else {
			let name = self.parse_ident();
			if self.consume_if(Token::LBrace) {
				self.parse_attrset_type(Node(Delim::Brace, self.last.1), Some(name))
			} else {
				ast::Type::Named(name)
			}
		};

		let end = self.last.1;
		Node(ty, start.merge(end))
	}

	fn parse_bracket_type(&mut self, opening: Node<Delim>) -> ast::Type<'a> {
		let first = self.parse_type();
		if !self.consume_if(Token::Comma) {
			self.close_delim(opening);
			return ast::Type::List(Box::new(first));
		}

		let mut elements = vec![first];
		while self.curr.0 != Token::RBrack && self.curr.0 != Token::Eof {
			elements.push(self.parse_type());
			if !self.consume_if(Token::Comma) {
				break;
			}
		}
		self.close_delim(opening);
		ast::Type::Tuple(elements)
	}

	fn parse_attrset_type(
		&mut self,
		opening: Node<Delim>,
		name: Option<Node<&'a str>>,
	) -> ast::Type<'a> {
		let mut fields = Vec::new();
		while self.curr.0 != Token::RBrace && self.curr.0 != Token::Eof {
			let start = self.curr.1;
			let name = self.parse_ident();
			if !self.consume_if(Token::ColonColon) {
				self.reports.emit(UnexpectedTokenExprError {
					span: self.curr.1,
					token: self.curr.0,
					expected: Some(Token::ColonColon),
				});
			}
			let ty = self.parse_type();
			let span = start.merge(ty.1);
			fields.push(Node(ast::TypeAttr { name, ty }, span));

			if !self.consume_if(Token::Comma) {
				break;
			}
		}
		self.close_delim(opening);
		ast::Type::AttrSet { name, fields }
	}
}

#[cfg(test)]
mod tests {
	use std::rc::Rc;

	use crate::{
		files::{FileLoader, Node},
		mir::lowerer::MirLowerer,
		parse::{Parser, ast},
	};

	#[test]
	fn function_types_are_right_associative() {
		let source: Rc<String> = Rc::new("x :: A -> B -> C: x".into());
		let loader = FileLoader::new(move |_| Ok(source.clone()));
		let (source, fid) = loader.load("test.mix".as_ref()).unwrap();
		let (expr, reports) = Parser::parse(&source, fid);
		assert!(!reports.has_errors());

		let expr = expr.unwrap();
		let ast::Expr::Lambda(lambda) = expr.0 else {
			panic!("expected lambda expression");
		};
		let ty = lambda.arg.0.ty.unwrap();
		let ast::Type::Lambda { arg, ret } = ty.0 else {
			panic!("expected function type");
		};
		assert!(matches!(arg.0, ast::Type::Named(Node("A", _))));
		assert!(matches!(
			ret.0,
			ast::Type::Lambda {
				arg,
				ret
			} if matches!(arg.0, ast::Type::Named(Node("B", _)))
				&& matches!(ret.0, ast::Type::Named(Node("C", _)))
		));
	}

	#[test]
	fn bracket_types_distinguish_lists_and_tuples() {
		for (source, tuple) in [
			("x :: [A]: x", false),
			("x :: [A,]: x", true),
			("x :: [A, B]: x", true),
			("x :: [A, B,]: x", true),
		] {
			let loaded: Rc<String> = Rc::new(source.into());
			let loader = FileLoader::new({
				let loaded = loaded.clone();
				move |_| Ok(loaded.clone())
			});
			let (loaded, fid) = loader.load("test.mix".as_ref()).unwrap();
			let (expr, reports) = Parser::parse(&loaded, fid);
			assert!(!reports.has_errors());
			let ast::Expr::Lambda(lambda) = expr.unwrap().0 else {
				panic!("expected lambda expression");
			};
			let ty = lambda.arg.0.ty.unwrap().0;

			if tuple {
				assert!(matches!(ty, ast::Type::Tuple(_)));
			} else {
				assert!(matches!(
					ty,
					ast::Type::List(element)
						if matches!(element.0, ast::Type::Named(Node("A", _)))
				));
			}
		}
	}

	#[test]
	fn let_bindings_accept_type_annotations() {
		let source: Rc<String> = Rc::new("let f :: [A, B] -> [C] = x: x in f".into());
		let loader = FileLoader::new(move |_| Ok(source.clone()));
		let (source, fid) = loader.load("test.mix".as_ref()).unwrap();
		let (expr, reports) = Parser::parse(&source, fid);
		assert!(!reports.has_errors());

		let (expr, reports) = MirLowerer::new(reports).lower(expr.unwrap());
		assert!(!reports.has_errors());
		assert!(expr.is_ok());
	}

	#[test]
	fn attrset_types_can_be_anonymous_or_named() {
		for (source, expected_name, field_count) in [
			("x :: { a :: A, b :: B }: x", None, 2),
			("x :: Type { a :: A, }: x", Some("Type"), 1),
		] {
			let loaded: Rc<String> = Rc::new(source.into());
			let loader = FileLoader::new({
				let loaded = loaded.clone();
				move |_| Ok(loaded.clone())
			});
			let (loaded, fid) = loader.load("test.mix".as_ref()).unwrap();
			let (expr, reports) = Parser::parse(&loaded, fid);
			assert!(!reports.has_errors());
			let ast::Expr::Lambda(lambda) = expr.unwrap().0 else {
				panic!("expected lambda expression");
			};
			let ast::Type::AttrSet { name, fields } = lambda.arg.0.ty.unwrap().0 else {
				panic!("expected attrset type");
			};
			assert_eq!(name.map(|name| name.0), expected_name);
			assert_eq!(fields.len(), field_count);
		}
	}

	#[test]
	fn union_types_bind_tighter_than_function_types() {
		let source: Rc<String> = Rc::new("x :: A | B -> C | D: x".into());
		let loader = FileLoader::new(move |_| Ok(source.clone()));
		let (source, fid) = loader.load("test.mix".as_ref()).unwrap();
		let (expr, reports) = Parser::parse(&source, fid);
		assert!(!reports.has_errors());
		let ast::Expr::Lambda(lambda) = expr.unwrap().0 else {
			panic!("expected lambda expression");
		};
		let ast::Type::Lambda { arg, ret } = lambda.arg.0.ty.unwrap().0 else {
			panic!("expected function type");
		};
		assert!(matches!(arg.0, ast::Type::Union(ref variants) if variants.len() == 2));
		assert!(matches!(ret.0, ast::Type::Union(ref variants) if variants.len() == 2));
	}
}
