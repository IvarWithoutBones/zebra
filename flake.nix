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
      lib = nixpkgs.lib;
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      sourceCode = lib.cleanSourceWith {
        src = self;
        filter = path: type:
          (lib.hasSuffix ".s" path)
          || (lib.hasSuffix ".ld" path)
          || (craneLib.filterCargoSources path type);
      };

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

            src = sourceCode;

            # Tests are broken inside of the sandbox, they cannot find the `test` crate.
            doCheck = false;

            meta = with lib; {
              license = licenses.asl20;
              platforms = platforms.unix;
            };
          })
        { inherit craneLib; };

      zebra-runner = pkgs.callPackage
        ({ lib
         , writeShellScriptBin
         , zebra
         , qemu
         }:
          writeShellScriptBin "zebra" ''
            ${qemu}/bin/qemu-system-riscv64 \
              -machine virt \
              -cpu rv64 \
              -smp 2 \
              -m 128M \
              -bios none \
              -nographic \
              -serial mon:stdio \
              -kernel ${zebra}/bin/zebra-kernel
          '')
        { inherit zebra; };
    in
    {
      packages = {
        default = zebra-runner;
        inherit zebra zebra-runner;
      };

      devShells.default = pkgs.mkShell {
        inputsFrom = [ zebra ];

        packages = [
          rustToolchain.availableComponents.rust-analyzer
          rustToolchain.availableComponents.clippy
          rustToolchain
          pkgs.cargo-binutils
          pkgs.qemu
        ];
      };
    });
}
