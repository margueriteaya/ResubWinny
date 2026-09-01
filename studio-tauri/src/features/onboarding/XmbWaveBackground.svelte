<script lang="ts">
  // Direct integration of linkev/PlayStation-3-XMB at
  // 1ec453a9dddec5448d615116ff428349f42d454e (MIT).
  import { onMount } from "svelte";

  type RenderLayer = { render: (timeSeconds: number) => void };
  type XmbGlobals = Window & {
	SPLINE_SETTINGS?: Record<string, unknown>;
	PARTICLE_SETTINGS?: Record<string, unknown>;
	PS3SplineReverse?: unknown;
    createSplineLayer?: (gl: WebGL2RenderingContext, canvas: HTMLCanvasElement) => RenderLayer;
    createParticlesLayer?: (gl: WebGL2RenderingContext, canvas: HTMLCanvasElement) => RenderLayer;
    RESUBWINNY_XMB_BACKGROUND_ALPHA?: number;
    RESUBWINNY_XMB_VERTICAL_OFFSET?: number;
  };

  const upstreamScripts = [
    "spline-settings.js",
    "particles-settings.js",
    "spline-reverse.js",
    "spline.js",
    "particles.js",
  ];

  let canvas: HTMLCanvasElement;
  let ready = false;

  function loadScript(source: string) {
    return new Promise<void>((resolve, reject) => {
      const existing = document.querySelector<HTMLScriptElement>(`script[data-xmb-source="${source}"]`);
      if (existing?.dataset.loaded === "true") return resolve();
      const script = existing ?? document.createElement("script");
      const onLoad = () => {
        script.dataset.loaded = "true";
        resolve();
      };
      script.addEventListener("load", onLoad, { once: true });
      script.addEventListener("error", () => reject(new Error(`Unable to load ${source}`)), { once: true });
      if (!existing) {
        script.src = `/onboarding/xmb-upstream/${source}`;
        script.dataset.xmbSource = source;
        document.head.append(script);
      }
    });
  }

  async function loadUpstream() {
    const globals = window as XmbGlobals;
    if (globals.createSplineLayer && globals.createParticlesLayer) return;
    await Promise.all(upstreamScripts.slice(0, 2).map(loadScript));
    await loadScript(upstreamScripts[2]);
    await Promise.all(upstreamScripts.slice(3).map(loadScript));
  }

  onMount(() => {
    let stopped = false;
    let frame = 0;
    let previous = performance.now();
    let splineTime = 0;
    let particleTime = Math.random() * 1000;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    let gl: WebGL2RenderingContext | null = null;

    const start = async () => {
      try {
        await loadUpstream();
        if (stopped) return;
        gl = canvas.getContext("webgl2", {
          antialias: true,
          alpha: true,
          premultipliedAlpha: true,
          powerPreference: "high-performance",
        });
        if (!gl) return;
        gl.getExtension("OES_texture_float_linear");
        gl.getExtension("EXT_color_buffer_float");

        const globals = window as XmbGlobals;
        const splineLayer = globals.createSplineLayer?.(gl, canvas);
        const particlesLayer = globals.createParticlesLayer?.(gl, canvas);
        if (!splineLayer || !particlesLayer) return;
        const splineSettings = globals.SPLINE_SETTINGS as Record<string, number>;
        const particleSettings = globals.PARTICLE_SETTINGS as Record<string, number>;
        const baseWaveOpacity = splineSettings.opacity;
        const baseWaveBrightness = splineSettings.brightness;
        const baseParticleOpacity = particleSettings.opacity;

        const resize = () => {
          if (!gl) return;
          const rect = canvas.getBoundingClientRect();
          const dpr = Math.min(devicePixelRatio || 1, 1.5);
          const width = Math.max(1, Math.floor(rect.width * dpr));
          const height = Math.max(1, Math.floor(rect.height * dpr));
          if (canvas.width !== width || canvas.height !== height) {
            canvas.width = width;
            canvas.height = height;
            gl.viewport(0, 0, width, height);
          }
        };

        const draw = (now: number) => {
          if (stopped || !gl) return;
          resize();
          const staticMotion = reduced.matches || document.documentElement.dataset.glassStatic === "true";
          const delta = staticMotion ? 0 : Math.max(0, (now - previous) / 1000);
          previous = now;
          splineTime += delta;
          particleTime += delta;
          const isDark = document.documentElement.dataset.theme === "dark";
          const modulation = .5 + .25 * Math.sin(splineTime * .73 + 1.2) + .25 * Math.sin(splineTime * .31 + 2.1);
          const dayBoost = 1.05 + .02 * Math.max(0, Math.min(1, modulation));
          splineSettings.opacity = baseWaveOpacity * (isDark ? 1 : dayBoost);
          splineSettings.brightness = baseWaveBrightness * (isDark ? 1 : dayBoost);
          particleSettings.opacity = baseParticleOpacity * (isDark ? 1 : dayBoost);
          globals.RESUBWINNY_XMB_BACKGROUND_ALPHA = 0;
          const canvasRect = canvas.getBoundingClientRect();
          const axisRect = document.querySelector<HTMLElement>("[data-wave-axis]")?.getBoundingClientRect();
          const axisY = axisRect ? axisRect.top - canvasRect.top : canvasRect.height * .5;
          globals.RESUBWINNY_XMB_VERTICAL_OFFSET = 1 - 2 * axisY / Math.max(1, canvasRect.height);
          splineLayer.render(staticMotion ? 8.25 : splineTime);
          particlesLayer.render(staticMotion ? 308.25 : particleTime);
          if (!staticMotion && document.visibilityState === "visible") frame = requestAnimationFrame(draw);
        };

        const restart = () => {
          cancelAnimationFrame(frame);
          previous = performance.now();
          frame = requestAnimationFrame(draw);
        };
        const preferenceObserver = new MutationObserver(restart);
        preferenceObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-glass-static", "data-theme"] });
        const resizeObserver = new ResizeObserver(restart);
        resizeObserver.observe(canvas);
        document.addEventListener("visibilitychange", restart);
        reduced.addEventListener("change", restart);
        ready = true;
        restart();

        return () => {
          splineSettings.opacity = baseWaveOpacity;
          splineSettings.brightness = baseWaveBrightness;
          particleSettings.opacity = baseParticleOpacity;
          preferenceObserver.disconnect();
          resizeObserver.disconnect();
          document.removeEventListener("visibilitychange", restart);
          reduced.removeEventListener("change", restart);
        };
      } catch (error) {
        console.warn("Original XMB wave background unavailable", error);
      }
    };

    let cleanup: (() => void) | undefined;
    void start().then((value) => cleanup = value);
    return () => {
      stopped = true;
      ready = false;
      cancelAnimationFrame(frame);
      cleanup?.();
      gl?.getExtension("WEBGL_lose_context")?.loseContext();
    };
  });
</script>

<canvas bind:this={canvas} class:ready aria-hidden="true"></canvas>

<style>
  canvas{position:absolute;z-index:1;inset:0;display:block;width:100%;height:100%;pointer-events:none;visibility:hidden;clip-path:inset(0 0 0 100%);opacity:0}
  canvas.ready{visibility:visible;animation:wave-arrive 1500ms 90ms var(--rw-ease-fluid) forwards}
  @keyframes wave-arrive{0%{clip-path:inset(0 0 0 100%);opacity:0;transform:translateX(7%)}28%{opacity:.92}100%{clip-path:inset(0);opacity:1;transform:none}}
  :global(html[data-glass-static="true"]) canvas.ready{animation:none;clip-path:inset(0);opacity:1;transform:none}
  @media(prefers-reduced-motion:reduce){canvas.ready{animation:none;clip-path:inset(0);opacity:1;transform:none}}
  @media(forced-colors:active){canvas{display:none}}
</style>
