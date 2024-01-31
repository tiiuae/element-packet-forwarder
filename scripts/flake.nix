{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

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
          name = "element-packet-forwarder-docker";
          tag="latest";
           copyToRoot = pkgs.buildEnv {
              name = "image-root";
              paths = [ pkgs.git pkgs.cmake pkgs.dockerTools.caCertificates pkgs.gcc
                        pkgs.bashInteractive pkgs.fakeNss pkgs.coreutils pkgs.gnused pkgs.openssh pkgs.binutils pkgs.curl pkgs.pkg-config pkgs.gnupg pkgs.rustup ];
              pathsToLink = [ "/bin" "/etc" "/var"];
    };

    runAsRoot = ''
      mkdir -p /prj
    '';
          config = {
            Cmd = ["/bin/sh"
           ];
           Env = [ "USER=root" ];
           WorkingDir = "/prj"; # Set your desired working directory
          };
        };
      in {
        packages = {
          rustPackage = myRustBuild;
          docker = dockerImage;
        };
        defaultPackage = dockerImage;
        devShell = pkgs.mkShell {
          buildInputs =
            [  pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.pre-commit pkgs.rustPackages.clippy 
              (rustVersion.override { extensions = [ "rust-src" ]; })
              pkgs.coreutils
               
            ];
        };
      });


}