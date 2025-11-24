let
  pkgs = import <nixpkgs> { };
  whispercli = pkgs.callPackage ./whisper-cpp/package.nix {};
  respeaker-record = pkgs.rustPlatform.buildRustPackage rec {
    pname = "respeaker-record";
    version = "0.2.0";
    cargoLock.lockFile = ./Cargo.lock;
    src = pkgs.lib.cleanSource ./.;
    nativeBuildInputs = with pkgs; [ pkg-config ];
    buildInputs = with pkgs; [
      cargo rustc pkg-config alsa-lib libusb1 ];
  };
in 
pkgs.buildEnv {
  name = "respeaker-env";
  paths = [
    respeaker-record whispercli
  ];
}