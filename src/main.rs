use mix::runtime::{Runtime, Value, files::Files};

fn main() {
    let files = Files::new(|path| match std::fs::read_to_string(path) {
        Ok(ok) => Ok(ok.into()),
        Err(err) => Err(format!("{}: {err}", path.display()).into()),
    });

    // let top_scope = {
    //     let mut map: std::collections::HashMap<std::borrow::Cow<'_, str>, mix::runtime::LazyExpr<'_>> = Default::default();
    //     map.insert("null".into(), Value::Null.into());
    //     map.insert("false".into(), Value::Bool(false).into());
    //     map.insert("true".into(), Value::Bool(true).into());
    //     ExprScope::bottom(map)
    // };
    let mut runtime = Runtime::new(&files, Default::default());
    // println!("{runtime:#?}");
    let res = runtime.load("test.mix");
    println!("{res:#?}");
    // let res = runtime.eval(res);
    // println!("{res:#?}");
}
