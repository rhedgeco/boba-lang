{ inputs, ... }: {
  # apply the rust overlay to get rust-bin in pkgs
  overlays = [ inputs.rust-overlay.overlays.default ];

  # stable owns the default cargo/rustc on PATH.
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
