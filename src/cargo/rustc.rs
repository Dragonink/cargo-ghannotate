//! Provides the structs to work with rustc's output

use super::{HandleMessage, SummaryWriter};
use crate::github::{Annotation, AnnotationKind};
use serde::Deserialize;
use std::{
	borrow::Cow,
	collections::HashMap,
	fmt::{self, Write as FmtWrite},
	io::{self, Write as IoWrite},
};

/// Message output by rustc
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Diagnostic<'m> {
	/// Primary message
	pub(crate) message: &'m str,
	/// Severity of the diagnostic
	pub(crate) level: DiagnosticLevel,
	/// Locations in the source code of this diagnostic
	#[serde(borrow)]
	pub(crate) spans: Vec<DiagnosticSpan<'m>>,
	/// Diagnostic as rendered by rustc
	#[serde(borrow)]
	pub(crate) rendered: Option<Cow<'m, str>>,
}
impl<'m> HandleMessage<'m> for Diagnostic<'m> {
	type Summary = DiagnosticSummary;

	fn into_annotations(self) -> Vec<Annotation<'m>> {
		let primary_span = self
			.spans
			.iter()
			.find(|span| span.is_primary)
			.expect("Missing primary span");

		vec![Annotation {
			kind: self.level.into(),
			file: Cow::Borrowed(primary_span.file_name),
			line: primary_span.line_start,
			end_line: Some(primary_span.line_end),
			col: Some(primary_span.column_start),
			end_column: Some(primary_span.column_end),
			title: self
				.rendered
				.as_ref()
				.map(|_rendered| Cow::Borrowed(self.message)),
			message: self.rendered.unwrap_or(Cow::Borrowed(self.message)),
		}]
	}

	#[inline]
	fn summarize(&self) -> Vec<Self::Summary> {
		vec![DiagnosticSummary::from(self)]
	}
}

/// Severity of a [`Diagnostic`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DiagnosticLevel {
	/// A fatal error that prevents compilation
	Error,
	/// A possible error of concern
	Warning,
	/// Additional information or context about the diagnostic
	Note,
	/// A suggestion on how to resolve the diagnostic
	Help,
	/// A note attached to the message for further information
	FailureNote,
	/// Indicates a bug within the compiler
	#[serde(rename = "error: internal compiler error")]
	InternalCompilerError,
}

/// The location of a diagnostic in the source code
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct DiagnosticSpan<'m> {
	/// The file where the span is located
	///
	/// This path may not exist or may point to the source of an external crate.
	pub(crate) file_name: &'m str,
	/// The first line number of the span (1-based, inclusive)
	pub(crate) line_start: usize,
	/// The last line number of the span (1-based, inclusive)
	pub(crate) line_end: usize,
	/// The first column number of the span (1-based, inclusive)
	pub(crate) column_start: usize,
	/// The last column number of the span (1-based, exclusive)
	pub(crate) column_end: usize,
	/// This span is the "primary" span
	pub(crate) is_primary: bool,
}

/// Summary info of [`Diagnostic`]
#[derive(Debug, Clone)]
pub(crate) struct DiagnosticSummary {
	/// [`Diagnostic.level`](Diagnostic#structfield.level)
	level: DiagnosticLevel,
	/// [`Diagnostic.message`](Diagnostic#structfield.message)
	message: String,
	/// Location of the diagnostic (primary [span](cargo::DiagnosticSpan))
	location: Option<(String, usize)>,
}
impl<'c> From<&'c Diagnostic<'c>> for DiagnosticSummary {
	#[inline]
	fn from(message: &'c Diagnostic<'c>) -> Self {
		Self {
			level: message.level,
			message: message.message.to_owned(),
			location: message.spans.iter().find_map(|span| {
				span.is_primary
					.then(|| (span.file_name.to_owned(), span.line_start))
			}),
		}
	}
}

/// [`SummaryWriter`] for [`DiagnosticSummary`]
#[derive(Debug, Default, Clone)]
pub(crate) struct DiagnosticSummaryWriter {
	/// Counter for each [`AnnotationKind`]
	kind_count: HashMap<AnnotationKind, usize>,
}
impl SummaryWriter for DiagnosticSummaryWriter {
	type Summary = DiagnosticSummary;

	fn write_summary(&mut self, summary: Self::Summary, content: &mut dyn FmtWrite) -> fmt::Result {
		let kind = AnnotationKind::from(summary.level);
		*self.kind_count.entry(kind).or_default() += 1;
		let location = summary
			.location
			.as_ref()
			.map(|location| format!("`{}:{}`", location.0, location.1))
			.unwrap_or_default();
		writeln!(content, "|{kind}|{}|{location}|", summary.message)
	}

	fn write_preamble(&self, file: &mut dyn IoWrite) -> io::Result<()> {
		writeln!(
			file,
			"> **TOTAL:** {} {}s, {} {}s, {} {}s\n",
			self.kind_count
				.get(&AnnotationKind::Error)
				.copied()
				.unwrap_or_default(),
			AnnotationKind::Error,
			self.kind_count
				.get(&AnnotationKind::Warning)
				.copied()
				.unwrap_or_default(),
			AnnotationKind::Warning,
			self.kind_count
				.get(&AnnotationKind::Notice)
				.copied()
				.unwrap_or_default(),
			AnnotationKind::Notice,
		)?;
		writeln!(file, "|Level|Message|Location|")?;
		writeln!(file, "|:--|:--|--:|")
	}
}
