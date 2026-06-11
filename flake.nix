{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs@{ ... }:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.devshell.flakeModule ];

      systems = [ "x86_64-linux" ];

      perSystem =
        { pkgs, system, ... }:
        let
          name = (pkgs.lib.importTOML ./Cargo.toml).package.name;

          rustToolchain = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable.latest.default.override {
            targets = [ "thumbv7em-none-eabihf" ];
          };

          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

          verusToolchain = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable."1.95.0".default;
          verusRustlib = "${verusToolchain}/lib:${verusToolchain}/lib/rustlib/x86_64-unknown-linux-gnu/lib";
          verusRustupShim = pkgs.writeShellScript "rustup" ''
            case "$1" in
              toolchain) echo "1.95.0-x86_64-unknown-linux-gnu" ;;
              run)
                shift 2
                [ "$1" = "--" ] && shift
                export LD_LIBRARY_PATH="${verusRustlib}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                exec "$@" ;;
              *) exit 0 ;;
            esac
          '';

          verus = pkgs.stdenv.mkDerivation rec {
            pname = "verus";
            version = "0.2026.06.07.cd03505";
            src = pkgs.fetchzip {
              url = "https://github.com/verus-lang/verus/releases/download/release/${version}/verus-${version}-x86-linux.zip";
              hash = "sha256-P2xOGd2/7DdXdq42J+VaAc7rPy8AldjcT70FHDqCqRY=";
              stripRoot = false;
            };
            nativeBuildInputs = [
              pkgs.autoPatchelfHook
              pkgs.makeWrapper
            ];
            buildInputs = [
              pkgs.stdenv.cc.cc.lib
              verusToolchain
              pkgs.zlib
            ];
            dontConfigure = true;
            dontBuild = true;
            installPhase = ''
              runHook preInstall
              mkdir -p $out/libexec/verus $out/bin
              cp -r ./verus-x86-linux/* $out/libexec/verus/
              chmod -R u+w $out/libexec/verus
              chmod +x $out/libexec/verus/{verus,cargo-verus,rust_verify,z3}
              install -Dm755 ${verusRustupShim} $out/libexec/shim/rustup
              for cmd in verus cargo-verus; do
                makeWrapper $out/libexec/verus/$cmd $out/bin/$cmd \
                  --set VERUS_Z3_PATH $out/libexec/verus/z3 \
                  --prefix PATH : $out/libexec/shim
              done
              runHook postInstall
            '';
          };

          verusfmt = pkgs.stdenv.mkDerivation rec {
            pname = "verusfmt";
            version = "0.7.2";
            src = pkgs.fetchzip {
              url = "https://github.com/verus-lang/verusfmt/releases/download/v${version}/verusfmt-x86_64-unknown-linux-gnu.tar.xz";
              hash = "sha256-sufqI+gABGR52/oViIihmpgk+G+5Mg2RzZfapyZsWVw=";
              stripRoot = false;
            };
            nativeBuildInputs = [ pkgs.autoPatchelfHook ];
            buildInputs = [ pkgs.stdenv.cc.cc.lib ];
            dontConfigure = true;
            dontBuild = true;
            installPhase = ''
              runHook preInstall
              install -Dm755 verusfmt-x86_64-unknown-linux-gnu/verusfmt $out/bin/verusfmt
              runHook postInstall
            '';
          };

          verus-analyzer = pkgs.stdenv.mkDerivation rec {
            pname = "verus-analyzer";
            version = "2026-04-29";
            src = pkgs.fetchurl {
              url = "https://github.com/verus-lang/verus-analyzer/releases/download/${version}/verus-analyzer-x86_64-unknown-linux-gnu.gz";
              hash = "sha256-+cjFqNMYQfNDKsyc+21v0/BWu2zAZQWNpZPM5Kv//Ig=";
            };
            nativeBuildInputs = [
              pkgs.autoPatchelfHook
              pkgs.gzip
            ];
            buildInputs = [
              pkgs.stdenv.cc.cc.lib
              pkgs.zlib
            ];
            dontUnpack = true;
            dontConfigure = true;
            dontBuild = true;
            installPhase = ''
              runHook preInstall
              mkdir -p $out/bin
              gunzip -c $src > $out/bin/verus-analyzer
              chmod +x $out/bin/verus-analyzer
              runHook postInstall
            '';
          };

          my-crate = craneLib.buildPackage {
            src = pkgs.lib.cleanSourceWith {
              src = ./.;
              filter =
                path: type: (craneLib.filterCargoSources path type) || (builtins.baseNameOf path == "memory.x");
            };

            strictDeps = true;

            cargoExtraArgs = "--target thumbv7em-none-eabihf";

            doCheck = false;

            cargoArtifacts = null;

            buildInputs = [ ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
          };

          flash = pkgs.writeShellScriptBin "flash" ''
            exec ${pkgs.probe-rs-tools}/bin/probe-rs run --chip STM32F407VGTx ${my-crate}/bin/stm32
          '';
        in
        rec {
          checks = {
            inherit my-crate;
            verify = pkgs.runCommand "verus-verify" { } ''
              cp ${./src/temp_convert.rs} ./temp_convert.rs
              ${verus}/bin/verus --crate-type=lib ./temp_convert.rs
              touch $out
            '';
          };

          packages = {
            ${name} = my-crate;
            default = packages.${name};
          };

          apps.default = {
            type = "app";
            program = pkgs.lib.getExe flash;
          };

          devshells.default = {
            packages = [
              rustToolchain
              pkgs.stdenv.cc
              pkgs.rust-analyzer
              pkgs.probe-rs-tools
              verus
              verusfmt
              verus-analyzer
            ];

            commands = [
              {
                name = "verify";
                help = "verify src/temp_convert.rs with verus";
                command = "exec ${verus}/bin/verus --crate-type=lib src/temp_convert.rs";
              }
            ];
          };
        };
    };
}
