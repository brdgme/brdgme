{ pkgs, lib, config, inputs, options, ... }:

{
  languages.go.enable = true;
  languages.javascript = {
    enable = true;
    npm.enable = true;
  };
  languages.rust = {
    enable = true;
    channel = "nightly";
    targets = [
      "wasm32-unknown-unknown"
    ];
  };
}
