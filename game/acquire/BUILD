package(default_visibility = ["//visibility:public"])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

rust_library(
    name = "acquire",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = ["src/bin.rs", "src/bin/**/*.rs"],
    ),
    edition = "2018",
    deps = [
        "//lib/color:brdgme_color",
        "//lib/game:brdgme_game",
        "//lib/markup:brdgme_markup",
        "//third_party/cargo:failure",
        "//third_party/cargo:rand",
        "//third_party/cargo:serde",
        "//third_party/cargo:serde_derive",
    ],
)

rust_test(
    name = "test",
    timeout = "short",
    edition = "2018",
    deps = [":acquire"],
)

rust_binary(
    name = "cli",
    edition = "2018",
    srcs = ["src/bin/cli.rs"],
    deps = [
        ":acquire",
        "//lib/cmd:brdgme_cmd",
    ],
)

rust_binary(
    name = "fuzz",
    edition = "2018",
    srcs = ["src/bin/fuzz.rs"],
    deps = [
        ":acquire",
        "//tools/fuzz:brdgme_fuzz",
    ],
)

rust_binary(
    name = "repl",
    edition = "2018",
    srcs = ["src/bin/repl.rs"],
    deps = [
        ":acquire",
        "//lib/cmd:brdgme_cmd",
    ],
)