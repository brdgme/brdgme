package(default_visibility = ["//visibility:public"])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

rust_library(
    name = "brdgme_fuzz",
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
        "//lib/rand_bot:brdgme_rand_bot",
        "//third_party/cargo:failure",
        "//third_party/cargo:num_cpus",
        "//third_party/cargo:rand",
        "//third_party/cargo:serde",
    ],
)

rust_test(
    name = "test",
    timeout = "short",
    edition = "2018",
    crate = ":brdgme_fuzz",
)

rust_binary(
    name = "brdgme_fuzz_bin",
    edition = "2018",
    srcs = ["src/main.rs"],
    deps = [
        ":brdgme_fuzz",
        "//lib/cmd:brdgme_cmd",
    ],
)
