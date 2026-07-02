use super::*;

#[derive(Debug)]
pub struct ExprBuilder<'a> {
    code: Vec<OpCode>,
    program: &'a mut Program
}

impl<'a> ExprBuilder<'a>{
    pub fn new(program: &'a mut Program) -> Self{
        Self { code: Default::default(), program }
    }
}

impl<'a> ProgramBuilder for ExprBuilder<'a> {
    fn emit_str(&mut self, str: &str) -> StrId {
        self.program.emit_str(str)
    }

    fn emit_expr(&mut self, loc: Loc, expr: impl FnOnce(&mut ExprBuilder)) -> (ExprId, CodeLoc) {
        self.program.emit_expr(loc, expr)
    }

    fn emit_lambda(&mut self, loc: Loc, expr: impl FnOnce(&mut ExprBuilder)) -> (LambdaId, CodeLoc) {
        self.program.emit_lambda(loc, expr)
    }
}


pub trait ProgramBuilder{    
    fn emit_str(&mut self, str: &str) -> StrId;
    fn emit_expr(&mut self, loc: Loc, expr: impl FnOnce(&mut ExprBuilder)) -> (ExprId, CodeLoc);
    fn emit_lambda(&mut self, loc: Loc, expr: impl FnOnce(&mut ExprBuilder)) -> (LambdaId, CodeLoc);
}

pub trait ByteCodeBuilder: ProgramBuilder{
    fn emit(&mut self, op: OpCode) -> &mut Self;

    fn emit_add(&mut self) -> &mut Self {
        self.emit(OpCode::Add)
    }
    fn emit_sub(&mut self) -> &mut Self {
        self.emit(OpCode::Sub)
    }
    fn emit_mul(&mut self) -> &mut Self  {
        self.emit(OpCode::Mul)
    }
    fn emit_div(&mut self) -> &mut Self  {
        self.emit(OpCode::Div)
    }
    fn emit_rem(&mut self) -> &mut Self  {
        self.emit(OpCode::Rem)
    }

    fn emit_eq(&mut self) -> &mut Self  {
        self.emit(OpCode::Eq)
    }
    fn emit_ne(&mut self) -> &mut Self  {
        self.emit(OpCode::Ne)
    }
    fn emit_lt(&mut self) -> &mut Self  {
        self.emit(OpCode::Lt)
    }
    fn emit_gt(&mut self) -> &mut Self  {
        self.emit(OpCode::Gt)
    }
    fn emit_lte(&mut self) -> &mut Self  {
        self.emit(OpCode::Lte)
    }
    fn emit_gte(&mut self) -> &mut Self  {
        self.emit(OpCode::Gte)
    }

    fn emit_not(&mut self) -> &mut Self  {
        self.emit(OpCode::Not)
    }
    fn emit_neg(&mut self) -> &mut Self  {
        self.emit(OpCode::Neg)
    }

    // fn emit_and(&mut self, second: impl FnOnce(&mut ExprBuilder)) -> &mut Self  {
    //     let second = self.emit_expr(second).1;
    //     self.emit(OpCode::And(second))
    // }
    // fn emit_or(&mut self, second: impl FnOnce(&mut ExprBuilder)) -> &mut Self  {
    //     let second = self.emit_expr(second).1;
    //     self.emit(OpCode::Or(second))
    // }
    // fn emit_log_imp(&mut self, second: impl FnOnce(&mut ExprBuilder)) -> &mut Self  {
    //     let second = self.emit_expr(second).1;
    //     self.emit(OpCode::LogImp(second))
    // }

    // fn emit_if_then(&mut self, then_expr: impl FnOnce(&mut ExprBuilder), else_expr: impl FnOnce(&mut ExprBuilder)) -> &mut Self{
    //     let then_expr = self.emit_expr(then_expr).1;
    //     let else_expr = self.emit_expr(else_expr).1;
    //     self.emit(OpCode::If(then_expr, else_expr))
    // }

    fn emit_load_str(&mut self, str: &str) -> &mut Self {
        let id = self.emit_str(str);
        self.emit(OpCode::LoadStr(id))
    }
    fn emit_load_int(&mut self, int: i64) -> &mut Self{
        self.emit(OpCode::LoadInt(int))
    }
    fn emit_load_float(&mut self, float: f64) -> &mut Self{
        self.emit(OpCode::LoadFloat(float))
    }
}
