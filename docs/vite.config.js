import { defineConfig } from "vite";
import path from "path";

export default defineConfig({
  base: process.env.GITHUB_PAGES ? "/ratride/" : "/",
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
