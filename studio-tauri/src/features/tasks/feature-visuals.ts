import IconArrowsMove from "@tabler/icons-svelte/icons/arrows-move";
import IconEar from "@tabler/icons-svelte/icons/ear";
import IconGridDots from "@tabler/icons-svelte/icons/grid-dots";
import IconLanguage from "@tabler/icons-svelte/icons/language";
import IconLanguageHiragana from "@tabler/icons-svelte/icons/language-hiragana";
import IconPalette from "@tabler/icons-svelte/icons/palette";
import type { ExportPreservation } from "../../backend";

export const featureVisuals: Record<keyof ExportPreservation, { icon: any }> = {
  position: { icon: IconArrowsMove },
  color: { icon: IconPalette },
  ruby: { icon: IconLanguageHiragana },
  gaiji: { icon: IconLanguage },
  drcs: { icon: IconGridDots },
  accessibility: { icon: IconEar },
};
