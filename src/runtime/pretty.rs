use std::collections::{BTreeMap};
use std::fmt;
use crate::{HashMap, HashSet};

use crate::runtime::lazy::LazyValue;
use crate::{
	bytecode::CodePos,
	files::Span,
	runtime::{
		Runtime,
		thunk::{Thunk, ThunkSnapshot},
		value::{AttrSet, Lambda, List, Value},
	},
};

pub fn render_value(runtime: &Runtime<'_>, value: &Value) -> String {
	let mut printer = PrettyPrinter::new(runtime);
	printer.render_root_value(value)
}

pub fn render_lazy_value(runtime: &Runtime<'_>, value: &LazyValue) -> String {
	let mut printer = PrettyPrinter::new(runtime);
	printer.render_root_lazy(value)
}

struct PrettyPrinter<'rt, 'a> {
	runtime: &'rt Runtime<'a>,
	counts: HashMap<ObjectKey, usize>,
	expanded: HashSet<ObjectKey>,
	labels: HashMap<ObjectKey, usize>,
	next_label: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ObjectKey {
	Thunk(usize),
	List(usize),
	AttrSet(usize),
}

impl<'rt, 'a> PrettyPrinter<'rt, 'a> {
	fn new(runtime: &'rt Runtime<'a>) -> Self {
		Self {
			runtime,
			counts: HashMap::default(),
			expanded: HashSet::default(),
			labels: HashMap::default(),
			next_label: 1,
		}
	}

	fn render_root_value(&mut self, value: &Value) -> String {
		let mut seen = HashSet::default();
		self.count_value(value, &mut seen);
		self.render_value_inner(value, 0)
	}

	fn render_root_lazy(&mut self, value: &LazyValue) -> String {
		let mut seen = HashSet::default();
		self.count_lazy(value, &mut seen);
		self.render_lazy_inner(value, 0)
	}

	fn count_value(&mut self, value: &Value, seen: &mut HashSet<ObjectKey>) {
		match value {
			Value::List(list) => self.count_list(list, seen),
			Value::AttrSet(attrset) => self.count_attrset(attrset, seen),
			Value::Lambda(_) => {}
			Value::Bool(_)
			| Value::Int(_)
			| Value::Float(_)
			| Value::String(_)
			| Value::Path(_) => {}
		}
	}

	fn count_lazy(&mut self, value: &LazyValue, seen: &mut HashSet<ObjectKey>) {
		match value.try_get_value() {
			Err(thunk) => self.count_thunk(&thunk, seen),
			Ok(value) => self.count_value(&value, seen),
		}
	}

	fn count_thunk(&mut self, thunk: &Thunk, seen: &mut HashSet<ObjectKey>) {
		let key = ObjectKey::Thunk(thunk.id());
		*self.counts.entry(key).or_default() += 1;
		if !seen.insert(key) {
			return;
		}
		if let Some(ThunkSnapshot::Evaluated(value)) = thunk.snapshot() {
			self.count_value(&value, seen);
		}
	}

	fn count_list(&mut self, list: &List, seen: &mut HashSet<ObjectKey>) {
		let key = ObjectKey::List(list.id());
		*self.counts.entry(key).or_default() += 1;
		if !seen.insert(key) {
			return;
		}
		for value in list.iter() {
			self.count_lazy(value, seen);
		}
	}

	fn count_attrset(&mut self, attrset: &AttrSet, seen: &mut HashSet<ObjectKey>) {
		let key = ObjectKey::AttrSet(attrset.id());
		*self.counts.entry(key).or_default() += 1;
		if !seen.insert(key) {
			return;
		}
		for value in attrset.values() {
			self.count_lazy(value, seen);
		}
	}

	fn render_value_inner(&mut self, value: &Value, indent: usize) -> String {
		match value {
			Value::Bool(value) => value.to_string(),
			Value::Int(value) => value.to_string(),
			Value::Float(value) => value.to_string(),
			Value::String(value) => format!("{value:?}"),
			Value::Path(path) => path.display().to_string(),
			Value::List(list) => self.render_list(list, indent),
			Value::AttrSet(attrset) => self.render_attrset(attrset, indent),
			Value::Lambda(lambda) => self.render_lambda(lambda),
		}
	}

	fn render_lazy_inner(&mut self, value: &LazyValue, indent: usize) -> String {
		match value.try_get_value() {
			Err(thunk) => self.render_thunk(&thunk, indent),
			Ok(value) => self.render_value_inner(&value, indent),
		}
	}

	fn render_list(&mut self, list: &List, indent: usize) -> String {
		let key = ObjectKey::List(list.id());
		let prefix = self.shared_prefix(key);
		if self.should_collapse(key) {
			return prefix;
		}

		if list.is_empty() {
			return format!("{prefix}[ ]");
		}

		let mut out = String::new();
		out.push_str(&prefix);
		out.push('[');
		out.push('\n');
		for value in list.iter() {
			out.push_str(&"  ".repeat(indent + 1));
			out.push_str(&self.render_lazy_inner(value, indent + 1));
			out.push('\n');
		}
		out.push_str(&"  ".repeat(indent));
		out.push(']');
		out
	}

