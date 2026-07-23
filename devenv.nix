{ pkgs, lib, config, inputs, ... }:

{
  dotenv.enable = true;
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
    act
    awscli2
    binaryen
    cargo-binstall
    cargo-leptos
    cilium-cli
    ctlptl
    doctl
    mirrord
    opentofu
    wasm-bindgen-cli
    chromium
    dart-sass
    gh
    k9s
    kind
    kubectl
    retry
    playwright-driver.browsers
    postgresql
    sqlx-cli
    tilt
    uv
    poppler-utils
  ];

  # The tofu S3 state backend only reads AWS_* credentials; alias the Spaces
  # key pair from .env so it only has to be defined once.
  enterShell = ''
    if [ -n "''${SPACES_ACCESS_KEY_ID:-}" ]; then
      export AWS_ACCESS_KEY_ID="$SPACES_ACCESS_KEY_ID"
      export AWS_SECRET_ACCESS_KEY="$SPACES_SECRET_ACCESS_KEY"
    fi
  '';

  env.DATABASE_URL = "postgres://brdgme_user:brdgme_password@localhost:5432/brdgme";
  env.PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
  env.PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
}
