use crate::{
    files::Span, runtime::{Runtime, eval::{EvalError, Evaluator}},
};

pub struct ErrorTrace<'a> {
    pub kind: EvalError<'a>,
    pub stack: Vec<Frame>,
}

impl<'a> ErrorTrace<'a> {
    pub fn render(&self, runtime: &Runtime<'a>) -> String {
        use annotate_snippets::{Group, Level, Renderer};

        let renderer =
            Renderer::styled().decor_style(annotate_snippets::renderer::DecorStyle::Unicode);
        let title = match &self.kind {
            EvalError::Custom(message) => message.to_string(),
            EvalError::ByteCode(message) => format!("bytecode error: {message}"),
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
            let title = match frame.kind{
                FrameKind::Fn => "called from here",
                FrameKind::LazyEval => "while evaluating this expression",
            };
            let label = match frame.kind{
                FrameKind::Fn => "function call",
                FrameKind::LazyEval => "lazy value forced here",
            };
            render_frame(
                runtime,
                frame,
                Level::ERROR.secondary_title(title),
                label,
            )
        }));

        renderer.render(&groups)
    }
    
    pub fn build(eval: &super::eval::Evaluator<'a, '_>, kind: EvalError<'a>)  -> Self {
        Self { kind, stack: Self::build_trace(eval) }
    }

    fn build_trace(eval: &Evaluator<'_, '_>) -> Vec<Frame> {
        let mut stack: Vec<Frame> = eval
            .call_stack
            .iter()
            .map(|(pos, _, kind)| Frame {
                span: eval.runtime.program.find_pos(*pos),
                kind: FrameKind::Fn,
            })
            .collect();

        // stack.push(Frame {
        //     span: eval.runtime.program.find_pos(self.pos),
        //     is_fn: self
        //         .call_stack
        //         .last()
        //         .is_some_and(|(_, _, kind)| matches!(kind, LazyUpdate::None)),
        //     kind: todo!(),
        // });

        stack
    }
}

fn render_frame<'a>(
    runtime: &Runtime<'a>,
    frame: &Frame,
    title: annotate_snippets::Title<'a>,
    label: &'static str,
) -> annotate_snippets::Group<'a> {
    use annotate_snippets::{AnnotationKind, Snippet};

    let (path, source) = runtime.loader.file(frame.span.fid);
    let annotation = AnnotationKind::Primary
        .span(frame.span.range.clone().into())
        .label(label);
    let snippet = Snippet::source(source)
        .path(path.display().to_string())
        .annotation(annotation);

    annotate_snippets::Group::with_title(title).element(snippet)
}


pub enum FrameKind{
    Fn,
    LazyEval,
}

pub struct Frame {
    pub span: Span,
    pub kind: FrameKind,
}
