{ pkgs, lib, config, inputs, ... }:

{
  languages.go.enable = true;
  languages.javascript = {
    enable = true;
    npm.enable = true;
  };
  languages.typescript.enable = true;
  languages.rust.enable = true;
  packages = with pkgs; [
    k9s
    kubectl
    minikube
    postgresql
    skaffold
  ];
}
