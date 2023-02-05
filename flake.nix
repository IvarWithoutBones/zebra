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
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      zebra = pkgs.callPackage
        ({ lib
         , craneLib
         }:
          craneLib.buildPackage {
            pname = "zebra";
            version =
              let
                year = lib.substring 0 4 self.lastModifiedDate;
                month = lib.substring 4 2 self.lastModifiedDate;
                day = lib.substring 6 2 self.lastModifiedDate;
              in
              "0.pre+date=${year}-${month}-${day}";

            src = craneLib.cleanCargoSource self;

            meta = with lib; {
              license = licenses.asl20;
              platforms = platforms.unix;
            };
          })
        { inherit craneLib; };
    in
    {
      packages.default = zebra;

      devShells.default = pkgs.mkShell {
        inputsFrom = [ zebra ];

        packages = with pkgs; [
          rustToolchain.availableComponents.rust-analyzer
          rustToolchain
          qemu
          cargo-binutils
        ];
      };
    });
}
