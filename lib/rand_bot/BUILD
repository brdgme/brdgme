package(default_visibility = ["//visibility:public"])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

rust_library(
    name = "brdgme_rand_bot",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = [
            "src/bin.rs",
            "src/bin/**/*.rs",
        ],
    ),
    edition = "2018",
    deps = [
        "//lib/cmd:brdgme_cmd",
        "//lib/game:brdgme_game",
        "//third_party/cargo:chrono",
        "//third_party/cargo:rand",
        "//third_party/cargo:serde_json",
    ],
)

rust_test(
    name = "test",
    timeout = "short",
    edition = "2018",
    deps = [":brdgme_rand_bot"],
)

rust_binary(
    name = "brdgme_rand_bot_bin",
    edition = "2018",
    srcs = ["src/main.rs"],
    deps = [":brdgme_rand_bot"],
)
