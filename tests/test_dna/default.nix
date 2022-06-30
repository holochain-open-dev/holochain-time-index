{
  holonixPath ?  builtins.fetchTarball { url = "https://github.com/holochain/holonix/archive/c0ba02d2f72940724f6d4769e4eb4e6cc0b27337.tar.gz"; }
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