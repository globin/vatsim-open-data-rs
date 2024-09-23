{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        nixpkgs-stable.follows = "nixpkgs";
      };
    };
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "riscv64-linux"
      ];

      imports = [
        inputs.treefmt-nix.flakeModule
        inputs.pre-commit-hooks.flakeModule
      ];

      perSystem = {
        pkgs,
        system,
        config,
        ...
      }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [inputs.rust-overlay.overlays.default];
        };

        devShells = {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rust-bin.stable.latest.default
              cargo-watch
              config.treefmt.build.wrapper
            ];
          };
        };

        pre-commit = {
          check.enable = true;
          settings.hooks.treefmt = {
            enable = true;
          };
        };
        treefmt = {
          projectRootFile = "flake.lock";

          settings = {
            formatter = {
              nix = {
                command = pkgs.nixfmt-rfc-style;
                includes = ["*.nix"];
              };
              rustfmt = {
                command = pkgs.rustfmt;
                options = [
                  "--edition"
                  "2021"
                ];
                includes = ["*.rs"];
              };
            };
          };
        };
      };
    };
}