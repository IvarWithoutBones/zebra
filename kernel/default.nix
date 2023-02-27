{ lib
, craneLib
, version
, sourceCode
, writeShellScriptBin
, qemu
, zebra-kernel
}:

craneLib.buildPackage {
  pname = "zebra-kernel";
  inherit version;

  src = sourceCode;

  # Tests are broken inside of the sandbox, they cannot find the `test` crate.
  doCheck = false;

  # TODO: maybe switch to a justfile as this is repeated inside the cargo config file
  passthru.runner = writeShellScriptBin "zebra-runner" ''
    ${qemu}/bin/qemu-system-riscv64 \
      -machine virt \
      -cpu rv64 \
      -smp 2 \
      -m 128M \
      -bios none \
      -nographic \
      -serial mon:stdio \
      -kernel ${zebra-kernel}/bin/zebra-kernel
  '';

  meta = with lib; {
    license = licenses.asl20;
    platforms = platforms.unix;
  };
}
