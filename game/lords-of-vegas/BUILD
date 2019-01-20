package(default_visibility = ["//visibility:public"])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

rust_library(
    name = "lords_of_vegas",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = ["src/bin.rs", "src/bin/**/*.rs"],
    ),
    rustc_flags = ["--edition", "2018"],
    deps = [
        "//lib/color:brdgme_color",
        "//lib/game:brdgme_game",
        "//lib/markup:brdgme_markup",
        "//third_party/cargo:failure",
        "//third_party/cargo:lazy_static",
        "//third_party/cargo:rand",
        "//third_party/cargo:serde",
        "//third_party/cargo:serde_derive",
        "//third_party/cargo:serde_json",
    ],
)

rust_test(
    name = "test",
    timeout = "short",
    rustc_flags = ["--edition", "2018"],
    deps = [":lords_of_vegas"],
)

rust_binary(
    name = "cli",
    rustc_flags = ["--edition", "2018"],
    srcs = ["src/bin/cli.rs"],
    deps = [
        ":lords_of_vegas",
        "//lib/cmd:brdgme_cmd",
    ],
)

rust_binary(
    name = "fuzz",
    rustc_flags = ["--edition", "2018"],
    srcs = ["src/bin/fuzz.rs"],
    deps = [
        ":lords_of_vegas",
        "//tools/fuzz:brdgme_fuzz",
    ],
)

rust_binary(
    name = "repl",
    rustc_flags = ["--edition", "2018"],
    srcs = ["src/bin/repl.rs"],
    deps = [
        ":lords_of_vegas",
        "//lib/cmd:brdgme_cmd",
    ],
)