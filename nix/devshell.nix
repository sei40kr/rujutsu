{ inputs, pkgs, ... }:
let
  pre-commit-check = import ./checks/pre-commit-check.nix { inherit inputs pkgs; };
in
pkgs.mkShell {
  # Add build dependencies
  packages = [
    pkgs.rustc
    pkgs.cargo
    pkgs.clippy
    pkgs.rustfmt
    pkgs.rust-analyzer
  ];

  # Add environment variables
  env = {
    RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
  };

  shellHook = ''
    ${pre-commit-check.shellHook}

    # Load custom bash code
  '';
}
