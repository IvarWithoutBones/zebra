{ inputs
, final
, prev
}:

with final;
let
  rustToolchain = rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  craneLib = (inputs.crane.mkLib final).overrideToolchain rustToolchain;

  sourceCode = lib.cleanSourceWith {
    src = inputs.self;
    filter = path: type:
      !(lib.hasSuffix ".nix" path)
      || !(builtins.baseNameOf path == "flake.lock")
      || (lib.hasSuffix ".s" path)
      || (lib.hasSuffix ".ld" path)
      || (craneLib.filterCargoSources path type);
  };

  version =
    let
      year = lib.substring 0 4 inputs.self.lastModifiedDate;
      month = lib.substring 4 2 inputs.self.lastModifiedDate;
      day = lib.substring 6 2 inputs.self.lastModifiedDate;
    in
    "0.pre+date=${year}-${month}-${day}";
in
{
  zebraPackages = recurseIntoAttrs {
    inherit rustToolchain;

    zebra-kernel = callPackage ./kernel {
      inherit sourceCode version craneLib;
      inherit (zebraPackages) zebra-kernel;
    };

    zebra-runner = zebraPackages.zebra-kernel.runner;

    xv6-riscv-book = callPackage ./resources/xv6-riscv-book {
      texlive-combined = texlive.combine {
        inherit (texlive)
          scheme-basic
          pdftex
          listings
          xcolor
          imakeidx
          xkeyval
          booktabs
          etoolbox
          preprint
          soul
          metafont
          fancyvrb
          collection-fontsrecommended;
      };
    };
  };
}
