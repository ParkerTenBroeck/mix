use std::fmt;

use crate::{
    bytecode::{CodePos, CodeLocOffset, OpCode, Program},
    files::{Files, Span},
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
            let arg = lambda
                .arg_name
                .map(|id| format!(" arg=#{} {:?}", id.index(), program.get_str(id)))
                .unwrap_or_default();
            out.push_str(&format!(
                "  lambda#{}{} @{} {}\n",
                idx + 1,
                arg,
                fmt_pos(lambda.code),
                format_span(files, lambda.span)
            ));
        }
    }

    out.push_str("\nops:\n");
    let mut expr_starts: Vec<_> = program.expressions().iter().collect();
    expr_starts.sort_by_key(|expr| expr.start.index());
    let mut next_expr = 0usize;
    let mut flow = FlowPrinter::default();

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
        out.push_str(&format!(
            "  {}  {}{: <24} {}\n",
            fmt_pos(pos),
            guide,
            format_op(program, pos, op),
            format_span(files, span)
        ));
        flow.observe(pos, op);
    }

    out
}

pub struct PrettyProgram<'a> {
    program: &'a Program,
    files: &'a Files,
}

impl<'a> PrettyProgram<'a> {
    pub fn new(program: &'a Program, files: &'a Files) -> Self {
        Self { program, files }
    }
}

impl fmt::Display for PrettyProgram<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&render_program(self.program, self.files))
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
        OpCode::InitAttrExpr(expr) => format!("InitAttrExpr @{}", fmt_pos(expr)),
        OpCode::FinalizeAttrSetRec => "FinalizeAttrSetRec".into(),
        OpCode::FinalizeAttrSet => "FinalizeAttrSet".into(),
        OpCode::CreateList(capacity) => format!("CreateList {capacity}"),
        OpCode::AppendList(expr) => format!("AppendList @{}", fmt_pos(expr)),
        OpCode::Apply(expr) => format!("Apply @{}", fmt_pos(expr)),
        OpCode::LoadLambda(lambda) => {
            let extra = program
                .get_lambda(lambda)
                .and_then(|lambda| lambda.arg_name)
                .map(|id| format!(" (#{} {:?})", id.index(), program.get_str(id)))
                .unwrap_or_default();
            format!("LoadLambda #{}{}", lambda.index(), extra)
        }
        OpCode::LoadStr(id) => format!("LoadStr #{} {:?}", id.index(), program.get_str(id)),
        OpCode::LoadInt(int) => format!("LoadInt {int}"),
        OpCode::LoadFloat(float) => format!("LoadFloat {float}"),
        OpCode::LoadBool(value) => format!("LoadBool {value}"),
        OpCode::LoadScope => "LoadScope".into(),
        OpCode::WithScope => "WithScope".into(),
        OpCode::LastScope => "LastScope".into(),
        OpCode::HasAttr => "HasAttr".into(),
        OpCode::GetAttr => "GetAttr".into(),
        OpCode::GetAttrOr(expr) => format!("GetAttrOr @{}", fmt_pos(expr)),
        OpCode::Branch(offset) => format_jump("Branch", pos, offset),
        OpCode::Ret => "Ret".into(),
    }
}

fn format_jump(name: &str, pos: CodePos, offset: CodeLocOffset) -> String {
    let target = pos + offset + CodeLocOffset(1);
    format!("{name} +{} -> {}", offset.len(), fmt_pos(target))
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

#[derive(Default)]
struct FlowPrinter {
    regions: Vec<FlowRegion>,
}

struct FlowRegion {
    end: usize,
    kind: FlowKind,
}

#[derive(Clone, Copy)]
enum FlowKind {
    Branch,
    Else,
}

impl FlowPrinter {
    fn guide_for(&mut self, pos: CodePos) -> String {
        let idx = pos.index();
        self.regions.retain(|region| region.end > idx);

        let mut guide = String::new();
        for region in &self.regions {
            match region.kind {
                FlowKind::Branch => guide.push_str("│  "),
                FlowKind::Else => guide.push_str("   "),
            }
        }
        guide
    }

    fn observe(&mut self, pos: CodePos, op: OpCode) {
        let start = pos.index() + 1;
        match op {
            OpCode::If(offset) => self.push_region(start, offset, FlowKind::Branch),
            OpCode::And(offset) | OpCode::Or(offset) | OpCode::LogImp(offset) => {
                self.push_region(start, offset, FlowKind::Branch)
            }
            OpCode::Branch(offset) => self.push_region(start, offset, FlowKind::Else),
            _ => {}
        }
    }

    fn push_region(&mut self, start: usize, offset: CodeLocOffset, kind: FlowKind) {
        let end = start + offset.len();
        if end > start {
            self.regions.push(FlowRegion { end, kind });
        }
    }
}
