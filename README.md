# casperfpga_rs

A Rust library for interfacing with CASPER Collaboration FPGA devices. Unlike the [python version](https://github.com/casper-astro/casperfpga), this library is intended for mission-critical deployments, where breaking changes, memory bugs, and slow/interpreted languages are unacceptable. Additionally, this library will be rigorously tested, documented, and utilize fully specified interfaces.

## Python Integration

We use [py03](https://github.com/PyO3/pyo3) to create a python wrapper to act as a multipurpose rewrite of the python version. This won't be as typesafe (of course), but should act as a more stable and tested stand-in for the previous python version.