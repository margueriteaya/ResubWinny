<script lang="ts">
  import { onMount } from 'svelte'
  import { Check, FileText, FolderOpen, MonitorCog, Palette, RotateCcw, Save, Type } from '@lucide/svelte'
  import { backend, type AppSettings, type PreviewRuntime } from '../../backend'
  import { availableLocales, registerLanguagePacks, t } from '../../i18n'
  import { isDesktopRuntime } from '../../shell/desktop'
  import PopupButton from '../../components/PopupButton.svelte'
  import MacSegmentedControl from '../../components/MacSegmentedControl.svelte'

  type Panel = 'appearance' | 'typography' | 'output' | 'player'
  export let saveCaptionFont: (font: string) => void = () => {}
  export let onSettingsSaved: (settings: AppSettings) => void | Promise<void> = () => {}
  export let onSettingsPreview: (settings: AppSettings) => void | Promise<void> = () => {}
  const defaults: AppSettings = { uiFont: 'system', captionFont: 'arib', defaultFormat: 'ASS', defaultTimeline: 'Auto (Gap Merge + Overlap Resolve)', locale: 'system', theme: 'system', workspaceLayout: { sourceWidth: 240, outputWidth: 300, sourceCollapsed: false, outputCollapsed: false } }
  let preferences: AppSettings = { ...defaults }
  export let panel: Panel = 'typography'
  let saved = false
  let previewRuntime: PreviewRuntime | null = null
  let installedLocales = availableLocales()
  let languageRefreshBusy = false
  let languageError = ''
  $: languageOptions = [
    { value: 'system', label: t('settings.languageSystem') },
    ...installedLocales.map((pack) => ({ value: pack.locale, label: `${pack.name} (${pack.locale})` })),
  ]

  function applyFont() {
    const font = preferences.uiFont === 'system'
      ? 'var(--rw-font-ui)'
      : preferences.uiFont === 'cjk'
        ? '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "PingFang SC", "PingFang TC", "Hiragino Sans", "Yu Gothic UI", "Microsoft YaHei UI", "Microsoft JhengHei UI", sans-serif'
        : '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Rounded M+ 1m for ARIB", "Hiragino Sans", "Yu Gothic UI", sans-serif'
    document.documentElement.style.setProperty('--resubwinny-ui-font', font)
  }

  async function save() {
    if (isDesktopRuntime()) preferences = await backend.updateSettings(preferences)
    saveCaptionFont(preferences.captionFont)
    await onSettingsSaved(preferences)
    applyFont(); saved = true
    window.setTimeout(() => saved = false, 2500)
  }

  async function reset() {
    preferences = isDesktopRuntime() ? await backend.updateSettings({ ...defaults }) : { ...defaults, workspaceLayout: { ...defaults.workspaceLayout } }
    saveCaptionFont('arib'); await onSettingsSaved(preferences); applyFont()
  }

  async function previewAppearance() {
    if (isDesktopRuntime()) preferences = await backend.updateSettings(preferences)
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

  async function selectLanguage(next: string) {
    preferences = { ...preferences, locale: next }
    await previewAppearance()
  }

  async function previewCaptionFont() {
    if (isDesktopRuntime()) preferences = await backend.updateSettings(preferences)
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
      <article><h3>{t('settings.language')}</h3><p>{t('settings.languageDescription')}</p><div class="field-label"><span>{t('settings.language')}</span><div class="language-row"><PopupButton label={t('settings.language')} value={preferences.locale} options={languageOptions} disabled={languageRefreshBusy} onOpen={refreshLanguagePacks} onChange={(value) => void selectLanguage(value)} /><button class="icon-button liquid-control" data-tooltip={t('settings.openLanguagePackDirectory')} aria-label={t('settings.openLanguagePackDirectory')} onclick={openLanguagePackDirectory}><FolderOpen size={18} /></button></div></div><p class="language-folder-hint">{t('settings.languagePackFolderDescription')}</p>{#if languageError}<p class="settings-error">{languageError}</p>{/if}</article>
      <article><h3>{t('settings.theme')}</h3><p>{t('settings.themeDescription')}</p><div class="field-label"><span>{t('settings.theme')}</span><div class="theme-control"><MacSegmentedControl ariaLabel={t('settings.theme')} value={preferences.theme} options={[{value:'system',label:t('settings.themeSystem')},{value:'light',label:t('settings.themeLight')},{value:'dark',label:t('settings.themeDark')}]} onChange={(value) => { preferences = {...preferences, theme: value as AppSettings['theme']}; void previewAppearance() }} /></div></div></article>
    {:else if panel === 'typography'}
      <header><h2>{t('settings.typographyTitle')}</h2><p>{t('settings.typographyDescription')}</p></header>
      <article><h3>{t('settings.uiFallback')}</h3><p>{t('settings.uiFallbackDescription')}</p><label>{t('settings.interfaceProfile')}<PopupButton label={t('settings.interfaceProfile')} value={preferences.uiFont} options={[{value:'system',label:t('settings.systemFallback')},{value:'cjk',label:t('settings.cjkFallback')},{value:'arib',label:t('settings.aribFirst')}]} onChange={(value) => { preferences = {...preferences, uiFont: value as AppSettings['uiFont']}; void previewAppearance() }} /></label><div class="font-preview">日本語字幕 · 简体中文 · 繁體中文 · 한국어 · English<br /><small>{t('settings.fallbackPreview', 'Fallback preview — missing glyphs are never silently replaced by a generic icon.')}</small></div></article>
      <article><h3>{t('settings.captionFont')}</h3><p>{t('settings.captionFontDescription')}</p><label>{t('settings.captionFont')}<PopupButton label={t('settings.captionFont')} value={preferences.captionFont} options={[{value:'arib',label:t('settings.aribBundled', 'Rounded M+ 1m for ARIB (bundled)')},{value:'system',label:t('settings.systemFallbackShort', 'System fallback')}]} onChange={(value) => { preferences = {...preferences, captionFont: value as AppSettings['captionFont']}; void previewCaptionFont() }} /></label><div class="caption-sample"><span>ニュースをお伝えします</span><b>{t('settings.aribPreview', 'ARIB / DRCS-aware preview')}</b></div></article>
    {:else if panel === 'output'}
      <header><h2>{t('settings.output')}</h2><p>{t('settings.outputDescription')}</p></header>
      <article><h3>{t('settings.faithful')}</h3><p>{t('settings.faithfulDescription')}</p><label>{t('settings.defaultFormat')}<PopupButton label={t('settings.defaultFormat')} value={preferences.defaultFormat} options={['ASS','TTML','JSON','Raw Data'].map((value) => ({value,label:value}))} onChange={(value) => preferences = {...preferences, defaultFormat: value as AppSettings['defaultFormat']}} /></label></article>
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
  .settings-shell{display:grid;grid-template-columns:184px minmax(0,720px);gap:28px;width:min(100%,932px);margin:20px auto 12px;color:var(--rw-text)}
  .settings-nav{position:sticky;top:0;display:grid;align-content:start;gap:2px;height:max-content;padding:6px}
  .settings-nav button{display:flex;align-items:center;gap:9px;height:34px;padding:0 9px;border:0;border-radius:7px;color:var(--rw-text-secondary);background:transparent;font-size:12px;text-align:left}
  .settings-nav button.selected{color:var(--rw-text);background:color-mix(in srgb,var(--rw-text) 10%,transparent);box-shadow:inset 0 .5px rgba(255,255,255,.48)}
  .settings-nav button :global(svg){width:16px;height:16px;color:var(--rw-accent);stroke-width:1.8}
  .settings-content{min-width:0}
  .settings-content header{padding:2px 0 16px;border-bottom:1px solid var(--rw-border-subtle)}
  .settings-content h2{margin:0;font-size:20px;line-height:25px;font-weight:680}
  .settings-content header p,.settings-content article>p{color:var(--rw-text-secondary);font-size:12px;line-height:17px}
  .settings-content header p{margin:3px 0 0}.settings-content article{padding:17px 0 18px;border-bottom:1px solid var(--rw-border-subtle)}
  .settings-content h3{margin:0;font-size:13px;line-height:17px;font-weight:650}
  .settings-content article>p{margin:4px 0 13px}.settings-content label,.field-label{display:block;max-width:470px;color:var(--rw-text);font-size:11px;line-height:15px;font-weight:600}
  .language-row{display:grid;grid-template-columns:minmax(0,1fr) 36px;gap:6px;align-items:center;margin-top:6px}.language-row :global(.popup-button){margin-top:0}
  .icon-button{display:grid;place-items:center;width:36px;height:36px;min-height:36px;padding:0;border:.5px solid var(--rw-glass-border);border-radius:18px;color:var(--rw-text-secondary);background:transparent}
  .language-folder-hint{max-width:470px;margin:7px 0 0!important;color:var(--rw-muted)!important;font-size:10px!important;line-height:14px!important}.settings-error{color:#c24848!important}
  .font-preview,.caption-sample{max-width:470px;margin-top:10px;padding:11px 12px;border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-surface-muted);font-size:12px;line-height:18px}
  .font-preview small{color:var(--rw-muted);font-size:10px}.caption-sample{display:flex;justify-content:space-between;align-items:center;gap:16px;color:#fff;background:#17191d}
  .caption-sample span{font-family:"Rounded M+ 1m for ARIB","Hiragino Sans","Yu Gothic UI",sans-serif;font-size:18px}.caption-sample b{color:#a9b2bd;font-size:9px;text-align:right}
  .runtime-status{display:grid;gap:7px;margin:12px 0 0}.runtime-status div{display:grid;grid-template-columns:128px minmax(0,1fr);gap:10px}.runtime-status dt{color:var(--rw-muted);font-size:10px}.runtime-status dd{margin:0;color:var(--rw-warning);font-size:10px;line-height:14px;word-break:break-word}.runtime-status dd.available{color:var(--rw-success)}
  footer{display:flex;justify-content:flex-end;gap:8px;padding:17px 0}.reset,.save{display:flex;align-items:center;justify-content:center;gap:6px;height:32px;padding:0 12px;border-radius:7px;font-size:11px}.reset{color:var(--rw-text);border:.5px solid var(--rw-glass-border);background:transparent;box-shadow:var(--rw-control-shadow)}.save{color:#fff;background:var(--rw-accent)}
  .theme-control{margin-top:6px}.theme-control :global(.mac-segmented){min-width:270px}
  @media(max-width:860px){.settings-shell{grid-template-columns:1fr;gap:14px;margin-top:0}.settings-nav{position:static;grid-template-columns:repeat(4,minmax(0,1fr))}.settings-nav button{justify-content:center}.settings-nav button :global(svg){display:none}.runtime-status div{grid-template-columns:1fr}}
  @media(max-width:620px){.settings-nav{grid-template-columns:repeat(2,minmax(0,1fr))}.caption-sample{align-items:flex-start;flex-direction:column}.theme-control :global(.mac-segmented){width:100%;min-width:0}}
</style>
