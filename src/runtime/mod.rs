pub mod eval;
pub mod scope;
mod value;

pub use value::*;

use crate::{
    bytecode::{CodeLoc, Program}, files::{Files, Node, Span}, parse::{Parser, ast}, report::Reports, runtime::{eval::Evaluator, scope::Scope}
};

#[derive(Debug)]
pub struct Runtime<'a> {
    loader: &'a Files,
    program: Program,
    default_scope: Scope,
}

impl<'a> Runtime<'a> {
    pub fn new(loader: &'a Files, top_scope: Scope) -> Self {
        Self {
            loader,
            default_scope: top_scope,
            program: Default::default(),
        }
    }

    pub fn load(&mut self, path: &str) -> Result<LazyValue, Reports<'a>> {
        let (file, fid) = self.loader.load(path.as_ref()).unwrap();

        let expr = Parser::parse(file, fid)?;

        let expr = self.program.compile(&expr);
        let expr = LazyValue::uneval(expr, self.default_scope.clone());
        Ok(expr)
    }

    pub fn eval_lazy(&mut self, expr: LazyValue) -> Value {
        match expr {
            LazyValue::Unevaluated(state) => {
                let mut state = state.borrow_mut();
                match &*state {
                    LazyExprState::Unevaluated(code_loc, scope) => {
                        let res = Evaluator::new(self, *code_loc, scope.clone()).eval();
                        *state = LazyExprState::Evaluated(res.clone());
                        res
                    }
                    LazyExprState::Evaluating => todo!(),
                    LazyExprState::Evaluated(value) => value.clone(),
                    LazyExprState::Constructing(code_loc) => todo!(),
                }
            }
            LazyValue::Evaluated(value) => value,
        }
    }

    pub fn deep_eval(&mut self, value: Value) -> Value {
        todo!();
        // let value = self.eval(value);

        value
    }
}
