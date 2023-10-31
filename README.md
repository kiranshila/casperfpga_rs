# casperfpga_rs

[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)](#license)
[![docs](https://img.shields.io/docsrs/casperfpga?logo=rust&style=flat-square)](https://docs.rs/casperfpga/latest/casperfpga/index.html)
[![rustc](https://img.shields.io/badge/rustc-1.65+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![build status](https://img.shields.io/github/actions/workflow/status/kiranshila/casperfpga_rs/ci.yml?branch=main&style=flat-square&logo=github)](https://github.com/kiranshila/casperfpga_rs/actions)
[![Codecov](https://img.shields.io/codecov/c/github/kiranshila/casperfpga_rs?style=flat-square)](https://app.codecov.io/gh/kiranshila/casperfpga_rs)

A Rust library for interfacing with CASPER Collaboration FPGA devices. Unlike the [python version](https://github.com/casper-astro/casperfpga), this library is intended for mission-critical deployments, where breaking changes, memory bugs, and slow/interpreted languages are unacceptable. Additionally, this library will be rigorously tested, documented, and utilize fully specified interfaces.

## Goals

- Lightweight, fast, correct by construction interfaces
- Type-checked constructors based on device information (FPG file)
- Generic fall back interfaces

## Future Work

### Device Tree

Currently, `.fpg` files are the source of automatically generating typesafe interfaces for a given design. If CASPER adopts device tree, we'd want to write a parser that performs the same translation.

### Python Integration

We started working on a python wrapper utilizing [py03](https://github.com/PyO3/pyo3) to act as a multipurpose rewrite of the python version. This won't be as typesafe (of course), but should act as a more stable and tested stand-in for the previous python version.

## Contributing

Please run `cargo +nightly fmt --all` before commiting as well as check clippy with `cargo clippy --all`.

### License

casperfpga_rs is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See LICENSE-APACHE and LICENSE-MIT for details.
