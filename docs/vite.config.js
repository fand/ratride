import { defineConfig } from "vite";
import path from "path";
import fs from "fs";

export default defineConfig({
  server: {
    fs: {
      allow: [path.resolve(__dirname, "..")],
    },
  },
  plugins: [
    {
      name: "serve-parent-files",
      configureServer(server) {
        // Serve files from project root (e.g. /examples/test.md → ../examples/test.md)
        server.middlewares.use((req, _res, next) => {
          const filePath = path.resolve(__dirname, "..", req.url.slice(1));
          if (fs.existsSync(filePath) && fs.statSync(filePath).isFile()) {
            req.url = "/@fs/" + filePath;
          }
          next();
        });
      },
      handleHotUpdate({ file, server }) {
        if (file.endsWith(".md")) {
          server.ws.send({ type: "full-reload" });
        }
      },
    },
  ],
});
