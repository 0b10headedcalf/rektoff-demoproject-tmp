// Serves the page and forwards /api/* to the Rust backend, so the browser
// only ever talks to one origin.
import index from "./index.html";

const BACKEND_URL = process.env.BACKEND_URL ?? "http://127.0.0.1:8787";
const PORT = Number(process.env.PORT ?? 3000);

async function proxy(req: Request): Promise<Response> {
  const { pathname, search } = new URL(req.url);
  const hasBody = req.method !== "GET" && req.method !== "HEAD";
  try {
    return await fetch(BACKEND_URL + pathname + search, {
      method: req.method,
      headers: req.headers,
      body: hasBody ? await req.arrayBuffer() : undefined,
    });
  } catch {
    return new Response("backend unavailable", { status: 502 });
  }
}

const server = Bun.serve({
  port: PORT,
  // Result streams stay open while a job runs; the backend pings every 5s.
  idleTimeout: 60,
  routes: {
    "/": index,
    "/api/*": proxy,
  },
  development: process.env.NODE_ENV !== "production" && { hmr: true, console: true },
});

console.log(`frontend listening on ${server.url}`);
