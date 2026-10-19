{ pkgs, inputs, ... }: {
  # apply the rust overlay to get rust-bin in pkgs
  overlays = [ inputs.rust-overlay.overlays.default ];

  packages = with pkgs; [
    crates-lsp # crates.io language server

    # include nightly rust to work with miri
    (pkgs.rust-bin.nightly.latest.default.override {
      extensions = [
        "rust-src"
        "miri"
      ];
    })
  ];

  # set up default rust profile
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [
      "rustc"
      "cargo"
      "rust-std"
      "clippy"
      "rustfmt"
      "rust-analyzer"
    ];
  };
}
