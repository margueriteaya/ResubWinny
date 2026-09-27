type BufferInfo = {
  kind: "control" | "pixels";
  generation: number;
  slot?: number;
};
type BufferEvent = Event & { additionalData: BufferInfo | string; getBuffer(): ArrayBuffer };
type Bridge = {
  addEventListener(type: "sharedbufferreceived", listener: (event: BufferEvent) => void): void;
  removeEventListener(type: "sharedbufferreceived", listener: (event: BufferEvent) => void): void;
  releaseBuffer(buffer: ArrayBuffer): void;
};
declare global { interface Window { chrome?: { webview?: Bridge } } }

const GENERATION = 2, RUNNING = 3, DROPPED = 5, SLOT_BASE = 16, SLOT_WORDS = 5;
const STATE = 0, SEQUENCE = 1, WIDTH = 2, HEIGHT = 3, FREE = 0, READY = 2, READING = 3;

function atomicLoad(words: Uint32Array, index: number) {
  return Atomics.load(words, index);
}
function atomicStore(words: Uint32Array, index: number, value: number) {
  Atomics.store(words, index, value);
}
function atomicAdd(words: Uint32Array, index: number, value: number) {
  Atomics.add(words, index, value);
}
function claim(words: Uint32Array, index: number) {
  return Atomics.compareExchange(words, index, READY, READING) === READY;
}
function parseInfo(value: BufferInfo | string) {
  try { return typeof value === "string" ? JSON.parse(value) as BufferInfo : value; } catch { return null; }
}
function compileShader(gl: WebGLRenderingContext, kind: number, source: string) {
  const shader = gl.createShader(kind);
  if (!shader) throw new Error("Could not create the preview shader.");
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS))
    throw new Error(gl.getShaderInfoLog(shader) || "Could not compile the preview shader.");
  return shader;
}
function createProgram(gl: WebGLRenderingContext) {
  const program = gl.createProgram();
  if (!program) throw new Error("Could not create the preview WebGL program.");
  gl.attachShader(program, compileShader(gl, gl.VERTEX_SHADER,
    "attribute vec2 position;attribute vec2 uv;varying vec2 textureUv;void main(){textureUv=uv;gl_Position=vec4(position,0.0,1.0);}"));
  gl.attachShader(program, compileShader(gl, gl.FRAGMENT_SHADER,
    "precision mediump float;uniform sampler2D frame;varying vec2 textureUv;void main(){gl_FragColor=texture2D(frame,textureUv);}"));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS))
    throw new Error(gl.getProgramInfoLog(program) || "Could not link the preview WebGL program.");
  return program;
}

export class PreviewFrameRenderer {
  private bridge = window.chrome?.webview;
  private gl: WebGLRenderingContext;
  private texture: WebGLTexture;
  private control: ArrayBuffer | null = null;
  private words: Uint32Array | null = null;
  private pixels: Array<ArrayBuffer | null> = [null, null, null];
  private generation = 0;
  private animation = 0;
  private lastSequence = 0;
  private shown = false;
  private textureWidth = 0;
  private textureHeight = 0;

