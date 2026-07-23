pub mod lexer;
pub mod mir;
pub mod parser;

use std::borrow::Cow;

use crate::files::{FileId, Files, Span};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reports {
	reports: Vec<Report>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
	pub level: ReportLevel,
	pub span: Span,
	pub title: Cow<'static, str>,
	pub annotations: Vec<ReportAnnotation>,
	pub helps: Vec<ReportHelp>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportLevel {
	Error,
	Warning,
	Info,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportAnnotation {
	pub kind: ReportAnnotationKind,
	pub span: Span,
	pub label: Option<Cow<'static, str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportAnnotationKind {
	Primary,
	Context,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportHelp {
	pub title: Cow<'static, str>,
	pub patches: Vec<ReportPatch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportPatch {
	pub span: Span,
	pub replacement: Cow<'static, str>,
}

#[derive(Clone, Debug)]
struct FileAnnotations {
	fid: FileId,
	annotations: Vec<FileAnnotation>,
}

#[derive(Clone, Debug)]
struct FileAnnotation {
	kind: ReportAnnotationKind,
	span: Span,
	label: Option<String>,
}

#[derive(Clone, Debug)]
struct FilePatches {
	fid: FileId,
	patches: Vec<FilePatch>,
}

#[derive(Clone, Debug)]
struct FilePatch {
	span: Span,
	replacement: String,
}

impl ReportAnnotation {
	pub fn primary(span: Span) -> Self {
		Self {
			kind: ReportAnnotationKind::Primary,
			span,
			label: None,
		}
	}

	pub fn context(span: Span, label: impl Into<Cow<'static, str>>) -> Self {
		Self {
			kind: ReportAnnotationKind::Context,
			span,
			label: Some(label.into()),
		}
	}
}

impl ReportHelp {
	pub fn new(title: impl Into<Cow<'static, str>>) -> Self {
		Self {
			title: title.into(),
			patches: vec![],
		}
	}

	pub fn with_patch(mut self, span: Span, replacement: impl Into<Cow<'static, str>>) -> Self {
		self.patches.push(ReportPatch {
			span,
			replacement: replacement.into(),
		});
		self
	}
}

impl Reports {
	pub fn emit(&mut self, report: impl Into<Report>) {
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

	pub fn render(&self, files: &Files<'_>) -> Vec<String> {
		self.reports
			.iter()
			.map(|report| report.render(files))
			.collect()
	}
}

impl Report {
	pub fn render(&self, files: &Files<'_>) -> String {
		use annotate_snippets::*;
		

		let renderer = Renderer::styled().decor_style(renderer::DecorStyle::Unicode);
		let annotation_groups = self.annotation_groups(files);
		let help_groups = self.help_groups(files);
		let groups: Vec<Group<'_>> = annotation_groups.into_iter().chain(help_groups).collect();

		renderer.render(&groups)
	}

	fn annotation_groups<'f>(&self, files: &'f Files<'f>) -> Vec<annotate_snippets::Group<'f>> {
		use annotate_snippets::{Annotation, AnnotationKind, Group, Snippet};

		self.group_annotations()
			.into_iter()
			.enumerate()
			.map(|(index, file_annotations)| {
				let (path, source) = files.file(file_annotations.fid);
				let annotations: Vec<Annotation<'f>> = file_annotations
					.annotations
					.into_iter()
					.map(|annotation| match annotation.kind {
						ReportAnnotationKind::Primary => {
							AnnotationKind::Primary.span(annotation.span.range.into())
						}
						ReportAnnotationKind::Context => {
							let kind = AnnotationKind::Context.span(annotation.span.range.into());
							match annotation.label {
								Some(label) => kind.label(label),
								None => kind,
							}
						}
					})
					.collect();
				let snippet = Snippet::source(&**source)
					.path(path.display().to_string())
					.annotations(annotations);

				if index == 0 {
					self.level
						.annotate_level()
						.primary_title(self.title.to_string())
						.element(snippet)
				} else {
					Group::with_level(self.level.annotate_level()).element(snippet)
				}
			})
			.collect()
	}

	fn help_groups<'f>(&self, files: &'f Files<'f>) -> Vec<annotate_snippets::Group<'f>> {
		use annotate_snippets::{Group, Level, Patch, Snippet};

		self.helps
			.iter()
			.flat_map(|help| {
				self.group_patches(help).into_iter().enumerate().map(
					move |(index, file_patches)| {
						let (path, source) = files.file(file_patches.fid);
						let snippet = Snippet::source(&**source)
							.path(path.display().to_string())
							.patches(file_patches.patches.into_iter().map(|patch| {
								Patch::new(patch.span.range.into(), patch.replacement)
							}));

						if index == 0 {
							Level::HELP
								.secondary_title(help.title.to_string())
								.element(snippet)
						} else {
							Group::with_level(Level::HELP).element(snippet)
						}
					},
				)
			})
			.collect()
	}

	fn group_annotations(&self) -> Vec<FileAnnotations> {
		let mut grouped = Vec::new();

		for annotation in &self.annotations {
			let Some(existing) = grouped
				.iter_mut()
				.find(|existing: &&mut FileAnnotations| existing.fid == annotation.span.fid)
			else {
				grouped.push(FileAnnotations {
					fid: annotation.span.fid,
					annotations: vec![FileAnnotation {
						kind: annotation.kind,
						span: annotation.span,
						label: annotation.label.as_ref().map(|label| label.to_string()),
					}],
				});
				continue;
			};

			existing.annotations.push(FileAnnotation {
				kind: annotation.kind,
				span: annotation.span,
				label: annotation.label.as_ref().map(|label| label.to_string()),
			});
		}

		grouped
	}

	fn group_patches(&self, help: &ReportHelp) -> Vec<FilePatches> {
		let mut grouped = Vec::new();

		for patch in &help.patches {
			let Some(existing) = grouped
				.iter_mut()
				.find(|existing: &&mut FilePatches| existing.fid == patch.span.fid)
			else {
				grouped.push(FilePatches {
					fid: patch.span.fid,
					patches: vec![FilePatch {
						span: patch.span,
						replacement: patch.replacement.to_string(),
					}],
				});
				continue;
			};

			existing.patches.push(FilePatch {
				span: patch.span,
				replacement: patch.replacement.to_string(),
			});
		}

		grouped
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
