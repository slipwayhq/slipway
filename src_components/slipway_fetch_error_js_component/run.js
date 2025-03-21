export async function run() {
  const requestOptions = {
    // ERROR: We are passing three-tuple list instead of a two-tuple list.
    // We are using this component to test that a sensible set of error messages
    // propagates to the user.
    headers: [
      ["Content-Type", "application/json", "oops"],
    ],
  };

  await slipway_host.fetch_text("https://example.com", requestOptions);
}
