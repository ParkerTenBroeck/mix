pub mod builtin;
pub mod eval;
pub mod lazy;
pub mod pretty;
pub mod scope;
pub mod thunk;
pub mod trace;
pub mod value;

use crate::{
	bytecode::Program,
	files::FileLoader,
	mir::lowerer::MirLowerer,
	parse::Parser,
	report::Reports,
	runtime::{
		eval::{Evaluator, Fule},
		lazy::LazyValue,
		scope::Scope,
		trace::ErrorTrace,
		value::Value,
	},
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
			program: Program::new(),
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

	pub fn eval_lazy(&mut self, lazy: LazyValue, deep: bool) -> Result<Value, ErrorTrace> {
		let mut eval = Evaluator::begin_eval(self, lazy, deep)?;
		let res = eval.run(self, Fule::unlimited());
		Ok(res
			.map_err(|err| ErrorTrace::build(self, &eval, err))?
			.unwrap())
	}

	// pub fn eval_lazy(&mut self, lazy: LazyValue, deep: bool) -> Result<Value, ErrorTrace> {
	// 	let mut eval = Evaluator::begin_eval(self, lazy, deep)?;
	// 	let res = eval.run(self, Fule::unlimited());
	// 	Ok(res
	// 		.map_err(|err| ErrorTrace::build(self, &eval, err))?
	// 		.unwrap())
	// }
}
