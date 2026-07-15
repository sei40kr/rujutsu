_: {
  projectRootFile = "flake.nix";

  programs = {
    # Nix
    nixfmt.enable = true;

    # Rust. Pin the edition to the crate's (Cargo.toml) so rustfmt's style
    # matches `cargo fmt`; treefmt-nix otherwise defaults to edition 2024,
    # whose import sorting churns the existing code.
    rustfmt = {
      enable = true;
      edition = "2021";
    };
  };
}
