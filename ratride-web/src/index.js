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

  // --- Touch / swipe navigation ---
  canvas.style.touchAction = "none";

  let touchStartX = 0;
  let touchStartY = 0;
  let touchLastY = 0;
  let swipeAxis = null; // null | "horizontal" | "vertical"
  let accumulatedScrollY = 0;
  const AXIS_LOCK_THRESHOLD = 10; // px before axis is decided
  const SWIPE_THRESHOLD = 50; // px for horizontal page change

  canvas.addEventListener("touchstart", (e) => {
    if (e.touches.length !== 1) return;
    const t = e.touches[0];
    touchStartX = t.clientX;
    touchStartY = t.clientY;
    touchLastY = t.clientY;
    swipeAxis = null;
    accumulatedScrollY = 0;
  }, { passive: false });

  canvas.addEventListener("touchmove", (e) => {
    if (e.touches.length !== 1) return;
    e.preventDefault();
    const t = e.touches[0];
    const dx = t.clientX - touchStartX;
    const dy = t.clientY - touchStartY;

    // Lock axis once movement exceeds threshold
    if (swipeAxis === null) {
      if (Math.abs(dx) > AXIS_LOCK_THRESHOLD || Math.abs(dy) > AXIS_LOCK_THRESHOLD) {
        swipeAxis = Math.abs(dx) > Math.abs(dy) ? "horizontal" : "vertical";
      } else {
        return;
      }
    }

    if (swipeAxis === "vertical") {
      const deltaY = touchLastY - t.clientY; // positive = finger moves up = scroll down
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
    // horizontal: do nothing during move, decide on touchend
  }, { passive: false });

  canvas.addEventListener("touchend", (e) => {
    if (swipeAxis === "horizontal") {
      const dx = (e.changedTouches[0]?.clientX ?? touchStartX) - touchStartX;
      if (dx < -SWIPE_THRESHOLD) {
        instance.next_page();
      } else if (dx > SWIPE_THRESHOLD) {
        instance.prev_page();
      }
    }
    swipeAxis = null;
  }, { passive: true });

  return {
    destroy() {
      instance.free();
      ro.disconnect();
      container.remove();
    },
  };
}
