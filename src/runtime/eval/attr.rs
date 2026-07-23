use super::*;

impl Evaluator{
    pub(super) fn get_attr(
		&mut self,
		indexing: &Value,
		index: &Value,
	) -> Result<Option<LazyValue>, EvalError> {
		match index {
			Value::String(name) => match indexing {
				Value::AttrSet(attrset) => Ok(attrset.get(name).cloned()),
				Value::List(list) => {
					if &**name != "len" {
						Ok(None)
					} else {
						Ok(Some(Value::Int(list.len() as i64).into()))
					}
				}
				value => Err(EvalError::TypeMismatch {
					expected: ValueType::AttrSet,
					got: value.ty(),
				}),
			},
			Value::Int(index) => match indexing {
				Value::List(list) => {
					Ok(list.get((*index).try_into().unwrap_or(usize::MAX)).cloned())
				}
				value => Err(EvalError::TypeMismatch {
					expected: ValueType::List,
					got: value.ty(),
				}),
			},
			value => Err(EvalError::TypeMismatch {
				expected: ValueType::AttrSet,
				got: value.ty(),
			}),
		}
	}
}