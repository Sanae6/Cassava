{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    {
      nixpkgs,
      flake-utils,
      ...
    }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ inputs.fenix.overlays.default ];
        };
      in
      with pkgs;
      {
        formatter = nixfmt-tree;
        inherit inputs;
        devShells.default = mkShell rec {
          buildInputs = [
            (fenix.combine [
              fenix.default.toolchain
              fenix.complete.rust-src
            ])

            pnpm
            nodejs
            webkitgtk_6_0
            gtk4
            gsettings-desktop-schemas
            cairo
            dconf
            libsoup_3
            gdk-pixbuf
            glib
            renderdoc
            clang
            pkg-config
            libxkbcommon
            libGL
            openssl
            udev
            fontconfig
            alsa-lib
            vulkan-loader
            vulkan-tools
            libudev-zero
            libgcc.lib

            # WINIT_UNIX_BACKEND=wayland
            wayland

            # WINIT_UNIX_BACKEND=x11
            libxcursor
            libxrandr
            libxi
            libx11
          ];
          shellHook = ''
            export XDG_DATA_DIRS=$XDG_DATA_DIRS:$GSETTINGS_SCHEMAS_PATH
          '';
          LD_LIBRARY_PATH = lib.makeLibraryPath (
            buildInputs
            ++ [
              fontconfig.lib
              dbus.lib
            ]
          );
        };
      }
    );
}
