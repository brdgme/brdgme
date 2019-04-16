package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library", "rust_test")

rust_library(
    name = "brdgme_cmd",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = [
            "src/bin.rs",
            "src/bin/**/*.rs",
        ],
    ),
    edition = "2018",
    deps = [
        "//lib/color:brdgme_color",
        "//lib/game:brdgme_game",
        "//lib/markup:brdgme_markup",
        "//third_party/cargo:chrono",
        "//third_party/cargo:failure",
        "//third_party/cargo:serde",
        "//third_party/cargo:serde_derive",
        "//third_party/cargo:serde_json",
        "//third_party/cargo:term_size",
    ],
)

rust_test(
    name = "test",
    timeout = "short",
    edition = "2018",
    crate = ":brdgme_cmd",
)
