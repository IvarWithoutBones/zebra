{ lib
, craneLib
, version
, sourceCode
  # Runner deps
, runCommand
, makeWrapper
, zebra-kernel
, just
, qemu
}:

craneLib.buildPackage {
  pname = "zebra-kernel";
  inherit version;

  src = sourceCode;

  # Tests are broken inside of the sandbox, they cannot find the `test` crate.
  doCheck = false;

  passthru.runner = runCommand "zebra-runner"
    {
      nativeBuildInputs = [ makeWrapper ];
    } ''
    makeWrapper ${just}/bin/just $out/bin/zebra-runner \
      --suffix PATH : ${lib.makeBinPath [ qemu ]} \
      --set-default KERNEL_IMAGE ${zebra-kernel}/bin/zebra-kernel \
      --add-flags "--justfile ${sourceCode}/justfile run"
  '';

  meta = with lib; {
    license = licenses.asl20;
    platforms = platforms.unix;
  };
}
