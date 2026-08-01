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

		let renderer =
			Renderer::styled().decor_style(annotate_snippets::renderer::DecorStyle::Unicode);
		let title = match &self.kind {
			EvalError::Custom(message) => message.to_string(),
            EvalError::TypeMismatch { expected, got } => {
                format!("type mismatch: expected {expected}, got {got}")
            }
            EvalError::BinOpTypeMismatch { details } => details.to_string(),
            EvalError::Arithmetic(message) => message.to_string(),
            EvalError::MissingAttr(message) => message.to_string(),
            EvalError::MissingBinding(message) => message.to_string(),
            EvalError::Internal(message) => format!("internal runtime error: {message}"),
            EvalError::ByteCode(message) => format!("bytecode error: {message}"),
            EvalError::ThunkEval(thunk_eval_err) => match thunk_eval_err {
                ThunkEvalErr::InfiniteRec => "infinite recursion",
                ThunkEvalErr::NotConstructed => "trying to access partially constructed value.. this indicates an error in the compiler/bytecode/runtime",
                ThunkEvalErr::AlreadyEvaluated => "trying to re-evaluate already evaluated thunk.. this indicates an error in the compiler/bytecode/runtime",
            }.into(),
        };

		let mut frames = self.stack.iter().rev();
		let Some(frame) = frames.next() else {
			let group = Group::with_title(Level::ERROR.primary_title(title));
			return renderer.render(&[group]);
		};

		let title = match &frame.kind {
			FrameKind::NativeFn(name) => format!("{title} (in native function \"{name}\")"),
			_ => title,
		};
		let mut groups = vec![render_frame(
			&files,
			frame,
			Level::ERROR.primary_title(title),
			match &frame.kind {
				FrameKind::Fn => "function call failed here",
				FrameKind::NativeFn(_) => "native function call failed",
				FrameKind::LazyEval => "evaluation failed here",
				FrameKind::DeepEvalValue => "deep evaluation failed here",
				FrameKind::DeepEvalExpr => "deep evaluation failed here",
				FrameKind::DeepEval => "deep evaluation failed",
			},
		)];

		groups.extend(frames.map(|frame| {
			let title = match &frame.kind {
				FrameKind::Fn => "called from here".into(),
				FrameKind::NativeFn(name) => format!("while calling native function \"{name}\""),
				FrameKind::LazyEval => "while evaluating this expression".into(),
				FrameKind::DeepEvalValue => "while deeply evaluating this value".into(),
				FrameKind::DeepEvalExpr => "while deeply evaluating this expression".into(),
				FrameKind::DeepEval => "while deeply evaluating a value".into(),
			};
			let label = match &frame.kind {
				FrameKind::Fn => "function call",
				FrameKind::NativeFn(_) => "native function call",
				FrameKind::LazyEval => "lazy value forced here",
				FrameKind::DeepEvalValue => "value created here",
				FrameKind::DeepEvalExpr => "expression evaluated here",
				FrameKind::DeepEval => "deep evaluation",
			};
			render_frame(&files, frame, Level::ERROR.secondary_title(title), label)
		}));

		renderer.render(&groups)
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

pub struct FrameInfo {
	pub span: Option<Span>,
	pub kind: FrameKind,
}
