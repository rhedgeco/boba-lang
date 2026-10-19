{ pkgs, ... }: {
  packages = with pkgs; [
    tombi # toml language server
  ];
}
