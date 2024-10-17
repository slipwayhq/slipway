default:
  just --list
  
build: build-wasm build-src

test *FLAGS: build
  cd src && cargo nextest run {{FLAGS}}

clean: clean-src clean-wasm
  rm -rf test-components

build-src:
  cd src && cargo build

build-wasm: && assemble-test-components
  cd wasm && cargo build
  
clean-src:
  cd src && cargo clean

clean-wasm:
  cd wasm && cargo clean

assemble-test-components:
  rm -rf test-components
  mkdir -p test-components/slipway_test_component
  cp wasm/target/wasm32-wasi/debug/slipway_test_component.wasm test-components/slipway_test_component/slipway_component.wasm
  cp wasm/slipway-test-component/slipway_component.json test-components/slipway_test_component/slipway_component.json

  mkdir -p test-components/slipway_test_component_json_schema
  cp wasm/target/wasm32-wasi/debug/slipway_test_component.wasm test-components/slipway_test_component_json_schema/slipway_component.wasm
  cp wasm/alternative-definition-files/slipway_component_json_schema.json test-components/slipway_test_component_json_schema/slipway_component.json
  cp wasm/alternative-definition-files/input-schema.json test-components/slipway_test_component_json_schema/input-schema.json
  cp wasm/alternative-definition-files/output-schema.json test-components/slipway_test_component_json_schema/output-schema.json

  tar -cf test-components/slipway_test_component_json_schema.tar -C test-components/slipway_test_component_json_schema .
