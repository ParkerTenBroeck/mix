pub mod eval;
pub mod scope;
mod value;
pub mod trace;

pub use value::*;

use crate::{
    bytecode::Program, files::Files, mir::lowerer::MirLowerer, parse::Parser, report::Reports, runtime::{eval::Evaluator, scope::Scope, trace::ErrorTrace},
};

#[derive(Debug)]
pub struct Runtime<'a> {
    pub loader: &'a Files,
    pub program: Program,
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

        let (expr, reports) = Parser::parse(file, fid);
        let Ok(expr) = expr else {
            return Err(reports);
        };
        let (expr, reports) = MirLowerer::new(reports).lower(expr);
        let Ok(expr) = expr else {
            return Err(reports);
        };

        let expr = self.program.compile(&expr);
        let expr = LazyValue::uneval(expr, self.default_scope.clone());
        Ok(expr)
    }

    pub fn eval(&mut self, lazy: LazyValue) -> Result<Value, ErrorTrace<'a>> {
        Evaluator::eval(self, lazy, false)
    }

    pub fn deep_eval(&mut self, lazy: LazyValue) -> Result<Value, ErrorTrace<'a>> {
        Evaluator::eval(self, lazy, true)
    }
}
