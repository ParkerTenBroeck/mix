use std::borrow::Cow;

use crate::{
    files::{Node, Span},
    mir::ast::{
        self, AttrPathPart, AttrSet, DynamicAttr, Expr, Lambda, LetBinding, Num, Pattern,
        StaticAttr,
    },
    parse,
    report::{Reports, mir::DuplicateAttrError},
};

pub type MirLowerResult<'a> = (Result<Node<ast::Expr<'a>>, ()>, Reports<'a>);

pub struct MirLowerer<'a> {
    reports: Reports<'a>,
}

#[derive(Clone, Debug)]
struct StaticAttrBuilder<'a> {
    name: Node<&'a str>,
    full_span: Span,
    value: Option<Node<ast::Expr<'a>>>,
    children: Vec<StaticAttrBuilder<'a>>,
}

impl<'a> MirLowerer<'a> {
    pub fn new(reports: Reports<'a>) -> Self {
        Self { reports }
    }

    pub fn lower(mut self, expr: Node<parse::ast::Expr<'a>>) -> MirLowerResult<'a> {
        let expr = self.lower_expr(expr);
        let reports = self.reports;
        (
            reports.has_errors().not().then_some(expr).ok_or(()),
            reports,
        )
    }

    fn lower_expr(&mut self, Node(expr, span): Node<parse::ast::Expr<'a>>) -> Node<ast::Expr<'a>> {
        let expr = match expr {
            parse::ast::Expr::Lambda(lambda) => ast::Expr::Lambda(Lambda {
                arg: self.lower_pattern(lambda.arg),
                body: Box::new(self.lower_expr(*lambda.body)),
            }),
            parse::ast::Expr::FuncApp { func, arg } => ast::Expr::FuncApp {
                func: Box::new(self.lower_expr(*func)),
                arg: Box::new(self.lower_expr(*arg)),
            },
            parse::ast::Expr::IfThenElse {
                cond,
                then_expr,
                else_expr,
            } => ast::Expr::IfThenElse {
                cond: Box::new(self.lower_expr(*cond)),
                then_expr: Box::new(self.lower_expr(*then_expr)),
                else_expr: Box::new(self.lower_expr(*else_expr)),
            },
            parse::ast::Expr::BinOp { lhs, op, rhs } => self.lower_binop(*lhs, op, *rhs, span),
            parse::ast::Expr::UnOp { expr, op } => ast::Expr::UnOp {
                expr: Box::new(self.lower_expr(*expr)),
                op: Node(self.lower_unop(op.0), op.1),
            },
            parse::ast::Expr::Let { bindings } => ast::Expr::Let {
                bindings: bindings
                    .into_iter()
                    .map(|binding| LetBinding {
                        id: self.lower_pattern(binding.id),
                        value: self.lower_expr(binding.value),
                    })
                    .collect(),
            },
            parse::ast::Expr::AttrSet { attrs } => ast::Expr::AttrSet(self.lower_attr_set(attrs)),
            parse::ast::Expr::List { elements } => ast::Expr::List {
                elements: elements
                    .into_iter()
                    .map(|element| self.lower_expr(element))
                    .collect(),
            },
            parse::ast::Expr::AccessAttr { expr, path, or } => ast::Expr::AccessAttr {
                expr: Box::new(self.lower_expr(*expr)),
                path: self.lower_attr_path(path),
                or: or.map(|or| Box::new(self.lower_expr(*or))),
            },
            parse::ast::Expr::HasAttr { expr, path } => ast::Expr::HasAttr {
                expr: Box::new(self.lower_expr(*expr)),
                path: self.lower_attr_path(path),
            },
            parse::ast::Expr::Paren(expr) => self.lower_expr(*expr).0,
            parse::ast::Expr::Ident(ident) => ast::Expr::Ident(ident),
            parse::ast::Expr::Num(parse::ast::Num::Float(float)) => {
                ast::Expr::Num(Num::Float(float))
            }
            parse::ast::Expr::Num(parse::ast::Num::Int(int)) => ast::Expr::Num(Num::Int(int)),
            parse::ast::Expr::Str(str) => ast::Expr::Str(str),
        };

        Node(expr, span)
    }

    fn lower_pattern(
        &mut self,
        Node(pattern, span): Node<parse::ast::Pattern<'a>>,
    ) -> Node<Pattern<'a>> {
        Node(
            Pattern {
                binding: pattern.binding,
                destruct: pattern.destruct,
                strict_destruct: pattern.strict_destruct,
            },
            span,
        )
    }

    fn lower_binop(
        &mut self,
        lhs: Node<parse::ast::Expr<'a>>,
        op: Node<parse::ast::BinOp>,
        rhs: Node<parse::ast::Expr<'a>>,
        _span: Span,
    ) -> ast::Expr<'a> {
        match op.0 {
            parse::ast::BinOp::PipeL => ast::Expr::FuncApp {
                func: Box::new(self.lower_expr(lhs)),
                arg: Box::new(self.lower_expr(rhs)),
            },
            parse::ast::BinOp::PipeR => ast::Expr::FuncApp {
                func: Box::new(self.lower_expr(rhs)),
                arg: Box::new(self.lower_expr(lhs)),
            },
            op_kind => ast::Expr::BinOp {
                lhs: Box::new(self.lower_expr(lhs)),
                op: Node(self.map_binop(op_kind), op.1),
                rhs: Box::new(self.lower_expr(rhs)),
            },
        }
    }

