import wasmInit, { RatRide } from "../pkg/ratride_web.js";

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

  // --- Touch navigation ---
  canvas.style.touchAction = "none";

  let touchStartX = 0;
  let touchStartY = 0;
  let touchLastY = 0;
  let touchStartTime = 0;
  let didScroll = false;
  let accumulatedScrollY = 0;
  const TAP_ZONE = 0.4; // left/right 40% of canvas width
  const MOVE_THRESHOLD = 10; // px to distinguish tap from scroll
  const TAP_MAX_DURATION = 200; // ms

  canvas.addEventListener("touchstart", (e) => {
    if (e.touches.length !== 1) return;
    const t = e.touches[0];
    touchStartX = t.clientX;
    touchStartY = t.clientY;
    touchLastY = t.clientY;
    touchStartTime = e.timeStamp;
    didScroll = false;
    accumulatedScrollY = 0;
  }, { passive: false });

  canvas.addEventListener("touchmove", (e) => {
    if (e.touches.length !== 1) return;
    e.preventDefault();
    const t = e.touches[0];
    const dy = t.clientY - touchStartY;

    if (Math.abs(dy) > MOVE_THRESHOLD) {
      didScroll = true;
    }

    if (didScroll) {
      const deltaY = touchLastY - t.clientY; // positive = finger up = scroll down
      touchLastY = t.clientY;
      accumulatedScrollY += deltaY;
      const cellH = instance.cell_height();
      if (cellH > 0) {
        while (accumulatedScrollY >= cellH) {
          instance.scroll_down(1);
          accumulatedScrollY -= cellH;
        }
        while (accumulatedScrollY <= -cellH) {
          instance.scroll_up(1);
          accumulatedScrollY += cellH;
        }
      }
    }
  }, { passive: false });

  canvas.addEventListener("touchend", (e) => {
    if (didScroll || e.timeStamp - touchStartTime >= TAP_MAX_DURATION) return;
    // Tap: check if in left/right 40% zone
    const rect = canvas.getBoundingClientRect();
    const relX = (touchStartX - rect.left) / rect.width;
    if (relX <= TAP_ZONE) {
      instance.prev_page();
    } else if (relX >= 1 - TAP_ZONE) {
      instance.next_page();
    }
  }, { passive: true });

  return {
    destroy() {
      instance.free();
      ro.disconnect();
      container.remove();
    },
  };
}
