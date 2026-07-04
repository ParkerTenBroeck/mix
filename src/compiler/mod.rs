use crate::{
    bytecode::{ByteCodeBuilder, CodePos, ExprBuilder, OpCode, ProgramBuilder},
    files::Node,
    mir::ast,
};

#[derive(Default)]
pub struct Compiler {}

impl Compiler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn compile_top_level(
        &mut self,
        mut builder: impl ProgramBuilder,
        expr: &Node<ast::Expr>,
    ) -> CodePos {
        let (_, loc) = builder.emit_expr(expr.1, |eb| {
            self.compile_expr(eb, expr);
        });
        loc
    }

    fn compile_expr<'a, 'b>(
        &mut self,
        builder: &'b mut ExprBuilder<'a>,
        expr: &Node<ast::Expr>,
    ) -> &'b mut ExprBuilder<'a> {
        let Node(ast_expr, span) = expr;

        match ast_expr {
            ast::Expr::Lambda(lambda) => {
                builder.emit_load_lambda(*span, |builder| {
                    //TODO how do I want to do argument stuff?
                    self.compile_expr(builder, &lambda.body);
                });
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
                lhs,
                op: op @ Node(ast::BinOp::Or | ast::BinOp::And | ast::BinOp::LogImp, _),
                rhs,
            } => {
                self.compile_expr(builder, lhs);

                match op.0 {
                    ast::BinOp::And => {
                        builder.emit_and(|builder| _ = self.compile_expr(builder, rhs))
                    }
                    ast::BinOp::Or => {
                        builder.emit_or(|builder| _ = self.compile_expr(builder, rhs))
                    }
                    ast::BinOp::LogImp => {
                        builder.emit_log_imp(|builder| _ = self.compile_expr(builder, rhs))
                    }
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
            ast::Expr::AttrSet(attrs) => {
                builder.emit(OpCode::CreateAttrSet);

                for attr in &attrs.static_attrs {
                    if let Some(value) = &attr.0.value {
                        builder.emit_load_str(attr.0.name.0);
                        let expr = builder
                            .emit_expr(value.1, |builder| _ = self.compile_expr(builder, value));
                        builder.emit(OpCode::InitAttrExpr(expr.1));
                    } else {
                        todo!()
                    }
                }

                for attr in &attrs.dynamic_attrs {
                    if let Some(value) = &attr.0.value {
                        self.compile_attr_part(builder, &attr.0.part);
                        let expr = builder
                            .emit_expr(value.1, |builder| _ = self.compile_expr(builder, value));
                        builder.emit(OpCode::InitAttrExpr(expr.1));
                    } else {
                        todo!()
                    }
                }
                builder.emit(OpCode::FinalizeAttrSetRec);
            }
            ast::Expr::List { elements } => {
                builder.emit_create_list(elements.len());
                for element in elements {
                    builder.emit_append_list(element.1, |builder| {
                        _ = self.compile_expr(builder, element)
                    });
                }
            }
            ast::Expr::AccessAttr { expr, path, or } => {
                self.compile_expr(builder, expr);
                for part in &path.0.parts{
                    self.compile_attr_part(builder, part);
                    builder.emit(OpCode::GetAttr);
                }
            }
            ast::Expr::HasAttr { expr, path } => {}
            ast::Expr::Ident("true") => _ = builder.emit_load_bool(true),
            ast::Expr::Ident("false") => _ = builder.emit_load_bool(false),
            ast::Expr::Ident(ident) => _ = builder.emit_load_str(ident).emit(OpCode::LoadScope),
            ast::Expr::Num(ast::Num::Float(float)) => _ = builder.emit_load_float(*float),
            ast::Expr::Num(ast::Num::Int(int)) => _ = builder.emit_load_int(*int),
            ast::Expr::Str(str) => _ = builder.emit_load_str(str),
        };

        builder
    }

    fn compile_attr_part(&mut self, builder: &mut ExprBuilder, part: &Node<ast::AttrPathPart>) {
        match &part.0 {
            ast::AttrPathPart::Ident(ident) => {
                builder.emit_load_str(ident);
            }
            ast::AttrPathPart::Expr(expr) => {
                _ = self.compile_expr(builder, &Node(expr.clone(), part.1));
            }
        }
    }
}
