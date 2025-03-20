export async function run(input) {
  switch (input.type) {
    case "increment":
      console.error("This is an error.");
      console.warn("This is a warning.");
      console.log("This is information.");
      console.debug("This is debug information.");
      console.trace("This is trace information.");
      return {
        value: input.value + 1
      };
    
    case "callout_increment":
        if (input.ttl === 0) {
          if (input.result_type === "increment") {
            return run({
              type: "increment",
              value: input.value
            });
          }
          else {
            return run({
              type: input.result_type,
            });
          }
        }
        else {
          var callout_input = {
            type: "callout_increment",
            value: input.value + 1,
            ttl: input.ttl - 1,
            result_type: input.result_type
          };
          return await slipway_host.run("increment", callout_input);
        }
      
    case "error":
      throw new Error("slipway-increment-js-component-error.");

    default:
      throw new Error("Unexpected input type: " + input.type);
  }  
}
