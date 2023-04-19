//! Provides structures to parse Cargo JSON data

use crate::github::Annotation;
use std::{
	fmt::{self, Write as FmtWrite},
	io::{self, Write as IoWrite},
};

/// Converts this struct into a [`Vec<Annotation>`] and a [`Vec<Summary>`]
pub(crate) trait HandleMessage<'m> {
	/// Type used to store summary data
	type Summary;

	/// Converts `self` into a list of [`Annotation`]
	fn into_annotations(self) -> Vec<Annotation<'m>>;

	/// Extracts summaries
	///
	/// The default implementation is [`unimplemented!`].
	fn summarize(&self) -> Vec<Self::Summary> {
		unimplemented!()
	}
}

/// Enables types to be written as job summaries
pub(crate) trait SummaryWriter: Sized {
	/// Type used to store summary data
	type Summary;

	/// Writes the given `summary`
	fn write_summary(&mut self, summary: Self::Summary, content: &mut dyn FmtWrite) -> fmt::Result;

	#[allow(unused_variables)]
	/// Writes the preamble
	///
	/// This function is meant to be called after all calls to [`write_summary`](Self::write_summary).
	#[inline]
	fn write_preamble(&self, file: &mut dyn IoWrite) -> io::Result<()> {
		Ok(())
	}

	#[allow(unused_variables)]
	/// Writes the "postamble"
	///
	/// This function is meant to be called after all calls to [`write_summary`](Self::write_summary).
	#[inline]
	fn write_postamble(self, file: &mut dyn IoWrite) -> io::Result<()> {
		Ok(())
	}
}
impl SummaryWriter for () {
	type Summary = ();

	#[inline]
	fn write_summary(
		&mut self,
		_summary: Self::Summary,
		_content: &mut dyn FmtWrite,
	) -> fmt::Result {
		Ok(())
	}
}

mod rustc;
mod rustfmt;

pub(crate) use self::rustfmt::*;
pub(crate) use rustc::*;
