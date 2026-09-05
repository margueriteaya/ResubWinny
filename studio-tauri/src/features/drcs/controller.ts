import { backend, type DrcsGlyph, type DrcsMapping } from "../../backend";

export type SavedDrcsMapping = {
  text: string;
  action: DrcsMapping["action"];
};

export function serialiseDrcsMappings(
  mappings: Record<string, SavedDrcsMapping>,
): DrcsMapping[] {
  return Object.entries(mappings).map(([id, mapping]) => ({
    id,
    text: mapping.text,
    action: mapping.action,
  }));
}

type DrcsHooks = {
  desktopRuntime: boolean;
  sourcePath: () => string | undefined;
  mappings: () => Record<string, SavedDrcsMapping>;
  updateMappings: (mappings: Record<string, SavedDrcsMapping>) => void;
  updateGlyphs: (glyphs: DrcsGlyph[]) => void;
  updateMessage: (message: string) => void;
  message: (code: string, parameters?: Record<string, unknown>) => string;
};

export class DrcsDictionaryController {
  constructor(private readonly hooks: DrcsHooks) {}

  get(id: string) {
    return this.hooks.mappings()[id];
  }

  export(): DrcsMapping[] {
    return serialiseDrcsMappings(this.hooks.mappings());
  }

  async load(sourceOverride?: string) {
    const source = sourceOverride ?? this.hooks.sourcePath();
    if (!source) {
      this.hooks.updateGlyphs([]);
      this.hooks.updateMessage(this.hooks.message("drcs.selectTask"));
      return;
    }
    if (!this.hooks.desktopRuntime) {
      this.hooks.updateMessage(this.hooks.message("drcs.desktopOnly"));
      return;
    }
    try {
      const glyphs = await backend.loadDrcsReport(source);
      this.hooks.updateGlyphs(glyphs);
      this.hooks.updateMessage(
        glyphs.length
          ? this.hooks.message("drcs.loaded", { count: glyphs.length })
          : this.hooks.message("drcs.noneInTask"),
      );
    } catch (reason) {
      this.hooks.updateGlyphs([]);
      this.hooks.updateMessage(
        this.hooks.message("error.backend", { message: String(reason) }),
      );
    }
  }

  save(id: string, text: string, action: SavedDrcsMapping["action"]) {
    const mappings = { ...this.hooks.mappings(), [id]: { text, action } };
    this.hooks.updateMappings(mappings);
    if (this.hooks.desktopRuntime) void backend.saveDrcsMappings(this.export());
    this.hooks.updateMessage(this.hooks.message("drcs.mappingSaved", { id }));
  }
}
