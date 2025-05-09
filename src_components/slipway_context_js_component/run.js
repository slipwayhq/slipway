export function run(input) {
  return {
    tz: process.env.TZ,
    lc: process.env.LC,
    input,
  }
}
