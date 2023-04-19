//! Tool to annotate [GitHub Actions](https://docs.github.com/en/actions) from the output of Cargo commands
#![warn(
	// Restriction
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	unreachable_pub,
	unused,
	unused_crate_dependencies,
	unused_lifetimes,
	unused_tuple_struct_fields,
	clippy::dbg_macro,
	clippy::empty_structs_with_brackets,
	clippy::enum_glob_use,
	clippy::float_cmp_const,
	clippy::format_push_string,
	clippy::match_on_vec_items,
	clippy::missing_docs_in_private_items,
	clippy::mod_module_files,
	clippy::option_option,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::str_to_string,
	clippy::verbose_file_reads,
	// Suspicious
	noop_method_call,
	meta_variable_misuse,
	// Pedantic
	unused_qualifications,
	clippy::doc_link_with_quotes,
	clippy::doc_markdown,
	clippy::filter_map_next,
	clippy::float_cmp,
	clippy::inefficient_to_string,
	clippy::macro_use_imports,
	clippy::manual_let_else,
	clippy::match_wildcard_for_single_variants,
	clippy::mem_forget,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc,
	clippy::needless_continue,
	clippy::semicolon_if_nothing_returned,
	clippy::unnested_or_patterns,
	clippy::unused_self,
	// Style
	unused_import_braces,
	// Nursery
	clippy::empty_line_after_outer_attr,
	clippy::imprecise_flops,
	clippy::missing_const_for_fn,
	clippy::suboptimal_flops,
)]
#![deny(
	// Correctness
	pointer_structural_match,
	// Restriction
	keyword_idents,
	non_ascii_idents,
	missing_abi,
	unsafe_op_in_unsafe_fn,
	unused_must_use,
	clippy::exit,
	clippy::lossy_float_literal,
	clippy::undocumented_unsafe_blocks,
)]
#![forbid(unsafe_code)]

use clap::{Args, Parser, Subcommand, ValueHint};
use std::{
	collections::BTreeSet,
	ffi::OsString,
	fs::File,
	io::{self, BufRead, Cursor, Write as IoWrite},
	process::{Command, ExitCode, Output, Stdio},
};

/// Environment variable containing the path to the special summary file
const SUMMARY_PATH_VAR: &str = "GITHUB_STEP_SUMMARY";
/// Path to the summary file used in debug contexts
const DEBUG_SUMMARY_PATH: &str = "SUMMARY.md";

mod cargo;
mod github;

use cargo::{
	Diagnostic, DiagnosticSummaryWriter, FormatMismatchSummaryWriter, FormatMismatches,
	HandleMessage, SummaryWriter,
};
use github::AnnotationKind;

fn main() -> ExitCode {
	let cli = Cli::parse_from(std::env::args_os().filter(|arg| arg != "ghannotate"));

	let annotation_threshold = if cli.allow_warnings {
		AnnotationKind::Error
	} else {
		AnnotationKind::Warning
	};
	let mut max_annotation = AnnotationKind::Notice;

	let cargo = cli.invoke_cargo().expect("Cargo invocation failed");
	let mut annotations_buf = BTreeSet::new();
	let mut stdout = io::stdout().lock();
	let mut summary_content = String::new();
	/// Common code for all messages
	macro_rules! handle_message {
		($parse:expr, $summary_writer:ty) => {{
			let mut summary_writer = <$summary_writer>::default();
			for line in Cursor::new(cargo.stdout).lines() {
				let line = line.unwrap();
				let line = line.as_str();
				if let Ok(message) = $parse(line) {
					let summaries = message.summarize();
					let mut write_summaries = false;
					for annotation in message.into_annotations() {
						if annotations_buf.insert(annotation.to_owned()) {
							writeln!(stdout, "{annotation}").unwrap();
							max_annotation = max_annotation.max(annotation.kind);
							write_summaries = true;
						}
					}
					if write_summaries {
						summaries.into_iter().for_each(|summary| {
							summary_writer
								.write_summary(summary, &mut summary_content)
								.unwrap();
						});
					}
				}
			}
			if let Some(mut file) = std::env::var_os(SUMMARY_PATH_VAR)
				.or(cfg!(debug_assertions).then(|| OsString::from(DEBUG_SUMMARY_PATH)))
				.and_then(|path| File::create(path).ok())
			{
				summary_writer.write_preamble(&mut file).unwrap();
				file.write_all(summary_content.as_bytes()).unwrap();
				summary_writer.write_postamble(&mut file).unwrap();
			}
		}};
	}
	match cli.command {
		CliCommand::Check(_) | CliCommand::Clippy(_) | CliCommand::Build(_) => {
			handle_message!(serde_json::from_str::<Diagnostic>, DiagnosticSummaryWriter);
		}
		CliCommand::Fmt(_) => {
			handle_message!(
				serde_json::from_str::<Vec<FormatMismatches>>,
				FormatMismatchSummaryWriter
			);
		}
	}

	if max_annotation >= annotation_threshold {ExitCode::FAILURE} else {ExitCode::SUCCESS}
}

