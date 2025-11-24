let
  pkgs = import <nixpkgs> { };
  whispercli = pkgs.callPackage ./whisper-cpp/package.nix {};
in 
  pkgs.rustPlatform.buildRustPackage rec {
    pname = "respeaker-record";
    version = "0.2.0";
    cargoLock.lockFile = ./Cargo.lock;
    src = pkgs.lib.cleanSource ./.;
    nativeBuildInputs = with pkgs; [ pkg-config whispercli ];
    buildInputs = with pkgs; [
      cargo rustc pkg-config alsa-lib libusb1 ];
  }