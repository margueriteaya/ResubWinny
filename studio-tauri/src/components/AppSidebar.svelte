<script lang="ts">
  import { Archive, BookOpen, CircleUserRound, HardDrive, Home, Layers3, ListTodo, Settings } from "@lucide/svelte";
  import packageMetadata from "../../package.json";
  import { t } from "../i18n";
  import type { Page } from "./navigation";

  export let page: Page = "home";
  export let hasTask = false;
  export let busy = false;
  export let theme: "system" | "light" | "dark" = "system";
  export let onNavigate: (page: Page) => void = () => {};

  const displayVersion = `v${packageMetadata.version.replace(/-alpha(?:\.\d+)?$/, "α")}`;
</script>

<aside class="sidebar">
  <div class="brand"><span class="brand-mark">R</span><div><strong>ResubWinny</strong><small>{t("app.tagline")}</small></div></div>
  <span class="version">{displayVersion}</span>
  <nav aria-label={t("app.navigation")}>
    <button title={t("nav.home")} class:active={page === "home"} onclick={() => onNavigate("home")}><Home size={21} /><span>{t("nav.home")}</span></button>
    <button title={t("nav.tasks")} class:active={page === "tasks"} onclick={() => onNavigate("tasks")}><ListTodo size={21} /><span>{t("nav.tasks")}</span>{#if hasTask}<em>1</em>{/if}</button>
    <button title={t("nav.batch")} class:active={page === "batch"} onclick={() => onNavigate("batch")}><Layers3 size={21} /><span>{t("nav.batch")}</span></button>
    <button title={t("nav.drcs")} class:active={page === "drcs"} onclick={() => onNavigate("drcs")}><BookOpen size={21} /><span>{t("nav.drcs")}</span></button>
    <button title={t("nav.settings")} class:active={page === "settings"} onclick={() => onNavigate("settings")}><Settings size={21} /><span>{t("nav.settings")}</span></button>
  </nav>
  <div class="runtime-card"><p><i class:busy></i>{busy ? t("task.processing") : t("common.ready")}</p><small><HardDrive size={13} /> {t("app.runtimeWorker")}</small><small><Archive size={13} /> {t("app.runtimeStandard")}</small><small><CircleUserRound size={13} /> {theme === "system" ? t("settings.themeSystem") : theme === "dark" ? t("settings.themeDark") : t("settings.themeLight")}</small></div>
</aside>
