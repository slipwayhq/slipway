# Slipway

Slipway is an open source framework for displaying useful information on your devices,
from eInk screens to phones to monitor walls.

For more information please see [our website](https://slipwayhq.com/).

# Contributing

Please read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening any PRs.

# Compiling

## Prerequisites
We use [Just](https://github.com/casey/just) to automate building this repository,
and [Nextest](https://github.com/nextest-rs/nextest) to run tests.
You can install both of these with:
```sh
cargo install just cargo-nextest
```

You'll also want to have the WASI Preview 2 target installed:
```sh
rustup target add wasm32-wasip2
```

When building on Linux there are a few dependencies, which you can install with your package manager, for example:
```sh
sudo apt-get install libssl-dev libsixel-bin fontconfig
```

## Building

To build the repository, run:
```sh
just build
```

## Running Tests

To run the tests, run:
```sh
just test
```

## Using

We provide a shell script `symlink.sh` which will symlink `~/bin/slipway` to the release build,
and `~/bin/slipwayd` to the debug build.

This allows you to easily run your locally compiled version of Slipway on path:

```sh
slipway --help
```

# Project Structure

## `/src`

The source code of the Slipway CLI, Slipway Engine, and associated crates.

### `/src/slipway`

The Slipway CLI. This allows you to run and debug Rigs from the command line,
as well as serve Rigs from a web server.

### `/src/slipway_engine`

The core of Slipway which evaluates the current state of a Rig along with the
inputs and outputs of the Components for the current state, and lets you move the Rig
through states until it has been fully evaluated.

### `/src/slipway_host`

A utility crate used by the various Component runners to implement the interface between
the host (Slipway) and a Component.

They make it easy for different runners, such as the WASM and Javascript runners, to behave
in a consistent manner both from the point of view of Slipway and the running Component.

### `/src/slipway_wasmtime_runner`

Runs Slipway Components which have been compiled to WebAssembly (WASM). This runner uses Wasmtime crate to execute the components.

It uses the WIT file in `/src/wit/latest` to define the interface between the host and the Component.

### `/src/slipway_js_boa_runner`

Runs Slipway Components written in Javascript. This Javascript runner uses the Boa crate, which is Javascript lexer, parser and interpreter written in Rust.

We are currently focused on using Boa for running Javascript crates because:
- We want to maintain a consistent experience (in terms of behavior, security and sandboxing)
whether we are running Slipway Rigs from the command line, or in a browser.
- We want to encourage and support the development of a native Rust Javascript engine.

### `/src/slipway_fragment_runner`

Runs "Fragment" Slipway Components. A Fragment Component is essentially a Rig which takes an
input and returns an output. A Fragment Component is used to represent part of (or a fragment of) a complete Rig as a Component.

As an example, the `echarts` Slipway Component is a Fragment Component.
It rigs together the `echarts_svg` Component (which takes an echarts definition and outputs an SVG) and the `svg` Component (which takes an SVG input and outputs a Canvas), to provide
a new Component which takes an echarts definition as an input and returns a Canvas
as an output.

### `/src/common_macros`

Rust macros used by other crates in this repository.

### `/src/common_test_utils`

Test utilities used by other crates in this repository.

## `/src_components`

The source code of Slipway Components used for testing.

## `/src/wit/latest`

The current WebAssembly Interface Type (WIT) file describing the interface between WASM
Slipway Components and the Slipway host.