/// Annotates GitHub Actions from the output of Cargo subcommands
#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
#[command(override_usage = "cargo ghannotate check [OPTIONS] [ARGS]...\n       \
	cargo ghannotate clippy [OPTIONS] [ARGS]...\n       \
	cargo ghannotate build [OPTIONS] [ARGS]...\n       \
	cargo ghannotate fmt [OPTIONS] [ARGS]...")]
struct Cli {
	/// Path to the `cargo` executable
	#[arg(long, env = "CARGO", value_name = "PATH", value_hint = ValueHint::ExecutablePath)]
	cargo: OsString,
	/// If warnings were to be raised, they would not cause the job to fail
	#[arg(long)]
	allow_warnings: bool,
	/// Cargo subcommand
	#[command(subcommand)]
	command: CliCommand,
}
impl Cli {
	/// Invokes Cargo with the passed arguments and returns its output
	#[inline]
	fn invoke_cargo(&self) -> io::Result<Output> {
		#[allow(clippy::enum_glob_use)]
		use CliCommand::*;

		match self.command {
			Check(_) => {
				let mut command = Command::new(&self.cargo);
				command
					.args(["check", "--message-format=json"])
					.args(self.command.as_ref().as_ref());
				command
			}
			Clippy(_) => {
				let mut command = Command::new(&self.cargo);
				command
					.args(["clippy", "--message-format=json"])
					.args(self.command.as_ref().as_ref());
				command
			}
			Build(_) => {
				let mut command = Command::new(&self.cargo);
				command
					.args(["build", "--message-format=json"])
					.args(self.command.as_ref().as_ref());
				command
			}
			Fmt(_) => {
				let mut command = Command::new("rustup");
				command
					.args(["run", "nightly", "cargo", "fmt", "--message-format=json"])
					.args(self.command.as_ref().as_ref());
				command
			}
		}
		.stdin(Stdio::null())
		.stderr(Stdio::inherit())
		.output()
	}
}

/// Cargo subcommand
#[derive(Debug, Clone, Subcommand)]
enum CliCommand {
	/// Runs `cargo check` and annotates from its output
	Check(CliCommandArgs),
	/// Runs `cargo clippy` and annotates from its output
	Clippy(CliCommandArgs),
	/// Runs `cargo build` and annotates from its output
	Build(CliCommandArgs),
	/// Runs `cargo fmt` and annotates from its output
	///
	/// WARNING: This requires a nightly toolchain!
	Fmt(CliCommandArgs),
}
impl AsRef<CliCommandArgs> for CliCommand {
	#[inline]
	fn as_ref(&self) -> &CliCommandArgs {
		match self {
			Self::Check(args) | Self::Clippy(args) | Self::Build(args) | Self::Fmt(args) => args,
		}
	}
}

/// Arguments to be passed down to Cargo
#[derive(Debug, Clone, Args)]
#[repr(transparent)]
struct CliCommandArgs {
	/// Arguments to be passed down to Cargo
	#[arg(
		num_args = 0..,
		trailing_var_arg = true,
		allow_hyphen_values = true,
	)]
	args: Vec<OsString>,
}
impl AsRef<[OsString]> for CliCommandArgs {
	#[inline]
	fn as_ref(&self) -> &[OsString] {
		self.args.as_ref()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use clap::CommandFactory;

	#[test]
	fn cli() {
		Cli::command().debug_assert();
	}
}
