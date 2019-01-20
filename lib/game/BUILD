package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library", "rust_test")

rust_library(
    name = "brdgme_game",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = ["src/bin.rs", "src/bin/**/*.rs"],
    ),
    rustc_flags = ["--edition", "2018"],
    deps = [
        "//lib/color:brdgme_color",
        "//lib/markup:brdgme_markup",
        "//third_party/cargo:chrono",
        "//third_party/cargo:combine",
        "//third_party/cargo:failure",
        "//third_party/cargo:log",
        "//third_party/cargo:rand",
        "//third_party/cargo:serde",
        "//third_party/cargo:serde_derive",
        "//third_party/cargo:unicase",
    ],
)

rust_test(
    name = "test",
    timeout = "short",
    rustc_flags = ["--edition", "2018"],
    deps = [":brdgme_game"],
)
