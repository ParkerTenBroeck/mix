use crate::{
    files::Node, lex::LexError, parse::ParseError
};

#[derive(Clone, Debug, Default)]
pub struct Report<'a> {
    reports: Vec<Node<ParseError<'a>>>,
}

impl<'a> Report<'a> {
    pub fn push(&mut self, err: Node<ParseError<'a>>) {
        self.reports.push(err);
    }

    pub fn count_errors(&self) -> usize {
        self.reports.len()
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
        use annotate_snippets::*;
        let renderer = Renderer::styled().decor_style(renderer::DecorStyle::Unicode);
        let render = |&Node(ref err, span)| {
            let groups: &[Group<'_>] = match err {
                ParseError::Lex(lex_error) => {
                    let title = match lex_error {
                        LexError::UnexpectedChar(char) => format!("unexpected char {char:?}"),
                        LexError::UnclosedComment => "unclosed comment".to_string(),
                        LexError::UnclosedString => "unclosed string literal".to_string(),
                        LexError::NumberError => {
                            "number cannot contain more than one decimal".to_string()
                        }
                    };
                    &[Level::ERROR.primary_title(title).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(span.range.into())),
                    )]
                }
                ParseError::UnclosedDelim { opening, closing } => {
                    let title = "unclosed delimiter";
                    &[
                        Level::ERROR.primary_title(title).element(
                            Snippet::source(file)
                                .path(path)
                                .annotation(AnnotationKind::Primary.span(closing.1.range.into()))
                                .annotation(
                                    AnnotationKind::Context
                                        .span(opening.1.range.into())
                                        .label("opened here"),
                                ),
                        ),
                        Level::HELP
                            .secondary_title("consider closing here")
                            .element(
                                Snippet::source(file)
                                    .patch(Patch::new(closing.1.range.into(), closing.0.closing())),
                            ),
                    ]
                }
                ParseError::MismatchedDelim { opening, closing } => &[
                    Level::ERROR.primary_title("mismatched delimiter").element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(closing.1.range.into()))
                            .annotation(
                                AnnotationKind::Context
                                    .span(opening.1.range.into())
                                    .label("opened here"),
                            ),
                    ),
                    Level::HELP
                        .secondary_title("use correct delimiter")
                        .element(
                            Snippet::source(file)
                                .patch(Patch::new(closing.1.range.into(), opening.0.closing())),
                        ),
                ],
                ParseError::ExpectedClosingDelim(delim, token) => {
                    let title = format!(
                        "expected closing delim '{:}' but got {token:#}",
                        delim.closing()
                    );
                    &[Level::ERROR.primary_title(title).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(span.before().range.into())),
                    )]
                }
                ParseError::UnexpectedTokenExpr(token) => {
                    let title = format!("expected token in expr {token:#}");
                    &[Level::ERROR.primary_title(title).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(span.range.into())),
                    )]
                }
                ParseError::UnexpectedTokenAttrPath(token) => todo!(),
                ParseError::FuncAppInList { func } => &[
                    Level::ERROR
                        .primary_title("function application in list")
                        .element(
                            Snippet::source(file)
                                .path(path)
                                .annotation(AnnotationKind::Primary.span(span.range.into())),
                        ),
                    Level::HELP
                        .secondary_title("consider wrapping in parenthesis if intended")
                        .element(
                            Snippet::source(file)
                                .patch(Patch::new(span.before().range.into(), "("))
                                .patch(Patch::new(span.after().range.into(), ")")),
                        ),
                    Level::HELP.secondary_title("or add comma if not").element(
                        Snippet::source(file).patch(Patch::new(func.after().range.into(), ",")),
                    ),
                ],
                ParseError::FuncDefInList => &[
                    Level::ERROR
                        .primary_title("function declaration in list")
                        .element(
                            Snippet::source(file)
                                .path(path)
                                .annotation(AnnotationKind::Primary.span(span.range.into())),
                        ),
                    Level::HELP
                        .secondary_title("consider wrapping in parenthesis")
                        .element(
                            Snippet::source(file)
                                .patch(Patch::new(span.before().range.into(), "("))
                                .patch(Patch::new(span.after().range.into(), ")")),
                        ),
                ],
                ParseError::ExpectedEof(token) => {
                    let title = format!("expected eof but got token {token:#}");
                    &[Level::ERROR.primary_title(title).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(span.range.into())),
                    )]
                }
                ParseError::FloatErr(err) => {
                    &[Level::ERROR.primary_title(format!("{err}")).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(span.range.into())),
                    )]
                }
                ParseError::IntErr(err) => &[Level::ERROR.primary_title(format!("{err}")).element(
                    Snippet::source(file)
                        .path(path)
                        .annotation(AnnotationKind::Primary.span(span.range.into())),
                )],
            };
            renderer.render(groups)
        };
        self.reports.iter().map(render).collect()
    }
}
