# casperfpga_rs

A Rust library for interfacing with CASPER Collaboration FPGA devices. Unlike the [python version](https://github.com/casper-astro/casperfpga), this library is intended for mission-critical deployments, where breaking changes, memory bugs, and slow/interpreted languages are unacceptable. Additionally, this library will be rigorously tested, documented, and utilize fully specified interfaces.

## Future Goals

Use [py03](https://github.com/PyO3/pyo3) to create a python wrapper to act as a multipurpose rewrite of the python version. As the interface is through C FFI, this library could theoretically be embedded in any language with a C FFI (all of them), implying we can get the benefit of writing systems code in Rust and interactive code in Python, Julia, Clojure, etc.