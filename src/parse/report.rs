use crate::{
    lex::LexError,
    parse::{ParseError, ast::Node},
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
        let render = |&Node(ref err, range)| {
            let groups: &[Group<'_>] = match err {
                ParseError::Lex(lex_error) => {
                    let title = match lex_error {
                        LexError::UnexpectedChar(char) => format!("unexpected char {char:?}"),
                        LexError::UnclosedComment => format!("unclosed comment"),
                        LexError::UnclosedString => format!("unclosed string literal"),
                        LexError::NumberError => {
                            format!("number cannot contain more than one decimal")
                        }
                    };
                    &[Level::ERROR.primary_title(title).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(range.into())),
                    )]
                }
                ParseError::UnclosedDelim { opening, closing } => {
                    let title = "unclosed delimiter";
                    &[
                        Level::ERROR.primary_title(title).element(
                            Snippet::source(file)
                                .path(path)
                                .annotation(AnnotationKind::Primary.span(closing.1.into()))
                                .annotation(
                                    AnnotationKind::Context
                                        .span(opening.1.into())
                                        .label("opened here"),
                                ),
                        ),
                        Level::HELP
                            .secondary_title("consider closing here")
                            .element(
                                Snippet::source(file)
                                    .patch(Patch::new(closing.1.into(), closing.0.closing())),
                            ),
                    ]
                }
                ParseError::MismatchedDelim { opening, closing } => &[
                    Level::ERROR.primary_title("mismatched delimiter").element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(closing.1.into()))
                            .annotation(
                                AnnotationKind::Context
                                    .span(opening.1.into())
                                    .label("opened here"),
                            ),
                    ),
                    Level::HELP
                        .secondary_title("use correct delimiter")
                        .element(
                            Snippet::source(file)
                                .patch(Patch::new(closing.1.into(), opening.0.closing())),
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
                            .annotation(AnnotationKind::Primary.span(range.start..range.start)),
                    )]
                }
                ParseError::UnexpectedTokenExpr(token) => {
                    let title = format!("expected token in expr {token:#}");
                    &[Level::ERROR.primary_title(title).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(range.into())),
                    )]
                }
                ParseError::UnexpectedTokenAttrPath(token) => todo!(),
                ParseError::FuncAppInList { func } => &[
                    Level::ERROR
                        .primary_title("function application in list")
                        .element(
                            Snippet::source(file)
                                .path(path)
                                .annotation(AnnotationKind::Primary.span(range.into())),
                        ),
                    Level::HELP
                        .secondary_title("consider wrapping in parenthesis if intended")
                        .element(
                            Snippet::source(file)
                                .patch(Patch::new(range.start..range.start, "("))
                                .patch(Patch::new(range.end..range.end, ")")),
                        ),
                    Level::HELP
                        .secondary_title("or add comma if not")
                        .element(Snippet::source(file).patch(Patch::new(func.end..func.end, ","))),
                ],
                ParseError::FuncDefInList => &[
                    Level::ERROR
                        .primary_title("function declaration in list")
                        .element(
                            Snippet::source(file)
                                .path(path)
                                .annotation(AnnotationKind::Primary.span(range.into())),
                        ),
                    Level::HELP
                        .secondary_title("consider wrapping in parenthesis")
                        .element(
                            Snippet::source(file)
                                .patch(Patch::new(range.start..range.start, "("))
                                .patch(Patch::new(range.end..range.end, ")")),
                        ),
                ],
                ParseError::ExpectedEof(token) => {
                    let title = format!("expected eof but got token {token:#}");
                    &[Level::ERROR.primary_title(title).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(range.into())),
                    )]
                }
                ParseError::FloatErr(err) => {
                    &[Level::ERROR.primary_title(format!("{err}")).element(
                        Snippet::source(file)
                            .path(path)
                            .annotation(AnnotationKind::Primary.span(range.into())),
                    )]
                }
                ParseError::IntErr(err) => &[Level::ERROR.primary_title(format!("{err}")).element(
                    Snippet::source(file)
                        .path(path)
                        .annotation(AnnotationKind::Primary.span(range.into())),
                )],
            };
            renderer.render(groups)
        };
        self.reports.iter().map(render).collect()
    }
}
