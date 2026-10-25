# The display server: one binary with the UI kit, pages and fonts compiled in.
# Inter and Archivo are in the repository. A licensed mono face placed at crates/ui/fonts/mono.woff2 (tracked, e.g. in a private fork) is compiled in too.
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
