run(input);

async function run(input) {
  if (input.file_type === "text") {
    var text = await slipway_host.load_text(input.handle, input.path);
    return {
      text
    };
  }
  else {
    var bin = await slipway_host.load_bin(input.handle, input.path);
    return {
      bin
    };
  }
}