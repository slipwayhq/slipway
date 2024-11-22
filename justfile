default:
  just --list
  
build configuration="debug": (build-components configuration) (build-src configuration)

test *FLAGS: build
  cd src && cargo nextest run {{FLAGS}}

clean: clean-src clean-components
  rm -rf test_components

build-src configuration="debug":
  cd src && cargo build {{ if configuration == "release" { "--release" } else { "" } }}

build-components configuration="debug": && (assemble-test-components configuration)
  cp wit/latest/slipway_component.wit src_components/slipway_test_component/wit/world.wit
  cd src_components && \
    cargo component build -p slipway_test_component {{ if configuration == "release" { "--release" } else { "" } }}
  
clean-src:
  cd src && cargo clean

clean-components:
  cd src_components && cargo clean

assemble-test-components configuration="debug":
  rm -rf test_components

  mkdir -p test_components/slipway_test_component
  cp src_components/target/wasm32-wasip1/{{configuration}}/slipway_test_component.wasm test_components/slipway_test_component/slipway_component.wasm
  cp src_components/slipway_test_component/slipway_component.json test_components/slipway_test_component/slipway_component.json

  mkdir -p test_components/slipway_test_component_json_schema
  cp src_components/target/wasm32-wasip1/{{configuration}}/slipway_test_component.wasm test_components/slipway_test_component_json_schema/slipway_component.wasm
  cp src_components/alternative_definition_files/slipway_component_json_schema.json test_components/slipway_test_component_json_schema/slipway_component.json
  cp src_components/alternative_definition_files/input_schema.json test_components/slipway_test_component_json_schema/input_schema.json
  cp src_components/alternative_definition_files/output_schema.json test_components/slipway_test_component_json_schema/output_schema.json

  tar -cf test_components/slipway_test_component_json_schema.tar -C test_components/slipway_test_component_json_schema .
  cp test_components/slipway_test_component_json_schema.tar test_components/slipway.test_component_json_schema.0.1.2.tar 
