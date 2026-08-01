use std::{borrow::Cow, collections::HashSet};

use dumpster::Trace;

use crate::runtime::{
	eval::{EvalError, NativeCtx, NativeLambdaAsync},
	lazy::{LazyValue, LazyValueKind},
	value::Value,
};

#[derive(Trace, Clone)]
pub struct ToJson;

impl NativeLambdaAsync for ToJson {
	fn identifier(&self) -> Cow<'static, str> {
		"toJSON".into()
	}

	async fn apply(self, mut ctx: NativeCtx, arg: LazyValue) -> Result<Value, EvalError> {
		let value = ctx.eval_lazy_deep(arg).await?;
		let json = value_to_json(&value, &mut HashSet::new())?;
		serde_json::to_string(&json)
			.map(Value::from)
			.map_err(|error| EvalError::Internal(format!("could not encode JSON: {error}").into()))
	}
}

fn value_to_json(
	value: &Value,
	visiting: &mut HashSet<usize>,
) -> Result<serde_json::Value, EvalError> {
	Ok(match value {
		Value::Bool(value) => (*value).into(),
		Value::Int(value) => (*value).into(),
		Value::Float(value) => serde_json::Number::from_f64(*value)
			.map(serde_json::Value::Number)
			.ok_or_else(|| json_error("non-finite floats cannot be represented"))?,
		Value::String(value) => value.to_string().into(),
		Value::Path(value) => value.display().to_string().into(),
		Value::Lambda(_) => return Err(json_error("functions cannot be represented")),
		Value::List(list) => {
			if !visiting.insert(list.id()) {
				return Err(json_error("cyclic lists cannot be represented"));
			}
			let values = list
				.iter()
				.map(|value| lazy_to_json(value, visiting))
				.collect::<Result<Vec<_>, _>>()?;
			visiting.remove(&list.id());
			values.into()
		}
		Value::AttrSet(attrs) => {
			if !visiting.insert(attrs.id()) {
				return Err(json_error("cyclic attribute sets cannot be represented"));
			}
			let mut object = serde_json::Map::new();
			for (name, value) in attrs.iter() {
				object.insert(name.to_string(), lazy_to_json(value, visiting)?);
			}
			visiting.remove(&attrs.id());
			object.into()
		}
	})
}

fn lazy_to_json(
	value: &LazyValue,
	visiting: &mut HashSet<usize>,
) -> Result<serde_json::Value, EvalError> {
	match value.try_get_value() {
		LazyValueKind::Value(value) => value_to_json(&value, visiting),
		LazyValueKind::Thunk(_) => Err(json_error("encountered an unevaluated value")),
	}
}

fn json_error(message: &str) -> EvalError {
	EvalError::Internal(format!("cannot convert value to JSON: {message}").into())
}
