default:
  just --list
  
build: build-wasm build-src

test: build
  cd src && cargo test

clean: clean-src clean-wasm
  rm -rf test-components

build-src:
  cd src && cargo build

build-wasm:
  cd wasm && cargo build
  mkdir -p test-components/slipway_test_component
  cp wasm/slipway-test-component/slipway_component.json test-components/slipway_test_component/slipway_component.json
  cp wasm/target/wasm32-wasi/debug/slipway_test_component.wasm test-components/slipway_test_component/slipway_component.wasm

clean-src:
  cd src && cargo clean

clean-wasm:
  cd wasm && cargo clean
