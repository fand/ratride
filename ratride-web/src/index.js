import wasmInit, { RatRide } from "../pkg/ratride_web.js";

let wasmReady;
function ensureInit() {
  if (!wasmReady) {
    wasmReady = wasmInit();
  }
  return wasmReady;
}

let counter = 0;

/**
 * @param {string} md
 * @param {{ parent?: HTMLElement, fontSize?: number, theme?: string }} [config]
 * @returns {Promise<{ destroy(): void }>}
 */
export async function run(md, config = {}) {
  await ensureInit();

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

  // Sizing helper
  function resize() {
    const w = container.clientWidth;
    const h = container.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
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
