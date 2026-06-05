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
        in
        rec {
          checks = { inherit my-crate; };

          packages = {
            ${name} = my-crate;
            default = packages.${name};
          };

          devshells.default = {
            packages = [
              rustToolchain
              pkgs.stdenv.cc
              pkgs.rust-analyzer
              pkgs.probe-rs-tools
            ];

            commands = [
              {
                name = "run-stm32";
                help = "flash and run the firmware on the STM32F407";
                command = "exec ${pkgs.probe-rs-tools}/bin/probe-rs run --chip STM32F407VGTx ${my-crate}/bin/stm32";
              }
            ];
          };
        };
    };
}
