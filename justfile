default:
  just --list
  
build configuration="debug": (build-components configuration) (build-src configuration)

test *FLAGS: build
  cd src && cargo nextest run {{FLAGS}}

clean: clean-src clean-components
  rm -rf test-components

build-src configuration="debug":
  cd src && cargo build {{ if configuration == "release" { "--release" } else { "" } }}

build-components configuration="debug": && (assemble-test-components configuration)
  cd src-components && \
    cargo component build -p slipway-test-component {{ if configuration == "release" { "--release" } else { "" } }}
  
clean-src:
  cd src && cargo clean

clean-components:
  cd src-components && cargo clean

assemble-test-components configuration="debug":
  rm -rf test-components

  mkdir -p test-components/slipway_test_component
  cp src-components/target/wasm32-wasi/{{configuration}}/slipway_test_component.wasm test-components/slipway_test_component/slipway_component.wasm
  cp src-components/slipway-test-component/slipway_component.json test-components/slipway_test_component/slipway_component.json

  mkdir -p test-components/slipway_test_component_json_schema
  cp src-components/target/wasm32-wasi/{{configuration}}/slipway_test_component.wasm test-components/slipway_test_component_json_schema/slipway_component.wasm
  cp src-components/alternative-definition-files/slipway_component_json_schema.json test-components/slipway_test_component_json_schema/slipway_component.json
  cp src-components/alternative-definition-files/input-schema.json test-components/slipway_test_component_json_schema/input-schema.json
  cp src-components/alternative-definition-files/output-schema.json test-components/slipway_test_component_json_schema/output-schema.json

  tar -cf test-components/slipway_test_component_json_schema.tar -C test-components/slipway_test_component_json_schema .
  cp test-components/slipway_test_component_json_schema.tar test-components/slipway.test_component_json_schema.0.1.2.tar 
