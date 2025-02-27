# Slipway

Slipway is an application written in Rust which allows the user to render useful information
using reusable components which can be displayed on any device you like.

For more information please see [our website](https://slipwayhq.com/).

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

It uses the WIT file in `/wit/latest` to define the interface between the host and the Component.

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

### `/src/slipway_js_deno_runner`

Runs Slipway Components written in Javascript. This Javascript runner which is does not
currently have development focus. Although it can run Components, it has not (yet) had the host interface
implemented so doesn't support Components which call back into the host.

This Javascript runner uses the `deno_engine` crate for running Javascript Components,
which under the hood uses the C++ V8 engine.

### `/src/common_macros`

Rust macros used by other crates in this repository.

### `/src/common_test_utils`

Test utilities used by other crates in this repository.

## `/src_components`

The source code of Slipway Components used for testing.

## `/wit/latest`

The current WebAssembly Interface Type (WIT) file describing the interface between WASM
Slipway Components and the Slipway host.
