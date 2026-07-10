# Builds every image produced from rust/Dockerfile in a single BuildKit
# graph, so the shared builder stages (cargo-chef cook + workspace build)
# compile once instead of once per image.
#
# Used by the build-rust job in .github/workflows/ci.yml.
# Local smoke test: docker buildx bake --print

variable "OWNER" {
  default = "brdgme"
}

# Short commit SHA used for the sha-<short> image tag.
variable "SHORT_SHA" {
  default = "dev"
}

# "true" to also tag :latest (master builds).
variable "PUSH_LATEST" {
  default = "false"
}

# "true" to export layer cache to the registry. Only master builds write
# the cache; PR builds read it but don't have (or need) write access.
variable "WRITE_CACHE" {
  default = "false"
}

group "default" {
  targets = ["image"]
}

target "image" {
  matrix = {
    tgt = [
      "web",
      "migrate",
      "acquire-1",
      "battleship-2",
      "bot",
      "category-5-2",
      "farkle-2",
      "for-sale-2",
      "greed-2",
      "jaipur-2",
      "liars-dice-2",
      "lost-cities-1",
      "lost-cities-2",
      "no-thanks-2",
      "operator",
      "sushi-go-2",
      "sushizock-2",
      "tic-tac-toe-2",
      "zombie-dice-2"
    ]
  }
  name       = tgt
  context    = "."
  dockerfile = "rust/Dockerfile"
  target     = tgt
  tags = concat(
    ["ghcr.io/${OWNER}/brdgme/${tgt}:sha-${SHORT_SHA}"],
    PUSH_LATEST == "true" ? ["ghcr.io/${OWNER}/brdgme/${tgt}:latest"] : []
  )
  labels = {
    "org.opencontainers.image.source" = "https://github.com/${OWNER}/brdgme"
  }
  cache-from = [
    "type=registry,ref=ghcr.io/${OWNER}/brdgme/build-cache:${tgt}"
  ]
  cache-to = WRITE_CACHE == "true" ? [
    "type=registry,ref=ghcr.io/${OWNER}/brdgme/build-cache:${tgt},mode=max,image-manifest=true,oci-mediatypes=true"
  ] : []
}
