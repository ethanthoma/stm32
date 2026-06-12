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
          name = (pkgs.lib.importTOML ./crates/firmware/Cargo.toml).package.name;

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

          kaniToolchain = (inputs.rust-overlay.lib.mkRustBin { } pkgs).nightly."2025-11-21".default.override {
            extensions = [
              "rustc-dev"
              "rust-src"
              "llvm-tools-preview"
            ];
          };

          kani = pkgs.stdenv.mkDerivation rec {
            pname = "kani";
            version = "0.67.0";
            src = pkgs.fetchzip {
              url = "https://github.com/model-checking/kani/releases/download/kani-${version}/kani-${version}-x86_64-unknown-linux-gnu.tar.gz";
              hash = "sha256-ZYBkm6sWowerivMwIsDDROACGvk3b6OHfSpYP6SdF6g=";
              stripRoot = false;
            };
            nativeBuildInputs = [
              pkgs.autoPatchelfHook
              pkgs.makeWrapper
            ];
            buildInputs = [
              pkgs.stdenv.cc.cc.lib
              pkgs.zlib
              kaniToolchain
            ];
            dontConfigure = true;
            dontBuild = true;
            installPhase = ''
              runHook preInstall
              mkdir -p $out/libexec/kani $out/bin
              cp -r ./kani-${version}/* $out/libexec/kani/
              chmod -R u+w $out/libexec/kani
              ln -s ${kaniToolchain} $out/libexec/kani/toolchain
              for cmd in kani cargo-kani; do
                makeWrapper $out/libexec/kani/bin/kani-driver $out/bin/$cmd \
                  --argv0 $cmd \
                  --prefix PATH : $out/libexec/kani/bin \
                  --prefix PATH : ${kaniToolchain}/bin \
                  --prefix PATH : ${pkgs.gcc}/bin
              done
              runHook postInstall
            '';
          };

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type: (craneLib.filterCargoSources path type) || (builtins.baseNameOf path == "memory.x");
          };

          cargoVendorDir = craneLib.vendorCargoDeps { inherit src; };

          my-crate = craneLib.buildPackage {
            inherit src;

            pname = name;
            version = "0.1.0";

            strictDeps = true;

            cargoExtraArgs = "-p stm32 --target thumbv7em-none-eabihf";

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
            verify =
              pkgs.runCommand "verus-verify"
                {
                  nativeBuildInputs = [
                    rustToolchain
                    verus
                  ];
                }
                ''
                  cp -r ${src} build
                  chmod -R +w build
                  cd build
                  export CARGO_HOME=$PWD/.cargo-home
                  mkdir -p $CARGO_HOME
                  cp ${cargoVendorDir}/config.toml $CARGO_HOME/config.toml
                  cargo verus focus -p stm32-core --features verus --offline \
                    --target x86_64-unknown-linux-gnu
                  touch $out
                '';

            kani =
              pkgs.runCommand "kani-verify"
                {
                  nativeBuildInputs = [ kani ];
                }
                ''
                  cp -r ${src} build
                  chmod -R +w build
                  cd build
                  export CARGO_HOME=$PWD/.cargo-home
                  mkdir -p $CARGO_HOME
                  cp ${cargoVendorDir}/config.toml $CARGO_HOME/config.toml
                  export HOME=$PWD/home
                  mkdir -p $HOME
                  export CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu
                  export CARGO_NET_OFFLINE=true
                  cargo-kani kani -p stm32-core
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
            meta.description = "flash and run the firmware on the STM32F407";
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
              kani
            ];

            commands = [
              {
                name = "verify";
                help = "verify the stm32-core crate with cargo-verus";
                command = "exec cargo verus focus -p stm32-core --features verus --target x86_64-unknown-linux-gnu";
              }
              {
                name = "kani-check";
                help = "cross-check stm32-core with kani (bounded model checking)";
                command = "CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu exec cargo kani -p stm32-core";
              }
            ];
          };
        };
    };
}
