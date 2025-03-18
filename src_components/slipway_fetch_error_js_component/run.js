async function run() {
  const requestOptions = {
    // ERROR: We are passing a map instead of a list of key-value pairs.
    // We are using this component to test that a sensible set of error messages
    // propagates to the user.
    headers: {
      "Content-Type": "application/json",
    }, 
  };

  await slipway_host.fetch_text("https://example.com", requestOptions);
}

export let output = run();
