<script lang="ts">
  import DrcsDictionary from "./DrcsDictionary.svelte";
  import type { DrcsGlyph, DrcsMapping } from "../../backend";
  import { t } from "../../i18n";

  let {
    glyphs = [],
    message = "",
    canRefresh = false,
    onRefresh = () => {},
    getMapping = () => undefined,
    onSaveMapping = () => {},
  }: {
    glyphs?: DrcsGlyph[];
    message?: string;
    canRefresh?: boolean;
    onRefresh?: () => void;
    getMapping?: (id: string) => { text: string; action: DrcsMapping["action"] } | undefined;
    onSaveMapping?: (id: string, text: string, action: DrcsMapping["action"]) => void;
  } = $props();
</script>

<header class="workspace-header">
  <div><h1>{t("drcs.title")}</h1><p>{t("drcs.description")}</p></div>
  <div class="header-actions"><button class="outline" onclick={onRefresh} disabled={!canRefresh}>{t("drcs.refreshResources")}</button></div>
</header>
<section class="drcs-page">
  <DrcsDictionary {glyphs} {message} {getMapping} saveMapping={onSaveMapping} />
</section>

<style>
  .drcs-page{min-width:0;min-height:0;overflow:hidden}
  :global(main[data-page="drcs"] .application){display:grid;grid-template-rows:auto minmax(0,1fr);overflow:hidden}
  @media(max-width:720px){:global(main[data-page="drcs"] .application){overflow:auto}.drcs-page{overflow:visible}}
</style>
