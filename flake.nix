{
  description = "Rust development environment";

  inputs = {
    nixpkgs = {
      url = "github:nixos/nixpkgs/nixos-25.05";
    };
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            cargo-semver-checks
            clippy
            rust-analyzer
            rustc
            rust-bin.nightly.latest.rustfmt
            rustPlatform.rustLibSrc
          ];

          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

          shellHook = ''
            printf "\n    \033[1;35m🔨 Rust DevShell\033[0m\n\n"
            echo "Rust $(rustc --version)"
            echo "Cargo $(cargo --version)"
          '';
        };
      }
    );
}
