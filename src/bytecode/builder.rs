use crate::files::Span;

use super::*;

#[derive(Debug)]
pub struct ExprBuilder<'a> {
    code: Vec<OpCode>,
    program: &'a mut Program,
}

impl<'a> ExprBuilder<'a> {
    pub fn new(program: &'a mut Program) -> Self {
        Self {
            code: Default::default(),
            program,
        }
    }

    pub fn finish(self) -> Vec<OpCode> {
        self.code
    }
}

impl<'a> ProgramBuilder for ExprBuilder<'a> {
    fn emit_str(&mut self, str: &str) -> StrId {
        self.program.emit_str(str)
    }

    fn emit_expr(&mut self, span: Span, expr: impl FnOnce(&mut ExprBuilder)) -> (ExprId, CodeLoc) {
        self.program.emit_expr(span, expr)
    }

    fn emit_lambda(
        &mut self,
        span: Span,
        expr: impl FnOnce(&mut ExprBuilder),
    ) -> (LambdaId, CodeLoc) {
        self.program.emit_lambda(span, expr)
    }
}

impl<'a> ByteCodeBuilder for ExprBuilder<'a> {
    fn emit(&mut self, op: OpCode) -> &mut Self {
        self.code.push(op);
        self
    }

    fn clone(&mut self) -> ExprBuilder<'_> {
        ExprBuilder {
            code: Default::default(),
            program: self.program,
        }
    }
}

impl<T: ProgramBuilder> ProgramBuilder for &mut T {
    fn emit_str(&mut self, str: &str) -> StrId {
        (*self).emit_str(str)
    }

    fn emit_expr(&mut self, span: Span, expr: impl FnOnce(&mut ExprBuilder)) -> (ExprId, CodeLoc) {
        (*self).emit_expr(span, expr)
    }

    fn emit_lambda(
        &mut self,
        span: Span,
        expr: impl FnOnce(&mut ExprBuilder),
    ) -> (LambdaId, CodeLoc) {
        (*self).emit_lambda(span, expr)
    }
}

pub trait ProgramBuilder {
    fn emit_str(&mut self, str: &str) -> StrId;
    fn emit_expr(&mut self, span: Span, expr: impl FnOnce(&mut ExprBuilder)) -> (ExprId, CodeLoc);
    fn emit_lambda(
        &mut self,
        span: Span,
        expr: impl FnOnce(&mut ExprBuilder),
    ) -> (LambdaId, CodeLoc);
}

pub trait ByteCodeBuilder: ProgramBuilder {
    fn emit(&mut self, op: OpCode) -> &mut Self;
    fn clone(&mut self) -> ExprBuilder<'_>;

    fn emit_add(&mut self) -> &mut Self {
        self.emit(OpCode::Add)
    }
    fn emit_sub(&mut self) -> &mut Self {
        self.emit(OpCode::Sub)
    }
    fn emit_mul(&mut self) -> &mut Self {
        self.emit(OpCode::Mul)
    }
    fn emit_div(&mut self) -> &mut Self {
        self.emit(OpCode::Div)
    }
    fn emit_rem(&mut self) -> &mut Self {
        self.emit(OpCode::Rem)
    }

    fn emit_eq(&mut self) -> &mut Self {
        self.emit(OpCode::Eq)
    }
    fn emit_ne(&mut self) -> &mut Self {
        self.emit(OpCode::Ne)
    }
    fn emit_lt(&mut self) -> &mut Self {
        self.emit(OpCode::Lt)
    }
    fn emit_gt(&mut self) -> &mut Self {
        self.emit(OpCode::Gt)
    }
    fn emit_lte(&mut self) -> &mut Self {
        self.emit(OpCode::Lte)
    }
    fn emit_gte(&mut self) -> &mut Self {
        self.emit(OpCode::Gte)
    }

