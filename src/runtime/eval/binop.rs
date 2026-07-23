use super::*;

macro_rules! binop_cmp {
	($op:literal, $lhs_value:expr, $rhs_value:expr, $lhs:ident, $rhs:ident, $expr:expr) => {{
		let lhs = $lhs_value;
		let rhs = $rhs_value;
		let result = match (lhs, rhs) {
			(Value::Int($lhs), Value::Int($rhs)) => $expr,
			(Value::Float($lhs), Value::Int($rhs)) => {
				let $lhs = $lhs as f64;
				let $rhs = $rhs as f64;
				$expr
			}
			(Value::Int($lhs), Value::Float($rhs)) => {
				let $lhs = $lhs as f64;
				let $rhs = $rhs as f64;
				$expr
			}
			(Value::Float($lhs), Value::Float($rhs)) => $expr,
			#[allow(clippy::bool_comparison)]
			(Value::Bool($lhs), Value::Bool($rhs)) => $expr,
			(Value::String($lhs), Value::String($rhs)) => $expr,
			(lhs, rhs) => {
				return Err(EvalError::BinOpTypeMismatch {
					details: format!("cannot compare {} and {} with {}", lhs.ty(), rhs.ty(), $op)
						.into(),
				});
			}
		};
		result
	}};
}

macro_rules! checked_int_result {
	($op_name:expr, $display:expr, $value:expr $(,)?) => {
		$value.map(Value::Int).ok_or_else(|| {
			EvalError::Arithmetic(format!("{} overflowed for {}", $op_name, $display).into())
		})
	};
}

macro_rules! checked_numeric_op {
	(
		$lhs:expr,
		$rhs:expr,
		type_error($bad_lhs:ident, $bad_rhs:ident) = $type_error:block,
		int($int_lhs:ident, $int_rhs:ident) = $int_eval:block,
		float($float_lhs:ident, $float_rhs:ident) = $float_eval:block $(,)?
	) => {{
		match ($lhs, $rhs) {
			(Value::Int($int_lhs), Value::Int($int_rhs)) => $int_eval,
			(Value::Float($float_lhs), Value::Int($float_rhs)) => {
				let $float_rhs = $float_rhs as f64;
				$float_eval
			}
			(Value::Int($float_lhs), Value::Float($float_rhs)) => {
				let $float_lhs = $float_lhs as f64;
				$float_eval
			}
			(Value::Float($float_lhs), Value::Float($float_rhs)) => $float_eval,
			($bad_lhs, $bad_rhs) => Err($type_error),
		}
	}};
}

macro_rules! checked_numeric_method {
	(
		$name:ident,
		$this:ident,
		op_name = $op_name:literal,
		symbol = $symbol:literal,
		type_error($bad_lhs:ident, $bad_rhs:ident) = $type_error:block,
		int = $int_method:ident,
		float($float_lhs:ident, $float_rhs:ident) = $float_eval:block $(,)?
	) => {
		pub(super) fn $name(&self, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
			let $this = self;
			checked_numeric_op!(
				lhs,
				rhs,
				type_error($bad_lhs, $bad_rhs) = $type_error,
				int(lhs, rhs) = {
					checked_int_result!(
						$op_name,
						format!("{lhs} {} {rhs}", $symbol),
						lhs.$int_method(rhs),
					)
				},
				float($float_lhs, $float_rhs) = $float_eval,
			)
		}
	};
}

macro_rules! checked_zero_numeric_method {
	(
		$name:ident,
		$this:ident,
		op_name = $op_name:literal,
		symbol = $symbol:literal,
		type_error($bad_lhs:ident, $bad_rhs:ident) = $type_error:block,
		int = $int_method:ident,
		float($float_lhs:ident, $float_rhs:ident) = $float_eval:block,
		zero = $zero_message:literal $(,)?
	) => {
		pub(super) fn $name(&self, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
			let $this = self;
			checked_numeric_op!(
				lhs,
				rhs,
				type_error($bad_lhs, $bad_rhs) = $type_error,
				int(lhs, rhs) = {
					if rhs == 0 {
						return Err(EvalError::Arithmetic(format!($zero_message, lhs).into()));
					}

					checked_int_result!(
						$op_name,
						format!("{lhs} {} {rhs}", $symbol),
						lhs.$int_method(rhs),
					)
				},
				float($float_lhs, $float_rhs) = {
					if $float_rhs == 0.0 {
						return Err(EvalError::Arithmetic(
							format!($zero_message, $float_lhs).into(),
						));
					}

					$float_eval
				},
			)
		}
	};
}

