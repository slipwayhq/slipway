run();

function run() {
  let font_stack = input.font_stack;
  let font_result = slipway_host.font(font_stack);
  return {
    bin_length: font_result ? font_result.data.length : 0,
  }
}