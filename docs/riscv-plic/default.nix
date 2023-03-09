{ lib
, stdenvNoCC
, fetchurl
}:

stdenvNoCC.mkDerivation rec {
  pname = "riscv-plic";
  version = "1.0.0_rc5";

  src = fetchurl {
    url = "https://github.com/riscv/riscv-plic-spec/releases/download/${version}/riscv-plic-${version}.pdf";
    sha256 = "sha256-LS8+0ebmE1UuVvHmiCcXo4M/pX9Hfwk81fYd9aHjxxI=";
  };

  dontUnpack = true;
  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall

    mkdir -p $out/share/doc
    cp $src $out/share/doc/riscv-plic.pdf

    runHook postInstall
  '';

  meta = with lib; {
    description = "The RISC-V platform-level interrupt controller specification";
    homepage = "https://github.com/riscv/riscv-plic-spec";
    license = licenses.cc-by-40;
  };
}
