{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils}:
    flake-utils.lib.eachDefaultSystem (system:
      let
      overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [
            "rust-src"  # for rust-analyzer
          ];
        };
      in with pkgs; {
        devShell = mkShell {
          buildInputs = [
            (pkgs.python313.withPackages (ps: [
                ps.pip
                ps.ruff
                ps.mypy
            ]))
            just
            rust
            mado
          ];
        };
      }
    );
}
