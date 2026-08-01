use std::{
	collections::HashSet,
	io::{self, Read},
	path::PathBuf,
	process::ExitCode,
	rc::Rc,
};

use clap::{ArgAction, Parser, ValueEnum};
use mix::{
	bytecode::PrettyProgram,
	files::FileLoader,
	runtime::{
		Runtime,
		lazy::{LazyValue, LazyValueKind},
		pretty::{PrettyLazyValue, PrettyValue},
		scope::ScopeBuilder,
		value::Value,
	},
};

const EXPR_NAME: &str = "<expression>";
const STDIN_NAME: &str = "<stdin>";

#[derive(Debug, Parser)]
#[command(
	name = "mix",
	version,
	about = "Evaluate Mix expressions and files",
	arg_required_else_help = true,
	group(clap::ArgGroup::new("input").required(true).args(["file", "expr"]))
)]
struct Cli {
	/// A Mix source file to evaluate. Use '-' to read stdin.
	#[arg(value_name = "FILE", conflicts_with = "expr")]
	file: Option<PathBuf>,

	/// Evaluate an expression supplied on the command line.
	#[arg(short = 'e', long, value_name = "EXPR", conflicts_with = "file")]
	expr: Option<String>,

	/// How to render the result.
	#[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Pretty)]
	format: OutputFormat,

	/// Recursively force lists and attribute sets (the default).
	#[arg(long, action = ArgAction::SetTrue, conflicts_with = "no_deep")]
	deep: bool,

	/// Only evaluate the outermost value, leaving children lazy.
	#[arg(long = "no-deep", action = ArgAction::SetTrue)]
	no_deep: bool,

	/// Expose builtins.import. Imports are disabled by default.
	#[arg(long)]
	allow_imports: bool,

	/// Print compiled bytecode to stderr before the result.
	#[arg(long)]
	dump_bytecode: bool,

	/// Print the lazy value to stderr before evaluation.
	#[arg(long)]
	show_lazy: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
	/// Mix syntax, intended for people.
	Pretty,
	/// JSON (requires a deeply evaluated, data-only result).
	Json,
	/// Rust's structural debug representation.
	Debug,
}

enum Input {
	File(PathBuf),
	Virtual { name: PathBuf, source: Rc<String> },
}

fn main() -> ExitCode {
	let result = run(Cli::parse());
	dumpster::unsync::collect();
	match result {
		Ok(()) => ExitCode::SUCCESS,
		Err(message) => {
			eprintln!("{message}");
			ExitCode::FAILURE
		}
	}
}

fn run(cli: Cli) -> Result<(), String> {
	let input = match (cli.expr, cli.file) {
		(Some(source), None) => Input::Virtual {
			name: PathBuf::from(EXPR_NAME),
			source: Rc::new(source),
		},
		(None, Some(path)) if path.as_os_str() == "-" => {
			let mut source = String::new();
			io::stdin()
				.read_to_string(&mut source)
				.map_err(|error| format!("failed to read stdin: {error}"))?;
			Input::Virtual {
				name: PathBuf::from(STDIN_NAME),
				source: Rc::new(source),
			}
		}
		(None, Some(path)) => Input::File(path),
		(None, None) => unreachable!("clap requires an input"),
		(Some(_), Some(_)) => unreachable!("clap rejects conflicting inputs"),
	};

	let (entry, entry_source) = match input {
		Input::File(path) => {
			let source = std::fs::read_to_string(&path)
				.map_err(|error| format!("cannot read {}: {error}", path.display()))?;
			(path.clone(), (path, Rc::new(source)))
		}
		Input::Virtual { name, source } => (name.clone(), (name, source)),
	};

	let loader = FileLoader::new(move |path| {
		if path == entry_source.0 {
			return Ok(entry_source.1.clone());
		}
		std::fs::read_to_string(path)
			.map(Rc::new)
			.map_err(|error| format!("{}: {error}", path.display()).into())
	});
	let scope = ScopeBuilder::new()
		.with_builtins_and_imports(cli.allow_imports)
		.with("false", false)
		.with("true", true)
		.bottom();
	let mut runtime = Runtime::new(loader.clone(), scope);
	let entry = entry.to_string_lossy();
	let lazy = runtime.load(&entry).map_err(|reports| {
		reports
			.render(&loader.files())
			.into_iter()
			.map(|report| report.to_string())
			.collect::<Vec<_>>()
			.join("\n")
	})?;

	if cli.dump_bytecode {
		eprintln!("{}", PrettyProgram::new(&runtime.program, &loader));
	}
	if cli.show_lazy {
		eprintln!("{}", PrettyLazyValue::new(&runtime, &lazy));
	}

	let deep = cli.deep || !cli.no_deep;
	if matches!(cli.format, OutputFormat::Json) && !deep {
		return Err("--format json cannot be combined with --no-deep".into());
	}
	let value = runtime
		.eval_lazy(lazy, deep)
		.map_err(|trace| trace.render(&runtime))?;

	match cli.format {
		OutputFormat::Pretty => println!("{}", PrettyValue::new(&runtime, &value)),
		OutputFormat::Debug => println!("{value:#?}"),
		OutputFormat::Json => {
			let json = value_to_json(&value, &mut HashSet::new())?;
			println!(
				"{}",
				serde_json::to_string_pretty(&json)
					.map_err(|error| format!("failed to encode JSON: {error}"))?
			);
		}
	}
	Ok(())
}

fn value_to_json(
	value: &Value,
	visiting: &mut HashSet<usize>,
) -> Result<serde_json::Value, String> {
	Ok(match value {
		Value::Bool(value) => (*value).into(),
		Value::Int(value) => (*value).into(),
		Value::Float(value) => serde_json::Number::from_f64(*value)
			.map(serde_json::Value::Number)
			.ok_or_else(|| "cannot represent a non-finite float as JSON".to_owned())?,
		Value::String(value) => value.to_string().into(),
		Value::Path(value) => value.display().to_string().into(),
		Value::Lambda(_) => return Err("cannot represent a function as JSON".into()),
		Value::List(list) => {
			if !visiting.insert(list.id()) {
				return Err("cannot represent a cyclic list as JSON".into());
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
				return Err("cannot represent a cyclic attribute set as JSON".into());
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
) -> Result<serde_json::Value, String> {
	match value.try_get_value() {
		LazyValueKind::Value(value) => value_to_json(&value, visiting),
		LazyValueKind::Thunk(_) => {
			Err("encountered an unevaluated value while producing JSON".into())
		}
	}
}
