publisher := "slipwayhq"

default:
  just --list
  
build configuration="debug": (build-src configuration) (build-components configuration)

test *FLAGS: (build-src "release") (build-components "release")
  cd src && RUST_LOG="debug,cranelift_codegen=info,wasmtime_cranelift=info" cargo nextest run --no-fail-fast --release {{FLAGS}}

test-only *FLAGS:
  cd src && RUST_LOG="debug,cranelift_codegen=info,wasmtime_cranelift=info" cargo nextest run --no-fail-fast --release {{FLAGS}}

clean: clean-src clean-components (clean-component-artifacts "")

clean-component-artifacts configuration:
  mkdir -p components
  rm -rf components

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
  (clean-component-artifacts configuration) \
  (assemble-rust-component "increment" configuration) \
  (assemble-rust-component "component_file" configuration) \
  (assemble-rust-component "fetch" configuration) \
  (assemble-rust-component "font" configuration) \
  (assemble-rust-component "env" configuration) \
  (assemble-js-component "increment_js" configuration) \
  (assemble-js-component "component_file_js" configuration) \
  (assemble-js-component "fetch_js" configuration) \
  (assemble-js-component "fetch_error_js" configuration) \
  (assemble-js-component "font_js" configuration) \
  (assemble-js-component "env_js" configuration) \
  && \
  (tar-component-files "increment_ten" configuration) \
  (tar-component-files "increment_json_schema" configuration) \
  (tar-component-files "fragment" configuration) \
  (tar-component-files "slipway_increment_invalid_callout_permissions" configuration) \
  (tar-component-files "slipway_increment_js_invalid_callout_permissions" configuration) \
  
  mkdir -p components/{{publisher}}.increment_ten
  cp src_components/target/increment-ten/wasm32-wasip2/{{configuration}}/slipway_increment_component.wasm components/{{publisher}}.increment_ten/run.wasm
  cp src_components/slipway_increment_component/slipway_component.json components/{{publisher}}.increment_ten/slipway_component.json
  jq '.name = "increment_ten"' components/{{publisher}}.increment_ten/slipway_component.json > components/{{publisher}}.increment_ten/slipway_component2.json
  mv components/{{publisher}}.increment_ten/slipway_component2.json components/{{publisher}}.increment_ten/slipway_component.json

  mkdir -p components/{{publisher}}.increment_json_schema
  cp src_components/target/wasm32-wasip2/{{configuration}}/slipway_increment_component.wasm components/{{publisher}}.increment_json_schema/run.wasm
  cp src_components/slipway_increment_json_schema_component/slipway_component.json components/{{publisher}}.increment_json_schema/slipway_component.json
  cp src_components/slipway_increment_json_schema_component/input_schema.json components/{{publisher}}.increment_json_schema/input_schema.json
  cp src_components/slipway_increment_json_schema_component/output_schema.json components/{{publisher}}.increment_json_schema/output_schema.json

  mkdir -p components/{{publisher}}.fragment
  cp src_components/slipway_fragment_component/slipway_component.json components/{{publisher}}.fragment/slipway_component.json

  mkdir -p components/{{publisher}}.slipway_increment_invalid_callout_permissions
  cp components/{{publisher}}.increment/* components/{{publisher}}.slipway_increment_invalid_callout_permissions
  jq '.name = "increment_invalid_callout_permissions" | del(.callouts.increment.allow)' components/{{publisher}}.slipway_increment_invalid_callout_permissions/slipway_component.json > components/{{publisher}}.slipway_increment_invalid_callout_permissions/slipway_component.temp
  mv components/{{publisher}}.slipway_increment_invalid_callout_permissions/slipway_component.temp components/{{publisher}}.slipway_increment_invalid_callout_permissions/slipway_component.json

  mkdir -p components/{{publisher}}.slipway_increment_js_invalid_callout_permissions
  cp components/{{publisher}}.increment_js/* components/{{publisher}}.slipway_increment_js_invalid_callout_permissions
  jq '.name = "increment_js_invalid_callout_permissions" | del(.callouts.increment.allow)' components/{{publisher}}.slipway_increment_js_invalid_callout_permissions/slipway_component.json > components/{{publisher}}.slipway_increment_js_invalid_callout_permissions/slipway_component.temp
  mv components/{{publisher}}.slipway_increment_js_invalid_callout_permissions/slipway_component.temp components/{{publisher}}.slipway_increment_js_invalid_callout_permissions/slipway_component.json

tar-component-files name configuration="debug":
  src/target/{{configuration}}/slipway package components/{{publisher}}.{{name}}

assemble-rust-component name configuration="debug": \
  && \
  (tar-component-files name configuration) \

  mkdir -p components/{{publisher}}.{{name}}
  cp src_components/target/wasm32-wasip2/{{configuration}}/slipway_{{name}}_component.wasm components/{{publisher}}.{{name}}/run.wasm
  cp src_components/slipway_{{name}}_component/slipway_component.json components/{{publisher}}.{{name}}/slipway_component.json

assemble-js-component name configuration="debug": \
  && \
  (tar-component-files name configuration) \

  mkdir -p components/{{publisher}}.{{name}}
  cp src_components/slipway_{{name}}_component/* components/{{publisher}}.{{name}}

push-docker-image:
  docker buildx build --platform linux/amd64,linux/arm64 -t slipwayhq/slipway:latest . --push
  