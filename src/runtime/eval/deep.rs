use crate::runtime::{
	eval::{EvalError, LocalEvaluator},
	value::Value,
};

impl LocalEvaluator {
	fn spill_deep_value(&mut self, value: &Value) -> Result<u32, EvalError> {
		let start = self.thunk_stack.len();
		match &value {
			Value::AttrSet(attrs) => {
				for lazy in attrs.values() {
					self.push_thunk(lazy.clone())?;
				}
			}
			Value::List(list) => {
				for lazy in list.iter() {
					self.push_thunk(lazy.clone())?;
				}
			}
			_ => {}
		}
		Ok((self.thunk_stack.len() - start) as u32)
	}
}
