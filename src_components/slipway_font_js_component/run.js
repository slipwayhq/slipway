export async function run(input) {
  let font_stack = input.font_stack;
  let font_result = await slipway_host.font(font_stack);

  if (font_result && font_result.data) {
    if (!(font_result.data instanceof Uint8Array)) {
      throw new Error("Expected font data to be a Uint8Array.");      
    }
    
    return {
      bin_length: font_result.data.length,
    }
  }

  return {
    bin_length: 0,
  }
}
