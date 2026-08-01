use crate::{
	files::{Files, Span},
	runtime::{
		Runtime,
		eval::{
			EvalError, Evaluator, Frame as EvalFrame, FrameKind as EvalFrameKind, NativePosKind,
			ThunkEvalErr,
		},
	},
};

pub struct ErrorTrace {
	pub kind: EvalError,
	pub stack: Vec<FrameInfo>,
}

impl ErrorTrace {
	pub fn render(&self, runtime: &Runtime) -> String {
		use annotate_snippets::{Group, Level, Renderer};
		let files = &runtime.loader.files();
		let nested_reports = match &self.kind {
			EvalError::Reports(reports) => Some(reports.render(&files).join("\n")),
			_ => None,
		};

		let renderer =
			Renderer::styled().decor_style(annotate_snippets::renderer::DecorStyle::Unicode);
		let title = error_title(&self.kind);

		if self.stack.is_empty() {
			if let Some(reports) = nested_reports {
				return reports;
			}
			let group = Group::with_title(Level::ERROR.primary_title(title));
			return renderer.render(&[group]);
		}

		let has_nested_report = nested_reports.is_some();
		let last = self.stack.len() - 1;
		let mut groups = Vec::with_capacity(self.stack.len());
		for (index, frame) in self.stack.iter().enumerate() {
			let is_error = index == last && !has_nested_report;
			let (frame_title, label) = if is_error {
				(frame.kind.error_title(&title), frame.kind.error_label())
			} else {
				(frame.kind.context_title(), frame.kind.context_label())
			};
			let annotation = if is_error {
				Level::ERROR.primary_title(frame_title)
			} else {
				Level::ERROR.secondary_title(frame_title)
			};
			groups.push(render_frame(&files, frame, annotation, label));
		}

		let trace = renderer.render(&groups);
		match nested_reports {
			Some(reports) => format!("{trace}\n{reports}"),
			None => trace,
		}
	}

	pub fn build(runtime: &Runtime, eval: &Evaluator, kind: EvalError) -> Self {
		Self {
			kind,
			stack: Self::build_trace(runtime, eval),
		}
	}

	fn build_trace(runtime: &Runtime, eval: &Evaluator) -> Vec<FrameInfo> {
		eval.frames
			.iter()
			.filter_map(|frame| frame_info(runtime, frame))
			.collect()
	}
}

fn error_title(error: &EvalError) -> String {
	match error {
		EvalError::Custom(message) => message.to_string(),
		EvalError::Reports(_) => "evaluation failed".into(),
		EvalError::TypeMismatch { expected, got } => {
			format!("type mismatch: expected {expected}, got {got}")
		}
		EvalError::BinOpTypeMismatch { details } => details.to_string(),
		EvalError::Arithmetic(message)
		| EvalError::MissingAttr(message)
		| EvalError::MissingBinding(message) => message.to_string(),
		EvalError::Internal(message) => format!("internal runtime error: {message}"),
		EvalError::ByteCode(message) => format!("bytecode error: {message}"),
		EvalError::ThunkEval(ThunkEvalErr::InfiniteRec) => "infinite recursion".into(),
		EvalError::ThunkEval(ThunkEvalErr::NotConstructed) =>
			"trying to access partially constructed value; this indicates a compiler or runtime error".into(),
		EvalError::ThunkEval(ThunkEvalErr::AlreadyEvaluated) =>
			"trying to re-evaluate an evaluated thunk; this indicates a compiler or runtime error".into(),
	}
}

fn frame_info(runtime: &Runtime, frame: &EvalFrame) -> Option<FrameInfo> {
	match &frame.kind {
		EvalFrameKind::ByteCode(bytecode) => Some(FrameInfo {
			span: runtime.program.find_pos(bytecode.pos),
			kind: if frame.thunk.is_some() {
				FrameKind::LazyEval
			} else {
				FrameKind::Fn
			},
		}),
		EvalFrameKind::Native(native) if native.name == "deep eval" => {
			let (pos, kind) = match native.pos {
				NativePosKind::Value(pos) => (Some(pos), FrameKind::DeepEvalValue),
				NativePosKind::Expr(pos) => (Some(pos), FrameKind::DeepEvalExpr),
				NativePosKind::None => (None, FrameKind::DeepEval),
			};
			Some(FrameInfo {
				span: pos.and_then(|pos| runtime.program.find_pos(pos)),
				kind,
			})
		}
		EvalFrameKind::Native(native) => Some(FrameInfo {
			span: match native.pos {
				NativePosKind::Value(pos) | NativePosKind::Expr(pos) => {
					runtime.program.find_pos(pos)
				}
				NativePosKind::None => None,
			},
			kind: FrameKind::NativeFn(native.name.clone()),
		}),
	}
}

fn render_frame<'a>(
	files: &'a Files<'a>,
	frame: &FrameInfo,
	title: annotate_snippets::Title<'a>,
	label: &'static str,
) -> annotate_snippets::Group<'a> {
	use annotate_snippets::{AnnotationKind, Snippet};

	let Some(span) = frame.span else {
		return annotate_snippets::Group::with_title(title);
	};
	let (path, source) = files.file(span.fid);
	let annotation = AnnotationKind::Primary.span(span.range.into()).label(label);
	let snippet = Snippet::source(&**source)
		.path(path.display().to_string())
		.annotation(annotation);

	annotate_snippets::Group::with_title(title).element(snippet)
}

pub enum FrameKind {
	Fn,
	NativeFn(std::borrow::Cow<'static, str>),
	LazyEval,
	DeepEvalValue,
	DeepEvalExpr,
	DeepEval,
}

impl FrameKind {
	fn context_title(&self) -> String {
		match self {
			Self::Fn => "called from here".into(),
			Self::NativeFn(name) => format!("while calling native function \"{name}\""),
			Self::LazyEval => "while evaluating this expression".into(),
			Self::DeepEvalValue => "while deeply evaluating this value".into(),
			Self::DeepEvalExpr => "while deeply evaluating this expression".into(),
			Self::DeepEval => "while deeply evaluating a value".into(),
		}
	}

	fn context_label(&self) -> &'static str {
		match self {
			Self::Fn => "function call",
			Self::NativeFn(_) => "native function call",
			Self::LazyEval => "lazy value forced here",
			Self::DeepEvalValue => "value created here",
			Self::DeepEvalExpr => "expression evaluated here",
			Self::DeepEval => "deep evaluation",
		}
	}

	fn error_title(&self, title: &str) -> String {
		match self {
			Self::NativeFn(name) => format!("{title} (in native function \"{name}\")"),
			_ => title.to_owned(),
		}
	}

	fn error_label(&self) -> &'static str {
		match self {
			Self::Fn => "function call failed here",
			Self::NativeFn(_) => "native function call failed",
			Self::LazyEval => "evaluation failed here",
			Self::DeepEvalValue | Self::DeepEvalExpr => "deep evaluation failed here",
			Self::DeepEval => "deep evaluation failed",
		}
	}
}

pub struct FrameInfo {
	pub span: Option<Span>,
	pub kind: FrameKind,
}
