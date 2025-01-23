run();

function run() {
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
    
      case "throw":
        throw new Error("This is a thrown error.");

      default:
          throw new Error("Unexpected input type: " + input.type);
  }  
}
