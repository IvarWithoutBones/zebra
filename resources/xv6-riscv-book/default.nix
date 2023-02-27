{ lib
, stdenvNoCC
, fetchFromGitHub
, fetchpatch
, python3
  # texlive with the required dependencies
, texlive-combined
}:

stdenvNoCC.mkDerivation {
  pname = "xv6-riscv-book";
  version = "0.pre+date=2022-08-31";

  src = fetchFromGitHub {
    owner = "mit-pdos";
    repo = "xv6-riscv-book";
    rev = "6bec22aeaf2da7458253698f9f8f039189439629";
    sha256 = "sha256-YWtnha/WGf0vH5c7z6zr0w9GnwSEl7b8RjWvu90um6A=";
  };

  patches = [
    # Stop downloading the xv6-riscv source code at compile time, and fix
    # https://github.com/mit-pdos/xv6-riscv-book/issues/33
    ./dont-clone-src.patch

    # Dont show red boxes around hyperlinks
    (fetchpatch {
      name = "hide-boxes-around-hyperlinks.patch";
      url = "https://github.com/mit-pdos/xv6-riscv-book/pull/30/commits/c7bb8b40cf35fa49cbab3d1a6bbfd696cb41cf30.patch";
      sha256 = "sha256-03KHofSxa0eCAW2OECGeNDcj67MgxOLxGgU9jUmqdNc=";
    })
  ];

  nativeBuildInputs = [
    python3
    texlive-combined
  ];

  postPatch = ''
    patchShebangs ./lineref
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out/share/doc
    cp ./book.pdf $out/share/doc/xv6-riscv-book.pdf

    runHook postInstall
  '';

  meta = with lib; {
    description = "Text describing xv6 on RISC-V";
    homepage = "https://github.com/mit-pdos/xv6-riscv-book";
    platforms = platforms.all;
    license = licenses.mit;
  };
}

