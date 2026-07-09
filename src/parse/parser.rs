use std::ops::Not;

use crate::{
    files::{FileId, Node, Span},
    lex::{Lexer, Token},
    parse::ast::{BinOp, UnOp},
    report::{
        Reports,
        parser::{
            ExpectedClosingDelimError, ExpectedEofError, FloatError, FuncAppInListError,
            FuncDefInListError, IntError, MismatchedDelimError, UnclosedDelimError,
            UnexpectedTokenAttrPathError, UnexpectedTokenExprError,
        },
    },
};

use super::ast;

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
            Token::DotDotDot => false,
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

    fn start_fn_arg(&self) -> bool {
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
            Token::DotDotDot => false,
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

    fn parse_pattern(&mut self) -> Node<ast::Pattern<'a>> {
        let start = self.curr.1;

        let binding = match self.curr.0 {
            Token::Ident(ident) => {
                self.next();
                Some(Node(ident, self.last.1))
            }
            _ => None,
        };

        let pattern = ast::Pattern {
            binding,
            destruct: vec![],
            strict_destruct: false,
        };

        let end = self.last.1;

        Node(pattern, start.merge(end))
    }

    fn parse_expr(&mut self) -> Node<ast::Expr<'a>> {
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
                    if !self.consume_if(Token::Comma) {
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
                        Err(_) => {
                            todo!();
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