    fn emit_not(&mut self) -> &mut Self {
        self.emit(OpCode::Not)
    }
    fn emit_neg(&mut self) -> &mut Self {
        self.emit(OpCode::Neg)
    }

    fn emit_and(&mut self, second_expr: impl FnOnce(&mut ExprBuilder)) -> &mut Self {
        let mut second_code = self.clone();
        second_expr(&mut second_code);
        let second_code = second_code.finish();

        self.emit(OpCode::And(CodeLocOffset(second_code.len())));

        for code in second_code {
            self.emit(code);
        }

        self
    }
    fn emit_or(&mut self, second_expr: impl FnOnce(&mut ExprBuilder)) -> &mut Self {
        let mut second_code = self.clone();
        second_expr(&mut second_code);
        let second_code = second_code.finish();

        self.emit(OpCode::Or(CodeLocOffset(second_code.len())));

        for code in second_code {
            self.emit(code);
        }

        self
    }
    fn emit_log_imp(&mut self, second_expr: impl FnOnce(&mut ExprBuilder)) -> &mut Self {
        let mut second_code = self.clone();
        second_expr(&mut second_code);
        let second_code = second_code.finish();

        self.emit(OpCode::LogImp(CodeLocOffset(second_code.len())));

        for code in second_code {
            self.emit(code);
        }

        self
    }

    fn emit_if_then(&mut self, then_expr: impl FnOnce(&mut ExprBuilder)) -> ThenBuilder<'_, Self> {
        let mut then_builder = self.clone();
        then_expr(&mut then_builder);
        ThenBuilder {
            code: then_builder.finish(),
            builder: &mut *self,
        }
    }

    fn emit_fn_app(&mut self, span: Span, arg: impl FnMut(&mut ExprBuilder<'_>)) -> &mut Self {
        let arg = self.emit_expr(span, arg).1;
        self.emit(OpCode::Apply(arg))
    }

    fn emit_create_list(&mut self, len: usize) -> &mut Self {
        self.emit(OpCode::CreateList(len))
    }
    fn emit_append_list(&mut self, span: Span, arg: impl FnMut(&mut ExprBuilder<'_>)) -> &mut Self {
        let arg = self.emit_expr(span, arg).1;
        self.emit(OpCode::AppendList(arg))
    }

    fn emit_load_str(&mut self, str: &str) -> &mut Self {
        let id = self.emit_str(str);
        self.emit(OpCode::LoadStr(id))
    }
    fn emit_load_int(&mut self, int: i64) -> &mut Self {
        self.emit(OpCode::LoadInt(int))
    }
    fn emit_load_float(&mut self, float: f64) -> &mut Self {
        self.emit(OpCode::LoadFloat(float))
    }
    fn emit_load_bool(&mut self, bool: bool) -> &mut Self {
        self.emit(OpCode::LoadBool(bool))
    }
    fn emit_load_lambda(&mut self, span: Span, body: impl FnMut(&mut ExprBuilder<'_>)) -> &mut Self {
        let lambda = self.emit_lambda(span, body).0;
        self.emit(OpCode::LoadLambda(lambda))
    }
}

#[must_use]
pub struct ThenBuilder<'a, T: ByteCodeBuilder + ?Sized> {
    code: Vec<OpCode>,
    builder: &'a mut T,
}

impl<'a, T: ByteCodeBuilder + ?Sized> ThenBuilder<'a, T> {
    pub fn emit_else(self, else_expr: impl FnOnce(&mut ExprBuilder)) -> &'a mut T {
        let mut else_builder = self.builder.clone();
        else_expr(&mut else_builder);
        let mut then_code = self.code;
        let else_code = else_builder.finish();

        then_code.push(OpCode::Branch(CodeLocOffset(else_code.len())));

        self.builder
            .emit(OpCode::If(CodeLocOffset(then_code.len())));
        for op in then_code {
            self.builder.emit(op);
        }
        for op in else_code {
            self.builder.emit(op);
        }

        self.builder
    }
}
