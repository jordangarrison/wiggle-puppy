{
  description = "Wiggle Puppy - Autonomous AI agent loop in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        wiggle-puppy = pkgs.rustPlatform.buildRustPackage {
          pname = "wiggle-puppy";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          # Only build the CLI binary
          cargoBuildFlags = [ "-p" "wiggle-puppy-cli" ];

          meta = with pkgs.lib; {
            description = "Autonomous AI agent loop in Rust";
            homepage = "https://github.com/jordangarrison/wiggle-puppy";
            license = licenses.mit;
            mainProgram = "wiggle-puppy";
          };
        };
      in
      {
        packages = {
          default = wiggle-puppy;
          wiggle-puppy = wiggle-puppy;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = wiggle-puppy;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
          ];

          shellHook = ''
            echo "wiggle-puppy dev environment"
            echo "Rust: $(rustc --version)"
          '';
        };
      }
    );
}
