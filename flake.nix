{
  description = "SatsHunt dev env";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        lib = nixpkgs.lib;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        build_arch_underscores =
          lib.strings.replaceStrings [ "-" ] [ "_" ]
            pkgs.stdenv.buildPlatform.config;

        rocksdb = pkgs.rocksdb_8_11.override { enableLiburing = false; };

        rustAnalyzerMcp = pkgs.rustPlatform.buildRustPackage rec {
          pname = "rust-analyzer-mcp";
          version = "0.2.0";

          src = pkgs.fetchFromGitHub {
            owner = "zeenix";
            repo = "rust-analyzer-mcp";
            rev = "v${version}";
            hash = "sha256-brnzVDPBB3sfM+5wDw74WGqN5ahtuV4OvaGhnQfDqM0=";
          };

          cargoHash = "sha256-7t4bjyCcbxFAO/29re7cjoW1ACieeEaM4+QT5QAwc34=";

          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl ];

          doCheck = false;

          meta = {
            description = "MCP server for rust-analyzer integration";
            homepage = "https://github.com/zeenix/rust-analyzer-mcp";
          };
        };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "satshunt";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            clang
            llvmPackages.libclang
          ];

          buildInputs = with pkgs; [
            rocksdb
          ];

          NIX_CFLAGS_COMPILE = "-Wno-error=stringop-overflow";

          buildPhase = ''
            export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib";
            export "ROCKSDB_${build_arch_underscores}_STATIC=true";
            export "ROCKSDB_${build_arch_underscores}_LIB_DIR=${rocksdb}/lib/";
            cargo build --release --frozen
          '';

          installPhase = ''
            mkdir -p $out/bin
            cp target/release/satshunt $out/bin/
            mkdir -p $out/share/satshunt
            cp -r migrations $out/share/satshunt/
          '';

          # Skip tests during build
          doCheck = false;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            rust-bin.nightly.latest.rustfmt
            pkg-config
            cmake
            clang
            llvmPackages.libclang
            llvmPackages.libcxxClang
            rustAnalyzerMcp
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          RUSTFMT = "${pkgs.rust-bin.nightly.latest.rustfmt}/bin/rustfmt";
          "ROCKSDB_${build_arch_underscores}_STATIC" = "true";
          "ROCKSDB_${build_arch_underscores}_LIB_DIR" = "${rocksdb}/lib/";
        };
      });
}
