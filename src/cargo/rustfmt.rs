//! Provides the structs to work with rustfmt's output

use super::{HandleMessage, SummaryWriter};
use crate::github::{Annotation, AnnotationKind};
use serde::Deserialize;
use std::{
	borrow::Cow,
	fmt::{self, Write as FmtWrite},
	io::{self, Write as IoWrite},
};

/// Message output by rustfmt
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FormatMismatches<'m> {
	/// The file where the mismatches are located
	pub(crate) name: &'m str,
	/// Reported errors and their locations
	#[serde(borrow)]
	pub(crate) mismatches: Vec<FormatMismatch<'m>>,
}
impl<'m> HandleMessage<'m> for Vec<FormatMismatches<'m>> {
	type Summary = FormatMismatchesSummary;

	#[inline]
	fn into_annotations(self) -> Vec<Annotation<'m>> {
		self.into_iter()
			.flat_map(|mismatches| {
				mismatches
					.mismatches
					.into_iter()
					.map(|mismatch| Annotation {
						kind: AnnotationKind::Warning,
						file: Cow::Borrowed(
							mismatches
								.name
								.trim_start_matches(
									std::env::current_dir()
										.unwrap()
										.as_os_str()
										.to_str()
										.unwrap(),
								)
								.trim_start_matches('/'),
						),
						line: mismatch.original_begin_line,
						end_line: Some(mismatch.original_end_line),
						col: None,
						end_column: None,
						title: Some(Cow::Borrowed("Format mismatch")),
						message: mismatch.expected,
					})
			})
			.collect()
	}

	#[inline]
	fn summarize(&self) -> Vec<Self::Summary> {
		self.iter().map(From::from).collect()
	}
}

#[allow(dead_code)]
/// Reported errors contained in a single file
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FormatMismatch<'m> {
	/// The first line of the span in the current file (1-based, inclusive)
	pub(crate) original_begin_line: usize,
	/// The last line of the span in the current file (1-based, inclusive)
	pub(crate) original_end_line: usize,
	/// The first line of the span if the file was correct (1-based, inclusive)
	pub(crate) expected_begin_line: usize,
	/// The last line of the span if the file was correct (1-based, inclusive)
	pub(crate) expected_end_line: usize,
	/// The current code
	pub(crate) original: Cow<'m, str>,
	/// The corrected code
	pub(crate) expected: Cow<'m, str>,
}

/// Summary info for [`FormatMismatches`]
#[derive(Debug, Clone)]
pub(crate) struct FormatMismatchesSummary {
	/// [`FormatMismatches.name`](FormatMismatches#structfield.name)
	file: String,
	/// Collection of [`FormatMismatch.original_begin_line`](FormatMismatch#structfield.original_begin_line)
	lines: Vec<usize>,
}
impl<'c> From<&'c FormatMismatches<'c>> for FormatMismatchesSummary {
	#[inline]
	fn from(message: &'c FormatMismatches<'c>) -> Self {
		Self {
			file: message
				.name
				.trim_start_matches(
					std::env::current_dir()
						.unwrap()
						.as_os_str()
						.to_str()
						.unwrap(),
				)
				.trim_start_matches('/')
				.to_owned(),
			lines: message
				.mismatches
				.iter()
				.map(|mismatch| mismatch.original_begin_line)
				.collect(),
		}
	}
}

/// [`SummaryWriter`] for [`FormatMismatchesSummary`]
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct FormatMismatchSummaryWriter {
	/// Counter of mismatches
	count: usize,
}
impl SummaryWriter for FormatMismatchSummaryWriter {
	type Summary = FormatMismatchesSummary;

	fn write_summary(&mut self, summary: Self::Summary, content: &mut dyn FmtWrite) -> fmt::Result {
		self.count += summary.lines.len();
		writeln!(content, "- `{}`", summary.file)?;
		for line in summary.lines {
			writeln!(content, "  - L{line}")?;
		}
		Ok(())
	}

	fn write_preamble(&self, file: &mut dyn IoWrite) -> io::Result<()> {
		writeln!(file, "> **TOTAL:** {} mismatches\n", self.count)
	}
}
