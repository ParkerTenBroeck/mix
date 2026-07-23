pub mod eval;
pub mod lazy;
pub mod pretty;
pub mod scope;
pub mod string;
pub mod thunk;
pub mod trace;
pub mod value;

use crate::{
	bytecode::Program,
	files::FileLoader,
	mir::lowerer::MirLowerer,
	parse::Parser,
	report::Reports,
	runtime::{eval::Evaluator, lazy::LazyValue, scope::Scope, trace::ErrorTrace, value::Value},
};

#[derive(Debug)]
pub struct Runtime {
	pub loader: FileLoader,
	pub program: Program,
	default_scope: Scope,
}

impl Runtime {
	pub fn new(loader: FileLoader, top_scope: Scope) -> Self {
		Self {
			loader,
			default_scope: top_scope,
			program: Default::default(),
		}
	}

	pub fn load(&mut self, path: &str) -> Result<LazyValue, Reports> {
		let (file, fid) = self.loader.load(path.as_ref()).unwrap();

		let (expr, reports) = Parser::parse(&*file, fid);
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

	pub fn eval(&mut self, lazy: LazyValue) -> Result<Value, ErrorTrace> {
		match lazy.try_get_value(){
			Ok(value) => Ok(value),
			Err(thunk) => Evaluator::begin_eval(thunk, false)?.run(self),
		}
	}

	pub fn deep_eval(&mut self, lazy: LazyValue) -> Result<Value, ErrorTrace> {
		match lazy.try_get_value(){
			Ok(value) => Ok(value),
			Err(thunk) => Evaluator::begin_eval(thunk, true)?.run(self),
		}
	}
}
