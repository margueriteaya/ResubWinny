/*
 * The displacement-map approach is adapted from shuding/liquid-glass (MIT).
 * Maps are generated once per control geometry and Chromium performs the live
 * backdrop sampling. No page snapshot or idle render loop is involved.
 */

const SVG_NS = "http://www.w3.org/2000/svg";

type FilterRecord = {
  id: string;
  width: number;
  height: number;
};

export type LiquidDisplacementController = {
  observe: (element: HTMLElement) => void;
  detach: (root: ParentNode) => void;
  setEnabled: (enabled: boolean) => void;
  destroy: () => void;
  supported: boolean;
};

function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(maximum, value));
}

function smoothstep(minimum: number, maximum: number, value: number) {
  const position = clamp((value - minimum) / (maximum - minimum), 0, 1);
  return position * position * (3 - 2 * position);
}

function roundedRectangleDistance(
  x: number,
  y: number,
  halfWidth: number,
  halfHeight: number,
  radius: number,
) {
  const qx = Math.abs(x) - halfWidth + radius;
  const qy = Math.abs(y) - halfHeight + radius;
  return Math.min(Math.max(qx, qy), 0)
    + Math.hypot(Math.max(qx, 0), Math.max(qy, 0))
    - radius;
}

function displacementMap(width: number, height: number, radius: number) {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d", { willReadFrequently: false });
  if (!context) return null;

  const image = context.createImageData(width, height);
  const horizontal = new Float32Array(width * height);
  const vertical = new Float32Array(width * height);
  const centerX = width / 2;
  const centerY = height / 2;
  const halfWidth = Math.max(1, width / 2 - .5);
  const halfHeight = Math.max(1, height / 2 - .5);
  const edgeBand = clamp(Math.min(width, height) * .24, 4, 11);
  const pull = Math.min(width, height) <= 40 ? .15 : .105;
  let maximumOffset = 1;

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = y * width + x;
      const localX = x + .5 - centerX;
      const localY = y + .5 - centerY;
      const distance = roundedRectangleDistance(
        localX,
        localY,
        halfWidth,
        halfHeight,
        radius,
      );
      const edge = distance <= 0 ? smoothstep(-edgeBand, 0, distance) : 0;
      const easedEdge = edge * edge * (3 - 2 * edge);
      const dx = -localX * pull * easedEdge;
      const dy = -localY * pull * easedEdge;
      horizontal[index] = dx;
      vertical[index] = dy;
      maximumOffset = Math.max(maximumOffset, Math.abs(dx), Math.abs(dy));
    }
  }

  const scale = maximumOffset * 2;
  for (let index = 0; index < horizontal.length; index += 1) {
    const pixel = index * 4;
    image.data[pixel] = clamp((horizontal[index] / scale + .5) * 255, 0, 255);
    image.data[pixel + 1] = clamp((vertical[index] / scale + .5) * 255, 0, 255);
    image.data[pixel + 2] = 128;
    image.data[pixel + 3] = 255;
  }

  context.putImageData(image, 0, 0);
  return { href: canvas.toDataURL("image/png"), scale };
}

function logicalRadius(element: HTMLElement, width: number, height: number) {
  const parsed = Number.parseFloat(getComputedStyle(element).borderTopLeftRadius);
  return clamp(Number.isFinite(parsed) ? parsed : 0, 0, Math.min(width, height) / 2);
}

