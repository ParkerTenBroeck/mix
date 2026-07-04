pub mod lexer;
pub mod parser;

use std::borrow::Cow;

use crate::files::Span;

#[derive(Clone, Debug, Default)]
pub struct Reports<'a> {
    reports: Vec<Report<'a>>,
}

#[derive(Clone, Debug)]
pub struct Report<'a> {
    pub level: ReportLevel,
    pub span: Span,
    pub title: Cow<'a, str>,
    pub annotations: Vec<ReportAnnotation<'a>>,
    pub helps: Vec<ReportHelp<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportLevel {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug)]
pub struct ReportAnnotation<'a> {
    pub kind: ReportAnnotationKind,
    pub span: Span,
    pub label: Option<Cow<'a, str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportAnnotationKind {
    Primary,
    Context,
}

#[derive(Clone, Debug)]
pub struct ReportHelp<'a> {
    pub title: Cow<'a, str>,
    pub patches: Vec<ReportPatch<'a>>,
}

#[derive(Clone, Debug)]
pub struct ReportPatch<'a> {
    pub span: Span,
    pub replacement: Cow<'a, str>,
}

impl<'a> ReportAnnotation<'a> {
    pub fn primary(span: Span) -> Self {
        Self {
            kind: ReportAnnotationKind::Primary,
            span,
            label: None,
        }
    }

    pub fn context(span: Span, label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            kind: ReportAnnotationKind::Context,
            span,
            label: Some(label.into()),
        }
    }
}

impl<'a> ReportHelp<'a> {
    pub fn new(title: impl Into<Cow<'a, str>>) -> Self {
        Self {
            title: title.into(),
            patches: vec![],
        }
    }

    pub fn with_patch(mut self, span: Span, replacement: impl Into<Cow<'a, str>>) -> Self {
        self.patches.push(ReportPatch {
            span,
            replacement: replacement.into(),
        });
        self
    }
}

impl<'a> Reports<'a> {
    pub fn emit(&mut self, report: impl Into<Report<'a>>) {
        self.reports.push(report.into());
    }

    pub fn has_errors(&self) -> bool {
        self.count_errors() != 0
    }

    fn count_errors(&self) -> usize {
        self.reports
            .iter()
            .filter(|report| report.level == ReportLevel::Error)
            .count()
    }

    pub fn state(&self) -> usize {
        self.reports.len()
    }

    pub fn restore(&mut self, count: usize) {
        while self.reports.len() > count {
            self.reports.pop();
        }
    }

    pub fn render(&self, path: &str, file: &str) -> Vec<String> {
        self.reports
            .iter()
            .map(|report| report.render(path, file))
            .collect()
    }
}

impl<'a> Report<'a> {
    pub fn render(&self, path: &str, file: &str) -> String {
        use annotate_snippets::*;

        let renderer = Renderer::styled().decor_style(renderer::DecorStyle::Unicode);
        let groups: Vec<Group<'_>> = std::iter::once(
            self.level
                .annotate_level()
                .primary_title(self.title.as_ref())
                .element(self.snippet(path, file)),
        )
        .chain(self.helps.iter().map(|help| {
            let snippet = help
                .patches
                .iter()
                .fold(Snippet::source(file), |snippet, patch| {
                    snippet.patch(Patch::new(
                        patch.span.range.into(),
                        patch.replacement.as_ref(),
                    ))
                });
            Level::HELP
                .secondary_title(help.title.as_ref())
                .element(snippet)
        }))
        .collect();

        renderer.render(&groups)
    }

    fn snippet<'s>(
        &'s self,
        path: &'s str,
        file: &'s str,
    ) -> annotate_snippets::Snippet<'s, annotate_snippets::Annotation<'s>>
    where
        'a: 's,
    {
        use annotate_snippets::{Annotation, AnnotationKind, Snippet};

        let annotations: Vec<Annotation<'_>> = self
            .annotations
            .iter()
            .map(|annotation| match annotation.kind {
                ReportAnnotationKind::Primary => {
                    AnnotationKind::Primary.span(annotation.span.range.into())
                }
                ReportAnnotationKind::Context => {
                    let kind = AnnotationKind::Context.span(annotation.span.range.into());
                    match annotation.label.as_deref() {
                        Some(label) => kind.label(label),
                        None => kind,
                    }
                }
            })
            .collect();

        Snippet::source(file).path(path).annotations(annotations)
    }
}

impl ReportLevel {
    fn annotate_level(self) -> annotate_snippets::Level<'static> {
        match self {
            ReportLevel::Error => annotate_snippets::Level::ERROR,
            ReportLevel::Warning => annotate_snippets::Level::WARNING,
            ReportLevel::Info => annotate_snippets::Level::INFO,
        }
    }
}
