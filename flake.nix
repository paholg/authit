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

          package = pkgs.rustPlatform.buildRustPackage {
            pname = "authit";
            version = "0.1.0";
            src = ./.;
            strictDeps = true;
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.dioxus-cli
              pkgs.wasm-bindgen-cli
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
              cp -r target/dx/web/release/web $out/bin/
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
