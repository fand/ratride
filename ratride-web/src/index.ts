import wasmInit, { RatRide } from "../pkg/ratride_web.js";

const wasmUrl = new URL("../pkg/ratride_web_bg.wasm", import.meta.url);

let wasmReady: Promise<unknown>;
function ensureInit(): Promise<unknown> {
  if (!wasmReady) {
    wasmReady = wasmInit({ module_or_path: wasmUrl });
  }
  return wasmReady;
}

let counter = 0;

export interface RatrideConfig {
  parent?: HTMLElement;
  fontSize?: number;
  theme?: string;
}

export interface RatrideInstance {
  destroy(): void;
}

export async function run(
  md: string,
  config: RatrideConfig = {},
): Promise<RatrideInstance> {
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

  // Store target physical size as data attributes so that the Rust backend
  // can apply them inside tick(). This avoids the flicker caused by setting
  // canvas.width (which clears the bitmap) in a separate callback from the
  // redraw that happens in tick().
  function updateTargetSize(): void {
    const isBody = parent === document.body;
    const w = isBody ? window.innerWidth : parent.clientWidth;
    const h = isBody ? window.innerHeight : parent.clientHeight;
    canvas.dataset.tw = String(Math.round(w * dpr));
    canvas.dataset.th = String(Math.round(h * dpr));
    // Don't update canvas.style.width/height here — Rust applies it
    // together with canvas.width/height to avoid bitmap stretching.
  }
  // Set initial size directly (no flicker on first frame)
  {
    const isBody = parent === document.body;
    const w = isBody ? window.innerWidth : parent.clientWidth;
    const h = isBody ? window.innerHeight : parent.clientHeight;
    const pw = Math.round(w * dpr);
    const ph = Math.round(h * dpr);
    canvas.width = pw;
    canvas.height = ph;
    canvas.style.width = w + "px";
    canvas.style.height = h + "px";
    canvas.dataset.tw = String(pw);
    canvas.dataset.th = String(ph);
  }

  const ro = new ResizeObserver(updateTargetSize);
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

  canvas.addEventListener("touchstart", (e: TouchEvent) => {
    if (e.touches.length !== 1) return;
    const t = e.touches[0];
    touchStartX = t.clientX;
    touchStartY = t.clientY;
    touchLastY = t.clientY;
    touchStartTime = e.timeStamp;
    didScroll = false;
    accumulatedScrollY = 0;
  }, { passive: false });

  canvas.addEventListener("touchmove", (e: TouchEvent) => {
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

  canvas.addEventListener("touchend", (e: TouchEvent) => {
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