  constructor(
    private canvas: HTMLCanvasElement,
    private onFirstFrame: () => void,
    private onError: (reason: unknown) => void,
  ) {
    if (!this.bridge) throw new Error("WebView2 shared video buffers are unavailable.");
    const context = canvas.getContext("webgl2", { alpha: false, antialias: false })
      ?? canvas.getContext("webgl", { alpha: false, antialias: false });
    if (!context) throw new Error("WebGL is unavailable for video preview.");
    this.gl = context as WebGLRenderingContext;
    const program = createProgram(this.gl);
    this.gl.useProgram(program);
    const vertices = this.gl.createBuffer();
    if (!vertices) throw new Error("Could not allocate the preview vertex buffer.");
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, vertices);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, new Float32Array([
      -1, -1, 0, 0, 1, -1, 1, 0, -1, 1, 0, 1, 1, 1, 1, 1,
    ]), this.gl.STATIC_DRAW);
    for (const [name, offset] of [["position", 0], ["uv", 8]] as const) {
      const location = this.gl.getAttribLocation(program, name);
      this.gl.enableVertexAttribArray(location);
      this.gl.vertexAttribPointer(location, 2, this.gl.FLOAT, false, 16, offset);
    }
    const texture = this.gl.createTexture();
    if (!texture) throw new Error("Could not allocate the preview texture.");
    this.texture = texture;
    this.gl.bindTexture(this.gl.TEXTURE_2D, texture);
    this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_MIN_FILTER, this.gl.LINEAR);
    this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_MAG_FILTER, this.gl.LINEAR);
    this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_WRAP_S, this.gl.CLAMP_TO_EDGE);
    this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_WRAP_T, this.gl.CLAMP_TO_EDGE);
    this.bridge.addEventListener("sharedbufferreceived", this.receive);
    this.animation = requestAnimationFrame(this.draw);
  }

  dispose() {
    cancelAnimationFrame(this.animation);
    this.bridge?.removeEventListener("sharedbufferreceived", this.receive);
    this.releasePool();
  }

  private receive = (event: BufferEvent) => {
    const info = parseInfo(event.additionalData);
    const buffer = event.getBuffer();
    if (!info || !Number.isInteger(info.generation)) {
      this.bridge?.releaseBuffer(buffer);
      return;
    }
    if (info.kind === "control") {
      if (info.generation < this.generation) {
        this.bridge?.releaseBuffer(buffer);
        return;
      }
      this.releasePool();
      this.generation = info.generation;
      this.control = buffer;
      this.words = new Uint32Array(buffer);
      this.lastSequence = 0;
      this.shown = false;
      return;
    }
    if (!this.control || info.generation !== this.generation
      || info.slot == null || info.slot < 0 || info.slot >= 3) {
      this.bridge?.releaseBuffer(buffer);
      return;
    }
    const previous = this.pixels[info.slot];
    if (previous) this.bridge?.releaseBuffer(previous);
    this.pixels[info.slot] = buffer;
  };

  private draw = () => {
    try {
      const words = this.words;
      if (words && atomicLoad(words, GENERATION) === this.generation) {
        if (atomicLoad(words, RUNNING) === 0) this.releasePool();
        else this.drawLatest(words);
      }
    } catch (reason) {
      this.onError(reason);
      this.releasePool();
    } finally {
      this.animation = requestAnimationFrame(this.draw);
    }
  };

  private drawLatest(words: Uint32Array) {
    let selected = -1;
    let sequence = this.lastSequence;
    for (let slot = 0; slot < 3; slot++) {
      const base = SLOT_BASE + slot * SLOT_WORDS;
      if (atomicLoad(words, base + STATE) !== READY) continue;
      const candidate = atomicLoad(words, base + SEQUENCE);
      if (candidate > sequence) { selected = slot; sequence = candidate; }
    }
    if (selected < 0) return;
    for (let slot = 0; slot < 3; slot++) {
      if (slot === selected) continue;
      const base = SLOT_BASE + slot * SLOT_WORDS;
      if (atomicLoad(words, base + STATE) === READY
        && atomicLoad(words, base + SEQUENCE) < sequence) {
        atomicStore(words, base + STATE, FREE);
        atomicAdd(words, DROPPED, 1);
      }
    }
    const base = SLOT_BASE + selected * SLOT_WORDS;
    if (!claim(words, base + STATE)) return;
    try {
      const width = atomicLoad(words, base + WIDTH);
      const height = atomicLoad(words, base + HEIGHT);
      const pixels = this.pixels[selected];
      if (!pixels || width < 1 || height < 1 || width > 1920 || height > 1080) return;
      if (this.canvas.width !== width || this.canvas.height !== height) {
        this.canvas.width = width;
        this.canvas.height = height;
        this.gl.viewport(0, 0, width, height);
      }
      this.gl.bindTexture(this.gl.TEXTURE_2D, this.texture);
      if (this.textureWidth !== width || this.textureHeight !== height) {
        this.gl.texImage2D(this.gl.TEXTURE_2D, 0, this.gl.RGBA, width, height, 0,
          this.gl.RGBA, this.gl.UNSIGNED_BYTE, null);
        this.textureWidth = width;
        this.textureHeight = height;
      }
      this.gl.texSubImage2D(this.gl.TEXTURE_2D, 0, 0, 0, width, height,
        this.gl.RGBA, this.gl.UNSIGNED_BYTE, new Uint8Array(pixels, 0, width * height * 4));
      this.gl.drawArrays(this.gl.TRIANGLE_STRIP, 0, 4);
      this.lastSequence = sequence;
      if (!this.shown) { this.shown = true; this.onFirstFrame(); }
    } finally {
      atomicStore(words, base + STATE, FREE);
    }
  }

  private releasePool() {
    const buffers = [this.control, ...this.pixels];
    this.control = null;
    this.words = null;
    this.pixels = [null, null, null];
    for (const buffer of buffers) {
      if (!buffer) continue;
      try { this.bridge?.releaseBuffer(buffer); } catch { /* detached during teardown */ }
    }
  }
}
