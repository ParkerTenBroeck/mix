use crate::{
    files::Span,
    runtime::{
        Runtime,
        eval::{EvalError, Evaluator, FrameKind as EvalFrameKind, PotentialFrame},
    },
};

pub struct ErrorTrace<'a> {
    pub kind: EvalError<'a>,
    pub stack: Vec<FrameInfo>,
}

impl<'a> ErrorTrace<'a> {
    pub fn render(&self, runtime: &Runtime<'a>) -> String {
        use annotate_snippets::{Group, Level, Renderer};

        let renderer =
            Renderer::styled().decor_style(annotate_snippets::renderer::DecorStyle::Unicode);
        let title = match &self.kind {
            EvalError::Custom(message) => message.to_string(),
            EvalError::ByteCode(message) => format!("bytecode error: {message}"),
            EvalError::ThunkEval(thunk_eval_err) => match thunk_eval_err {
                crate::runtime::thunk::ThunkEvalErr::InfiniteRec => "infinite recursion",
                crate::runtime::thunk::ThunkEvalErr::NotConstructed => "trying to access partially constructed value.. this indicates an error in the compiler/bytecode/runtime",
                crate::runtime::thunk::ThunkEvalErr::AlreadyEvaluated => "trying to re-evaluate already evaluated thunk.. this indicates an error in the compiler/bytecode/runtime",
            }.into(),
        };

        let mut frames = self.stack.iter().rev();
        let Some(frame) = frames.next() else {
            let group = Group::with_title(Level::ERROR.primary_title(title));
            return renderer.render(&[group]);
        };

        let mut groups = vec![render_frame(
            runtime,
            frame,
            Level::ERROR.primary_title(title),
            if matches!(frame.kind, FrameKind::Fn) {
                "function call failed here"
            } else {
                "evaluation failed here"
            },
        )];

        groups.extend(frames.map(|frame| {
            let title = match frame.kind {
                FrameKind::Fn => "called from here",
                FrameKind::LazyEval => "while evaluating this expression",
            };
            let label = match frame.kind {
                FrameKind::Fn => "function call",
                FrameKind::LazyEval => "lazy value forced here",
            };
            render_frame(runtime, frame, Level::ERROR.secondary_title(title), label)
        }));

        renderer.render(&groups)
    }

    pub fn build(eval: &super::eval::Evaluator<'a, '_>, kind: EvalError<'a>) -> Self {
        Self {
            kind,
            stack: Self::build_trace(eval),
        }
    }

    fn build_trace(eval: &Evaluator<'_, '_>) -> Vec<FrameInfo> {
        let mut stack: Vec<FrameInfo> = eval
            .frame_stack
            .iter()
            .filter_map(|frame| match frame {
                PotentialFrame::Realized(frame) => Some(FrameInfo {
                    span: eval.runtime.program.find_pos(frame.pos),
                    kind: map_frame_kind(&frame.kind),
                }),
                PotentialFrame::PotentialDeep(_) => None,
            })
            .collect();

        stack.push(FrameInfo {
            span: eval.runtime.program.find_pos(eval.curr_frame.pos),
            kind: map_frame_kind(&eval.curr_frame.kind),
        });

        stack
    }
}

fn map_frame_kind(kind: &EvalFrameKind) -> FrameKind {
    match kind {
        EvalFrameKind::Function => FrameKind::Fn,
        EvalFrameKind::ThunkEval(_)
        | EvalFrameKind::ThunkEvalDeep(_)
        | EvalFrameKind::ThunkEvalDeepRoot(_) => FrameKind::LazyEval,
    }
}

fn render_frame<'a>(
    runtime: &Runtime<'a>,
    frame: &FrameInfo,
    title: annotate_snippets::Title<'a>,
    label: &'static str,
) -> annotate_snippets::Group<'a> {
    use annotate_snippets::{AnnotationKind, Snippet};

    let (path, source) = runtime.loader.file(frame.span.fid);
    let annotation = AnnotationKind::Primary
        .span(frame.span.range.into())
        .label(label);
    let snippet = Snippet::source(source)
        .path(path.display().to_string())
        .annotation(annotation);

    annotate_snippets::Group::with_title(title).element(snippet)
}

pub enum FrameKind {
    Fn,
    LazyEval,
}

pub struct FrameInfo {
    pub span: Span,
    pub kind: FrameKind,
}
