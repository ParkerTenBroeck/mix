use mix::{
	bytecode::PrettyProgram, files::FileLoader, runtime::{Runtime, pretty::{PrettyLazyValue, PrettyValue}, scope::ScopeBuilder},
};

fn run() {
	let loader = FileLoader::new(|path| match std::fs::read_to_string(path) {
		Ok(ok) => Ok(ok.into()),
		Err(err) => Err(format!("{}: {err}", path.display()).into()),
	});

	let scope = ScopeBuilder::new()
		// .with("false", false)
		// .with("true", true)
		.bottom();

	let mut runtime = Runtime::new(loader.clone(), scope);
	let res = match runtime.load("test.mix") {
		Ok(ok) => ok,
		Err(reports) => {
			for report in reports.render(&loader.files()) {
				println!("{report}")
			}
			return;
		}
	};
	println!("{}", PrettyProgram::new(&runtime.program, &loader));
	
	println!("{}", PrettyLazyValue::new(&runtime, &res));
	let res = runtime.deep_eval(res);
	match res {
		Ok(ok) => println!("{}", PrettyValue::new(&runtime, &ok)),
		Err(trace) => println!("{}", trace.render(&runtime)),
	}
}

fn main() {
	run();
	dumpster::unsync::collect();
}
