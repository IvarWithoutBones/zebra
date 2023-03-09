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
      (baseNameOf path == "justfile")
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

    docs = callPackage ./docs { };
  };
}
