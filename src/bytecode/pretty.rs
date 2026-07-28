use std::fmt;

use crate::{
	bytecode::{CodeLocOffset, CodePos, OpCode, Program},
	files::{FileLoader, Files, Span},
};

pub fn render_program(program: &Program, files: &Files) -> String {
	let mut out = String::new();
	out.push_str("== Bytecode ==\n");

	if let Some(expr) = program.expressions().first() {
		out.push_str(&format!(
			"top expr @{}..{} {}\n",
			fmt_pos(expr.start),
			fmt_pos(expr.end),
			format_span(files, expr.span)
		));
	}

	if !program.lambdas().is_empty() {
		out.push_str("\nlambdas:\n");
		for (idx, lambda) in program.lambdas().iter().enumerate() {
			out.push_str(&format!(
				"  lambda#{} @{} {}\n",
				idx + 1,
				fmt_pos(lambda.code),
				format_span(files, lambda.span)
			));
		}
	}

	out.push_str("\nops:\n");
	let mut expr_starts: Vec<_> = program.expressions().iter().collect();
	expr_starts.sort_by_key(|expr| expr.start.index());
	let mut next_expr = 0usize;
	let flow = FlowPrinter::new(program.ops());

	for (idx, op) in program.ops().iter().copied().enumerate() {
		let pos = CodePos::from_index(idx);
		let span = program.find_pos(pos);
		while next_expr < expr_starts.len() && expr_starts[next_expr].start.index() < idx {
			next_expr += 1;
		}
		if next_expr < expr_starts.len() && expr_starts[next_expr].start.index() == idx {
			if idx != 0 {
				out.push('\n');
			}
			let expr = expr_starts[next_expr];
			out.push_str(&format!(
				"  expr {}..{} {}\n",
				fmt_pos(expr.start),
				fmt_pos(expr.end),
				format_span(files, expr.span)
			));
			next_expr += 1;
		}

		let guide = flow.guide_for(pos);
		let source = span
			.map(|span| format_span(files, span))
			.unwrap_or_else(|| "<unknown source>".into());
		out.push_str(&format!(
			"  {}  {}{: <24} {}\n",
			fmt_pos(pos),
			guide,
			format_op(program, pos, op),
			source
		));
	}

	out
}

pub struct PrettyProgram<'a> {
	program: &'a Program,
	files: &'a FileLoader,
}

impl<'a> PrettyProgram<'a> {
	pub fn new(program: &'a Program, files: &'a FileLoader) -> Self {
		Self { program, files }
	}
}

impl fmt::Display for PrettyProgram<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&render_program(self.program, &self.files.files()))
	}
}

