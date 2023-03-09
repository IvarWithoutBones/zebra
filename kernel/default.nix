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
, shfmt
}:

craneLib.buildPackage {
  pname = "zebra-kernel";
  inherit version;

  src = sourceCode;

  # Tests are broken inside of the sandbox, they cannot find the `test` crate.
  doCheck = false;

  passthru.runner = runCommand "zebra-runner"
    {
      nativeBuildInputs = [
        makeWrapper
        shfmt
        just
      ];
    } ''
    mkdir -p $out/bin

    # Generate a bash script that runs qemu, based on the justfile.
    just --justfile ${sourceCode}/justfile --dry-run run 2> $out/bin/zebra-runner
    shfmt --language-dialect bash --simplify --write $out/bin/zebra-runner
    chmod +x $out/bin/zebra-runner

    wrapProgram $out/bin/zebra-runner \
      --suffix PATH : ${lib.makeBinPath [ qemu ]} \
      --set-default KERNEL_IMAGE ${zebra-kernel}/bin/zebra-kernel
  '';

  meta = with lib; {
    license = licenses.asl20;
    platforms = platforms.unix;
  };
}