	fn render_attrset(&mut self, attrset: &AttrSet, indent: usize) -> String {
		let key = ObjectKey::AttrSet(attrset.id());
		let prefix = self.shared_prefix(key);
		if self.should_collapse(key) {
			return prefix;
		}

		if attrset.is_empty() {
			return format!("{prefix}{{ }}");
		}

		let mut attrs = BTreeMap::new();
		for (name, value) in attrset.iter() {
			attrs.insert(&**name, value);
		}

		let mut out = String::new();
		out.push_str(&prefix);
		out.push('{');
		out.push('\n');
		for (name, value) in attrs {
			out.push_str(&"  ".repeat(indent + 1));
			out.push_str(&format_attr_name(name));
			out.push_str(" = ");
			out.push_str(&self.render_lazy_inner(value, indent + 1));
			out.push(';');
			out.push('\n');
		}
		out.push_str(&"  ".repeat(indent));
		out.push('}');
		out
	}

	fn render_lambda(&self, lambda: &Lambda) -> String {
		match lambda {
			Lambda::Lambda { lambda, .. } => {
				if let Some(info) = self.runtime.program.get_lambda(*lambda) {
					format!("<<lambda {}>>", self.format_span(info.span))
				} else {
					"<<lambda>>".into()
				}
			}
		}
	}

	fn render_thunk(&mut self, thunk: &Thunk, indent: usize) -> String {
		let key = ObjectKey::Thunk(thunk.id());
		let prefix = self.shared_prefix(key);
		if self.should_collapse(key) {
			return prefix;
		}

		let Some(snapshot) = thunk.snapshot() else {
			return format!("{prefix}<<thunk busy>>");
		};

		match snapshot {
			ThunkSnapshot::Constructing(pos) => {
				format!(
					"{prefix}<<thunk constructing {}>>",
					self.format_code_pos(pos)
				)
			}
			ThunkSnapshot::Unevaluated(pos) => {
				format!(
					"{prefix}<<thunk unevaluated {}>>",
					self.format_code_pos(pos)
				)
			}
			ThunkSnapshot::Evaluating => format!("{prefix}<<thunk evaluating>>"),
			ThunkSnapshot::Evaluated(value) => {
				let value = self.render_value_inner(&value, indent);
				format!("{prefix}{value}")
			}
		}
	}

	fn shared_prefix(&mut self, key: ObjectKey) -> String {
		if self.counts.get(&key).copied().unwrap_or(0) <= 1 {
			return String::new();
		}
		let label = *self.labels.entry(key).or_insert_with(|| {
			let label = self.next_label;
			self.next_label += 1;
			label
		});
		format!("<<ref {label}>> ")
	}

	fn should_collapse(&mut self, key: ObjectKey) -> bool {
		self.counts.get(&key).copied().unwrap_or(0) > 1 && !self.expanded.insert(key)
	}

	fn format_code_pos(&self, pos: CodePos) -> String {
		self.format_span(self.runtime.program.find_pos(pos))
	}

	pub fn format_span(&self, span: Span) -> String {
		let (path, source) = self.runtime.loader.file(span.fid);
		let (start_line, start_col) = line_col(source, span.range.start);
		let (end_line, end_col) = line_col(source, span.range.end);
		if start_line == end_line {
			format!(
				"{}:{}:{}-{}",
				path.display(),
				start_line,
				start_col,
				end_col.max(start_col)
			)
		} else {
			format!(
				"{}:{}:{}-{}.{}",
				path.display(),
				start_line,
				start_col,
				end_line,
				end_col
			)
		}
	}
}

fn format_attr_name(name: &str) -> String {
	if is_ident(name) {
		name.to_string()
	} else {
		format!("{name:?}")
	}
}

fn is_ident(name: &str) -> bool {
	let mut chars = name.chars();
	match chars.next() {
		Some('a'..='z' | 'A'..='Z' | '_') => {}
		_ => return false,
	}
	chars.all(|char| matches!(char, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '\''))
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
	let mut line = 1usize;
	let mut col = 1usize;
	for (idx, ch) in source.char_indices() {
		if idx >= offset {
			break;
		}
		if ch == '\n' {
			line += 1;
			col = 1;
		} else {
			col += 1;
		}
	}
	(line, col)
}

pub struct PrettyValue<'rt, 'a> {
	runtime: &'rt Runtime<'a>,
	value: &'rt Value,
}

impl<'rt, 'a> PrettyValue<'rt, 'a> {
	pub fn new(runtime: &'rt Runtime<'a>, value: &'rt Value) -> Self {
		Self { runtime, value }
	}
}

impl fmt::Display for PrettyValue<'_, '_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&render_value(self.runtime, self.value))
	}
}

pub struct PrettyLazyValue<'rt, 'a> {
	runtime: &'rt Runtime<'a>,
	value: &'rt LazyValue,
}

impl<'rt, 'a> PrettyLazyValue<'rt, 'a> {
	pub fn new(runtime: &'rt Runtime<'a>, value: &'rt LazyValue) -> Self {
		Self { runtime, value }
	}
}

impl fmt::Display for PrettyLazyValue<'_, '_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&render_lazy_value(self.runtime, self.value))
	}
}
