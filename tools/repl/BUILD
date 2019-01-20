package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_binary(
    name = "brdgme_repl_bin",
    edition = "2018",
    srcs = ["src/main.rs"],
    deps = [
        "//lib/cmd:brdgme_cmd",
    ],
)
