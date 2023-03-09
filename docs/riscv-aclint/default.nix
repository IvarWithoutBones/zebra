{ lib
, stdenvNoCC
, fetchurl
}:

stdenvNoCC.mkDerivation rec {
  pname = "riscv-aclint";
  version = "1.0-rc4";

  src = fetchurl {
    url = "https://github.com/riscv/riscv-aclint/releases/download/v${version}/riscv-aclint-${version}.pdf";
    sha256 = "sha256-+38Uc0cPvCohVfCuNlpOlPldBFHKIHp9RWcae+a3Y3U=";
  };

  dontUnpack = true;
  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall

    mkdir -p $out/share/doc
    cp $src $out/share/doc/riscv-aclint.pdf

    runHook postInstall
  '';

  meta = with lib; {
    description = "The RISC-V Advanced Core Local Interruptor specification";
    homepage = "https://github.com/riscv/riscv-aclint";
    license = licenses.cc-by-40;
  };
}
