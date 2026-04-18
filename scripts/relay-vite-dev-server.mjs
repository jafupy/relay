import fs from "node:fs";
import http from "node:http";
import { createServer as createViteServer } from "vite";

const socketIndex = process.argv.indexOf("--socket");
const socketPath = socketIndex >= 0 ? process.argv[socketIndex + 1] : null;

if (!socketPath) {
  console.error("missing --socket");
  process.exit(2);
}

try {
  fs.rmSync(socketPath, { force: true });
} catch {
  // The socket may not exist yet.
}

let vite;
const server = http.createServer((request, response) => {
  vite.middlewares(request, response, (error) => {
    if (error) {
      vite.ssrFixStacktrace(error);
      response.statusCode = 500;
      response.end(error.stack || error.message);
      return;
    }
    response.statusCode = 404;
    response.end("Not found");
  });
});

vite = await createViteServer({
  appType: "spa",
  server: {
    middlewareMode: true,
    hmr: { server, path: "/@vite-hmr" },
  },
});

server.listen(socketPath, () => {
  console.log(JSON.stringify({ socketPath }));
});

const shutdown = async () => {
  server.close();
  await vite.close();
  try {
    fs.rmSync(socketPath, { force: true });
  } catch {
    // Best effort cleanup.
  }
  process.exit(0);
};

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
