pub mod builtin;
pub mod eval;
pub mod lazy;
pub mod pretty;
pub mod scope;
pub mod thunk;
pub mod trace;
pub mod value;

use std::{borrow::Cow, collections::HashMap};

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
	loaded: HashMap<String, LazyValue>,
	default_scope: Scope,
}

#[derive(Clone, Debug)]
pub enum LoadError {
	Io(Cow<'static, str>),
	Reports(Reports),
}

impl LoadError {
	pub fn render(&self, loader: &FileLoader) -> String {
		match self {
			Self::Io(error) => error.to_string(),
			Self::Reports(reports) => reports.render(&loader.files()).join("\n"),
		}
	}
}

impl Runtime {
	pub fn new(loader: FileLoader, top_scope: Scope) -> Self {
		Self {
			loader,
			default_scope: top_scope,
			program: Program::new(),
			loaded: Default::default(),
		}
	}

	pub fn load(&mut self, path: &str) -> Result<LazyValue, LoadError> {
		if let Some(loaded) = self.loaded.get(path) {
			return Ok(loaded.try_get_value().into());
		}

		let (file, fid) = self.loader.load(path.as_ref()).map_err(LoadError::Io)?;

		let (expr, reports) = Parser::parse(&*file, fid);
		let Ok(expr) = expr else {
			return Err(LoadError::Reports(reports));
		};
		let (expr, reports) = MirLowerer::new(reports).lower(expr);
		let Ok(expr) = expr else {
			return Err(LoadError::Reports(reports));
		};

		let expr = self.program.compile(&expr);
		let expr = LazyValue::uneval(expr, self.default_scope.clone());
		self.loaded.insert(path.into(), expr.clone());
		Ok(expr)
	}

	pub fn eval_lazy(&mut self, lazy: LazyValue, deep: bool) -> Result<Value, ErrorTrace> {
		let mut eval = Evaluator::begin_eval(self, lazy, deep)?;
		let res = eval.run(self, &mut Fule::unlimited());
		res.map_err(|err| ErrorTrace::build(self, &eval, err))?
			.ok_or_else(|| {
				ErrorTrace::build(
					self,
					&eval,
					eval::EvalError::ByteCode("missing result value"),
				)
			})
	}
}
