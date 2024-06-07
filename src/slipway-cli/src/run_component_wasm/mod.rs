use slipway_lib::ComponentExecutionData;
use wasmtime::*;

use self::errors::WasmExecutionError;

pub(super) mod errors;

pub(super) fn run_component_wasm(
    _execution_data: ComponentExecutionData,
) -> Result<serde_json::Value, WasmExecutionError> {
    todo!();
    // // Create an engine and store
    // let engine = Engine::default();
    // let mut store = Store::new(&engine, ());

    // // Compile the module
    // let module = Module::new(&engine, &*execution_data.wasm_bytes)?;

    // // Create an instance of the module
    // let instance = Instance::new(&mut store, &module, &[])?;

    // // Get the WASM function
    // let wasm_func = instance
    //     .get_func(&mut store, "your_wasm_function")
    //     .expect("function not found")
    //     .typed::<i32, i32, _>(&store)?;

    // // Example input JSON
    // let input_json: Value = serde_json::from_str(r#"{"key": "value"}"#)?;

    // // Serialize the input JSON to a vector of bytes
    // let input_bytes = to_vec(&input_json)?;

    // // Pass the serialized JSON to the WASM function
    // let input_ptr = store
    //     .get_global("input_ptr")
    //     .expect("input_ptr not found")
    //     .i32()
    //     .unwrap();

    // // Here, you might need to copy input_bytes to the memory buffer used by WASM.
    // // This step depends on your WASM memory management and API.
    // // For simplicity, let's assume you have a function to do this:
    // copy_bytes_to_wasm_memory(&mut store, input_ptr, &input_bytes)?;

    // // Call the function (assuming it returns a pointer to the output in WASM memory)
    // let output_ptr = wasm_func.call(&mut store, input_ptr)?;

    // // Retrieve the result from WASM memory
    // let output_bytes = read_bytes_from_wasm_memory(&mut store, output_ptr)?;

    // // Deserialize the output bytes back into JSON
    // let output_json: Value = from_slice(&output_bytes)?;

    // // Print the output JSON
    // println!("Output JSON: {}", output_json);
}

// fn copy_bytes_to_wasm_memory(
//     store: &mut Store<()>,
//     ptr: i32,
//     bytes: &[u8],
// ) -> Result<(), Box<dyn std::error::Error>> {
//     // Implementation to copy bytes to WASM memory
//     // This is a placeholder and should be implemented based on your WASM memory management
//     Ok(())
// }

// fn read_bytes_from_wasm_memory(
//     store: &mut Store<()>,
//     ptr: i32,
// ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
//     // Implementation to read bytes from WASM memory
//     // This is a placeholder and should be implemented based on your WASM memory management
//     Ok(Vec::new())
// }
