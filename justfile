publisher := "slipwayhq"

default:
  just --list
  
build configuration="debug": (build-src configuration) (build-components configuration)

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
  cp wit/latest/slipway.wit src_components/slipway_increment_component/wit/slipway.wit
  cp wit/latest/slipway.wit src_components/slipway_component_file_component/wit/slipway.wit
  cp wit/latest/slipway.wit src_components/slipway_fetch_component/wit/slipway.wit
  cp wit/latest/slipway.wit src_components/slipway_env_component/wit/slipway.wit
  cp wit/latest/slipway.wit src_components/slipway_font_component/wit/slipway.wit
  cd src_components && \
    cargo build --target wasm32-wasip2 {{ if configuration == "release" { "--release" } else { "" } }} && \
    cargo build -p slipway_increment_component --features increment-ten --target-dir target/increment-ten --target wasm32-wasip2 {{ if configuration == "release" { "--release" } else { "" } }}
  
clean-src:
  cd src && cargo clean

clean-components:
  cd src_components && cargo clean

assemble-test-components configuration="debug": \
  (clean-artifacts configuration) \
  (assemble-rust-component "increment" configuration) \
  (assemble-rust-component "component_file" configuration) \
  (assemble-rust-component "fetch" configuration) \
  (assemble-rust-component "font" configuration) \
  (assemble-rust-component "env" configuration) \
  && \
  (tar-component-files "increment_ten" configuration) \
  (tar-component-files "increment_json_schema" configuration) \
  (tar-component-files "fragment" configuration) \
  (tar-component-files "increment_js" configuration) \
  
  mkdir -p artifacts/{{publisher}}.increment_ten
  cp src_components/target/increment-ten/wasm32-wasip2/{{configuration}}/slipway_increment_component.wasm artifacts/{{publisher}}.increment_ten/slipway_component.wasm
  cp src_components/slipway_increment_component/slipway_component.json artifacts/{{publisher}}.increment_ten/slipway_component.json
  jq '.name = "increment_ten"' artifacts/{{publisher}}.increment_ten/slipway_component.json > artifacts/{{publisher}}.increment_ten/slipway_component2.json
  mv artifacts/{{publisher}}.increment_ten/slipway_component2.json artifacts/{{publisher}}.increment_ten/slipway_component.json

  mkdir -p artifacts/{{publisher}}.increment_json_schema
  cp src_components/target/wasm32-wasip2/{{configuration}}/slipway_increment_component.wasm artifacts/{{publisher}}.increment_json_schema/slipway_component.wasm
  cp src_components/slipway_increment_json_schema_component/slipway_component.json artifacts/{{publisher}}.increment_json_schema/slipway_component.json
  cp src_components/slipway_increment_json_schema_component/input_schema.json artifacts/{{publisher}}.increment_json_schema/input_schema.json
  cp src_components/slipway_increment_json_schema_component/output_schema.json artifacts/{{publisher}}.increment_json_schema/output_schema.json

  mkdir -p artifacts/{{publisher}}.fragment
  cp src_components/slipway_fragment_component/slipway_component.json artifacts/{{publisher}}.fragment/slipway_component.json

  mkdir -p artifacts/{{publisher}}.increment_js
  cp src_components/slipway_increment_js_component/slipway_component.json artifacts/{{publisher}}.increment_js/slipway_component.json
  cp src_components/slipway_increment_js_component/slipway_js_component.json artifacts/{{publisher}}.increment_js/slipway_js_component.json
  cp src_components/slipway_increment_js_component/run.js artifacts/{{publisher}}.increment_js/run.js

tar-component-files name configuration="debug":
  src/target/{{configuration}}/slipway package artifacts/{{publisher}}.{{name}}

assemble-rust-component name configuration="debug": \
  && \
  (tar-component-files name configuration) \

  mkdir -p artifacts/{{publisher}}.{{name}}
  cp src_components/target/wasm32-wasip2/{{configuration}}/slipway_{{name}}_component.wasm artifacts/{{publisher}}.{{name}}/slipway_component.wasm
  cp src_components/slipway_{{name}}_component/slipway_component.json artifacts/{{publisher}}.{{name}}/slipway_component.json
