{ pkgs, lib, config, inputs, ... }:

{
  languages.go.enable = true;
  languages.javascript = {
    enable = true;
    npm.enable = true;
  };
  languages.typescript.enable = true;
  languages.rust = {
    enable = true;
    channel = "stable";
    targets = [ "wasm32-unknown-unknown" ];
  };
  packages = with pkgs; [
    cargo-binutils
    cargo-generate
    cargo-leptos
    k9s
    kubectl
    minikube
    postgresql
    sass
    skaffold
    trunk
  ];
}
