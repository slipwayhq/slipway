import { BINARY } from './constants_module.js';

export async function run(input) {
  if (input.file_type === TEXT) {
    var text = await slipway_host.load_text(input.handle, input.path);
    return {
      text
    };
  }
  else if (input.file_type === BINARY) {
    var bin = await slipway_host.load_bin(input.handle, input.path);
    if (!(bin instanceof Uint8Array)) {
      throw new Error("Expected binary data to be a Uint8Array.");      
    }

    return {
      bin: Array.from(bin)
    };
  }
  else {
    throw new Error("Unexpected file type: " + input.file_type);
  }
}
