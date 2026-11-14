{
  description = "Local Rift — native Rust Riftbound practice client";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      perSystem = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          inherit (pkgs) lib;
          manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          # Miniquad loads these by name at runtime, so buildInputs alone is
          # insufficient. libGL includes the GLX/EGL dispatch libraries.
          graphicsLibraries = lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            pkgs.libX11
            pkgs.libXi
            pkgs.libxkbcommon
            pkgs.libGL
          ];
          graphicsPath = lib.makeLibraryPath graphicsLibraries;
          runtimeFiles = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./build.rs
            ./.config/nextest.toml
            ./src
            ./tests
            ./assets
          ];
          # Keep documentation and Python tooling out of the Rust build inputs.
          # Catalog checks retain all runtime evidence and their generated report.
          catalogSource = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              runtimeFiles
              ./tools
              ./docs/CARD_COVERAGE.md
            ];
          };
          package = pkgs.rustPlatform.buildRustPackage {
            pname = manifest.package.name;
            inherit (manifest.package) version;
            src = lib.fileset.toSource {
              root = ./.;
              fileset = runtimeFiles;
            };
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.makeWrapper;
            buildInputs = graphicsLibraries;
            doCheck = true;
            # Run the complete suite with assertions, matching `cargo test`.
            # Keep release optimization for the installed client and bot tools.
            checkType = "debug";
            # The same bounded runner used in CI schedules independent match
            # suites together; all tests remain enabled and retries stay off.
            useNextest = true;
            cargoTestFlags = [
              "--locked"
              "--all-targets"
              "--profile"
              "ci"
            ];
            # Nextest does not run Rust documentation examples.
            postCheck = ''
              cargo test --locked --offline --doc --target ${pkgs.stdenv.hostPlatform.rust.rustcTargetSpec}
            '';
            postFixup = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              wrapProgram "$out/bin/local-rift" \
                --prefix LD_LIBRARY_PATH : "${graphicsPath}"
            '';
            meta = {
              description = "Offline Riftbound practice client with an AI opponent and companion";
              mainProgram = "local-rift";
              platforms = systems;
            };
          };
        in
        {
          inherit pkgs package;
          shell = pkgs.mkShell {
            inputsFrom = [ package ];
            packages = with pkgs; [
              cargo
              cargo-nextest
              rustc
              rustfmt
              clippy
              rust-analyzer
              nixfmt
              python3
            ];
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            # Both `cargo run` and the launcher can find the native libraries.
            shellHook = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              export LD_LIBRARY_PATH="${graphicsPath}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
            '';
          };
          simulation = pkgs.runCommand "local-rift-simulation-check" { } ''
            ${package}/bin/simulate --seeds 8 > "$out"
          '';
          catalog =
            pkgs.runCommand "local-rift-catalog-check"
              {
                nativeBuildInputs = [
                  pkgs.python3
                  pkgs.gitMinimal
                ];
              }
              ''
                cp -r ${catalogSource} source
                chmod -R u+w source
                cd source
                python3 tools/import_catalog.py --check
                python3 -m unittest discover -s tools -p 'test_*.py'
                touch "$out"
              '';
        }
      );
    in
    {
      packages = forAllSystems (system: {
        default = perSystem.${system}.package;
        local-rift = perSystem.${system}.package;
      });
      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/local-rift";
          meta.description = "Launch the Local Rift practice client";
        };
        simulate = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/simulate";
          meta.description = "Run headless AI matchup simulations";
        };
        api = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/rift-api";
          meta.description = "Run the local JSON API for automated opponents";
        };
      });
      devShells = forAllSystems (system: {
        default = perSystem.${system}.shell;
      });
      checks = forAllSystems (system: {
        package = perSystem.${system}.package;
        simulation = perSystem.${system}.simulation;
        catalog = perSystem.${system}.catalog;
      });
      formatter = forAllSystems (system: perSystem.${system}.pkgs.nixfmt-tree);
    };
}
