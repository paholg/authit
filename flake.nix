{
  inputs = {
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      crane,
      flake-utils,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systemOutputs = flake-utils.lib.eachDefaultSystem (
        system:
        let
          overlays = [
            (import rust-overlay)
          ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

          rustMinimal = pkgs.rust-bin.stable.latest.minimal.override {
            targets = [ "wasm32-unknown-unknown" ];
          };
          rustDev = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-analyzer"
              "rust-src"
            ];
            targets = [ "wasm32-unknown-unknown" ];
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain (p: rustMinimal);

          commonArgs = {
            strictDeps = true;
            nativeBuildInputs = [ ];
          };

          artifacts = commonArgs // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          };

          package = craneLib.buildPackage (
            artifacts
            // {
              doCheck = false;
            }
          );

        in
        {
          checks = {
            clippy = craneLib.cargoClippy (
              artifacts
              // {
                cargoClippyExtraArgs = "-- --deny warnings";
              }
            );
            fmt = craneLib.cargoFmt artifacts;
            test = craneLib.cargoNextest artifacts;
          };
          packages = {
            default = package;
          };
          devShells.default = pkgs.mkShell {
            packages =
              with pkgs;
              [
                cargo-dist
                cargo-edit
                cargo-nextest
                just
                pkg-config
                openssl
                sqlx-cli
              ]
              ++ [ rustDev ];
          };
        }
      );
    in
    systemOutputs
    // {
      nixosModules.default =
        { lib, pkgs, ... }:
        {
          imports = [ ./nix/module.nix ];

          # Set default package based on system
          services.authit.package = lib.mkDefault self.packages.${pkgs.system}.default;
        };
    };
}
