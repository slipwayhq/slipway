// Polyfill some default objects.
global = {};
setTimeout = () => {};
clearTimeout = () => {};
process = { env: {} };

function btoa(str) {
  const bytes = new TextEncoder().encode(str);
  return slipway_host.encode_bin(bytes);
}

function atob(str) {
  const bytes = slipway_host.decode_bin(str);
  const decoder = new TextDecoder('utf-8');
  return decoder.decode(bytes);
}

// Polyfill fetch API.
class Response {
  constructor(binResponse) {
    this._binResponse = binResponse;
    this.status = binResponse.status_code;
    this.headers = new Map(binResponse.headers);
    this.ok = this.status >= 200 && this.status < 300;
    this.statusText = this.ok ? 'OK' : 'Error';
  }

  async text() {
    // Convert Uint8Array to string
    const decoder = new TextDecoder('utf-8');
    const bytes = await this.bytes();
    return decoder.decode(bytes);
  }

  async json() {
    // Convert to string then parse
    const text = await this.text();
    return JSON.parse(text);
  }

  async arrayBuffer() {
    const bytes = await this.bytes();
    return bytes.buffer;
  }

  async blob() {
    const bytes = await this.bytes();
    return new Blob([bytes]);
  }

  async bytes() {
    return this._binResponse.body;
  }

  async array() {
    const bytes = await this.bytes();
    Array.from(bytes);
  }
}

async function fetch(input, init = {}) {
  // If input is Request-like, pull fields. Otherwise it's a string URL
  const url = typeof input === 'string' ? input : input.url;

  // Convert standard fetch options to RequestOptions type
  let requestOptions = {
    method: init.method,
    headers: init.headers,
    body: init.body,
    timeout_ms: init.timeout_ms 
  };

  const binResponse = await slipway_host.fetch_bin(url, requestOptions);

  // Wrap it in a fetch-like Response
  return new Response(binResponse);
}
