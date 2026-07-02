use std::{ops::Not, range::Range};

use crate::{
    lex::{LexError, Lexer, Token},
    parse::{
        Report,
        ast::{BinOp, Node, UnOp},
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError<'a> {
    Lex(LexError),
    FloatErr(std::num::ParseFloatError),
    IntErr(std::num::ParseIntError),
    UnclosedDelim {
        opening: Node<Delim>,
        closing: Node<Delim>,
    },
    MismatchedDelim {
        opening: Node<Delim>,
        closing: Node<Delim>,
    },
    ExpectedClosingDelim(Delim, Token<'a>),
    UnexpectedTokenExpr(Token<'a>),
    UnexpectedTokenAttrPath(Token<'a>),
    FuncAppInList {
        func: Range<usize>,
    },
    FuncDefInList,
    ExpectedEof(Token<'a>),
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
        }
    }
}

pub struct Parser<'a> {
    lex: Lexer<'a>,
    reports: Report<'a>,
    last: Node<Token<'a>>,
    curr: Node<Token<'a>>,
}

struct State<'a> {
    lex: Lexer<'a>,
    reports: usize,
    last: Node<Token<'a>>,
    curr: Node<Token<'a>>,
}

pub type ParserResult<'a> = Result<Node<ast::Expr<'a>>, Report<'a>>;

impl<'a> Parser<'a> {
    pub fn parse(str: &'a str) -> ParserResult<'a> {
        let mut parser = Self {
            lex: Lexer::new(str),
            reports: Default::default(),
            curr: Node(Token::Eof, Default::default()),
            last: Node(Token::Eof, Default::default()),
        };
        _ = parser.next();

        let expr = parser.parse_expr();
        if parser.curr.0 != Token::Eof {
            parser
                .reports
                .push(Node(ParseError::ExpectedEof(parser.curr.0), parser.curr.1));
        }

        if parser.reports.count_errors() == 0 {
            return Ok(expr);
        }

        Err(parser.reports)
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
            let (tok, range) = self.lex.next();
            match tok {
                Err(e) => self.reports.push(Node(ParseError::Lex(e), range)),
                Ok(Token::Comment(_)) => {}
                Ok(tok) => {
                    self.curr = Node(tok, range);
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
                            let err = ParseError::MismatchedDelim { opening, closing };
                            self.reports.push(Node(err, self.last.1))
                        }
                        return;
                    } else {
                        level -= 1;
                    }
                }
                Token::Eof => {
                    let closing = Node(opening.0, self.last.1);
                    self.reports.push(Node(
                        ParseError::UnclosedDelim { opening, closing },
                        self.last.1,
                    ));
                    break;
                }
                token => {
                    if !error {
                        self.reports.push(Node(
                            ParseError::ExpectedClosingDelim(opening.0, token),
                            self.last.1,
                        ));
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
        let start = self.curr.1.start;

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

        let end = self.last.1.end;

        Node(pattern, Range { start, end })
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
            let range = Range {
                start: lhs.1.start,
                end: self.last.1.end,
            };
            lhs = Node(
                ast::Expr::BinOp {
                    lhs: Box::new(lhs),
                    op,
                    rhs,
                },
                range,
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
        let expr = self.parse_expr_unop();
        Node(
            expr.0,
            Range {
                start: op.1.start,
                end: expr.1.end,
            },
        )
    }

    fn parse_func_application(&mut self) -> Node<ast::Expr<'a>> {
        let mut expr = self.parse_expr_attr_path();

        while self.curr.0.starts_expr() {
            let func = Box::new(expr);
            let arg = Box::new(self.parse_expr_attr_path());
            let range = Range {
                start: func.1.start,
                end: arg.1.end,
            };
            expr = Node(ast::Expr::FuncApp { func, arg }, range)
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
            let range = Range {
                start: expr.1.start,
                end: self.last.1.end,
            };
            Node(ast::Expr::AccessAttr { expr, path, or }, range)
        } else {
            let range = Range {
                start: expr.1.start,
                end: self.last.1.end,
            };
            Node(ast::Expr::HasAttr { expr, path }, range)
        }
    }

    fn parse_expr_bottom(&mut self) -> Node<ast::Expr<'a>> {
        {
            let state = self.state();

            let arg = self.parse_pattern();
            if self.consume_if(Token::Colon) {
                let body = Box::new(self.parse_expr());
                let range = Range {
                    start: arg.1.start,
                    end: body.1.end,
                };
                let lambda = ast::Lambda { arg, body };
                return Node(ast::Expr::Lambda(lambda), range);
            } else {
                self.restore(state);
            }
        }

        let start = self.curr.1.start;
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
                        ast::Expr::FuncApp { func, .. } => self
                            .reports
                            .push(Node(ParseError::FuncAppInList { func: func.1 }, expr.1)),
                        ast::Expr::Lambda { .. } => {
                            self.reports.push(Node(ParseError::FuncDefInList, expr.1))
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
            Token::Ident(ident) => ast::Expr::Ident(ident),
            Token::Num(num) => ast::Expr::Num(if num.contains('.') {
                match num.parse() {
                    Ok(ok) => ast::Num::Float(ok),
                    Err(err) => {
                        self.reports
                            .push(Node(ParseError::FloatErr(err), self.last.1));
                        ast::Num::Float(0.0)
                    }
                }
            } else {
                match num.parse() {
                    Ok(ok) => ast::Num::Int(ok),
                    Err(err) => {
                        self.reports
                            .push(Node(ParseError::IntErr(err), self.last.1));
                        ast::Num::Int(0)
                    }
                }
            }),
            Token::String(str) => ast::Expr::Str(str),
            token => {
                self.reports
                    .push(Node(ParseError::UnexpectedTokenExpr(token), self.last.1));
                ast::Expr::Ident("<ERROR>")
            }
        };
        let end = self.last.1.end;
        Node(expr, Range { start, end })
    }

    fn parse_attr(&mut self) -> Node<ast::Attr<'a>> {
        let path = self.parse_attr_path();

        let value = self.consume_if(Token::Assign).then(|| self.parse_expr());
        let range = Range {
            start: path.1.start,
            end: self.last.1.end,
        };
        Node(ast::Attr { path, value }, range)
    }

    fn parse_attr_path(&mut self) -> Node<ast::AttrPath<'a>> {
        let start = self.curr.1.start;

        let mut parts = vec![];

        loop {
            let start = self.curr.1.start;
            let part = match self.curr.0 {
                Token::Ident(ident) => {
                    self.next();
                    ast::AttrPathPart::Ident(ident)
                }
                Token::String(str) => {
                    self.next();
                    ast::AttrPathPart::Str(str)
                }
                token => {
                    let err = ParseError::UnexpectedTokenAttrPath(token);
                    self.reports.push(Node(err, self.curr.1));
                    break;
                }
            };

            let range = Range {
                start,
                end: self.last.1.end,
            };
            parts.push(Node(part, range));

            if !self.consume_if(Token::Dot) {
                break;
            }
        }

        let range = Range {
            start,
            end: self.last.1.end,
        };
        Node(ast::AttrPath { parts }, range)
    }
}