impl Evaluator {
	pub(super) fn binop_eq(&self, op: OpCode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
		let (symbol, invert) = match op {
			OpCode::Eq => ("==", false),
			OpCode::Ne => ("!=", true),
			_ => {
				return Err(EvalError::ByteCode(
					"non-equality opcode passed to binop_eq",
				))
			}
		};
        // todo deep equality..
		let equal = match (lhs, rhs) {
			(Value::Int(lhs), Value::Int(rhs)) => lhs == rhs,
			(Value::Float(lhs), Value::Int(rhs)) => lhs == rhs as f64,
			(Value::Int(lhs), Value::Float(rhs)) => lhs as f64 == rhs,
			(Value::Float(lhs), Value::Float(rhs)) => lhs == rhs,
			(Value::Bool(lhs), Value::Bool(rhs)) => lhs == rhs,
			(Value::String(lhs), Value::String(rhs)) => lhs == rhs,
			(lhs, rhs) if lhs.ty() != rhs.ty() => false,
			(lhs, rhs) => {
				return Err(EvalError::BinOpTypeMismatch {
					details: format!(
						"cannot compare {} and {} with {}",
						lhs.ty(),
						rhs.ty(),
						symbol
					)
					.into(),
				});
			}
		};
		Ok(Value::Bool(equal ^ invert))
	}

	pub(super) fn binop_cmp(&self, op: OpCode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
		let result = match op {
			OpCode::Lt => binop_cmp!("<", lhs, rhs, lhs, rhs, Value::Bool(lhs < rhs)),
			OpCode::Lte => binop_cmp!("<=", lhs, rhs, lhs, rhs, Value::Bool(lhs <= rhs)),
			OpCode::Gt => binop_cmp!(">", lhs, rhs, lhs, rhs, Value::Bool(lhs > rhs)),
			OpCode::Gte => binop_cmp!(">=", lhs, rhs, lhs, rhs, Value::Bool(lhs >= rhs)),
			_ => {
				return Err(EvalError::ByteCode(
					"non-comparison opcode passed to binop_cmp",
				));
			}
		};

		Ok(result)
	}

	fn checked_float_result(
		&self,
		op_name: &'static str,
		lhs: f64,
		rhs: f64,
		eval: impl FnOnce(f64, f64) -> f64,
	) -> Result<Value, EvalError> {
		let value = eval(lhs, rhs);

		if value.is_finite() {
			Ok(Value::Float(value))
		} else {
			Err(EvalError::Arithmetic(
				format!("{op_name} overflowed or produced a non-finite float for {lhs} and {rhs}")
					.into(),
			))
		}
	}

	pub(super) fn checked_add(&self, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
		match (lhs, rhs) {
			(Value::String(mut lhs), Value::String(rhs)) => {
				lhs.get_mut().push_str(&rhs);
				Ok(Value::String(lhs))
			}
			(lhs, rhs) => checked_numeric_op!(
				lhs,
				rhs,
				type_error(lhs, rhs) = {
					EvalError::BinOpTypeMismatch {
						details: format!("cannot add {} to {}", rhs.ty(), lhs.ty()).into(),
					}
				},
				int(lhs, rhs) = {
					checked_int_result!("addition", format!("{lhs} + {rhs}"), lhs.checked_add(rhs),)
				},
				float(lhs, rhs) =
					{ self.checked_float_result("addition", lhs, rhs, |lhs, rhs| lhs + rhs) },
			),
		}
	}

	checked_numeric_method!(
		checked_sub,
		this,
		op_name = "subtraction",
		symbol = "-",
		type_error(lhs, rhs) = {
			EvalError::BinOpTypeMismatch {
				details: format!("cannot subtract {} from {}", rhs.ty(), lhs.ty()).into(),
			}
		},
		int = checked_sub,
		float(lhs, rhs) =
			{ this.checked_float_result("subtraction", lhs, rhs, |lhs, rhs| lhs - rhs) },
	);

	checked_numeric_method!(
		checked_mul,
		this,
		op_name = "multiplication",
		symbol = "*",
		type_error(lhs, rhs) = {
			EvalError::BinOpTypeMismatch {
				details: format!("cannot multiply {} by {}", lhs.ty(), rhs.ty()).into(),
			}
		},
		int = checked_mul,
		float(lhs, rhs) =
			{ this.checked_float_result("multiplication", lhs, rhs, |lhs, rhs| lhs * rhs) },
	);

	checked_zero_numeric_method!(
		checked_div,
		this,
		op_name = "division",
		symbol = "/",
		type_error(lhs, rhs) = {
			EvalError::BinOpTypeMismatch {
				details: format!("cannot divide {} by {}", lhs.ty(), rhs.ty()).into(),
			}
		},
		int = checked_div,
		float(lhs, rhs) = { this.checked_float_result("division", lhs, rhs, |lhs, rhs| lhs / rhs) },
		zero = "cannot divide {} by zero",
	);

	checked_zero_numeric_method!(
		checked_rem,
		this,
		op_name = "remainder",
		symbol = "%",
		type_error(lhs, rhs) = {
			EvalError::BinOpTypeMismatch {
				details: format!("cannot take {} % {}", lhs.ty(), rhs.ty()).into(),
			}
		},
		int = checked_rem,
		float(lhs, rhs) =
			{ this.checked_float_result("remainder", lhs, rhs, |lhs, rhs| lhs % rhs) },
		zero = "cannot calculate {} % 0",
	);
}
