{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;

      rustManifest = lib.importTOML ./Cargo.toml;

      revSuffix = lib.optionalString (self ? shortRev || self ? dirtyShortRev)
        "-${self.shortRev or self.dirtyShortRev}";

      makePackages = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = rustManifest.package.name;
            version = rustManifest.package.version + revSuffix;

            src = lib.sourceByRegex ./. [
              "^\.cargo(/.*)?$"
              "^build\.rs$"
              "^Cargo\.(lock|toml)$"
              "^sounds(/.*)?$"
              "^src(/.*)?$"
            ];

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            # fix for `Compatibility with CMake < 3.5 has been removed from CMake.`
            env.CMAKE_POLICY_VERSION_MINIMUM = "3.5";

            buildInputs = with pkgs; [
              alsa-lib
              openssl
            ];

            nativeBuildInputs = (with pkgs; [
              cmake
              pkg-config
              rustPlatform.bindgenHook
            ]) ++ (if dev then
              with pkgs; [
                clippy
                rust-analyzer
                (rustfmt.override { asNightly = true; })
              ] else [ ]);
          };
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system} = makePackages system true;
        packages.${system} = makePackages system false;
      })
      lib.systems.flakeExposed);
}
