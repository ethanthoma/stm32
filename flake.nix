{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
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
          ${name} = flake-utils.lib.mkApp {
            drv = my-crate;
          };
          default = apps.${name};
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = [
            pkgs.rust-analyzer
          ];
        };
      }
    );
}
