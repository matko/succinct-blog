{mkShell, clang, rust-bin, stdenv, gnuplot, texlive, openssl, pkg-config}:
mkShell {
  buildInputs = [
    clang
    gnuplot
    texlive.combined.scheme-full
    openssl
    pkg-config
    (rust-bin.nightly.latest.default.override {
      extensions = [ "rust-src" "rust-analyzer" ];
    })
  ];

  shellHook = if stdenv.hostPlatform.isx86_64 then ''
export RUSTFLAGS="-C target-feature=+avx2"
#export RUSTFLAGS="-C target-feature=+avx512f,+avx512vl"
'' else if stdenv.hostPlatform.isAarch64 then ''
export RUSTFLAGS="-C target-feature=+neon"
'' else throw "unsupported platform " + stdenv.hostPlatform.platform;
}
