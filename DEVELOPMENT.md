# Notes for developers

## Developing and testing `sqlquerypp`

NixOS and direnv (the latter with flake and nix-command support enabled) must
be set up to develop with the specified dependencies within the nix flake
(rust compiler, python etc.).

Following `justfile` commands are helpful for development:

- `just develop`: compiles everything and installs the latest compiled
  state of `sqlquerypp` into the current python virtual environment
  which is located in `.venv/` at the repository root. Please note that
  you might need to activate it manually using `source .venv/bin/activate`.

- `just lint` checks whether all coding conventions (as defined in
  `pyproject.toml` and `rustfmt.toml`) are fulfilled.

- `just format` autoformats as much code as possible according to coding conventions.

- `just test` runs all lints and tests.

## Architecture

This package is mainly separated into two components:

- High-level Python API: `python/sqlquerypp`

  The high level API architecture itself is very simple. It is recommended
  to take a look at the test cases (`python/tests/`) or the documentation
  in `sqlquerypp.compiler.Compiler` and its subclasses.

- Rust API: `src/`
   - `lib.rs` is the main entrypoint to look at. It constructs a module with
    the fully qualified name `sqlquerypp.sqlquerypp`. It is internal to the
    Python API and exposes internally used, fast SQL preprocessor
    implementations. Its Python interface declaration is located in
    `python/sqlquerypp/sqlquerypp.pyi`.
   - `error.rs`, `lex.rs`, `scanner.rs` and `types.rs` should be self-explanatory.
   - The code within `parser/` is responsible for parsing nodes (i.e.
   representations of `sqlquerypp` directives) and generating codes
   for them.
      - `ParserState` is a state automaton based parser implementation
      that handles the "magic" of transforming `sqlquerypp` code strings
      into internal data structures (in terms of compiler construction,
      called "nodes" in abstract syntax tree, although `sqlquerypp`
      does not provide a correct, academic-style AST-oriented implementation).
      - For example, while parsing `combined_result`, instructions are
      reflected as `CombinedResultNode` instances
      (`src/parser/nodes/combined_result.rs`). These node objects
      are obviously very low-level and stateful (many public and
      optional fields).
      - When generating code, it is recommended to use
      `CompleteCombinedResultNode` objects. This strategy
      applies to all nodes `sqlquerypp` supports. See also:
         - `ParserState::finalize()`
         - `FinalParserState`
   - `codegen/` provides common structs, traits and functions for
    generating valid SQL statements from a `FinalParserState`.

## Manual release workflow

- `source .venv/bin/activate`

- `maturin build --release`
   - if successful, returns output like "Built wheel for CPython 3.13 to 'PATH'"

- `maturin upload <PATH>` (use 'PATH' from last command)
   - **NOTE**: This requires token-based authentication. As this is just a
    quick-and-dirty solution which should not be necessary for long, I
    won't document this further.