    fn lower_attr_set(&mut self, attrs: Vec<Node<parse::ast::Attr<'a>>>) -> AttrSet<'a> {
        let mut static_attrs = Vec::new();
        let mut dynamic_attrs = Vec::new();

        for attr in attrs {
            let lowered_value = attr.0.value.map(|value| self.lower_expr(value));
            if let Some(parts) = self.static_attr_parts(&attr.0.path) {
                self.insert_static_attr(
                    &mut static_attrs,
                    &parts,
                    lowered_value,
                    attr.1,
                    String::new(),
                );
            } else {
                dynamic_attrs.push(self.lower_dynamic_attr(attr.0.path, lowered_value, attr.1));
            }
        }

        AttrSet {
            static_attrs: static_attrs
                .into_iter()
                .map(|attr| self.finish_static_attr(attr))
                .collect(),
            dynamic_attrs,
        }
    }

    fn static_attr_parts(
        &self,
        path: &Node<parse::ast::AttrPath<'a>>,
    ) -> Option<Vec<Node<&'a str>>> {
        path.0
            .parts
            .iter()
            .map(|part| match part.0 {
                parse::ast::AttrPathPart::Ident(name) | parse::ast::AttrPathPart::Str(name) => {
                    Some(Node(name, part.1))
                }
                parse::ast::AttrPathPart::Expr(_) => None,
            })
            .collect()
    }

    fn insert_static_attr(
        &mut self,
        attrs: &mut Vec<StaticAttrBuilder<'a>>,
        parts: &[Node<&'a str>],
        value: Option<Node<ast::Expr<'a>>>,
        attr_span: Span,
        prefix: String,
    ) {
        let Node(name, name_span) = parts[0];
        let path_name = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}.{name}")
        };

        if parts.len() == 1 {
            if let Some(existing) = attrs.iter().find(|existing| existing.name.0 == name) {
                self.reports.emit(DuplicateAttrError {
                    span: name_span,
                    first: existing.full_span,
                    name: Cow::Owned(path_name),
                });
                return;
            }

            attrs.push(StaticAttrBuilder {
                name: Node(name, name_span),
                full_span: attr_span,
                value,
                children: vec![],
            });
            return;
        }

        if let Some(existing) = attrs.iter_mut().find(|existing| existing.name.0 == name) {
            if existing.value.is_some() {
                self.reports.emit(DuplicateAttrError {
                    span: name_span,
                    first: existing.full_span,
                    name: Cow::Owned(path_name),
                });
                return;
            }

            self.insert_static_attr(
                &mut existing.children,
                &parts[1..],
                value,
                attr_span,
                path_name,
            );
            return;
        }

