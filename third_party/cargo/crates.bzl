"""
cargo-raze crate workspace functions

DO NOT EDIT! Replaced on runs of cargo-raze
"""
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")
load("@bazel_tools//tools/build_defs/repo:git.bzl", "new_git_repository")

def _new_http_archive(name, **kwargs):
    if not native.existing_rule(name):
        http_archive(name=name, **kwargs)

def _new_git_repository(name, **kwargs):
    if not native.existing_rule(name):
        new_git_repository(name=name, **kwargs)

def raze_fetch_remote_crates():

    _new_http_archive(
        name = "raze__aho_corasick__0_6_9",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/aho-corasick/aho-corasick-0.6.9.crate",
        type = "tar.gz",
        sha256 = "1e9a933f4e58658d7b12defcf96dc5c720f20832deebe3e0a19efd3b6aaeeb9e",
        strip_prefix = "aho-corasick-0.6.9",
        build_file = Label("//third_party/cargo/remote:aho-corasick-0.6.9.BUILD")
    )

    _new_http_archive(
        name = "raze__ascii__0_7_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/ascii/ascii-0.7.1.crate",
        type = "tar.gz",
        sha256 = "3ae7d751998c189c1d4468cf0a39bb2eae052a9c58d50ebb3b9591ee3813ad50",
        strip_prefix = "ascii-0.7.1",
        build_file = Label("//third_party/cargo/remote:ascii-0.7.1.BUILD")
    )

    _new_http_archive(
        name = "raze__autocfg__0_1_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/autocfg/autocfg-0.1.2.crate",
        type = "tar.gz",
        sha256 = "a6d640bee2da49f60a4068a7fae53acde8982514ab7bae8b8cea9e88cbcfd799",
        strip_prefix = "autocfg-0.1.2",
        build_file = Label("//third_party/cargo/remote:autocfg-0.1.2.BUILD")
    )

    _new_http_archive(
        name = "raze__backtrace__0_3_13",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/backtrace/backtrace-0.3.13.crate",
        type = "tar.gz",
        sha256 = "b5b493b66e03090ebc4343eb02f94ff944e0cbc9ac6571491d170ba026741eb5",
        strip_prefix = "backtrace-0.3.13",
        build_file = Label("//third_party/cargo/remote:backtrace-0.3.13.BUILD")
    )

    _new_http_archive(
        name = "raze__backtrace_sys__0_1_28",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/backtrace-sys/backtrace-sys-0.1.28.crate",
        type = "tar.gz",
        sha256 = "797c830ac25ccc92a7f8a7b9862bde440715531514594a6154e3d4a54dd769b6",
        strip_prefix = "backtrace-sys-0.1.28",
        build_file = Label("//third_party/cargo/remote:backtrace-sys-0.1.28.BUILD")
    )

    _new_http_archive(
        name = "raze__cc__1_0_28",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/cc/cc-1.0.28.crate",
        type = "tar.gz",
        sha256 = "bb4a8b715cb4597106ea87c7c84b2f1d452c7492033765df7f32651e66fcf749",
        strip_prefix = "cc-1.0.28",
        build_file = Label("//third_party/cargo/remote:cc-1.0.28.BUILD")
    )

    _new_http_archive(
        name = "raze__cfg_if__0_1_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/cfg-if/cfg-if-0.1.6.crate",
        type = "tar.gz",
        sha256 = "082bb9b28e00d3c9d39cc03e64ce4cea0f1bb9b3fde493f0cbc008472d22bdf4",
        strip_prefix = "cfg-if-0.1.6",
        build_file = Label("//third_party/cargo/remote:cfg-if-0.1.6.BUILD")
    )

    _new_http_archive(
        name = "raze__chrono__0_4_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/chrono/chrono-0.4.0.crate",
        type = "tar.gz",
        sha256 = "7c20ebe0b2b08b0aeddba49c609fe7957ba2e33449882cb186a180bc60682fa9",
        strip_prefix = "chrono-0.4.0",
        build_file = Label("//third_party/cargo/remote:chrono-0.4.0.BUILD")
    )

    _new_http_archive(
        name = "raze__combine__2_3_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/combine/combine-2.3.1.crate",
        type = "tar.gz",
        sha256 = "9a8b501521d2e997ef750e7313a7200b6e16c8d2f63e19311a3143339b202f76",
        strip_prefix = "combine-2.3.1",
        build_file = Label("//third_party/cargo/remote:combine-2.3.1.BUILD")
    )

    _new_http_archive(
        name = "raze__failure__0_1_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/failure/failure-0.1.5.crate",
        type = "tar.gz",
        sha256 = "795bd83d3abeb9220f257e597aa0080a508b27533824adf336529648f6abf7e2",
        strip_prefix = "failure-0.1.5",
        build_file = Label("//third_party/cargo/remote:failure-0.1.5.BUILD")
    )

    _new_http_archive(
        name = "raze__failure_derive__0_1_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/failure_derive/failure_derive-0.1.5.crate",
        type = "tar.gz",
        sha256 = "ea1063915fd7ef4309e222a5a07cf9c319fb9c7836b1f89b85458672dbb127e1",
        strip_prefix = "failure_derive-0.1.5",
        build_file = Label("//third_party/cargo/remote:failure_derive-0.1.5.BUILD")
    )

    _new_http_archive(
        name = "raze__itoa__0_4_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/itoa/itoa-0.4.3.crate",
        type = "tar.gz",
        sha256 = "1306f3464951f30e30d12373d31c79fbd52d236e5e896fd92f96ec7babbbe60b",
        strip_prefix = "itoa-0.4.3",
        build_file = Label("//third_party/cargo/remote:itoa-0.4.3.BUILD")
    )

    _new_http_archive(
        name = "raze__kernel32_sys__0_2_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/kernel32-sys/kernel32-sys-0.2.2.crate",
        type = "tar.gz",
        sha256 = "7507624b29483431c0ba2d82aece8ca6cdba9382bff4ddd0f7490560c056098d",
        strip_prefix = "kernel32-sys-0.2.2",
        build_file = Label("//third_party/cargo/remote:kernel32-sys-0.2.2.BUILD")
    )

    _new_http_archive(
        name = "raze__lazy_static__1_2_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/lazy_static/lazy_static-1.2.0.crate",
        type = "tar.gz",
        sha256 = "a374c89b9db55895453a74c1e38861d9deec0b01b405a82516e9d5de4820dea1",
        strip_prefix = "lazy_static-1.2.0",
        build_file = Label("//third_party/cargo/remote:lazy_static-1.2.0.BUILD")
    )

    _new_http_archive(
        name = "raze__libc__0_2_47",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/libc/libc-0.2.47.crate",
        type = "tar.gz",
        sha256 = "48450664a984b25d5b479554c29cc04e3150c97aa4c01da5604a2d4ed9151476",
        strip_prefix = "libc-0.2.47",
        build_file = Label("//third_party/cargo/remote:libc-0.2.47.BUILD")
    )

    _new_http_archive(
        name = "raze__log__0_3_7",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/log/log-0.3.7.crate",
        type = "tar.gz",
        sha256 = "5141eca02775a762cc6cd564d8d2c50f67c0ea3a372cbf1c51592b3e029e10ad",
        strip_prefix = "log-0.3.7",
        build_file = Label("//third_party/cargo/remote:log-0.3.7.BUILD")
    )

    _new_http_archive(
        name = "raze__memchr__2_1_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/memchr/memchr-2.1.2.crate",
        type = "tar.gz",
        sha256 = "db4c41318937f6e76648f42826b1d9ade5c09cafb5aef7e351240a70f39206e9",
        strip_prefix = "memchr-2.1.2",
        build_file = Label("//third_party/cargo/remote:memchr-2.1.2.BUILD")
    )

    _new_http_archive(
        name = "raze__num__0_1_42",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/num/num-0.1.42.crate",
        type = "tar.gz",
        sha256 = "4703ad64153382334aa8db57c637364c322d3372e097840c72000dabdcf6156e",
        strip_prefix = "num-0.1.42",
        build_file = Label("//third_party/cargo/remote:num-0.1.42.BUILD")
    )

    _new_http_archive(
        name = "raze__num_integer__0_1_39",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/num-integer/num-integer-0.1.39.crate",
        type = "tar.gz",
        sha256 = "e83d528d2677f0518c570baf2b7abdcf0cd2d248860b68507bdcb3e91d4c0cea",
        strip_prefix = "num-integer-0.1.39",
        build_file = Label("//third_party/cargo/remote:num-integer-0.1.39.BUILD")
    )

    _new_http_archive(
        name = "raze__num_iter__0_1_37",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/num-iter/num-iter-0.1.37.crate",
        type = "tar.gz",
        sha256 = "af3fdbbc3291a5464dc57b03860ec37ca6bf915ed6ee385e7c6c052c422b2124",
        strip_prefix = "num-iter-0.1.37",
        build_file = Label("//third_party/cargo/remote:num-iter-0.1.37.BUILD")
    )

    _new_http_archive(
        name = "raze__num_traits__0_2_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/num-traits/num-traits-0.2.6.crate",
        type = "tar.gz",
        sha256 = "0b3a5d7cc97d6d30d8b9bc8fa19bf45349ffe46241e8816f50f62f6d6aaabee1",
        strip_prefix = "num-traits-0.2.6",
        build_file = Label("//third_party/cargo/remote:num-traits-0.2.6.BUILD")
    )

    _new_http_archive(
        name = "raze__num_cpus__1_8_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/num_cpus/num_cpus-1.8.0.crate",
        type = "tar.gz",
        sha256 = "c51a3322e4bca9d212ad9a158a02abc6934d005490c054a2778df73a70aa0a30",
        strip_prefix = "num_cpus-1.8.0",
        build_file = Label("//third_party/cargo/remote:num_cpus-1.8.0.BUILD")
    )

    _new_http_archive(
        name = "raze__proc_macro2__0_4_25",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/proc-macro2/proc-macro2-0.4.25.crate",
        type = "tar.gz",
        sha256 = "d3797b7142c9aa74954e351fc089bbee7958cebbff6bf2815e7ffff0b19f547d",
        strip_prefix = "proc-macro2-0.4.25",
        build_file = Label("//third_party/cargo/remote:proc-macro2-0.4.25.BUILD")
    )

    _new_http_archive(
        name = "raze__quote__0_6_10",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/quote/quote-0.6.10.crate",
        type = "tar.gz",
        sha256 = "53fa22a1994bd0f9372d7a816207d8a2677ad0325b073f5c5332760f0fb62b5c",
        strip_prefix = "quote-0.6.10",
        build_file = Label("//third_party/cargo/remote:quote-0.6.10.BUILD")
    )

    _new_http_archive(
        name = "raze__rand__0_3_15",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rand/rand-0.3.15.crate",
        type = "tar.gz",
        sha256 = "022e0636ec2519ddae48154b028864bdce4eaf7d35226ab8e65c611be97b189d",
        strip_prefix = "rand-0.3.15",
        build_file = Label("//third_party/cargo/remote:rand-0.3.15.BUILD")
    )

    _new_http_archive(
        name = "raze__redox_syscall__0_1_50",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/redox_syscall/redox_syscall-0.1.50.crate",
        type = "tar.gz",
        sha256 = "52ee9a534dc1301776eff45b4fa92d2c39b1d8c3d3357e6eb593e0d795506fc2",
        strip_prefix = "redox_syscall-0.1.50",
        build_file = Label("//third_party/cargo/remote:redox_syscall-0.1.50.BUILD")
    )

    _new_http_archive(
        name = "raze__regex__1_0_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/regex/regex-1.0.5.crate",
        type = "tar.gz",
        sha256 = "2069749032ea3ec200ca51e4a31df41759190a88edca0d2d86ee8bedf7073341",
        strip_prefix = "regex-1.0.5",
        build_file = Label("//third_party/cargo/remote:regex-1.0.5.BUILD")
    )

    _new_http_archive(
        name = "raze__regex_syntax__0_6_4",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/regex-syntax/regex-syntax-0.6.4.crate",
        type = "tar.gz",
        sha256 = "4e47a2ed29da7a9e1960e1639e7a982e6edc6d49be308a3b02daf511504a16d1",
        strip_prefix = "regex-syntax-0.6.4",
        build_file = Label("//third_party/cargo/remote:regex-syntax-0.6.4.BUILD")
    )

    _new_http_archive(
        name = "raze__rustc_demangle__0_1_13",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rustc-demangle/rustc-demangle-0.1.13.crate",
        type = "tar.gz",
        sha256 = "adacaae16d02b6ec37fdc7acfcddf365978de76d1983d3ee22afc260e1ca9619",
        strip_prefix = "rustc-demangle-0.1.13",
        build_file = Label("//third_party/cargo/remote:rustc-demangle-0.1.13.BUILD")
    )

    _new_http_archive(
        name = "raze__rustc_version__0_1_7",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rustc_version/rustc_version-0.1.7.crate",
        type = "tar.gz",
        sha256 = "c5f5376ea5e30ce23c03eb77cbe4962b988deead10910c372b226388b594c084",
        strip_prefix = "rustc_version-0.1.7",
        build_file = Label("//third_party/cargo/remote:rustc_version-0.1.7.BUILD")
    )

    _new_http_archive(
        name = "raze__ryu__0_2_7",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/ryu/ryu-0.2.7.crate",
        type = "tar.gz",
        sha256 = "eb9e9b8cde282a9fe6a42dd4681319bfb63f121b8a8ee9439c6f4107e58a46f7",
        strip_prefix = "ryu-0.2.7",
        build_file = Label("//third_party/cargo/remote:ryu-0.2.7.BUILD")
    )

    _new_http_archive(
        name = "raze__semver__0_1_20",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/semver/semver-0.1.20.crate",
        type = "tar.gz",
        sha256 = "d4f410fedcf71af0345d7607d246e7ad15faaadd49d240ee3b24e5dc21a820ac",
        strip_prefix = "semver-0.1.20",
        build_file = Label("//third_party/cargo/remote:semver-0.1.20.BUILD")
    )

    _new_http_archive(
        name = "raze__serde__1_0_84",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/serde/serde-1.0.84.crate",
        type = "tar.gz",
        sha256 = "0e732ed5a5592c17d961555e3b552985baf98d50ce418b7b655f31f6ba7eb1b7",
        strip_prefix = "serde-1.0.84",
        build_file = Label("//third_party/cargo/remote:serde-1.0.84.BUILD")
    )

    _new_http_archive(
        name = "raze__serde_derive__1_0_84",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/serde_derive/serde_derive-1.0.84.crate",
        type = "tar.gz",
        sha256 = "b4d6115a3ca25c224e409185325afc16a0d5aaaabc15c42b09587d6f1ba39a5b",
        strip_prefix = "serde_derive-1.0.84",
        build_file = Label("//third_party/cargo/remote:serde_derive-1.0.84.BUILD")
    )

    _new_http_archive(
        name = "raze__serde_json__1_0_36",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/serde_json/serde_json-1.0.36.crate",
        type = "tar.gz",
        sha256 = "574378d957d6dcdf1bbb5d562a15cbd5e644159432f84634b94e485267abbcc7",
        strip_prefix = "serde_json-1.0.36",
        build_file = Label("//third_party/cargo/remote:serde_json-1.0.36.BUILD")
    )

    _new_http_archive(
        name = "raze__syn__0_15_26",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/syn/syn-0.15.26.crate",
        type = "tar.gz",
        sha256 = "f92e629aa1d9c827b2bb8297046c1ccffc57c99b947a680d3ccff1f136a3bee9",
        strip_prefix = "syn-0.15.26",
        build_file = Label("//third_party/cargo/remote:syn-0.15.26.BUILD")
    )

    _new_http_archive(
        name = "raze__synstructure__0_10_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/synstructure/synstructure-0.10.1.crate",
        type = "tar.gz",
        sha256 = "73687139bf99285483c96ac0add482c3776528beac1d97d444f6e91f203a2015",
        strip_prefix = "synstructure-0.10.1",
        build_file = Label("//third_party/cargo/remote:synstructure-0.10.1.BUILD")
    )

    _new_http_archive(
        name = "raze__term_size__0_2_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/term_size/term_size-0.2.3.crate",
        type = "tar.gz",
        sha256 = "07b6c1ac5b3fffd75073276bca1ceed01f67a28537097a2a9539e116e50fb21a",
        strip_prefix = "term_size-0.2.3",
        build_file = Label("//third_party/cargo/remote:term_size-0.2.3.BUILD")
    )

    _new_http_archive(
        name = "raze__thread_local__0_3_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/thread_local/thread_local-0.3.6.crate",
        type = "tar.gz",
        sha256 = "c6b53e329000edc2b34dbe8545fd20e55a333362d0a321909685a19bd28c3f1b",
        strip_prefix = "thread_local-0.3.6",
        build_file = Label("//third_party/cargo/remote:thread_local-0.3.6.BUILD")
    )

    _new_http_archive(
        name = "raze__time__0_1_42",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/time/time-0.1.42.crate",
        type = "tar.gz",
        sha256 = "db8dcfca086c1143c9270ac42a2bbd8a7ee477b78ac8e45b19abfb0cbede4b6f",
        strip_prefix = "time-0.1.42",
        build_file = Label("//third_party/cargo/remote:time-0.1.42.BUILD")
    )

    _new_http_archive(
        name = "raze__ucd_util__0_1_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/ucd-util/ucd-util-0.1.3.crate",
        type = "tar.gz",
        sha256 = "535c204ee4d8434478593480b8f86ab45ec9aae0e83c568ca81abf0fd0e88f86",
        strip_prefix = "ucd-util-0.1.3",
        build_file = Label("//third_party/cargo/remote:ucd-util-0.1.3.BUILD")
    )

    _new_http_archive(
        name = "raze__unicase__1_4_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/unicase/unicase-1.4.0.crate",
        type = "tar.gz",
        sha256 = "13a5906ca2b98c799f4b1ab4557b76367ebd6ae5ef14930ec841c74aed5f3764",
        strip_prefix = "unicase-1.4.0",
        build_file = Label("//third_party/cargo/remote:unicase-1.4.0.BUILD")
    )

    _new_http_archive(
        name = "raze__unicode_xid__0_1_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/unicode-xid/unicode-xid-0.1.0.crate",
        type = "tar.gz",
        sha256 = "fc72304796d0818e357ead4e000d19c9c174ab23dc11093ac919054d20a6a7fc",
        strip_prefix = "unicode-xid-0.1.0",
        build_file = Label("//third_party/cargo/remote:unicode-xid-0.1.0.BUILD")
    )

    _new_http_archive(
        name = "raze__utf8_ranges__1_0_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/utf8-ranges/utf8-ranges-1.0.2.crate",
        type = "tar.gz",
        sha256 = "796f7e48bef87609f7ade7e06495a87d5cd06c7866e6a5cbfceffc558a243737",
        strip_prefix = "utf8-ranges-1.0.2",
        build_file = Label("//third_party/cargo/remote:utf8-ranges-1.0.2.BUILD")
    )

    _new_http_archive(
        name = "raze__version_check__0_1_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/version_check/version_check-0.1.5.crate",
        type = "tar.gz",
        sha256 = "914b1a6776c4c929a602fafd8bc742e06365d4bcbe48c30f9cca5824f70dc9dd",
        strip_prefix = "version_check-0.1.5",
        build_file = Label("//third_party/cargo/remote:version_check-0.1.5.BUILD")
    )

    _new_http_archive(
        name = "raze__winapi__0_2_8",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi/winapi-0.2.8.crate",
        type = "tar.gz",
        sha256 = "167dc9d6949a9b857f3451275e911c3f44255842c1f7a76f33c55103a909087a",
        strip_prefix = "winapi-0.2.8",
        build_file = Label("//third_party/cargo/remote:winapi-0.2.8.BUILD")
    )

    _new_http_archive(
        name = "raze__winapi__0_3_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi/winapi-0.3.6.crate",
        type = "tar.gz",
        sha256 = "92c1eb33641e276cfa214a0522acad57be5c56b10cb348b3c5117db75f3ac4b0",
        strip_prefix = "winapi-0.3.6",
        build_file = Label("//third_party/cargo/remote:winapi-0.3.6.BUILD")
    )

    _new_http_archive(
        name = "raze__winapi_build__0_1_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi-build/winapi-build-0.1.1.crate",
        type = "tar.gz",
        sha256 = "2d315eee3b34aca4797b2da6b13ed88266e6d612562a0c46390af8299fc699bc",
        strip_prefix = "winapi-build-0.1.1",
        build_file = Label("//third_party/cargo/remote:winapi-build-0.1.1.BUILD")
    )

    _new_http_archive(
        name = "raze__winapi_i686_pc_windows_gnu__0_4_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi-i686-pc-windows-gnu/winapi-i686-pc-windows-gnu-0.4.0.crate",
        type = "tar.gz",
        sha256 = "ac3b87c63620426dd9b991e5ce0329eff545bccbbb34f3be09ff6fb6ab51b7b6",
        strip_prefix = "winapi-i686-pc-windows-gnu-0.4.0",
        build_file = Label("//third_party/cargo/remote:winapi-i686-pc-windows-gnu-0.4.0.BUILD")
    )

    _new_http_archive(
        name = "raze__winapi_x86_64_pc_windows_gnu__0_4_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi-x86_64-pc-windows-gnu/winapi-x86_64-pc-windows-gnu-0.4.0.crate",
        type = "tar.gz",
        sha256 = "712e227841d057c1ee1cd2fb22fa7e5a5461ae8e48fa2ca79ec42cfc1931183f",
        strip_prefix = "winapi-x86_64-pc-windows-gnu-0.4.0",
        build_file = Label("//third_party/cargo/remote:winapi-x86_64-pc-windows-gnu-0.4.0.BUILD")
    )

