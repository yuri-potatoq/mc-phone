{
  description = "Rust Project Template.";
  
  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust.url = "github:oxalica/rust-overlay";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, rust }:
    utils.lib.eachDefaultSystem
      (
        system:
        let
          project-name = "mc-phone";
          rust-channel = "nightly";
          rust-version = "2025-06-26"; # 1.88.0
          rust-overlay = import rust;

          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay ];
          };

          rust-toolchain = pkgs.rust-bin."${rust-channel}"."${rust-version}".default.override {
            extensions = [
              "rust-std"
              "rust-src"
            ];
          };

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rust-toolchain;
            rustc = rust-toolchain;
          };
        in
        rec {
          # `nix develop`
          devShell = pkgs.mkShell {
	    hardeningDisable = [ "fortify" ];
            nativeBuildInputs = with pkgs; [ pkg-config ];
            buildInputs = with pkgs; [
              rust-toolchain
              rust-analyzer
              openssl
              sqlx-cli
            ];
          };
        }
      );
}
