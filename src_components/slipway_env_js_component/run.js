export function run(input) {
  const env = slipway_host.env(input.key);

  return env === null ? {} : {
    value: env
  };
}