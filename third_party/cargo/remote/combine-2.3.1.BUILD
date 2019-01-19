"""
cargo-raze crate build file.

DO NOT EDIT! Replaced on runs of cargo-raze
"""
package(default_visibility = [
  # Public for visibility by "@raze__crate__version//" targets.
  #
  # Prefer access through "//third_party/cargo", which limits external
  # visibility to explicit Cargo.toml dependencies.
  "//visibility:public",
])

licenses([
  "notice", # "MIT"
])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)


# Unsupported target "buffered_stream" with type "test" omitted

rust_library(
    name = "combine",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "@raze__ascii__0_7_1//:ascii",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "2.3.1",
    crate_features = [
    ],
)

# Unsupported target "date" with type "test" omitted
# Unsupported target "http" with type "bench" omitted
# Unsupported target "ini" with type "test" omitted
# Unsupported target "json" with type "bench" omitted
# Unsupported target "mp4" with type "bench" omitted
# Unsupported target "readme" with type "test" omitted
