import type { DrcsMapping, PlaybackTimeMapping, PreviewRuntime } from "../backend";

type BootstrapHooks = {
  desktopRuntime: boolean;
  getPlaybackTimeMapping: () => Promise<PlaybackTimeMapping>;
  getPreviewRuntime: () => Promise<PreviewRuntime>;
  loadTaskHistory: () => Promise<unknown[]>;
  loadDrcsMappings: () => Promise<DrcsMapping[]>;
  onError: (reason: unknown) => void;
};

export type BootstrapState = {
  playbackMapping?: PlaybackTimeMapping;
  previewAvailable?: boolean;
  history: unknown[];
  drcsMappings: DrcsMapping[];
};

/** Loads independent desktop-only startup resources without coupling the shell to RPC details. */
export class BootstrapSession {
  constructor(private readonly hooks: BootstrapHooks) {}

  async load(): Promise<BootstrapState> {
    const empty: BootstrapState = { history: [], drcsMappings: [] };
    if (!this.hooks.desktopRuntime) return empty;
    const [mapping, runtime, history, drcs] = await Promise.all([
      this.hooks.getPlaybackTimeMapping().catch((reason) => { this.hooks.onError(reason); return undefined; }),
      this.hooks.getPreviewRuntime().catch((reason) => { this.hooks.onError(reason); return undefined; }),
      this.hooks.loadTaskHistory().catch((reason) => { this.hooks.onError(reason); return []; }),
      this.hooks.loadDrcsMappings().catch((reason) => { this.hooks.onError(reason); return []; }),
    ]);
    return {
      playbackMapping: mapping,
      previewAvailable: runtime?.available ?? false,
      history,
      drcsMappings: drcs,
    };
  }
}
