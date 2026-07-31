<script lang="ts">
  import { onMount } from 'svelte'
  import { Check, ChevronDown, FileText, FolderOpen, MonitorCog, Palette, RotateCcw, Save, Type } from '@lucide/svelte'
  import { backend, type AppSettings, type PreviewRuntime } from './backend'
  import { availableLocales, registerLanguagePacks, t } from './i18n'
  import { isDesktopRuntime } from './shell/desktop'

  type Panel = 'appearance' | 'typography' | 'output' | 'player'
  export let saveCaptionFont: (font: string) => void = () => {}
  export let onSettingsSaved: (settings: AppSettings) => void | Promise<void> = () => {}
  export let onSettingsPreview: (settings: AppSettings) => void | Promise<void> = () => {}
  const defaults: AppSettings = { uiFont: 'system', captionFont: 'arib', defaultFormat: 'ASS', defaultTimeline: 'Auto (Gap Merge + Overlap Resolve)', locale: 'system', theme: 'system' }
  let preferences: AppSettings = { ...defaults }
  export let panel: Panel = 'typography'
  let saved = false
  let previewRuntime: PreviewRuntime | null = null
  let installedLocales = availableLocales()
  let languageRefreshBusy = false
  let languageError = ''
  let languageMenuOpen = false
  $: selectedLanguageLabel = preferences.locale === 'system'
    ? t('settings.languageSystem')
    : installedLocales.find((pack) => pack.locale === preferences.locale)?.name ?? preferences.locale

  function applyFont() {
    const font = preferences.uiFont === 'system'
      ? 'Inter, "Segoe UI", "Noto Sans SC", "Noto Sans JP", "Noto Sans TC", "Microsoft YaHei UI", Meiryo, sans-serif'
      : preferences.uiFont === 'cjk'
        ? '"Noto Sans SC", "Noto Sans JP", "Noto Sans TC", "Microsoft YaHei UI", Meiryo, sans-serif'
        : '"Rounded M+ 1m for ARIB", "Noto Sans JP", "Microsoft YaHei UI", Meiryo, sans-serif'
    document.documentElement.style.setProperty('--resubwinny-ui-font', font)
  }

  async function save() {
    preferences = await backend.updateSettings(preferences)
    saveCaptionFont(preferences.captionFont)
    await onSettingsSaved(preferences)
    applyFont(); saved = true
    window.setTimeout(() => saved = false, 2500)
  }

  async function reset() {
    preferences = await backend.updateSettings({ ...defaults })
    saveCaptionFont('arib'); await onSettingsSaved(preferences); applyFont()
  }

  async function previewAppearance() {
    preferences = await backend.updateSettings(preferences)
    applyFont()
    await onSettingsPreview({ ...preferences })
  }

  async function refreshLanguagePacks() {
    if (!isDesktopRuntime() || languageRefreshBusy) return
    languageRefreshBusy = true
    languageError = ''
    try {
      registerLanguagePacks(await backend.listLanguagePacks())
      installedLocales = availableLocales()
      if (preferences.locale !== 'system' && !installedLocales.some((pack) => pack.locale === preferences.locale)) {
        preferences = { ...preferences, locale: 'system' }
        await previewAppearance()
      }
    } catch (reason) {
      languageError = String(reason)
    } finally {
      languageRefreshBusy = false
    }
  }

  async function openLanguagePackDirectory() {
    if (!isDesktopRuntime()) return
    languageError = ''
    try {
      await backend.openLanguagePackDirectory()
    } catch (reason) {
      languageError = String(reason)
    }
  }

  async function toggleLanguageMenu() {
    if (languageMenuOpen) {
      languageMenuOpen = false
      return
    }
    await refreshLanguagePacks()
    languageMenuOpen = true
  }

  async function selectLanguage(next: string) {
    preferences = { ...preferences, locale: next }
    languageMenuOpen = false
    await previewAppearance()
  }

  async function previewCaptionFont() {
    preferences = await backend.updateSettings(preferences)
    saveCaptionFont(preferences.captionFont)
    await onSettingsPreview({ ...preferences })
  }

  onMount(() => {
    backend.getSettings().then((settings) => { preferences = settings; applyFont() }).catch(() => applyFont())
    void refreshLanguagePacks()
    backend.getPreviewRuntime().then((runtime) => previewRuntime = runtime).catch(() => previewRuntime = null)
  })
