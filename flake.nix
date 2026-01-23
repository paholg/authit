{
  inputs = {
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

          # Extract wasm-bindgen version from Cargo.lock so we don't need to
          # keep nipkgs and Cargo.lock exactly in sync, even for dependents.
          wasmBindgenVersion =
            let
              lockFile = builtins.fromTOML (builtins.readFile ./Cargo.lock);
              wasmBindgen = builtins.head (builtins.filter (p: p.name == "wasm-bindgen") lockFile.package);
            in
            wasmBindgen.version;

          wasmBindgenCli = pkgs.rustPlatform.buildRustPackage rec {
            pname = "wasm-bindgen-cli";
            version = wasmBindgenVersion;
            src = pkgs.fetchCrate {
              inherit pname version;
              hash = "sha256-M6WuGl7EruNopHZbqBpucu4RWz44/MSdv6f0zkYw+44=";
            };
            cargoHash = "sha256-ElDatyOwdKwHg3bNH/1pcxKI7LXkhsotlDPQjiLHBwA=";
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [
              pkgs.openssl
            ];
          };

          package = pkgs.rustPlatform.buildRustPackage {
            pname = "authit";
            version = "0.1.0";
            src = ./.;
            strictDeps = true;
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.dioxus-cli
              wasmBindgenCli
              pkgs.binaryen
              rustMinimal
            ];
            buildInputs = [ pkgs.openssl ];
            SQLX_OFFLINE = "true";
            buildPhase = ''
              export HOME=$(mktemp -d)
              dx build --release --platform web --package web
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/dx/web/release/web/web $out/bin/
              cp -r target/dx/web/release/web/public $out/bin/
            '';
            cargoLock.lockFile = ./Cargo.lock;
          };

        in
        {
          packages.default = package;
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

          services.authit.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
    };
}
