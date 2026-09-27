import IconArrowsMove from "@tabler/icons-svelte/icons/arrows-move";
import type { ExportPreservation } from "../../backend";
import accessibilityAsset from "../../assets/arib/accessibility.svg";
import colorAsset from "../../assets/arib/color.svg";
import drcsAsset from "../../assets/arib/drcs.svg";
import gaijiAsset from "../../assets/arib/gaiji.svg";
import rubyAsset from "../../assets/arib/ruby.svg";

type FeatureVisual =
  | { kind: "component"; icon: any }
  | { kind: "asset"; asset: string };

export const featureVisuals: Record<keyof ExportPreservation, FeatureVisual> = {
  position: { kind: "component", icon: IconArrowsMove },
  color: { kind: "asset", asset: colorAsset },
  ruby: { kind: "asset", asset: rubyAsset },
  gaiji: { kind: "asset", asset: gaijiAsset },
  drcs: { kind: "asset", asset: drcsAsset },
  accessibility: { kind: "asset", asset: accessibilityAsset },
};
