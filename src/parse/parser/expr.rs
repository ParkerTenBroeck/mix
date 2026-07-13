use crate::{
	files::Node,
	lex::Token,
	parse::{
		Delim, Parser,
		ast::{self, BinOp, UnOp},
	},
	report::parser::*,
};

impl<'a> Parser<'a> {
	pub(super) fn parse_expr(&mut self) -> Node<ast::Expr<'a>> {
		self.parse_expr_binop(0)
	}

	fn parse_expr_binop(&mut self, min_prec: u32) -> Node<ast::Expr<'a>> {
		let mut lhs = self.parse_expr_unop();

		loop {
			let op = match self.curr.0 {
				Token::Eq if BinOp::Eq.precedence() >= min_prec => BinOp::Eq,
				Token::Ne if BinOp::Ne.precedence() >= min_prec => BinOp::Ne,
				Token::Gt if BinOp::Gt.precedence() >= min_prec => BinOp::Gt,
				Token::Lt if BinOp::Lt.precedence() >= min_prec => BinOp::Lt,
				Token::Gte if BinOp::Gte.precedence() >= min_prec => BinOp::Gte,
				Token::Lte if BinOp::Lte.precedence() >= min_prec => BinOp::Lte,

				Token::PipeR if BinOp::PipeR.precedence() >= min_prec => BinOp::PipeR,
				Token::PipeL if BinOp::PipeL.precedence() >= min_prec => BinOp::PipeL,

				Token::Plus if BinOp::Add.precedence() >= min_prec => BinOp::Add,
				Token::Minus if BinOp::Sub.precedence() >= min_prec => BinOp::Sub,
				Token::Star if BinOp::Mul.precedence() >= min_prec => BinOp::Mul,
				Token::Slash if BinOp::Div.precedence() >= min_prec => BinOp::Div,
				Token::Percent if BinOp::Rem.precedence() >= min_prec => BinOp::Rem,
				Token::SmallRArrow if BinOp::LogImp.precedence() >= min_prec => BinOp::LogImp,

				Token::Or if BinOp::Or.precedence() >= min_prec => BinOp::Or,
				Token::And if BinOp::And.precedence() >= min_prec => BinOp::And,
				_ => break,
			};
			self.next();
			let op = Node(op, self.last.1);
			let min_prec = min_prec
				+ match op.0.associativity() {
					ast::Associativity::Left => 1,
					ast::Associativity::None => 0,
					ast::Associativity::Right => 0,
				};
			let rhs = Box::new(self.parse_expr_binop(min_prec));
			let span = lhs.1.merge(self.last.1);
			lhs = Node(
				ast::Expr::BinOp {
					lhs: Box::new(lhs),
					op,
					rhs,
				},
				span,
			);
		}
		lhs
	}

	fn parse_expr_unop(&mut self) -> Node<ast::Expr<'a>> {
		let op = match self.curr.0 {
			Token::Minus => UnOp::Neg,
			Token::Bang => UnOp::Not,
			_ => return self.parse_func_application(),
		};
		self.next();
		let op = Node(op, self.last.1);
		let expr = Box::new(self.parse_expr_unop());
		Node(ast::Expr::UnOp { expr, op }, op.1.merge(self.last.1))
	}

	fn parse_func_application(&mut self) -> Node<ast::Expr<'a>> {
		let mut expr = self.parse_expr_attr_path();

		while self.curr.0.start_fn_arg() {
			let func = Box::new(expr);
			let arg = Box::new(self.parse_expr_attr_path());
			let span = func.1.merge(arg.1);
			expr = Node(ast::Expr::FuncApp { func, arg }, span)
		}

		expr
	}

	fn parse_expr_attr_path(&mut self) -> Node<ast::Expr<'a>> {
		let expr = self.parse_expr_bottom();
		let (path, dot) = match self.curr.0 {
			Token::Question => {
				self.next();
				(self.parse_attr_path(), false)
			}
			Token::Dot => {
				self.next();
				(self.parse_attr_path(), true)
			}
			_ => return expr,
		};
		let expr = Box::new(expr);
		if dot {
			let or = self
				.consume_if(Token::Question)
				.then(|| Box::new(self.parse_expr()));
			let span = expr.1.merge(self.last.1);
			Node(ast::Expr::AccessAttr { expr, path, or }, span)
		} else {
			let span = expr.1.merge(self.last.1);
			Node(ast::Expr::HasAttr { expr, path }, span)
		}
	}

	fn parse_expr_bottom(&mut self) -> Node<ast::Expr<'a>> {
		{
			let state = self.state();

			let arg = self.parse_pattern();
			if self.consume_if(Token::Colon) {
				let body = Box::new(self.parse_expr());
				let span = arg.1.merge(body.1);
				let lambda = ast::Lambda { arg, body };
				return Node(ast::Expr::Lambda(lambda), span);
			} else {
				self.restore(state);
			}
		}

		let start = self.curr.1;
		let expr = match self.next() {
			Token::LParen => {
				let delim = Node(Delim::Paren, self.last.1);
				let expr = self.parse_expr();
				self.close_delim(delim);
				ast::Expr::Paren(Box::new(expr))
			}
			Token::LBrace => {
				let delim = Node(Delim::Brace, self.last.1);
				let mut attrs = vec![];

				loop {
					if matches!(
						self.curr.0,
						Token::RParen | Token::RBrace | Token::RBrack | Token::Eof
					) {
						break;
					}
					attrs.push(self.parse_attr());
					if !(self.consume_if(Token::Comma) || self.consume_if(Token::Semicolon)) {
						break;
					}
				}
				self.close_delim(delim);

				ast::Expr::AttrSet { attrs }
			}
			Token::LBrack => {
				let delim = Node(Delim::Brack, self.last.1);
				let mut elements = vec![];
				loop {
					if matches!(
						self.curr.0,
						Token::RParen | Token::RBrace | Token::RBrack | Token::Eof
					) {
						break;
					}
					let expr = self.parse_expr();
					match &expr.0 {
						ast::Expr::FuncApp { func, .. } => self.reports.emit(FuncAppInListError {
							span: expr.1,
							func: func.1,
						}),
						ast::Expr::Lambda { .. } => {
							self.reports.emit(FuncDefInListError { span: expr.1 })
						}
						_ => {}
					}
					elements.push(expr);
					if !self.consume_if(Token::Comma) {
						break;
					}
				}
				self.close_delim(delim);
				ast::Expr::List { elements }
			}
			Token::If => {
				let cond = Box::new(self.parse_expr());
				if !self.consume_if(Token::Then) {
					self.reports.emit(UnexpectedTokenExprError {
						span: self.curr.1,
						token: self.curr.0,
						expected: Some(Token::Then),
					});
				}
				let then_expr = Box::new(self.parse_expr());
				if !self.consume_if(Token::Else) {
					self.reports.emit(UnexpectedTokenExprError {
						span: self.curr.1,
						token: self.curr.0,
						expected: Some(Token::Else),
					});
				}
				let else_expr = Box::new(self.parse_expr());
				ast::Expr::IfThenElse {
					cond,
					then_expr,
					else_expr,
				}
			}
			Token::Ident(ident) => ast::Expr::Ident(ident),
			Token::Num(num) => self.parse_num(Some(num)),
			Token::Dot => self.parse_num(None),
			Token::String(str) => ast::Expr::Str(str),
			token => {
				self.reports.emit(UnexpectedTokenExprError {
					span: self.last.1,
					token,
					expected: None,
				});
				ast::Expr::Ident("<ERROR>")
			}
		};
		let end = self.last.1;
		Node(expr, start.merge(end))
	}

	fn parse_num(&mut self, start: Option<&str>) -> ast::Expr<'a> {
		let start_span = self.last.1;

		let num = match start {
			Some(start) => {
				if self.consume_if(Token::Dot) {
					let end = if let Token::Num(end) = self.curr.0 {
						self.next();
						end
					} else {
						""
					};
					format!("{start}.{end}")
				} else {
					start.into()
				}
			}
			None => {
				let Token::Num(start) = self.next() else {
					todo!()
				};
				format!(".{start}")
			}
		};

		let end_span = self.last.1;

		let num = if num.contains('.') {
			ast::Num::Float(match num.parse() {
				Ok(num) => num,
				Err(err) => {
					self.reports.emit(FloatError {
						span: start_span.merge(end_span),
						err,
					});
					0.
				}
			})
		} else {
			ast::Num::Int(match num.parse() {
				Ok(num) => num,
				Err(err) => {
					self.reports.emit(IntError {
						span: start_span.merge(end_span),
						err,
					});
					0
				}
			})
		};

		ast::Expr::Num(num)
	}

	fn parse_attr(&mut self) -> Node<ast::Attr<'a>> {
		let path = self.parse_attr_path();

		let value = self.consume_if(Token::Assign).then(|| self.parse_expr());
		let span = path.1.merge(self.last.1);
		Node(ast::Attr { path, value }, span)
	}

	fn parse_attr_path(&mut self) -> Node<ast::AttrPath<'a>> {
		let start = self.curr.1;

		let mut parts = vec![];

		loop {
			let start = self.curr.1;
			let part = match self.curr.0 {
				Token::Ident(ident) => {
					self.next();
					ast::AttrPathPart::Ident(ident)
				}
				Token::String(str) => {
					self.next();
					ast::AttrPathPart::Str(str)
				}
				Token::Num(str) => {
					self.next();
					let num = match str.parse() {
						Ok(num) => num,
						Err(err) => {
							self.reports.emit(IntError {
								span: start.merge(self.curr.1),
								err,
							});
							0
						}
					};

					ast::AttrPathPart::Num(num)
				}
				Token::Dollar => {
					self.next();
					if !self.consume_if(Token::LBrace) {
						todo!()
					}

					let expr = self.parse_expr();

					self.close_delim(Node(Delim::Brace, start));

					ast::AttrPathPart::Expr(expr.0)
				}
				token => {
					let err = UnexpectedTokenAttrPathError {
						span: self.curr.1,
						token,
					};
					self.reports.emit(err);
					break;
				}
			};

			parts.push(Node(part, start.merge(self.last.1)));

			if !self.consume_if(Token::Dot) {
				break;
			}
		}

		Node(ast::AttrPath { parts }, start.merge(self.last.1))
	}
}
