pub mod eval;
pub mod lazy;
pub mod pretty;
pub mod scope;
pub mod thunk;
pub mod trace;
pub mod value;
pub mod string;

use crate::{
	bytecode::Program,
	files::Files,
	mir::lowerer::MirLowerer,
	parse::Parser,
	report::Reports,
	runtime::{eval::Evaluator, lazy::LazyValue, scope::Scope, trace::ErrorTrace, value::Value},
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

	pub fn pretty_value<'rt>(&'rt self, value: &'rt Value) -> pretty::PrettyValue<'rt, 'a> {
		pretty::PrettyValue::new(self, value)
	}

	pub fn pretty_lazy<'rt>(&'rt self, value: &'rt LazyValue) -> pretty::PrettyLazyValue<'rt, 'a> {
		pretty::PrettyLazyValue::new(self, value)
	}
}