fn format_op(program: &Program, pos: CodePos, op: OpCode) -> String {
	match op {
		OpCode::Add => "Add".into(),
		OpCode::Sub => "Sub".into(),
		OpCode::Mul => "Mul".into(),
		OpCode::Div => "Div".into(),
		OpCode::Rem => "Rem".into(),
		OpCode::Eq => "Eq".into(),
		OpCode::Ne => "Ne".into(),
		OpCode::Lt => "Lt".into(),
		OpCode::Lte => "Lte".into(),
		OpCode::Gt => "Gt".into(),
		OpCode::Gte => "Gte".into(),
		OpCode::Not => "Not".into(),
		OpCode::Neg => "Neg".into(),
		OpCode::And(offset) => format_jump("And", pos, offset),
		OpCode::Or(offset) => format_jump("Or", pos, offset),
		OpCode::LogImp(offset) => format_jump("LogImp", pos, offset),
		OpCode::If(offset) => format_jump("If", pos, offset),
		OpCode::CreateAttrSet => "CreateAttrSet".into(),
		OpCode::SetAttr => "SetAttr".into(),
		OpCode::FinalizeAttrSetRec => "FinalizeAttrSetRec".into(),
		OpCode::CreateList(capacity) => format!("CreateList {capacity}"),
		OpCode::AppendList => "AppendList".into(),
		OpCode::Apply => "Apply".into(),
		OpCode::LoadLambda(lambda) => {
			format!("LoadLambda #{}", lambda.index())
		}
		OpCode::LoadStr(id) => format!("LoadStr #{} {:?}", id.index(), &*program.get_str(id)),
		OpCode::LoadInt(int) => format!("LoadInt {int}"),
		OpCode::LoadFloat(float) => format!("LoadFloat {float}"),
		OpCode::LoadBool(value) => format!("LoadBool {value}"),
		OpCode::LoadScope => "LoadScope".into(),
		OpCode::HasAttr => "HasAttr".into(),
		OpCode::GetAttr => "GetAttr".into(),
		OpCode::GetAttrOr(offset) => format_jump("GetAttrOr", pos, offset),
		OpCode::Branch(offset) => format_jump("Branch", pos, offset),
		OpCode::PopV => "PopV".into(),
		OpCode::DupV => "DupV".into(),
		OpCode::PopT => "PopT".into(),
		OpCode::DupT => "DupT".into(),
		OpCode::Ret => "Ret".into(),
		OpCode::EvalThunk => "EvalThunk".into(),
		OpCode::UnEvalValue => "UnEvalValue".into(),
		OpCode::BindThunkScope => "BindThunkScope".into(),
		OpCode::BindValueScope => "BindValueScope".into(),
		OpCode::CreateThunk(expr) => format!("CreateThunk @{}", fmt_pos(expr)),
		OpCode::BeginThunk(expr) => format!("BeginThunk @{}", fmt_pos(expr)),
		OpCode::FinalizeThunk => "FinalizeThunk".into(),
	}
}

fn format_jump(name: &str, pos: CodePos, offset: CodeLocOffset) -> String {
	let target = pos + offset + CodeLocOffset(1);
	format!("{name} +{} -> {}", offset.offset(), fmt_pos(target))
}

fn format_span(files: &Files, span: Span) -> String {
	let (path, source) = files.file(span.fid);
	let (start_line, start_col) = line_col(source, span.range.start);
	let (end_line, end_col) = line_col(source, span.range.end);
	let path = path.display();
	format!(
		"{}:{}:{}-{}:{}",
		path,
		start_line,
		start_col,
		end_line,
		end_col.max(start_col + usize::from(span.range.start == span.range.end))
	)
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

fn fmt_pos(pos: CodePos) -> String {
	format!("{:04}", pos.index())
}

struct FlowPrinter {
	edges: Vec<FlowEdge>,
	lanes: usize,
}

struct FlowEdge {
	source: usize,
	target: usize,
	lane: usize,
}

impl FlowPrinter {
	fn new(ops: &[OpCode]) -> Self {
		let mut edges = Vec::new();
		let mut lane_ends = Vec::new();

		for (source, op) in ops.iter().copied().enumerate() {
			let Some(offset) = branch_offset(op) else {
				continue;
			};
			let target = source + 1 + offset.offset();
			let lane = lane_ends
				.iter()
				.position(|end| *end < source)
				.unwrap_or_else(|| {
					lane_ends.push(0);
					lane_ends.len() - 1
				});
			lane_ends[lane] = target;
			edges.push(FlowEdge {
				source,
				target,
				lane,
			});
		}

		Self {
			edges,
			lanes: lane_ends.len(),
		}
	}

	fn guide_for(&self, pos: CodePos) -> String {
		let idx = pos.index();
		let mut guide = String::new();

		for lane in 0..self.lanes {
			let edge = self
				.edges
				.iter()
				.find(|edge| edge.lane == lane && edge.source <= idx && idx <= edge.target);
			match edge {
				Some(edge) if idx == edge.source => guide.push_str("┌─ "),
				Some(edge) if idx == edge.target => guide.push_str("└▶ "),
				Some(_) => guide.push_str("│  "),
				None => guide.push_str("   "),
			}
		}

		guide
	}
}

fn branch_offset(op: OpCode) -> Option<CodeLocOffset> {
	match op {
		OpCode::And(offset)
		| OpCode::Or(offset)
		| OpCode::LogImp(offset)
		| OpCode::If(offset)
		| OpCode::GetAttrOr(offset)
		| OpCode::Branch(offset) => Some(offset),
		_ => None,
	}
}
