use std::rc::Rc;

use crate::{
	files::{FileLoader, Node},
	parse::{Parser, ast::Expr},
};

use super::*;

fn parse(source: &str) -> Node<Expr<'_>> {
	let stored = Rc::new(source.to_owned());
	let loader = FileLoader::new({
		let stored = stored.clone();
		move |_| Ok(stored.clone())
	});
	let (_, fid) = loader.load("format-test.mix".as_ref()).unwrap();
	let (expr, reports) = Parser::parse(source, fid);
	assert!(!reports.has_errors(), "source failed to parse");
	expr.unwrap()
}

#[test]
fn formats_with_configurable_layout() {
	let expr = parse("let x={a=1,b=[2,3]}; in x");
	let formatted = format_ast(&expr, &FormatOptions::default());
	assert_eq!(formatted, "let\n  x = { a = 1, b = [2, 3] };\nin x\n");

	let options = FormatOptions {
		indent_style: IndentStyle::Tabs,
		attr_separator: AttrSeparatorStyle::Comma,
		let_separator: SeparatorStyle::Comma,
		trailing_separator: false,
		final_newline: false,
		..FormatOptions::default()
	};
	assert_eq!(
		format_ast(&expr, &options),
		"let\n\tx = {\n\t\ta = 1,\n\t\tb = [\n\t\t\t2,\n\t\t\t3\n\t\t]\n\t}\nin x"
	);
}

#[test]
fn formatted_output_parses_and_is_idempotent() {
	let source = r#"let f = x :: A -> B: x; value = { dynamic.${"name"} = 1.0; }; in if value ? dynamic.name then f value.dynamic.name else "\\s""#;
	let first = format_ast(&parse(source), &FormatOptions::default());
	let second = format_ast(&parse(&first), &FormatOptions::default());
	assert_eq!(first, second);
}

#[test]
fn smart_attrsets_choose_commas_or_semicolons() {
	let options = FormatOptions {
		max_inline_width: 24,
		..FormatOptions::default()
	};
	assert_eq!(format_ast(&parse("{a,b,c}"), &options), "{ a, b, c }\n");
	assert_eq!(
		format_ast(&parse("{firstAttribute=1,secondAttribute=2}"), &options),
		"{\n  firstAttribute = 1;\n  secondAttribute = 2;\n}\n"
	);
	assert_eq!(
		format_ast(&parse("{firstAttribute,secondAttribute}"), &options),
		"{\n  firstAttribute,\n  secondAttribute,\n}\n"
	);
}

#[test]
fn else_if_chains_stay_at_the_same_level() {
	let source = "if first then one else if second then two else if third then three else four";
	assert_eq!(
		format_ast(&parse(source), &FormatOptions::default()),
		"if first then one\nelse if second then two\nelse if third then three\nelse four\n"
	);
}

#[test]
fn conditional_call_branches_are_indented_below_their_headers() {
	let source = "if !next.ok then fatal next.error next.state else if next.token.kind == \"eof\" then fail \"end\" state else succeed next.token next.state";
	assert_eq!(
		format_ast(&parse(source), &FormatOptions::default()),
		"if !next.ok then\n  fatal next.error next.state\nelse if next.token.kind == \"eof\" then\n  fail \"end\" state\nelse\n  succeed next.token next.state\n"
	);
}
