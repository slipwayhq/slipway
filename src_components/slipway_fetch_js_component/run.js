async function run(input) {
  const { url, method, headers, body, response_type } = input;
  const requestOptions = {
    headers: Object.entries(headers),
    method,
    body,
    timeout_ms: 1000,
  };

  function mapErrToOutput(e) {
    if (e.response) {
      let encoder = new TextEncoder();
      var body_bin = Array.from(encoder.encode(e.response.body));
      return {
        status_code: e.response.status_code,
        body_text: e.message,
        body_bin,
      };
    }
    throw e;
  }

  try {
    if (response_type === "text") {
      const res = await slipway_host.fetch_text(url, requestOptions);
      return {
        status_code: res.status_code,
        body_text: res.body,
      };
    } else if (response_type === "binary") {
      const res = await slipway_host.fetch_bin(url, requestOptions);
      return {
        status_code: res.status_code,
        body_bin: res.body,
      };
    } else {
      throw new Error(`Unsupported response_type: ${response_type}`);
    }
  } catch (e) {
    slipway_host.log_error(JSON.stringify(e));
    return mapErrToOutput(e);
  }
}

export let output = run(input);
