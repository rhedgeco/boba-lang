{ pkgs, ... }: {
  packages = with pkgs; [
    just # command runner
    just-lsp # language server
    just-formatter # formatter
  ];
}
