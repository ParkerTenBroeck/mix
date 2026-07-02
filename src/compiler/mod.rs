use std::range::Range;

use crate::{
    bytecode::{CodeLoc, ExprBuilder, Lambda, LambdaId, Loc, OpCode, ProgramBuilder}, parse::ast, runtime::files::FileId,
};

pub struct FileCompiler {
    fid: FileId,
}

impl<T> ast::Node<T>{
    pub fn loc(&self, fid: FileId) -> (Loc, &T){
        (Loc::new(self.1, fid), &self.0)
    }
}

impl FileCompiler {
    pub fn new(fid: FileId) -> Self {
        Self { fid }
    }


    fn compile_top_level(&self, mut builder: impl ProgramBuilder, expr: &ast::Node<ast::Expr>){
        let (loc, expr) = expr.loc(self.fid);
        builder.emit_expr(loc, |eb| {
            self.compile_expr(eb, expr)
        });
    }

    fn compile_lambda(&mut self, loc: Loc, lambda: &ast::Lambda) -> LambdaId {
        let lambda = Lambda {
            code: self.compile_expr(&lambda.body),
            loc,
        };
        self.emit_lambda(lambda)
    }

    pub(super) fn compile_expr(&mut self, expr: &ast::Node<ast::Expr>) -> CodeLoc {
        let (_, loc) = expr.loc(self.fid);
        let mut bc_expr = ExprBuilder::new(loc);

        self.compile_expr_inline(&mut bc_expr, expr);
        bc_expr.emit(OpCode::Ret);

        self.emit_expr(bc_expr)
    }

    fn compile_attr_path_inline(&mut self, bc: &mut ExprBuilder, path: &ast::Node<ast::AttrPath>) {
        bc.emit(OpCode::CreatePath);
        for part in &path.0.parts {
            match &part.0 {
                ast::AttrPathPart::Ident(ident) => {
                    _ = bc.emit(OpCode::LoadStr(ident)).emit(OpCode::PushPathPart)
                }
                ast::AttrPathPart::Str(str) => {
                    _ = bc.emit(OpCode::LoadStr(str)).emit(OpCode::PushPathPart)
                }
                ast::AttrPathPart::Expr(_) => {
                    todo!()
                }
            }
        }
    }

    fn compile_expr_inline(&mut self, bc: &mut ExprBuilder, expr: &ast::Node<ast::Expr>) {
        let (ast_expr, loc) = expr.loc(self.fid);

        match ast_expr {
            ast::Expr::Lambda(lambda) => {
                bc.emit(OpCode::LoadLambda(self.compile_lambda(loc, lambda)));
            }
            ast::Expr::FuncApp { func, arg } => {
                self.compile_expr_inline(bc, func);
                bc.emit(OpCode::Apply(self.compile_expr(arg)));
            }
            ast::Expr::IfThenElse {
                cond,
                then_expr,
                else_expr,
            } => {
                self.compile_expr_inline(bc, cond);
                bc.emit(OpCode::If(
                    self.compile_expr(then_expr),
                    self.compile_expr(else_expr),
                ));
            }
            ast::Expr::BinOp {
                lhs: func,
                op: ast::Node(ast::BinOp::PipeL, _),
                rhs: expr,
            }
            | ast::Expr::BinOp {
                lhs: expr,
                op: ast::Node(ast::BinOp::PipeR, _),
                rhs: func,
            } => {
                self.compile_expr_inline(bc, func);
                bc.emit(OpCode::Apply(self.compile_expr(expr)));
            }

            ast::Expr::BinOp {
                lhs,
                op: op @ ast::Node(ast::BinOp::Or | ast::BinOp::And | ast::BinOp::LogImp, _),
                rhs,
            } => {
                self.compile_expr_inline(bc, lhs);
                let rhs = self.compile_expr(rhs);
                let op = match op.0 {
                    ast::BinOp::And => OpCode::And(rhs),
                    ast::BinOp::Or => OpCode::Or(rhs),
                    ast::BinOp::LogImp => OpCode::LogImp(rhs),
                    _ => unreachable!(),
                };
                bc.emit(op);
            }
            ast::Expr::BinOp { lhs, op, rhs } => {
                self.compile_expr_inline(bc, lhs);
                self.compile_expr_inline(bc, rhs);

                let op = match op.0 {
                    ast::BinOp::Rem => OpCode::Rem,
                    ast::BinOp::Div => OpCode::Div,
                    ast::BinOp::Mul => OpCode::Mul,
                    ast::BinOp::Sub => OpCode::Sub,
                    ast::BinOp::Add => OpCode::Add,
                    ast::BinOp::Lt => OpCode::Lt,
                    ast::BinOp::Lte => OpCode::Lte,
                    ast::BinOp::Gt => OpCode::Gt,
                    ast::BinOp::Gte => OpCode::Gte,
                    ast::BinOp::Eq => OpCode::Eq,
                    ast::BinOp::Ne => OpCode::Ne,
                    _ => unreachable!(),
                };
                bc.emit(op);
            }
            ast::Expr::UnOp { expr, op } => {
                self.compile_expr_inline(bc, expr);
                let op = match op.0 {
                    ast::UnOp::Neg => OpCode::Neg,
                    ast::UnOp::Not => OpCode::Not,
                };
                bc.emit(op);
            }
            ast::Expr::Let { bindings } => todo!(),
            ast::Expr::AttrSet { attrs } => {
                bc.emit(OpCode::CreateAttrSet);

                for attr in attrs {
                    self.compile_attr_path_inline(bc, &attr.0.path);
                    if let Some(expr) = &attr.0.value {
                        bc.emit(OpCode::InitAttrExpr(self.compile_expr(expr)));
                    } else {
                        bc.emit(OpCode::InitAttrPath);
                    }
                }
            }
            ast::Expr::List { elements } => {
                bc.emit(OpCode::CreateList(elements.len()));
                for element in elements {
                    // bc.emit(Op::LoadExpr(self.compile_expr(element)))
                    //     .emit(Op::AppendList);
                }
            }
            ast::Expr::AccessAttr { expr, path, or } => {
                let or = or.as_ref().map(|expr| self.compile_expr(expr));
                self.compile_expr_inline(bc, expr);
                bc.emit(OpCode::GetAttr(or));
            }
            ast::Expr::HasAttr { expr, path } => {
                self.compile_expr_inline(bc, expr);
                bc.emit(OpCode::HasAttr);
            }
            ast::Expr::Paren(node) => return self.compile_expr_inline(bc, node),
            ast::Expr::Ident(ident) => _ = bc.emit(OpCode::LoadStr(ident)).emit(OpCode::WithScope),
            ast::Expr::Num(ast::Num::Float(float)) => _ = bc.emit(OpCode::LoadFloat(*float)),
            ast::Expr::Num(ast::Num::Int(int)) => _ = bc.emit(OpCode::LoadInt(*int)),
            ast::Expr::Str(str) => _ = bc.emit(OpCode::LoadStr(str)),
        };
    }
}
