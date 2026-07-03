pub mod eval;
pub mod files;
pub mod scope;
mod value;

pub use value::*;

use crate::{
    bytecode::{CodeLoc, Program},
    parse::{Parser, ast::{self, Span}},
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

        let expr = match Parser::parse(file, fid) {
            Ok(ok) => ok,
            Err(err) => {
                let range = 0..file.len();

                for err in err.render(path.as_ref(), file) {
                    println!("{err}")
                }

                ast::Node(ast::Expr::Ident("null"), Span::new(range.into(), fid))
            }
        };

        let expr = self.program.compile(&expr);
        LazyExpr::uneval(expr, self.top_scope.clone())
    }

    pub fn eval_lazy(&mut self, expr: LazyExpr) -> Value {
        match expr {
            LazyExpr::Unevaluated(state) => {
                let mut state = state.borrow_mut();
                match &*state {
                    LazyExprState::Unevaluated(code_loc, scope) => {
                        let res = Evaluator::new(self).eval_expr(scope.clone(), *code_loc);
                        *state = LazyExprState::Evaluated(res.clone());
                        res
                    },
                    LazyExprState::Evaluating => todo!(),
                    LazyExprState::Evaluated(value) => value.clone(),
                    LazyExprState::Constructing(code_loc) => todo!(),
                }
            },
            LazyExpr::Evaluated(value) => value,
        }

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
