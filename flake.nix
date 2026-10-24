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
          package = pkgs.rustPlatform.buildRustPackage {
            pname = manifest.package.name;
            inherit (manifest.package) version;
            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                ./build.rs
                ./src
                ./tests
                ./assets
              ];
            };
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.makeWrapper;
            buildInputs = graphicsLibraries;
            doCheck = true;
            postFixup = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              wrapProgram "$out/bin/local-rift" \
                --prefix LD_LIBRARY_PATH : "${graphicsPath}"
            '';
            meta = {
              description = "Offline 3D Riftbound practice client with an AI opponent and companion";
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
              rustc
              rustfmt
              clippy
              rust-analyzer
              nixfmt
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
      });
      formatter = forAllSystems (system: perSystem.${system}.pkgs.nixfmt-tree);
    };
}
