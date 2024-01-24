outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustVersion = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustVersion;
          rustc = rustVersion;
        };
        myRustBuild = rustPlatform.buildRustPackage {
          pname =
            "element-packet-forwarder"; # make this what ever your cargo.toml package.name is
          version = "0.1.0";
          src = ../.; # the folder with the cargo.toml
          cargoLock.lockFile = ../Cargo.lock;
        };
        dockerImage = pkgs.dockerTools.buildImage {
          name = "element-packet-forwarder-nix-docker-image";
          config = { Cmd = [ "${myRustBuild}/bin/element-packet-forwarder" ]; };
        };
      in {
        packages = {
          rustPackage = myRustBuild;
          docker = dockerImage;
        };
        defaultPackage = dockerImage;
        devShell = pkgs.mkShell {
          buildInputs =
            [ (rustVersion.override { extensions = [ "rust-src" ]; }) ];
        };
      });