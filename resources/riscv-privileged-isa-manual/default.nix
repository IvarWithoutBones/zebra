{ lib
, stdenvNoCC
, fetchFromGitHub
, texlive-combined
}:

stdenvNoCC.mkDerivation rec {
  pname = "riscv-privileged-isa-manual";
  version = "1.12";

  src = fetchFromGitHub {
    owner = "riscv";
    repo = "riscv-isa-manual";
    rev = "Priv-v${version}";
    sha256 = "sha256-6rD8faHiR9dMXUEs31yOjfdsUVipmiagjVAU/uo8qB4=";
  };

  nativeBuildInputs = [
    texlive-combined
  ];

  dontConfigure = true;

  preBuild = ''
    cd build
    export HOME="$TMPDIR"
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out/share/doc
    cp riscv-privileged.pdf $out/share/doc/riscv-privileged-isa-manual.pdf

    runHook postInstall
  '';

  meta = with lib; {
    description = "RISC-V Privileged Instruction Set Manual";
    homepage = "https://github.com/riscv/riscv-isa-manual";
    license = licenses.cc-by-40;
    platforms = platforms.all;
  };
}
