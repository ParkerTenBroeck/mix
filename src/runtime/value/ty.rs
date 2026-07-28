#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
	Number,
	Bool,
	Int,
	Float,
	String,
	Path,
	List,
	AttrSet,
	Lambda,
}

impl std::fmt::Display for ValueType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let name = match self {
			ValueType::Number => "number",
			ValueType::Bool => "bool",
			ValueType::Int => "int",
			ValueType::Float => "float",
			ValueType::String => "string",
			ValueType::Path => "path",
			ValueType::List => "list",
			ValueType::AttrSet => "attrset",
			ValueType::Lambda => "lambda",
		};
		f.write_str(name)
	}
}
