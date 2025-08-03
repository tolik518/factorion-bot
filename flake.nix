{
  description = "Development and package flake for factorion";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      # The set of systems to provide outputs for
      allSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      # A function that provides a system-specific Nixpkgs for the desired systems
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs allSystems (
          system:
          f {
            pkgs = import nixpkgs {
              inherit system;
              config.allowUnfree = true;
            };
            system = system;
          }
        );
    in
    {
      devShells = forAllSystems (
        { pkgs, ... }:
        with pkgs;
        {
          default = mkShell {
            packages = [
              # Rust
              cargo
              clippy
              rust-analyzer
              rustc
              rustfmt
              cargo-watch

              # Project Build-Dependencies
              pkg-config
              openssl
              m4
              gmp
              mpc
              mpfr
            ];
          };
        }
      );

      packages = forAllSystems (
        { pkgs, ... }:
        rec {
          factorion-bot-reddit = pkgs.callPackage ./default.nix { };
          default = factorion-bot-reddit;
        }
      );

      apps = forAllSystems (
        {system, ...}: {
          bot-reddit = {
            type = "app";
            program = "${self.packages.${system}.default}/bin/factorion-bot-reddit";
            meta.description = "factorion-bot server for reddit";
          };
        }
      );
    };
}
