{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , rust-overlay
    , crane
    } @ inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
      let
        lib = nixpkgs.lib;

        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            self.overlays.default
            (import rust-overlay)
          ];
        };

        # Attribute set from the overlay
        overlayPackages = lib.filterAttrs (n: v: lib.isDerivation v) pkgs.zebraPackages;
      in
      {
        packages = overlayPackages // {
          default = overlayPackages.zebra-runner;
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            overlayPackages.rustToolchain # Configured with `./rust-toolchain.toml`
            overlayPackages.rustToolchain.availableComponents.rust-analyzer
            just
            jq
            qemu
            cargo-binutils
            nil
            nixpkgs-fmt
          ] ++ lib.optional pkgs.stdenv.isLinux pkgs.gdb
          ++ lib.optional pkgs.stdenv.isDarwin (pkgs.gdb.override {
            # Fix for https://github.com/NixOS/nixpkgs/pull/223240#issuecomment-1527198145
            enableDebuginfod = false;
          });
        };
      }) // {
      # Does not need the system attribute
      overlays.default = final: prev:
        (import ./overlay.nix { inherit final prev inputs; });
    };
}
