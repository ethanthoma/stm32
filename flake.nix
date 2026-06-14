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

          fluxToolchain = (inputs.rust-overlay.lib.mkRustBin { } pkgs).nightly."2025-11-25".default.override {
            extensions = [
              "rustc-dev"
              "rust-src"
              "llvm-tools"
              "rustfmt"
              "clippy"
            ];
            # compile the firmware crate for host and target
            targets = [ "thumbv7em-none-eabihf" ];
          };

          fluxRustlib = "${fluxToolchain}/lib:${fluxToolchain}/lib/rustlib/x86_64-unknown-linux-gnu/lib";

          flux-fixpoint = pkgs.stdenv.mkDerivation {
            pname = "liquid-fixpoint";
            version = "nightly";
            src = pkgs.fetchurl {
              url = "https://github.com/ucsd-progsys/liquid-fixpoint/releases/download/nightly/fixpoint-x86_64-linux-gnu.tar.gz";
              hash = "sha256-orqsotO491rdNmspFx3CYajVWt7L5EPHxPVWD4DhlR0=";
            };
            sourceRoot = ".";
            nativeBuildInputs = [ pkgs.autoPatchelfHook ];
            buildInputs = [
              pkgs.gmp
              pkgs.stdenv.cc.cc.lib
            ];
            installPhase = ''
              runHook preInstall
              install -Dm755 fixpoint $out/bin/fixpoint
              runHook postInstall
            '';
          };

          fluxSrc = pkgs.fetchFromGitHub {
            owner = "flux-rs";
            repo = "flux";
            rev = "74f6e774f436c7152d9a6c487e347869bd39df8a";
            sha256 = "1q5k4aw8gb0rlnw7jc6wlmpfyxc1ny35z223zsq9x532mxab05ik";
          };

          fluxShims = pkgs.symlinkJoin {
            name = "flux-shims";
            paths = [
              (pkgs.writeShellScriptBin "cargo" ''
                case "$1" in +*) shift ;; esac
                exec ${fluxToolchain}/bin/cargo "$@"
              '')
              (pkgs.writeShellScriptBin "rustc" ''
                case "$1" in +*) shift ;; esac
                exec ${fluxToolchain}/bin/rustc "$@"
              '')
              (pkgs.writeShellScriptBin "rustup" ''
                [ "$1" = "which" ] || exit 0
                shift
                bin=""
                while [ $# -gt 0 ]; do
                  case "$1" in
                    --toolchain) shift 2 ;;
                    *) bin="$1"; shift ;;
                  esac
                done
                echo "${fluxToolchain}/bin/$bin"
              '')
            ];
          };

          fluxVendor = craneLib.vendorCargoDeps { src = fluxSrc; };

          flux = pkgs.stdenv.mkDerivation {
            pname = "flux";
            version = "0-unstable-2026-06-13";
            src = fluxSrc;
            nativeBuildInputs = [ pkgs.makeWrapper ];
            configurePhase = ''
              runHook preConfigure
              export CARGO_HOME=$TMPDIR/cargo-home
              mkdir -p $CARGO_HOME
              cp ${fluxVendor}/config.toml $CARGO_HOME/config.toml
              export HOME=$TMPDIR/home
              mkdir -p $HOME
              export PATH="${fluxShims}/bin:${fluxToolchain}/bin:${flux-fixpoint}/bin:${pkgs.z3}/bin:$PATH"
              export LD_LIBRARY_PATH="${fluxRustlib}"
              export CARGO_NET_OFFLINE=true
              runHook postConfigure
            '';
            buildPhase = ''
              runHook preBuild
              cargo x install
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p $out/libexec/flux $out/bin
              cp -r $CARGO_HOME/bin $out/libexec/flux/bin
              cp -r $HOME/.flux $out/libexec/flux/sysroot
              # shims must be LAST here so they end up FIRST on PATH (makeWrapper --prefix prepends in order)
              for cmd in flux cargo-flux; do
                makeWrapper $out/libexec/flux/bin/$cmd $out/bin/$cmd \
                  --prefix PATH : ${fluxToolchain}/bin \
                  --prefix PATH : ${flux-fixpoint}/bin \
                  --prefix PATH : ${pkgs.z3}/bin \
                  --prefix PATH : ${fluxShims}/bin \
                  --prefix LD_LIBRARY_PATH : ${fluxRustlib} \
                  --set FLUX_SYSROOT $out/libexec/flux/sysroot
              done
              runHook postInstall
            '';
            dontStrip = true;
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

            nativeBuildInputs = [ pkgs.flip-link ];

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
                    verusToolchain
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

            # smoke test
            flux =
              let
                probe = pkgs.writeText "flux-probe.rs" ''
                  #[flux::sig(fn(x: i32) -> i32{v: v == x})]
                  pub fn id(x: i32) -> i32 {
                      x
                  }
                '';
              in
              pkgs.runCommand "flux-check" { } ''
                export HOME=$TMPDIR
                ${flux}/bin/flux --crate-type=lib ${probe}
                touch $out
              '';
          };

          packages = {
            ${name} = my-crate;
            default = packages.${name};
            inherit flux;
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
              pkgs.flip-link
              verus
              verusfmt
              verus-analyzer
              kani
              flux
            ];

            commands = [
              {
                name = "check";
                help = "run a check: check {verus | kani | flux [file] | stack | all}";
                command = ''
                  verus() { PATH="${verusToolchain}/bin:$PATH" cargo verus focus -p stm32-core --features verus --target x86_64-unknown-linux-gnu; }
                  kani() { CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu cargo kani -p stm32-core; }
                  flux() { command flux --crate-type=lib "''${1:-$PRJ_ROOT/notes/flux-example.rs}"; }
                  stack() {
                    cargo build -p stm32 --release --target thumbv7em-none-eabihf || return 1
                    size "$PRJ_ROOT/target/thumbv7em-none-eabihf/release/stm32" | awk 'NR==2 {
                      ram = $2 + $3; total = 128 * 1024; free = total - ram;
                      printf "flash:          %6d bytes\n", $1;
                      printf "static ram:     %6d bytes (data %d + bss %d)\n", ram, $2, $3;
                      printf "stack headroom: %6d bytes of %d (%.1f%%) — flip-link faults on overflow\n", free, total, 100 * free / total;
                    }'
                  }
                  case "''${1:-}" in
                    verus) verus ;;
                    kani) kani ;;
                    flux) flux "''${2:-}" ;;
                    stack) stack ;;
                    all)
                      rc=0
                      verus || rc=1
                      kani || rc=1
                      flux || rc=1
                      exit $rc
                      ;;
                    *) echo "usage: check {verus | kani | flux [file] | stack | all}" >&2; exit 2 ;;
                  esac
                '';
              }
            ];
          };
        };
    };
}
