use mix::{
    files::Files,
    runtime::{Runtime, scope::ScopeBuilder, value::Value},
};

fn main() {
    let files = Files::new(|path| match std::fs::read_to_string(path) {
        Ok(ok) => Ok(ok.into()),
        Err(err) => Err(format!("{}: {err}", path.display()).into()),
    });

    let scope = ScopeBuilder::new()
        .with("null", Value::Null)
        .with("false", false)
        .with("true", true)
        .build();

    let mut runtime = Runtime::new(&files, scope);
    let res = match runtime.load("test2.mix") {
        Ok(ok) => ok,
        Err(reports) => {
            for report in reports.render(&files) {
                println!("{report}")
            }
            return;
        }
    };
    println!("{}", runtime.pretty_lazy(&res));
    let res = runtime.deep_eval(res);
    match res {
        Ok(ok) => println!("{}", runtime.pretty_value(&ok)),
        Err(trace) => println!("{}", trace.render(&runtime)),
    }
}
