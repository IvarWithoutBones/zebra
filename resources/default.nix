{ callPackage
, texlive
, symlinkJoin
}:

let
  riscv-aclint = callPackage ./riscv-aclint { };

  riscv-plic = callPackage ./riscv-plic { };

  xv6-riscv-book = callPackage ./xv6-riscv-book {
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

  riscv-privileged-isa-manual = callPackage ./riscv-privileged-isa-manual {
    texlive-combined = texlive.combine {
      inherit (texlive)
        scheme-basic
        pdftex
        placeins
        multirow
        float
        listings
        comment
        enumitem
        verbatimbox
        readarray
        forloop
        paralist
        metafont;
    };
  };
in
symlinkJoin {
  name = "riscv-resources";
  paths = [
    xv6-riscv-book
    riscv-privileged-isa-manual
    riscv-aclint
    riscv-plic
  ];

  meta.description = "A collection of resources for RISC-V operating system development";
}
