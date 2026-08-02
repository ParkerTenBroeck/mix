use std::{
	collections::HashSet,
	io::{self, Read},
	path::PathBuf,
	process::ExitCode,
	rc::Rc,
};

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use mix::{
	bytecode::PrettyProgram,
	files::FileLoader,
	formatter::{AttrSeparatorStyle, FormatOptions, IndentStyle, SeparatorStyle, format_ast},
	lex::{Lexer, Token},
	parse::Parser as MixParser,
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
	arg_required_else_help = true
)]
struct Cli {
	#[command(subcommand)]
	command: Option<Command>,

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

#[derive(Debug, Subcommand)]
enum Command {
	/// Format Mix source using the parsed AST.
	Fmt(FmtArgs),
}

#[derive(Debug, clap::Args)]
struct FmtArgs {
	/// Source file to format, or '-' for stdin.
	#[arg(value_name = "FILE", default_value = "-")]
	file: PathBuf,

	/// Check formatting without writing changes.
	#[arg(long, conflicts_with = "stdout")]
	check: bool,

	/// Print formatted source instead of updating the file.
	#[arg(long)]
	stdout: bool,

	/// Indentation characters to use.
	#[arg(long, value_enum, default_value_t = CliIndentStyle::Spaces)]
	indent: CliIndentStyle,

	/// Spaces per indentation level; ignored with --indent tabs.
	#[arg(long, default_value_t = 2)]
	indent_width: usize,

	/// Separator used between and after attribute definitions.
	#[arg(long, value_enum, default_value_t = CliAttrSeparator::Smart)]
	attr_separator: CliAttrSeparator,

	/// Maximum line width for smart inline attribute sets.
	#[arg(long, default_value_t = 80)]
	max_inline_width: usize,

	/// Separator used between and after let bindings.
	#[arg(long, value_enum, default_value_t = CliSeparator::Semicolon)]
	let_separator: CliSeparator,

	/// Separator used between attribute-pattern fields.
	#[arg(long, value_enum, default_value_t = CliSeparator::Comma)]
	pattern_separator: CliSeparator,

	/// Omit a separator after the final item in a block.
	#[arg(long)]
	no_trailing_separator: bool,

	/// Omit the final newline.
	#[arg(long)]
	no_final_newline: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliIndentStyle {
	Spaces,
	Tabs,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliSeparator {
	Comma,
	Semicolon,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliAttrSeparator {
	Smart,
	Comma,
	Semicolon,
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
	if let Some(Command::Fmt(args)) = cli.command {
		return run_fmt(args);
	}

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
		(None, None) => return Err("provide a FILE, --expr, or the fmt command".into()),
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
	let lazy = runtime
		.load(&entry)
		.map_err(|error| error.render(&loader))?;

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

fn run_fmt(args: FmtArgs) -> Result<(), String> {
	let stdin = args.file.as_os_str() == "-";
	let source = if stdin {
		let mut source = String::new();
		io::stdin()
			.read_to_string(&mut source)
			.map_err(|error| format!("failed to read stdin: {error}"))?;
		source
	} else {
		std::fs::read_to_string(&args.file)
			.map_err(|error| format!("cannot read {}: {error}", args.file.display()))?
	};

	if contains_comments(&source) {
		return Err(
			"cannot format source containing comments: the parser does not preserve them".into(),
		);
	}

	let source = Rc::new(source);
	let loaded = source.clone();
	let loader = FileLoader::new(move |_| Ok(loaded.clone()));
	let (_, fid) = loader
		.load(args.file.as_path())
		.map_err(|error| error.into_owned())?;
	let (expr, reports) = MixParser::parse(&source, fid);
	let expr = expr.map_err(|()| reports.render(&loader.files()).join("\n"))?;
	let options = FormatOptions {
		indent_style: match args.indent {
			CliIndentStyle::Spaces => IndentStyle::Spaces,
			CliIndentStyle::Tabs => IndentStyle::Tabs,
		},
		indent_width: args.indent_width,
		attr_separator: args.attr_separator.into(),
		let_separator: args.let_separator.into(),
		pattern_separator: args.pattern_separator.into(),
		trailing_separator: !args.no_trailing_separator,
		final_newline: !args.no_final_newline,
		max_inline_width: args.max_inline_width,
	};
	let formatted = format_ast(&expr, &options);

	if args.check {
		if *source == formatted {
			return Ok(());
		}
		return Err(format!("{} is not formatted", args.file.display()));
	}
	if stdin || args.stdout {
		print!("{formatted}");
	} else if *source != formatted {
		std::fs::write(&args.file, formatted)
			.map_err(|error| format!("cannot write {}: {error}", args.file.display()))?;
	}
	Ok(())
}

fn contains_comments(source: &str) -> bool {
	let mut lexer = Lexer::new(source);
	loop {
		match lexer.next_tok().0 {
			Ok(Token::Comment(_)) => return true,
			Ok(Token::Eof) => return false,
			Ok(_) | Err(_) => {}
		}
	}
}

impl From<CliSeparator> for SeparatorStyle {
	fn from(value: CliSeparator) -> Self {
		match value {
			CliSeparator::Comma => Self::Comma,
			CliSeparator::Semicolon => Self::Semicolon,
		}
	}
}

impl From<CliAttrSeparator> for AttrSeparatorStyle {
	fn from(value: CliAttrSeparator) -> Self {
		match value {
			CliAttrSeparator::Smart => Self::Smart,
			CliAttrSeparator::Comma => Self::Comma,
			CliAttrSeparator::Semicolon => Self::Semicolon,
		}
	}
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
