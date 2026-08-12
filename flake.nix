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
        cambalache-fixed = pkgs.callPackage ./nix/cambalache-fixed.nix { };
        qtEnv =
          with pkgs.qt6;
          env "qt-bridge-test-${qtbase.version}" [
            qtbase
            qttools
            qtdeclarative
          ];
        qtQmlCxxFlags =
          with pkgs;
          lib.concatStringsSep " " [
            "-I${qtEnv}/include"
            "-I${qtEnv}/include/QtQml"
            "-I${qtEnv}/include/QtQml/${qt6.qtdeclarative.version}"
            "-I${qtEnv}/include/QtQml/${qt6.qtdeclarative.version}/QtQml"
          ];
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

            # gtk
            pango
            cairo
            glib
            gtk4
            libadwaita
            libpanel
            qtEnv

            # WINIT_UNIX_BACKEND=wayland
            wayland

            # WINIT_UNIX_BACKEND=x11
            libxcursor
            libxrandr
            libxi
            libx11
          ];
          shellHook = ''
            echo "Entering Qt6 development environment"

            # Set up Qt6 library paths for linking
            export QT_PLUGIN_PATH="${qtEnv}/lib/qt-6/plugins"
            export QML_IMPORT_PATH="${qtEnv}/lib/qt-6/qml"
            export QT_QPA_PLATFORM_PLUGIN_PATH="${qtEnv}/lib/qt-6/plugins/platforms"

            # Additional Qt6 library paths
            export PKG_CONFIG_PATH="${qtEnv}/lib/pkgconfig:$PKG_CONFIG_PATH"
            export QT_QPA_PLATFORM=wayland
            export QMAKE=${qtEnv}/bin/qmake
          '';
          LD_LIBRARY_PATH = lib.makeLibraryPath (
            buildInputs
            ++ [
              fontconfig.lib
              dbus.lib
            ]
          );
          CXXFLAGS = qtQmlCxxFlags;
        };
      }
    );
}
