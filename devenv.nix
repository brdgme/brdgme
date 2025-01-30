{ pkgs, lib, config, inputs, ... }:

{
  languages.go.enable = true;
  languages.javascript = {
    enable = true;
    npm.enable = true;
  };
  languages.rust.enable = true;
}
