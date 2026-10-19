{ pkgs, ... }: {
  packages = with pkgs; [
    # language servers and static analyzers
    statix
    deadnix
    nixd
  ];
}
