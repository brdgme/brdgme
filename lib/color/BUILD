package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library", "rust_test")

rust_library(
    name = "brdgme_color",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = ["src/bin.rs", "src/bin/**/*.rs"],
    ),
    rustc_flags = ["--edition", "2018"],
    deps = [
        "//third_party/cargo:failure",
        "//third_party/cargo:lazy_static",
        "//third_party/cargo:regex",
        "//third_party/cargo:serde",
        "//third_party/cargo:serde_derive",
    ],
)

rust_test(
    name = "test",
    timeout = "short",
    rustc_flags = ["--edition", "2018"],
    deps = [":brdgme_color"],
)
