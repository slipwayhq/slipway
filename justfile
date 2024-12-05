default:
  just --list
  
build configuration="debug": (build-components configuration) (build-src configuration)

test *FLAGS: build
  cd src && cargo nextest run {{FLAGS}}

test-only *FLAGS:
  cd src && cargo nextest run {{FLAGS}}

clean: clean-src clean-components (clean-artifacts "")

clean-artifacts configuration:
  mkdir -p artifacts
  rm -rf artifacts

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

assemble-test-components configuration="debug": \
  (clean-artifacts configuration) \
  && \
  (tar-component-files "test") \
  (rename-component-artifacts "test") \
  (tar-component-files "test_json_schema") \
  (rename-component-artifacts "test_json_schema") \
  (tar-component-files "fragment") \
  (rename-component-artifacts "fragment") \

  mkdir -p artifacts/slipway_test
  cp src_components/target/wasm32-wasip1/{{configuration}}/slipway_test_component.wasm artifacts/slipway_test/slipway_component.wasm
  cp src_components/slipway_test_component/slipway_component.json artifacts/slipway_test/slipway_component.json

  mkdir -p artifacts/slipway_test_json_schema
  cp src_components/target/wasm32-wasip1/{{configuration}}/slipway_test_component.wasm artifacts/slipway_test_json_schema/slipway_component.wasm
  cp src_components/slipway_test_json_schema_component/slipway_component.json artifacts/slipway_test_json_schema/slipway_component.json
  cp src_components/slipway_test_json_schema_component/input_schema.json artifacts/slipway_test_json_schema/input_schema.json
  cp src_components/slipway_test_json_schema_component/output_schema.json artifacts/slipway_test_json_schema/output_schema.json

  mkdir -p artifacts/slipway_fragment
  cp src_components/slipway_fragment_component/slipway_component.json artifacts/slipway_fragment/slipway_component.json

tar-component-files name:
  tar -cf artifacts/slipway_{{name}}.tar -C artifacts/slipway_{{name}} .

rename-component-artifacts name:
  # Rename the tarball with a name that includes the publisher, name and version.
  publisher=$(jq -r '.publisher' src_components/slipway_{{name}}_component/slipway_component.json) && \
    name=$(jq -r '.name' src_components/slipway_{{name}}_component/slipway_component.json) && \
    version=$(jq -r '.version' src_components/slipway_{{name}}_component/slipway_component.json) && \
    new_filename="${publisher}.${name}.${version}" && \
    mv artifacts/slipway_{{name}} "artifacts/$new_filename" && \
    mv artifacts/slipway_{{name}}.tar "artifacts/$new_filename.tar"

