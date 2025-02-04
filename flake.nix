{
  description = "Static file serve";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      inherit (pkgs) lib;

      craneLib = crane.mkLib pkgs;
      src = let
        htmlFilter = path: _type: builtins.match ".*html$" path != null;
        htmlOrCargo = path: type:
          (htmlFilter path type) || (craneLib.filterCargoSources path type);
      in
        lib.cleanSourceWith {
          src = ./.;
          filter = htmlOrCargo;
          name = "source";
        };

      commonArgs = {
        inherit src;
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      serve = craneLib.buildPackage (commonArgs // {inherit cargoArtifacts;});
    in {
      checks = {
        inherit serve;

        serve-clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        serve-doc = craneLib.cargoDoc (commonArgs // {inherit cargoArtifacts;});
        serve-fmt = craneLib.cargoFmt {inherit src;};

        serve-toml-fmt = craneLib.taploFmt {
          src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];
        };
      };

      packages.default = serve;
      apps.default = flake-utils.lib.mkApp {drv = serve;};

      devShells.default = craneLib.devShell {
        checks = self.checks.${system};
      };
    });
}
