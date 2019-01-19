package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library", "rust_test")

rust_library(
    name = "brdgme_game",
    srcs = [
        "src/command/parser/chain.rs",
        "src/command/parser/mod.rs",
        "src/command/doc.rs",
        "src/command/mod.rs",
        "src/bot.rs",
        "src/errors.rs",
        "src/game_log.rs",
        "src/game.rs",
        "src/lib.rs",
    ],
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
