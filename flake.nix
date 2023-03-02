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
          packages = [
            overlayPackages.rustToolchain
            overlayPackages.rustToolchain.availableComponents.rust-analyzer
            pkgs.cargo-binutils
            pkgs.qemu
          ];
        };
      }) // {
      # Does not need the system attribute
      overlays.default = final: prev:
        (import ./overlay.nix { inherit final prev inputs; });
    };
}
