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
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          inherit (pkgs) lib;

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

          craneLib = (crane.mkLib pkgs).overrideToolchain rustMinimal;

          src =
            let
              extraFilter =
                path: _type:
                builtins.match ".*/migrations(/.*)?" path != null
                || builtins.match ".*/\\.sqlx(/.*)?" path != null
                || builtins.match ".*/web/assets(/.*)?" path != null
                || builtins.match ".*/ui/assets(/.*)?" path != null;
              cargoFilter = craneLib.filterCargoSources;
            in
            lib.cleanSourceWith {
              src = ./.;
              filter = path: type: (extraFilter path type) || (cargoFilter path type);
              name = "source";
            };

          # `dx` shells out to `wasm-bindgen` and requires the exact version the
          # project builds against, so read that out of Cargo.lock.
          lockFile = builtins.fromTOML (builtins.readFile ./Cargo.lock);
          wasmBindgenVersion =
            (builtins.head (builtins.filter (p: p.name == "wasm-bindgen") lockFile.package)).version;
          dioxusVersion = (builtins.head (builtins.filter (p: p.name == "dioxus") lockFile.package)).version;

          # nixpkgs ships one attribute per wasm-bindgen release. Fall back to
          # the newest it has when Cargo.lock runs ahead, so a version gap can
          # never break eval and block `just up` from closing it.
          wasmBindgenCli =
            let
              attr = "wasm-bindgen-cli_${builtins.replaceStrings [ "." ] [ "_" ] wasmBindgenVersion}";
              available = builtins.filter (lib.hasPrefix "wasm-bindgen-cli_0_") (builtins.attrNames pkgs);
              versionOf = n: builtins.replaceStrings [ "_" ] [ "." ] (lib.removePrefix "wasm-bindgen-cli_" n);
              newest = lib.last (
                builtins.sort (a: b: builtins.compareVersions (versionOf a) (versionOf b) < 0) available
              );
            in
            pkgs.${attr} or (lib.warn "nixpkgs has no ${attr}; falling back to ${newest}" pkgs.${newest});

          # nixpkgs is the source of truth for the `dx` version; pin the
          # `dioxus` crate to match. Warn rather than assert, because during
          # `just up` nixpkgs moves first and Cargo.lock trails it by a step.
          dioxusCli =
            lib.warnIf (pkgs.dioxus-cli.version != dioxusVersion)
              "nixpkgs dioxus-cli is ${pkgs.dioxus-cli.version} but Cargo.lock wants dioxus ${dioxusVersion}; run `just up`"
              pkgs.dioxus-cli;

          commonArgs = {
            inherit src;
            pname = "authit";
            version = "0.1.0";
            strictDeps = true;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
            SQLX_OFFLINE = "true";
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          authit-tests = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
              # Match `just test`; the workspace has no tests yet.
              cargoNextestExtraArgs = "--no-fail-fast --no-tests=pass";
            }
          );

          package = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
                pkgs.binaryen
                dioxusCli
                rustMinimal
                wasmBindgenCli
              ];
              doCheck = false;
              doNotPostBuildInstallCargoBinaries = true;
              buildPhaseCargoCommand = ''
                export HOME=$(mktemp -d)
                dx build --release --platform web --package web
                # dx's own wasm-opt invocation crashes in the sandbox (it runs
                # concurrently with the server cargo build) and dx falls back
                # to unoptimized wasm. Optimize it ourselves with the same
                # flags dx uses; run sequentially, this succeeds.
                for wasm in target/dx/web/release/web/public/assets/*.wasm; do
                  wasm-opt "$wasm" -Oz -o "$wasm.opt" \
                    --enable-reference-types --enable-bulk-memory \
                    --enable-mutable-globals --enable-nontrapping-float-to-int \
                    --enable-threads --strip-debug
                  mv "$wasm.opt" "$wasm"
                done
              '';
              installPhaseCommand = ''
                mkdir -p $out/bin
                # dx names the server binary "server"; keep "web" for the
                # NixOS module.
                cp target/dx/web/release/web/server $out/bin/web
                cp -r target/dx/web/release/web/public $out/bin/
              '';
            }
          );

        in
        {
          packages.default = package;
          checks = {
            inherit authit-tests;
          };
          devShells.default = pkgs.mkShell {
            packages =
              with pkgs;
              [
                binaryen
                cargo-dist
                cargo-edit
                cargo-nextest
                just
                pkg-config
                openssl
                sqlx-cli
              ]
              ++ [
                rustDev
                dioxusCli
                wasmBindgenCli
              ];
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

          services.authit.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
    };
}
