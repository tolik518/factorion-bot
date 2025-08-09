{ pkgs, pkg-config, openssl, m4, gmp }:

with pkgs.lib;

pkgs.rustPlatform.buildRustPackage {
  pname = "factorion-bot-reddit";
  version = "dev";
  cargoLock.lockFile = ./Cargo.lock;
  src = cleanSource ./.;

  nativeBuildInputs = [
    pkg-config
    m4
  ];
  buildInputs = [
    openssl
    gmp
  ];

  meta = {
    description = "The reddit factorion-bot";
    homepage = "https://github.com/tolik518/factorion-bot";
    license = licenses.mit;
  };
}

