load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

# Rust

#http_archive(
#    name = "io_bazel_rules_rust",
#    sha256 = "7fb8dd83d0bf9f3567b4fd84610d760edcdebba02c2202a455825daa5c6cd5fb",
#    strip_prefix = "rules_rust-5894d35bb7b5f982478dfbf71bc411426fae3451",
#    urls = [
#        # Master branch as of 2019-01-22
#        "https://github.com/bazelbuild/rules_rust/archive/5894d35bb7b5f982478dfbf71bc411426fae3451.tar.gz",
#    ],
#)
http_archive(
    name = "io_bazel_rules_rust",
    sha256 = "55d2ff891c25ebf589aff604c8f1b41afa3fe88dbc3b6f912cd44974111b413e",
    strip_prefix = "rules_rust-2215277a2be52263ca5cd4e547cc4a50e320b828",
    urls = [
        # Master branch as of 2019-04-06
        "https://github.com/bazelbuild/rules_rust/archive/2215277a2be52263ca5cd4e547cc4a50e320b828.tar.gz",
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

# Go

http_archive(
    name = "io_bazel_rules_go",
    urls = ["https://github.com/bazelbuild/rules_go/releases/download/0.17.0/rules_go-0.17.0.tar.gz"],
    sha256 = "492c3ac68ed9dcf527a07e6a1b2dcbf199c6bf8b35517951467ac32e421c06c1",
)
load("@io_bazel_rules_go//go:deps.bzl", "go_rules_dependencies", "go_register_toolchains")
go_rules_dependencies()
go_register_toolchains()

http_archive(
    name = "bazel_gazelle",
    urls = ["https://github.com/bazelbuild/bazel-gazelle/releases/download/0.16.0/bazel-gazelle-0.16.0.tar.gz"],
    sha256 = "7949fc6cc17b5b191103e97481cf8889217263acf52e00b560683413af204fcb",
)
load("@bazel_gazelle//:deps.bzl", "go_repository")
