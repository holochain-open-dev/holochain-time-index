{
  holonixPath ?  builtins.fetchTarball { url = "https://github.com/holochain/holonix/archive/2f642d6958ab7a70384031154fdea1e919535e92.tar.gz"; }
}:

let
  holonix = import (holonixPath) { };
  nixpkgs = holonix.pkgs;
in nixpkgs.mkShell {
  inputsFrom = [ holonix.main ];
  buildInputs = with nixpkgs; [
    binaryen
    nodejs-16_x
  ];
}