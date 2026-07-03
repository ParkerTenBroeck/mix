use crate::{
    bytecode::{ByteCodeBuilder, CodeLoc, ExprBuilder, OpCode, ProgramBuilder}, parse::ast,
};

#[derive(Default)]
pub struct Compiler {
}

impl Compiler {
    pub fn new() -> Self {
        Self{}
    }


    pub fn compile_top_level(&mut self, mut builder: impl ProgramBuilder, expr: &ast::Node<ast::Expr>) -> CodeLoc{
        let (_, loc) = builder.emit_expr(expr.1, |eb| {
            self.compile_expr(eb, expr);
        });
        loc
    }

    fn compile_expr<'a, 'b>(&mut self, builder: &'b mut ExprBuilder<'a>, expr: &ast::Node<ast::Expr>) -> &'b mut ExprBuilder<'a> {
        let ast::Node(ast_expr, loc) = expr;

        match ast_expr {
            ast::Expr::Lambda(lambda) => {
                // builder.emit_load_lambda(|builder|{

                // });
                // builder.emit(OpCode::LoadLambda(self.compile_lambda(loc, lambda)));
            }
            ast::Expr::FuncApp { func, arg } => {
                self.compile_expr(builder, func)
                    .emit_fn_app(arg.1, |builder| _ = self.compile_expr(builder, arg));
            }
            ast::Expr::IfThenElse {
                cond,
                then_expr,
                else_expr,
            } => {
                self.compile_expr(builder, cond)
                    .emit_if_then(|builder| _ = self.compile_expr(builder, then_expr))
                    .emit_else(|builder| _ = self.compile_expr(builder, else_expr));
            }
            ast::Expr::BinOp {
                lhs: func,
                op: ast::Node(ast::BinOp::PipeL, _),
                rhs: arg,
            }
            | ast::Expr::BinOp {
                lhs: arg,
                op: ast::Node(ast::BinOp::PipeR, _),
                rhs: func,
            } => {
                self.compile_expr(builder, func)
                    .emit_fn_app(arg.1, |builder| _ = self.compile_expr(builder, arg));
            }

            ast::Expr::BinOp {
                lhs,
                op: op @ ast::Node(ast::BinOp::Or | ast::BinOp::And | ast::BinOp::LogImp, _),
                rhs,
            } => {
                self.compile_expr(builder, lhs);
                
                match op.0 {
                    ast::BinOp::And => builder.emit_and(|builder| _ = self.compile_expr(builder, rhs)),
                    ast::BinOp::Or => builder.emit_or(|builder| _ = self.compile_expr(builder, rhs)),
                    ast::BinOp::LogImp => builder.emit_log_imp(|builder| _ = self.compile_expr(builder, rhs)),
                    _ => unreachable!(),
                };
            }
            ast::Expr::BinOp { lhs, op, rhs } => {
                self.compile_expr(builder, lhs);
                self.compile_expr(builder, rhs);

                match op.0 {
                    ast::BinOp::Rem => builder.emit_rem(),
                    ast::BinOp::Div => builder.emit_div(),
                    ast::BinOp::Mul => builder.emit_mul(),
                    ast::BinOp::Sub => builder.emit_sub(),
                    ast::BinOp::Add => builder.emit_add(),
                    ast::BinOp::Lt => builder.emit_lt(),
                    ast::BinOp::Lte => builder.emit_lte(),
                    ast::BinOp::Gt => builder.emit_gt(),
                    ast::BinOp::Gte => builder.emit_gte(),
                    ast::BinOp::Eq => builder.emit_eq(),
                    ast::BinOp::Ne => builder.emit_ne(),
                    _ => unreachable!(),
                };
            }
            ast::Expr::UnOp { expr, op } => {
                self.compile_expr(builder, expr);
                match op.0 {
                    ast::UnOp::Neg => builder.emit_neg(),
                    ast::UnOp::Not => builder.emit_not(),
                };
            }
            ast::Expr::Let { bindings } => todo!(),
            ast::Expr::AttrSet { attrs } => {

                for attr in attrs{
                    self.compile_attr_path(builder, &attr.0.path);
                }

                builder.emit(OpCode::CreateAttrSet(attrs.len()));

                for attr in attrs{
                    if let Some(value) = &attr.0.value{
                        let expr = builder.emit_expr(value.1, |builder| _ = self.compile_expr(builder, value));
                        builder.emit(OpCode::InitAttrExpr(expr.1));
                    }
                }
            }
            ast::Expr::List { elements } => {
                builder.emit_create_list(elements.len());
                for element in elements {
                    builder.emit_append_list(element.1, |builder| _ = self.compile_expr(builder, element));
                }
            }
            ast::Expr::AccessAttr { expr, path, or } => {
                // let or = or.as_ref().map(|expr| self.compile_expr(expr));
                // self.compile_expr_inline(bc, expr);
                // bc.emit(OpCode::GetAttr(or));
            }
            ast::Expr::HasAttr { expr, path } => {
                // self.compile_expr_inline(bc, expr);
                // bc.emit(OpCode::HasAttr);
            }
            ast::Expr::Paren(node) => _ = self.compile_expr(builder, node),
            ast::Expr::Ident("true") => _ = builder.emit_load_bool(true),
            ast::Expr::Ident("false") => _ = builder.emit_load_bool(false),
            ast::Expr::Ident(ident) => _ = builder.emit_load_str(ident).emit(OpCode::WithScope),
            ast::Expr::Num(ast::Num::Float(float)) => _ = builder.emit_load_float(*float),
            ast::Expr::Num(ast::Num::Int(int)) => _ = builder.emit_load_int(*int),
            ast::Expr::Str(str) => _ = builder.emit_load_str(str),
        };

        builder
    }

    fn compile_attr_path(&mut self, builder: &mut ExprBuilder, path: &ast::Node<ast::AttrPath>) {
        builder.emit(OpCode::CreatePath);
        for part in &path.0.parts {
            match &part.0 {
                ast::AttrPathPart::Ident(ident) => {
                    _ = builder.emit_load_str(ident).emit(OpCode::PushPathPart)
                }
                ast::AttrPathPart::Str(str) => {
                    _ = builder.emit_load_str(str).emit(OpCode::PushPathPart)
                }
                ast::AttrPathPart::Expr(_) => {
                    todo!()
                }
            }
        }
    }
}
