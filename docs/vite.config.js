import { defineConfig } from "vite";
import path from "path";

export default defineConfig({
  server: {
    fs: {
      allow: [path.resolve(__dirname, "..")],
    },
  },
  plugins: [
    {
      name: "md-hot-reload",
      handleHotUpdate({ file, server }) {
        if (file.endsWith(".md")) {
          server.ws.send({ type: "full-reload" });
        }
      },
    },
  ],
});