        let mut child = StaticAttrBuilder {
            name: Node(name, name_span),
            full_span: attr_span,
            value: None,
            children: vec![],
        };
        self.insert_static_attr(
            &mut child.children,
            &parts[1..],
            value,
            attr_span,
            path_name,
        );
        attrs.push(child);
    }

    fn finish_static_attr(&mut self, attr: StaticAttrBuilder<'a>) -> Node<StaticAttr<'a>> {
        let value = if !attr.children.is_empty() {
            let span = attr.full_span;
            Some(Node(
                Expr::AttrSet(AttrSet {
                    static_attrs: attr
                        .children
                        .into_iter()
                        .map(|child| self.finish_static_attr(child))
                        .collect(),
                    dynamic_attrs: vec![],
                }),
                span,
            ))
        } else {
            attr.value
        };

        Node(
            StaticAttr {
                name: attr.name,
                value,
            },
            attr.full_span,
        )
    }

    fn lower_dynamic_attr(
        &mut self,
        path: Node<parse::ast::AttrPath<'a>>,
        value: Option<Node<ast::Expr<'a>>>,
        span: Span,
    ) -> Node<DynamicAttr<'a>> {
        let mut parts = self.lower_attr_path_parts(path);
        let part = parts.remove(0);
        let value = if parts.is_empty() {
            value
        } else {
            Some(Node(
                Expr::AttrSet(AttrSet {
                    static_attrs: vec![],
                    dynamic_attrs: vec![self.build_dynamic_attr(parts, value, span)],
                }),
                span,
            ))
        };

        Node(DynamicAttr { part, value }, span)
    }

    fn build_dynamic_attr(
        &mut self,
        mut parts: Vec<Node<AttrPathPart<'a>>>,
        value: Option<Node<ast::Expr<'a>>>,
        span: Span,
    ) -> Node<DynamicAttr<'a>> {
        let part = parts.remove(0);
        let value = if parts.is_empty() {
            value
        } else {
            Some(Node(
                Expr::AttrSet(AttrSet {
                    static_attrs: vec![],
                    dynamic_attrs: vec![self.build_dynamic_attr(parts, value, span)],
                }),
                span,
            ))
        };

        Node(DynamicAttr { part, value }, span)
    }

    fn lower_attr_path(&mut self, path: Node<parse::ast::AttrPath<'a>>) -> Node<ast::AttrPath<'a>> {
        let span = path.1;
        Node(
            ast::AttrPath {
                parts: self.lower_attr_path_parts(path),
            },
            span,
        )
    }

    fn lower_attr_path_parts(
        &mut self,
        Node(path, _span): Node<parse::ast::AttrPath<'a>>,
    ) -> Vec<Node<AttrPathPart<'a>>> {
        path.parts
            .into_iter()
            .map(|Node(part, part_span)| {
                Node(
                    match part {
                        parse::ast::AttrPathPart::Ident(ident)
                        | parse::ast::AttrPathPart::Str(ident) => AttrPathPart::Ident(ident),
                        parse::ast::AttrPathPart::Expr(expr) => {
                            AttrPathPart::Expr(self.lower_expr(Node(expr, part_span)).0)
                        }
                    },
                    part_span,
                )
            })
            .collect()
    }

    fn map_binop(&self, op: parse::ast::BinOp) -> ast::BinOp {
        match op {
            parse::ast::BinOp::Rem => ast::BinOp::Rem,
            parse::ast::BinOp::Div => ast::BinOp::Div,
            parse::ast::BinOp::Mul => ast::BinOp::Mul,
            parse::ast::BinOp::Sub => ast::BinOp::Sub,
            parse::ast::BinOp::Add => ast::BinOp::Add,
            parse::ast::BinOp::Lt => ast::BinOp::Lt,
            parse::ast::BinOp::Lte => ast::BinOp::Lte,
            parse::ast::BinOp::Gt => ast::BinOp::Gt,
            parse::ast::BinOp::Gte => ast::BinOp::Gte,
            parse::ast::BinOp::Eq => ast::BinOp::Eq,
            parse::ast::BinOp::Ne => ast::BinOp::Ne,
            parse::ast::BinOp::And => ast::BinOp::And,
            parse::ast::BinOp::Or => ast::BinOp::Or,
            parse::ast::BinOp::LogImp => ast::BinOp::LogImp,
            parse::ast::BinOp::PipeL | parse::ast::BinOp::PipeR => unreachable!(),
        }
    }

    fn lower_unop(&self, op: parse::ast::UnOp) -> ast::UnOp {
        match op {
            parse::ast::UnOp::Neg => ast::UnOp::Neg,
            parse::ast::UnOp::Not => ast::UnOp::Not,
        }
    }
}

trait BoolNot {
    fn not(self) -> bool;
}

impl BoolNot for bool {
    fn not(self) -> bool {
        !self
    }
}
