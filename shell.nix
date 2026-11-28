# Rust toolchain and the native libraries bevy needs on NixOS.
#
#   nix-shell
#   cargo run -p tetris
with (import <nixpkgs> {});
let
  runtimeLibs = [
    alsa-lib
    udev
    vulkan-loader
    libxkbcommon
    wayland
    libx11
    libxcursor
    libxi
    libxrandr
  ];
in
mkShell {
  name = "rustgame";
  packages = [
    rustc
    cargo
    rustfmt
    clippy
    lld
    pkg-config
    wasm-bindgen-cli
    binaryen
  ] ++ runtimeLibs;
  # winit and wgpu load these at runtime with dlopen.
  LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibs;
  # The toolchain comes from nixpkgs, CI uses the one in rust-toolchain.toml:
  # say so when they differ, as format and lint results can too.
  shellHook = ''
    pinned=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
    actual=$(rustc --version | cut -d ' ' -f 2)
    if [ "$pinned" != "$actual" ]; then
      echo "warning: rustc $actual here, $pinned in rust-toolchain.toml (CI)" >&2
    fi
  '';
}
