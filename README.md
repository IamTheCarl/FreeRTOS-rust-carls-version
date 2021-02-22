# FreeRTOS-rust (Carl's version)

This project is based on code from [freertos-rust](https://github.com/lobaro/FreeRTOS-rust), which is based on [freertos.rs](https://github.com/hashmismatch/freertos.rs) and some additions to
 simplify the usage of [FreeRTOS](https://github.com/FreeRTOS/FreeRTOS-Kernel) in embedded applications written
 in Rust.

This is a work in progress but it's planned to eventually be significantly different from the original freertos-rust. Rather than wrapping C functions in structs and calling it a day, this library intends to make FreeRTOS feel like it was designed to be used with Rust.

Planned changes:
* Compiler enforced prevention of calling non-ISR safe functions from an ISR
* Safe deferred interrupts
* Have FreeRTOS use the Rust global heap, rather than Rust use the FreeRTOS heap.

Complete changes:
* Tasks are given a handle to themselves

## How it works

The `freertos-cargo-build` build-dependency compiles the FreeRTOS code from its original "C" sources files into an 
archive to be linked against your Rust app. Internally it uses the [cc crate](https://docs.rs/crate/cc) and some meta 
info provided by your apps `build.rs`:
 
 1. A path to the [FreeRTOS](https://github.com/FreeRTOS/FreeRTOS-Kernel) `Sources`
 1. A path to the app specific `FreeRTOSConfig.h`
 1. A relative path to the `FreeRTOS port` to be used, e.g. for ARM Cortex-M3 cores.
 1. Optional: Additional C code to be compiled
 
 The `freertos-rust` dependency provides an interface to access all FreeRTOS functionality from your (embedded) 
 Rust app.

# License
This repository is using the MIT License. Some parts might state different licenses that need to be respected when used.

* The [Linux port](https://github.com/michaelbecker/freertos-addons) is licensed under GPLv2




