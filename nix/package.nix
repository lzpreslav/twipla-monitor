{
  rustPlatform,
  pkg-config,
  openssl,
  cacert,
  lib,
}:

rustPlatform.buildRustPackage {
  pname = "twipla-monitor";
  version = "0.1.0";

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    cacert
  ];

  buildInputs = [
    openssl
  ];

  meta = with lib; {
    description = "Monitors Twipla.jp events for available slots and sends Slack notifications";
    license = licenses.mit;
    mainProgram = "twipla-monitor";
  };
}
