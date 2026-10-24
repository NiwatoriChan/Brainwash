# Development shell for NixOS.
#
#   nix-shell            # enter the shell
#   cargo run -p mybrain-app
#
# egui/eframe loads OpenGL, Wayland and X11 libraries at runtime via dlopen,
# so they must be visible through LD_LIBRARY_PATH.
{ pkgs ? import <nixpkgs> { } }:

let
  runtimeLibs = with pkgs; [
    libGL
    libxkbcommon
    wayland
    libx11
    libxcursor
    libxi
    libxrandr
    fontconfig
  ];
in
pkgs.mkShell {
  packages = with pkgs; [
    cargo
    rustc
    rustfmt
    clippy
    rust-analyzer
    pkg-config
  ];

  buildInputs = runtimeLibs;

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
  RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
}
