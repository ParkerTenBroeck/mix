pub mod eval;
pub mod files;
pub mod scope;
mod value;

pub use value::*;

use crate::{
    bytecode::{CodeLoc, Program},
    parse::{Parser, ast},
    runtime::{eval::Evaluator, files::Files, scope::Scope},
};

#[derive(Debug)]
pub struct Runtime<'a> {
    loader: &'a Files,
    program: Program,
    top_scope: Scope,
}

impl<'a> Runtime<'a> {
    pub fn new(loader: &'a Files, top_scope: Scope) -> Self {
        Self {
            loader,
            top_scope,
            program: Default::default(),
        }
    }

    pub fn load(&mut self, path: &str) -> LazyExpr {
        let (file, fid) = self.loader.load(path.as_ref()).unwrap();

        let expr = match Parser::parse(file) {
            Ok(ok) => ok,
            Err(err) => {
                let range = 0..file.len();

                for err in err.render(path.as_ref(), file) {
                    println!("{err}")
                }

                ast::Node(ast::Expr::Ident("null"), range.into())
            }
        };

        let expr = self.program.compile(fid, &expr);
        LazyExpr::uneval(expr, self.top_scope.clone())
    }

    pub fn eval(&mut self, expr: CodeLoc) -> Value {
        Evaluator::new(self).eval_expr(self.top_scope.clone(), expr)
    }

    pub fn deep_eval(&mut self, value: Value) -> Value {
        todo!();
        // let value = self.eval(value);

        value
    }
}

// #[derive(Clone, Debug)]
// pub struct Expr<'a> {
//     pub scope: ExprScope<'a>,
//     pub expr: CodeLoc,
// }
