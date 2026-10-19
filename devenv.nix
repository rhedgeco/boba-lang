{ inputs, ... }: {
  # recursively import all modules in the nix directory
  imports = [ (inputs.import-tree ./nix) ];
}
