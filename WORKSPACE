load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

http_archive(
    name = "io_bazel_rules_rust",
    sha256 = "7fb8dd83d0bf9f3567b4fd84610d760edcdebba02c2202a455825daa5c6cd5fb",
    strip_prefix = "rules_rust-5894d35bb7b5f982478dfbf71bc411426fae3451",
    urls = [
        # Master branch as of 2019-01-22
        "https://github.com/bazelbuild/rules_rust/archive/5894d35bb7b5f982478dfbf71bc411426fae3451.tar.gz",
    ],
)

http_archive(
    name = "bazel_skylib",
    sha256 = "eb5c57e4c12e68c0c20bc774bfbc60a568e800d025557bc4ea022c6479acc867",
    strip_prefix = "bazel-skylib-0.6.0",
    url = "https://github.com/bazelbuild/bazel-skylib/archive/0.6.0.tar.gz",
)

load("@io_bazel_rules_rust//rust:repositories.bzl", "rust_repositories")
rust_repositories()

load("@io_bazel_rules_rust//:workspace.bzl", "bazel_version")
bazel_version(name = "bazel_version")

load("//third_party/cargo:crates.bzl", "raze_fetch_remote_crates")
raze_fetch_remote_crates()