{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system}.extend (import rust-overlay);

        name = (pkgs.lib.importTOML ./Cargo.toml).package.name;

        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p:
          p.rust-bin.stable.latest.default.override {
            targets = [ "thumbv7em-none-eabihf" ];
          }
        );

        my-crate = craneLib.buildPackage {
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type: (craneLib.filterCargoSources path type) || (builtins.baseNameOf path == "memory.x");
          };

          strictDeps = true;

          cargoExtraArgs = "--target thumbv7em-none-eabihf";

          doCheck = false;

          buildInputs =
            [ ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];

          extraDummyScript = ''
            cp -a ${./memory.x} $out/memory.x
            rm -rf $out/src/bin/crane-dummy-*
          '';
        };

        probe = pkgs.writeScriptBin "run-stm32" ''
          #!${pkgs.bash}/bin/bash
          exec ${pkgs.probe-rs-tools}/bin/probe-rs run --chip STM32F407VGTx --connect-under-reset ${my-crate}/bin/stm32
        '';
      in
      rec {
        checks = {
          inherit my-crate;
        };

        packages = {
          ${name} = my-crate;
          default = packages.${name};
        };

        apps = {
          probe = flake-utils.lib.mkApp {
            drv = probe;
            name = "run-stm32";
          };
          default = apps.probe;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = [
            pkgs.rust-analyzer
            pkgs.probe-rs-tools
          ];
        };
      }
    );
}
