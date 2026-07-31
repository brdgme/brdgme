target "image" {
  matrix = {
    tgt = [
      "foo-1",
      "bar-1"
    ]
  }
  name       = tgt
  context    = "."
  dockerfile = "rust/Dockerfile"
  target     = tgt
}