export function createLiquidDisplacementController(): LiquidDisplacementController {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.classList.add("rw-liquid-filter-definitions");
  svg.setAttribute("width", "0");
  svg.setAttribute("height", "0");
  svg.setAttribute("aria-hidden", "true");
  const definitions = document.createElementNS(SVG_NS, "defs");
  svg.append(definitions);
  document.body.append(svg);

  const supportsBackdrop = typeof CSS !== "undefined"
    && (CSS.supports("backdrop-filter", "blur(1px)")
      || CSS.supports("-webkit-backdrop-filter", "blur(1px)"));
  const supportsSvgBackdrop = supportsBackdrop
    && (CSS.supports("backdrop-filter", "url(#rw-liquid-support-test)")
      || CSS.supports("-webkit-backdrop-filter", "url(#rw-liquid-support-test)"));
  const filters = new Map<string, FilterRecord>();
  const observed = new Set<HTMLElement>();
  const assignments = new WeakMap<HTMLElement, string>();
  let enabled = supportsSvgBackdrop;
  let sequence = 0;

  function recordFor(element: HTMLElement, width: number, height: number) {
    const radius = Math.round(logicalRadius(element, width, height) * 2) / 2;
    const key = `${width}:${height}:${radius}`;
    const cached = filters.get(key);
    if (cached) return cached;
    const map = displacementMap(width, height, radius);
    if (!map) return null;

    const id = `rw-liquid-displacement-${++sequence}`;
    const filter = document.createElementNS(SVG_NS, "filter");
    filter.setAttribute("id", id);
    filter.setAttribute("filterUnits", "userSpaceOnUse");
    filter.setAttribute("primitiveUnits", "userSpaceOnUse");
    filter.setAttribute("color-interpolation-filters", "sRGB");
    filter.setAttribute("x", "0");
    filter.setAttribute("y", "0");
    filter.setAttribute("width", String(width));
    filter.setAttribute("height", String(height));

    const image = document.createElementNS(SVG_NS, "feImage");
    image.setAttribute("href", map.href);
    image.setAttribute("width", String(width));
    image.setAttribute("height", String(height));
    image.setAttribute("preserveAspectRatio", "none");
    image.setAttribute("result", "displacement");

    const displacement = document.createElementNS(SVG_NS, "feDisplacementMap");
    displacement.setAttribute("in", "SourceGraphic");
    displacement.setAttribute("in2", "displacement");
    displacement.setAttribute("scale", map.scale.toFixed(3));
    displacement.setAttribute("xChannelSelector", "R");
    displacement.setAttribute("yChannelSelector", "G");

    filter.append(image, displacement);
    definitions.append(filter);
    const record = { id, width, height };
    filters.set(key, record);
    return record;
  }

  function update(element: HTMLElement) {
    if (!element.isConnected) return;
    if (!enabled) {
      element.dataset.liquidRefraction = supportsBackdrop ? "backdrop" : "static";
      element.style.removeProperty("backdrop-filter");
      element.style.removeProperty("-webkit-backdrop-filter");
      return;
    }

    const bounds = element.getBoundingClientRect();
    const width = clamp(Math.round(bounds.width), 1, 480);
    const height = clamp(Math.round(bounds.height), 1, 320);
    if (width < 4 || height < 4) return;
    const record = recordFor(element, width, height);
    if (!record) return;
    const assignment = `${record.id}:${record.width}:${record.height}`;
    if (assignments.get(element) === assignment) return;
    assignments.set(element, assignment);
    const value = `url("#${record.id}") blur(.35px) saturate(1.08) brightness(1.025)`;
    element.style.setProperty("backdrop-filter", value);
    element.style.setProperty("-webkit-backdrop-filter", value);
    element.dataset.liquidRefraction = "svg";
  }

  const resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) update(entry.target as HTMLElement);
  });

  function observe(element: HTMLElement) {
    if (observed.has(element)) return;
    observed.add(element);
    resizeObserver.observe(element);
    update(element);
  }

  function detach(root: ParentNode) {
    const candidates = root instanceof HTMLElement && root.matches(".liquid-visual-layer")
      ? [root]
      : Array.from(root.querySelectorAll<HTMLElement>(".liquid-visual-layer"));
    for (const element of candidates) {
      resizeObserver.unobserve(element);
      observed.delete(element);
    }
  }

  return {
    observe,
    detach,
    setEnabled(nextEnabled) {
      enabled = supportsSvgBackdrop && nextEnabled;
      for (const element of observed) update(element);
    },
    destroy() {
      resizeObserver.disconnect();
      observed.clear();
      filters.clear();
      svg.remove();
    },
    supported: supportsSvgBackdrop,
  };
}
