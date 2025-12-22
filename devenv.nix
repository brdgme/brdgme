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
  languages.python = {
    enable = true;
    venv = {
      enable = true;
      requirements = ''
        browser-use[cli]
      '';
    };
  };

  packages = with pkgs; [
    cargo-binstall
    cargo-leptos
    dart-sass
    k9s
    kubectl
    minikube
    postgresql
    skaffold
    sqlx-cli
    uv
    chromium
    playwright-driver.browsers
  ];

  env.DATABASE_URL = "postgres://brdgme_user:brdgme_password@localhost:5432/brdgme";
  env.PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
  env.PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
}
