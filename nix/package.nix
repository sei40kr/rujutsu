{
  pkgs,
  flake,
  ...
}:
let
  inherit (pkgs) lib rustPlatform;
  cargoToml = lib.importTOML "${flake}/Cargo.toml";
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  inherit (cargoToml.package) version;

  src = flake;

  cargoLock.lockFile = "${flake}/Cargo.lock";

  meta = {
    inherit (cargoToml.package) description;
    homepage = "https://github.com/sei40kr/rujutsu";
    license = lib.licenses.mit;
    mainProgram = "rujutsu";
  };
}
