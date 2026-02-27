import figlet from "figlet";
import standard from "figlet/importable-fonts/Standard.js";
import slant from "figlet/importable-fonts/Slant.js";
import banner from "figlet/importable-fonts/Banner.js";
import wasmInit, { RatRide } from "../pkg/ratride_web.js";

figlet.parseFont("standard", standard);
figlet.parseFont("slant", slant);
figlet.parseFont("banner", banner);

// Expose to WASM — called from Rust via wasm_bindgen extern
globalThis.renderFiglet = (text, font) => {
  try {
    return figlet.textSync(text, { font: font || "standard" });
  } catch {
    return null;
  }
};

const wasmUrl = new URL("../pkg/ratride_web_bg.wasm", import.meta.url);

let wasmReady;
function ensureInit() {
  if (!wasmReady) {
    wasmReady = wasmInit({ module_or_path: wasmUrl });
  }
  return wasmReady;
}

let counter = 0;

/**
 * @param {string} md
 * @param {{ parent?: HTMLElement, fontSize?: number, theme?: string, fonts?: Record<string, string> }} [config]
 * @returns {Promise<{ destroy(): void }>}
 */
export async function run(md, config = {}) {
  await ensureInit();

  // Register user-provided figlet fonts
  if (config.fonts) {
    for (const [name, data] of Object.entries(config.fonts)) {
      figlet.parseFont(name, data);
    }
  }

  const { parent = document.body, theme } = config;
  const dpr = window.devicePixelRatio || 1;

  // Compute fontSize from parent if not provided
  const fontSize =
    config.fontSize ??
    (parseFloat(getComputedStyle(parent).fontSize) || 16);

  // Unique IDs
  const id = `ratride-${counter++}`;
  const overlayId = `${id}-overlay`;

  // Container
  const container = document.createElement("div");
  container.style.cssText =
    "position:relative;width:100%;height:100%;overflow:hidden;";

  // Canvas
  const canvas = document.createElement("canvas");
  canvas.id = id;
  canvas.setAttribute("aria-hidden", "true");
  canvas.style.display = "block";

  // Overlay
  const overlay = document.createElement("div");
  overlay.id = overlayId;
  overlay.setAttribute("role", "document");
  overlay.style.cssText =
    "position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;overflow:hidden;";

  container.appendChild(canvas);
  container.appendChild(overlay);
  parent.appendChild(container);

  // Sizing helper — guard against no-op resizes to avoid clearing the canvas
  // (setting canvas.width resets the bitmap even if the value is unchanged)
  function resize() {
    const isBody = parent === document.body;
    const w = isBody ? window.innerWidth : parent.clientWidth;
    const h = isBody ? window.innerHeight : parent.clientHeight;
    const pw = Math.round(w * dpr);
    const ph = Math.round(h * dpr);
    if (canvas.width === pw && canvas.height === ph) return;
    canvas.width = pw;
    canvas.height = ph;
    canvas.style.width = w + "px";
    canvas.style.height = h + "px";
  }
  resize();

  const ro = new ResizeObserver(resize);
  ro.observe(container);

  const instance = RatRide.run(
    md,
    id,
    theme ?? undefined,
    fontSize,
  );

  return {
    destroy() {
      instance.free();
      ro.disconnect();
      container.remove();
    },
  };
}
