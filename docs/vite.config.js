import { defineConfig } from "vite";
import { spawn } from "child_process";
import path from "path";

function rebuildWasm() {
  return new Promise((resolve, reject) => {
    const proc = spawn("npm", ["run", "build"], {
      cwd: path.resolve(__dirname, "../ratride-web"),
      stdio: "inherit",
    });
    proc.on("close", (code) =>
      code === 0 ? resolve() : reject(new Error(`wasm build exit ${code}`))
    );
  });
}

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
    {
      name: "wasm-rebuild",
      configureServer(server) {
        // Watch Rust source files
        server.watcher.add([
          path.resolve(__dirname, "../ratride/src"),
          path.resolve(__dirname, "../ratride-web/src"),
        ]);

        let building = false;
        server.watcher.on("change", async (file) => {
          if (!file.endsWith(".rs") || building) return;
          building = true;
          console.log("\nRust file changed, rebuilding WASM...");
          try {
            await rebuildWasm();
            server.ws.send({ type: "full-reload" });
          } catch (e) {
            console.error("WASM build failed:", e.message);
          }
          building = false;
        });
      },
    },
  ],
});
