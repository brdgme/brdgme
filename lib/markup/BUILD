package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library", "rust_test")

rust_library(
    name = "brdgme_markup",
    srcs = [
        "src/ansi.rs",
        "src/ast.rs",
        "src/error.rs",
        "src/html.rs",
        "src/lib.rs",
        "src/parser.rs",
        "src/plain.rs",
        "src/transform.rs",
    ],
    rustc_flags = ["--edition", "2018"],
    deps = [
        "//lib/color:brdgme_color",
        "//third_party/cargo:combine",
        "//third_party/cargo:failure",
        "//third_party/cargo:serde",
        "//third_party/cargo:serde_derive",
    ],
)

rust_test(
    name = "test",
    timeout = "short",
    rustc_flags = ["--edition", "2018"],
    deps = [":brdgme_markup"],
)
