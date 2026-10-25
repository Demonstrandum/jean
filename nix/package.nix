# The display server: one binary with the UI kit, pages and fonts compiled in.
# The fonts are Git LFS objects (build from a checkout with `git lfs pull`, or a flake input with ?lfs=1). Berkeley Mono is compiled in encrypted;
# the server unlocks it at startup with secrets/font.key.
{ rustPlatform, lib }:
rustPlatform.buildRustPackage {
  pname = "display-server";
  version = "0.1.0";
  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = [ "-p" "preview" ];
  doCheck = false;
  postInstall = ''
    mv $out/bin/preview $out/bin/display-server
  '';
  meta.mainProgram = "display-server";
}
