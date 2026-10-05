// Loopback HTTP proxy for talking to a plain-HTTP (no TLS, no cert) gateway.
//
// adk-rust's Anthropic client refuses any base_url that isn't https:// unless
// the host is loopback (127.0.0.1 / localhost) — it's a guard against sending
// the API key in cleartext over a real network. If your gateway is plain
// HTTP with no certificate at all, point the Rust client at this proxy
// instead (which IS loopback), and this proxy makes the real plain-HTTP call
// to the gateway on your behalf.
//
// Usage:
//   node proxy.js <listen_port> <gateway_host> <gateway_port>
//
// Example:
//   node proxy.js 8787 internal-gateway.company.local 80
//
// Then in your .env / Rust code, point at the proxy instead of the gateway:
//   base_url = "http://127.0.0.1:8787/api/v1/llm/anthropic"
// (keep whatever path suffix your real gateway expects — it's forwarded unchanged.)

const http = require("node:http");

const [, , listenPortArg, gatewayHost, gatewayPortArg] = process.argv;

if (!gatewayHost) {
  console.error("Usage: node proxy.js <listen_port> <gateway_host> <gateway_port>");
  process.exit(1);
}

const listenPort = Number(listenPortArg) || 8787;
const gatewayPort = Number(gatewayPortArg) || 80;

const server = http.createServer((req, res) => {
  const proxyReq = http.request(
    {
      hostname: gatewayHost,
      port: gatewayPort,
      path: req.url,
      method: req.method,
      headers: { ...req.headers, host: `${gatewayHost}:${gatewayPort}` },
    },
    (proxyRes) => {
      res.writeHead(proxyRes.statusCode, proxyRes.headers);
      proxyRes.pipe(res, { end: true });
    },
  );

  proxyReq.on("error", (err) => {
    console.error(`[proxy] upstream error: ${err.message}`);
    if (!res.headersSent) res.writeHead(502);
    res.end(`Bad gateway: ${err.message}`);
  });

  req.pipe(proxyReq, { end: true });
});

server.listen(listenPort, "127.0.0.1", () => {
  console.log(`Proxying http://127.0.0.1:${listenPort} -> http://${gatewayHost}:${gatewayPort}`);
});