</script>

<section class="settings-shell">
  <nav class="settings-nav" aria-label={t('nav.settings')}><button class:selected={panel === 'appearance'} onclick={() => panel = 'appearance'}><Palette size={18} /> {t('settings.appearance')}</button><button class:selected={panel === 'typography'} onclick={() => panel = 'typography'}><Type size={18} /> {t('settings.typography')}</button><button class:selected={panel === 'output'} onclick={() => panel = 'output'}><FileText size={18} /> {t('settings.output')}</button><button class:selected={panel === 'player'} onclick={() => panel = 'player'}><MonitorCog size={18} /> {t('settings.player')}</button></nav>
  <section class="settings-content">
    {#if panel === 'appearance'}
      <header><h2>{t('settings.appearance')}</h2><p>{t('settings.appearanceDescription')}</p></header>
      <article><h3>{t('settings.language')}</h3><p>{t('settings.languageDescription')}</p><div class="field-label"><span>{t('settings.language')}</span><div class="language-row"><div class="language-select"><button class="language-trigger" aria-haspopup="listbox" aria-expanded={languageMenuOpen} onclick={toggleLanguageMenu} disabled={languageRefreshBusy}><span>{selectedLanguageLabel}</span><ChevronDown size={16} /></button>{#if languageMenuOpen}<div class="language-menu" role="listbox" aria-label={t('settings.language')}><button role="option" aria-selected={preferences.locale === 'system'} onclick={() => selectLanguage('system')}>{t('settings.languageSystem')}{#if preferences.locale === 'system'}<Check size={15} />{/if}</button>{#each installedLocales as pack}<button role="option" aria-selected={preferences.locale === pack.locale} onclick={() => selectLanguage(pack.locale)}><span>{pack.name}<small>{pack.locale}</small></span>{#if preferences.locale === pack.locale}<Check size={15} />{/if}</button>{/each}</div>{/if}</div><button class="icon-button" title={t('settings.openLanguagePackDirectory')} aria-label={t('settings.openLanguagePackDirectory')} onclick={openLanguagePackDirectory}><FolderOpen size={18} /></button></div></div><p class="language-folder-hint">{t('settings.languagePackFolderDescription')}</p>{#if languageError}<p class="settings-error">{languageError}</p>{/if}</article>
      <article><h3>{t('settings.theme')}</h3><p>{t('settings.themeDescription')}</p><label>{t('settings.theme')}<select bind:value={preferences.theme} onchange={previewAppearance}><option value="system">{t('settings.themeSystem')}</option><option value="light">{t('settings.themeLight')}</option><option value="dark">{t('settings.themeDark')}</option></select></label></article>
    {:else if panel === 'typography'}
      <header><h2>{t('settings.typographyTitle')}</h2><p>{t('settings.typographyDescription')}</p></header>
      <article><h3>{t('settings.uiFallback')}</h3><p>{t('settings.uiFallbackDescription')}</p><label>{t('settings.interfaceProfile')}<select bind:value={preferences.uiFont} onchange={previewAppearance}><option value="system">{t('settings.systemFallback')}</option><option value="cjk">{t('settings.cjkFallback')}</option><option value="arib">{t('settings.aribFirst')}</option></select></label><div class="font-preview">日本語字幕 · 简体中文 · 繁體中文 · 한국어 · English<br /><small>{t('settings.fallbackPreview', 'Fallback preview — missing glyphs are never silently replaced by a generic icon.')}</small></div></article>
      <article><h3>{t('settings.captionFont')}</h3><p>{t('settings.captionFontDescription')}</p><label>{t('settings.captionFont')}<select bind:value={preferences.captionFont} onchange={previewCaptionFont}><option value="arib">{t('settings.aribBundled', 'Rounded M+ 1m for ARIB (bundled)')}</option><option value="system">{t('settings.systemFallbackShort', 'System fallback')}</option></select></label><div class="caption-sample"><span>ニュースをお伝えします</span><b>{t('settings.aribPreview', 'ARIB / DRCS-aware preview')}</b></div></article>
    {:else if panel === 'output'}
      <header><h2>{t('settings.output')}</h2><p>{t('settings.outputDescription')}</p></header>
      <article><h3>{t('settings.faithful')}</h3><p>{t('settings.faithfulDescription')}</p><label>{t('settings.defaultFormat')}<select bind:value={preferences.defaultFormat}><option>ASS</option><option>TTML</option><option>JSON</option><option>Raw Data</option></select></label></article>
    {:else}
      <header><h2>{t('settings.player')}</h2><p>{t('settings.playerDescription')}</p></header>
      <article><h3>{t('settings.player')}</h3><p>{t('settings.playerDescription')}</p><p>{t('settings.previewControlsDescription')}</p>
        {#if previewRuntime}
          <dl class="runtime-status">
            <div><dt>{t('settings.runtimeStatus')}</dt><dd class:available={previewRuntime.available}>{previewRuntime.available ? t('settings.runtimeReady') : t('settings.runtimeMissing')}</dd></div>
            <div><dt>{t('settings.renderApi')}</dt><dd class:available={previewRuntime.renderApiAvailable}>{previewRuntime.renderApiAvailable ? t('settings.renderApiAvailable') : t('settings.renderApiUnavailable')}</dd></div>
            <div><dt>{t('settings.runtimeDetail')}</dt><dd>{previewRuntime.detail}</dd></div>
          </dl>
        {/if}
      </article>
      <article><h3>{t('settings.previewControls')}</h3><p>{t('settings.previewControlsDescription')}</p></article>
    {/if}
    <footer><button class="reset" onclick={reset}><RotateCcw size={17} /> {t('settings.reset')}</button><button class="save" onclick={save}>{#if saved}<Check size={17} /> {t('settings.saved')}{:else}<Save size={17} /> {t('settings.save')}{/if}</button></footer>
  </section>
</section>

<style>
  .settings-shell{display:grid;grid-template-columns:220px minmax(0,760px);gap:34px;max-width:1050px;margin:34px auto}.settings-nav{display:grid;align-content:start;gap:6px}.settings-nav button{display:flex;align-items:center;gap:10px;padding:11px 12px;color:#657488;border-radius:7px;background:transparent;text-align:left}.settings-nav button.selected{color:#176be6;background:#eaf3ff}.settings-content header{padding-bottom:21px;border-bottom:1px solid #dfe7ef}.settings-content h2{margin:0;font-size:23px}.settings-content header p,.settings-content article>p{color:#68778a;line-height:1.5}.settings-content article{padding:23px 0;border-bottom:1px solid #e1e8f0}.settings-content h3{margin:0;font-size:16px}.settings-content article>p{margin:9px 0 18px;font-size:13px}.settings-content label,.field-label{display:block;color:#354355;font-size:13px;font-weight:650}.settings-content select{width:100%;margin-top:7px;padding:10px 11px;color:#293749;border:1px solid #d4e0eb;border-radius:7px;background:#fff}.language-row{display:grid;grid-template-columns:minmax(0,1fr) 42px;gap:7px;align-items:end;margin-top:7px}.language-select{position:relative;min-width:0}.language-trigger{display:flex;align-items:center;justify-content:space-between;width:100%;height:42px;padding:0 11px;color:var(--rw-text);border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-surface-raised);text-align:left}.language-trigger span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.language-menu{position:absolute;z-index:20;top:46px;left:0;right:0;overflow-y:auto;max-height:260px;padding:5px;border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-surface-raised);box-shadow:0 10px 28px #08111c2b}.language-menu button{display:flex;align-items:center;justify-content:space-between;width:100%;padding:8px;color:var(--rw-text);border-radius:4px;background:transparent;text-align:left}.language-menu button:hover,.language-menu button[aria-selected="true"]{background:color-mix(in srgb,var(--rw-accent) 10%,var(--rw-surface-raised))}.language-menu small{display:block;margin-top:2px;color:var(--rw-muted);font-size:10px}.icon-button{display:grid;place-items:center;width:42px;height:42px;color:var(--rw-text-secondary);border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-surface-raised)}.language-folder-hint{margin:8px 0 0!important;color:var(--rw-muted)!important;font-size:11px!important}.settings-error{color:#c24848!important}.font-preview,.caption-sample{margin-top:13px;padding:14px;border:1px solid #dbe5ef;border-radius:7px;background:#f8fbfe;line-height:1.7}.font-preview small{color:#718093}.caption-sample{display:flex;justify-content:space-between;align-items:center;color:#fff;background:#101923}.caption-sample span{font-family:"Rounded M+ 1m for ARIB","Noto Sans JP",sans-serif;font-size:22px}.caption-sample b{color:#9eb5cd;font-size:11px}.runtime-status{display:grid;gap:8px;margin:16px 0 0}.runtime-status div{display:grid;grid-template-columns:145px minmax(0,1fr);gap:12px}.runtime-status dt{color:#68778a;font-size:12px}.runtime-status dd{margin:0;color:#a85314;font-size:12px;line-height:1.45;word-break:break-word}.runtime-status dd.available{color:#147a52}footer{display:flex;justify-content:flex-end;gap:10px;padding:22px 0}.reset,.save{display:flex;align-items:center;gap:8px;padding:10px 14px;border-radius:7px}.reset{color:#3d4b5c;border:1px solid #d7e1eb;background:#fff}.save{color:#fff;background:#176ce7}@media (max-width:860px){.settings-shell{grid-template-columns:1fr;gap:16px}.settings-nav{grid-template-columns:repeat(2,1fr)}.runtime-status div{grid-template-columns:1fr}}
  @media (prefers-color-scheme: dark){.settings-nav button{color:#aebdcb}.settings-nav button.selected{color:#61aaff;background:#17304b}.settings-content header,.settings-content article{border-color:#2d3a46}.settings-content h2,.settings-content h3,.settings-content label,.field-label{color:#e0e8f1}.settings-content header p,.settings-content article>p{color:#9aaaba}.settings-content select,.font-preview,.reset{color:#dbe5ee;border-color:#354350;background:#19232c}.font-preview{background:#171f28}.font-preview small{color:#9aaaba}.runtime-status dt{color:#9aaaba}}
  :global([data-theme="dark"]) .settings-nav button{color:#aebdcb}:global([data-theme="dark"]) .settings-nav button.selected{color:#61aaff;background:#17304b}:global([data-theme="dark"]) .settings-content header,:global([data-theme="dark"]) .settings-content article{border-color:#2d3a46}:global([data-theme="dark"]) .settings-content h2,:global([data-theme="dark"]) .settings-content h3,:global([data-theme="dark"]) .settings-content label,:global([data-theme="dark"]) .field-label{color:#e0e8f1}:global([data-theme="dark"]) .settings-content header p,:global([data-theme="dark"]) .settings-content article>p{color:#9aaaba}:global([data-theme="dark"]) .settings-content select,:global([data-theme="dark"]) .font-preview,:global([data-theme="dark"]) .reset{color:#dbe5ee;border-color:#354350;background:#19232c}
</style>
