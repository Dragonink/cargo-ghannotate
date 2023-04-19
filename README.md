# Tool to annotate [GitHub Actions](https://docs.github.com/en/actions) from the output of Cargo commands

This program parses the output of several Cargo commands
and creates annotations from them.

## Action usage

Include a step with **`uses: Dragonink/cargo-ghannotate@v1`** in your workflow.

> **WARNING: This action does *not* install a Rust toolchain!**
> You need to call another action (like [`actions-rs/toolchain`](https://github.com/actions-rs/toolchain)) before to install a Rust toolchain with Cargo.

### `command` parameter

You also need to pass a `command` input parameter.
This is a string containing the Cargo command and its arguments (without the `cargo` program).
Let us say that Cargo commands have the following pattern:
```
cargo <command> [arguments]...
```
Then the step in your workflow should look like:
```yaml
- uses: Dragonink/cargo-ghannotate@v1
  with:
    command: <command> [arguments]...
```
Currently supported Cargo commands:
- `check`
- `clippy`
- `build`
- `fmt` (requires a *nightly* toolchain)

For example:
```yaml
- uses: Dragonink/cargo-ghannotate@v1
  with:
    command: clippy --workspace --all-targets --all-features
```

### `allow-warnings` parameter

The `allow-warnings` parameter is optional.
This is just a boolean flag that, if `true`, makes the job succeed even if Cargo raises warnings.

The default value is `false`.
This allows the job to fail if a warning occurs (even without options like `-D warnings`).

## CLI usage

```
cargo ghannotate check [cargo-check ARGS]...
cargo ghannotate clippy [cargo-clippy ARGS]...
cargo ghannotate build [cargo-build ARGS]...
cargo ghannotate fmt [cargo-fmt ARGS]...
```

> It is recommended to invoke this program as a Cargo third-party command (`cargo ghannotate`).
>
> If you need to call it as a standalone program (`cargo-ghannotate`),
> you need to set the `CARGO` environment variable to the path to the `cargo` binary.

### Behavior of warnings

By default, this program will exit with an error if a warning is raised by Cargo.
This allows the job to fail if a warning occurs and still report it as a warning and not an error.

If you want the job to succeed if there are warnings and no error,
you may use the `--allow-warnings` option like so:
```
cargo ghannotate --allow-warnings clippy
```
