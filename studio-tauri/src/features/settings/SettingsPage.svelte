<script lang="ts">
  import { onMount } from 'svelte'
  import { FileText, FolderOpen, Info, MonitorCog, Palette, RotateCcw, Type } from '@lucide/svelte'
  import { backend, type AppSettings, type PreviewRuntime } from '../../backend'
  import { availableLocales, registerLanguagePacks, t } from '../../i18n'
  import { isDesktopRuntime } from '../../shell/desktop'
  import PopupButton from '../../components/PopupButton.svelte'
  import MacSegmentedControl from '../../components/MacSegmentedControl.svelte'
  import AboutPanel from './AboutPanel.svelte'

  type Panel = 'general' | 'typography' | 'output' | 'playback' | 'about'
  export let saveCaptionFont: (font: string) => void = () => {}
  export let onSettingsSaved: (settings: AppSettings) => void | Promise<void> = () => {}
  export let onSettingsPreview: (settings: AppSettings) => void | Promise<void> = () => {}
  export let onError: (reason: unknown) => void = () => {}
  const defaults: AppSettings = { uiFont: 'system', captionFont: 'arib', defaultFormat: 'ASS', locale: 'system', theme: 'system', workspaceLayout: { sourceWidth: 240, outputWidth: 300, sourceCollapsed: false, outputCollapsed: false } }
  let preferences: AppSettings = { ...defaults }
  export let panel: Panel = 'general'
  let persistenceState: 'idle' | 'saving' | 'saved' | 'error' = 'idle'
  let pendingPreferences: AppSettings | null = null
  let persistenceRunning = false
  let savedTimer = 0
  let previewRuntime: PreviewRuntime | null = null
  let installedLocales = availableLocales()
  let languageRefreshBusy = false
  let languageError = ''
  $: languageOptions = [
    { value: 'system', label: t('settings.languageSystem') },
    ...installedLocales.map((pack) => ({ value: pack.locale, label: `${pack.name} (${pack.locale})` })),
  ]
  $: categoryOptions = [
    { value: 'general', label: t('settings.general') },
    { value: 'typography', label: t('settings.typography') },
    { value: 'output', label: t('settings.output') },
    { value: 'playback', label: t('settings.playbackAndRuntime') },
    { value: 'about', label: t('settings.about') },
  ]

  function applyFont() {
    const font = preferences.uiFont === 'system'
      ? 'var(--rw-font-ui)'
      : preferences.uiFont === 'cjk'
        ? '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "PingFang SC", "PingFang TC", "Hiragino Sans", "Yu Gothic UI", "Microsoft YaHei UI", "Microsoft JhengHei UI", sans-serif'
        : '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Rounded M+ 1m for ARIB", "Hiragino Sans", "Yu Gothic UI", sans-serif'
    document.documentElement.style.setProperty('--resubwinny-ui-font', font)
  }

  function announceSaved() {
    persistenceState = 'saved'
    window.clearTimeout(savedTimer)
    savedTimer = window.setTimeout(() => persistenceState = 'idle', 2500)
  }

  async function drainPersistence() {
    if (persistenceRunning) return
    persistenceRunning = true
    while (pendingPreferences) {
      const candidate = pendingPreferences
      pendingPreferences = null
      persistenceState = 'saving'
      try {
        const persisted = isDesktopRuntime() ? await backend.updateSettings(candidate) : candidate
        if (!pendingPreferences) {
          preferences = persisted
          await onSettingsSaved({ ...persisted })
          announceSaved()
        }
      } catch (reason) {
        persistenceState = 'error'
        onError(reason)
      }
    }
    persistenceRunning = false
  }

  function updatePreferences(next: AppSettings, effect: 'appearance' | 'caption' | 'none' = 'none') {
    preferences = next
    if (effect === 'appearance') applyFont()
    if (effect === 'caption') saveCaptionFont(next.captionFont)
    void Promise.resolve(onSettingsPreview({ ...next })).catch(onError)
    pendingPreferences = { ...next, workspaceLayout: { ...next.workspaceLayout } }
    void drainPersistence()
  }

  function resetCategory() {
    if (panel === 'general') {
      updatePreferences({ ...preferences, locale: defaults.locale, theme: defaults.theme }, 'appearance')
    } else if (panel === 'typography') {
      updatePreferences({ ...preferences, uiFont: defaults.uiFont, captionFont: defaults.captionFont }, 'appearance')
      saveCaptionFont(defaults.captionFont)
    } else if (panel === 'output') {
      updatePreferences({ ...preferences, defaultFormat: defaults.defaultFormat })
    }
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
        updatePreferences(preferences, 'appearance')
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
    updatePreferences({ ...preferences, locale: next }, 'appearance')
  }

  onMount(() => {
    backend.getSettings().then((settings) => { preferences = settings; applyFont() }).catch(() => applyFont())
    void refreshLanguagePacks()
    backend.getPreviewRuntime().then((runtime) => previewRuntime = runtime).catch(() => previewRuntime = null)
  })
</script>

<section class="settings-shell">
  <div class="compact-category">
    <PopupButton label={t('nav.settings')} value={panel} options={categoryOptions} onChange={(value) => panel = value as Panel} />
  </div>
  <nav class="settings-nav" aria-label={t('nav.settings')}>
    <button type="button" aria-current={panel === 'general' ? 'page' : undefined} class:selected={panel === 'general'} onclick={() => panel = 'general'}><Palette size={18} /> {t('settings.general')}</button>
    <button type="button" aria-current={panel === 'typography' ? 'page' : undefined} class:selected={panel === 'typography'} onclick={() => panel = 'typography'}><Type size={18} /> {t('settings.typography')}</button>
    <button type="button" aria-current={panel === 'output' ? 'page' : undefined} class:selected={panel === 'output'} onclick={() => panel = 'output'}><FileText size={18} /> {t('settings.output')}</button>
    <button type="button" aria-current={panel === 'playback' ? 'page' : undefined} class:selected={panel === 'playback'} onclick={() => panel = 'playback'}><MonitorCog size={18} /> {t('settings.playbackAndRuntime')}</button>
    <span class="settings-nav-spacer" aria-hidden="true"></span>
    <button type="button" aria-current={panel === 'about' ? 'page' : undefined} class:selected={panel === 'about'} onclick={() => panel = 'about'}><Info size={18} /> {t('settings.about')}</button>
  </nav>
  <section class="settings-content">
    {#key panel}
    <div class="settings-panel">
    {#if panel === 'general'}
      <header><h2>{t('settings.general')}</h2><p>{t('settings.appearanceDescription')}</p></header>
      <section class="settings-group">
        <div class="setting-copy"><h3>{t('settings.language')}</h3><p>{t('settings.languageDescription')}</p></div>
        <div class="setting-control"><div class="language-row"><PopupButton label={t('settings.language')} value={preferences.locale} options={languageOptions} disabled={languageRefreshBusy} onOpen={refreshLanguagePacks} onChange={(value) => void selectLanguage(value)} /><button class="icon-button liquid-control" data-tooltip={t('settings.openLanguagePackDirectory')} aria-label={t('settings.openLanguagePackDirectory')} onclick={openLanguagePackDirectory}><FolderOpen size={18} /></button></div><p class="control-hint">{t('settings.languagePackFolderDescription')}</p>{#if languageError}<p class="settings-error" role="alert">{languageError}</p>{/if}</div>
      </section>
      <section class="settings-group">
        <div class="setting-copy"><h3>{t('settings.theme')}</h3><p>{t('settings.themeDescription')}</p></div>
        <div class="setting-control theme-control"><MacSegmentedControl ariaLabel={t('settings.theme')} value={preferences.theme} options={[{value:'system',label:t('settings.themeSystem')},{value:'light',label:t('settings.themeLight')},{value:'dark',label:t('settings.themeDark')}]} onChange={(value) => updatePreferences({...preferences, theme: value as AppSettings['theme']}, 'appearance')} /></div>
      </section>
    {:else if panel === 'typography'}
      <header><h2>{t('settings.typographyTitle')}</h2><p>{t('settings.typographyDescription')}</p></header>
      <section class="settings-group"><div class="setting-copy"><h3>{t('settings.uiFallback')}</h3><p>{t('settings.uiFallbackDescription')}</p></div><div class="setting-control"><PopupButton label={t('settings.interfaceProfile')} value={preferences.uiFont} options={[{value:'system',label:t('settings.systemFallback')},{value:'cjk',label:t('settings.cjkFallback')},{value:'arib',label:t('settings.aribFirst')}]} onChange={(value) => updatePreferences({...preferences, uiFont: value as AppSettings['uiFont']}, 'appearance')} /><div class="font-preview">日本語字幕 · 简体中文 · 繁體中文 · 한국어 · English<br /><small>{t('settings.fallbackPreview', 'Fallback preview — missing glyphs are never silently replaced by a generic icon.')}</small></div></div></section>
      <section class="settings-group"><div class="setting-copy"><h3>{t('settings.captionFont')}</h3><p>{t('settings.captionFontDescription')}</p></div><div class="setting-control"><PopupButton label={t('settings.captionFont')} value={preferences.captionFont} options={[{value:'arib',label:t('settings.aribBundled', 'Rounded M+ 1m for ARIB (bundled)')},{value:'system',label:t('settings.systemFallbackShort', 'System fallback')}]} onChange={(value) => updatePreferences({...preferences, captionFont: value as AppSettings['captionFont']}, 'caption')} /><div class="caption-sample"><span>ニュースをお伝えします</span><b>{t('settings.aribPreview', 'ARIB / DRCS-aware preview')}</b></div></div></section>
    {:else if panel === 'output'}
      <header><h2>{t('settings.output')}</h2><p>{t('settings.outputDescription')}</p></header>
      <section class="settings-group"><div class="setting-copy"><h3>{t('settings.defaultFormat')}</h3><p>{t('settings.faithfulDescription')}</p></div><div class="setting-control"><PopupButton label={t('settings.defaultFormat')} value={preferences.defaultFormat} options={['ASS','TTML','JSON','Raw Data'].map((value) => ({value,label:value}))} onChange={(value) => updatePreferences({...preferences, defaultFormat: value as AppSettings['defaultFormat']})} /></div></section>
    {:else if panel === 'playback'}
      <header><h2>{t('settings.playbackAndRuntime')}</h2><p>{t('settings.playerDescription')}</p></header>
      <section class="settings-group runtime-group"><div class="setting-copy"><h3>{t('settings.runtimeStatus')}</h3><p>{t('settings.previewControlsDescription')}</p></div><div class="setting-control">
        {#if previewRuntime}
          <dl class="runtime-status">
            <div><dt>{t('settings.runtimeStatus')}</dt><dd class:available={previewRuntime.available}>{previewRuntime.available ? t('settings.runtimeReady') : t('settings.runtimeMissing')}</dd></div>
            <div><dt>{t('settings.renderApi')}</dt><dd class:available={previewRuntime.renderApiAvailable}>{previewRuntime.renderApiAvailable ? t('settings.renderApiAvailable') : t('settings.renderApiUnavailable')}</dd></div>
            <div><dt>{t('settings.runtimeDetail')}</dt><dd>{previewRuntime.detail}</dd></div>
          </dl>
        {/if}
      </div></section>
    {:else}<AboutPanel />{/if}
    </div>
    {/key}
    <footer>
      <span class:error={persistenceState === 'error'} aria-live="polite">{persistenceState === 'saving' ? t('settings.saving') : persistenceState === 'saved' ? t('settings.saved') : persistenceState === 'error' ? t('settings.saveFailed') : ''}</span>
      {#if panel !== 'playback' && panel !== 'about'}<button class="reset liquid-control" onclick={resetCategory}><RotateCcw size={17} /> {t('settings.resetCategory')}</button>{/if}
    </footer>
  </section>
</section>

<style>
  .settings-shell{display:grid;grid-template-columns:190px minmax(0,760px);gap:24px;width:min(100%,974px);margin:18px auto 12px;color:var(--rw-text)}
  .settings-nav{position:sticky;top:0;display:grid;align-content:start;gap:2px;height:max-content;padding:6px;border:.5px solid var(--rw-glass-border);border-radius:10px;background:var(--rw-glass);box-shadow:var(--rw-control-shadow);backdrop-filter:blur(18px) saturate(1.18);-webkit-backdrop-filter:blur(18px) saturate(1.18)}
  .settings-nav button{display:flex;align-items:center;gap:9px;min-height:36px;padding:0 10px;border:0;border-radius:7px;color:var(--rw-text-secondary);background:transparent;font-size:12px;text-align:left;transition:color var(--rw-motion-responsive) var(--rw-ease-out),background-color var(--rw-motion-responsive) var(--rw-ease-out),box-shadow var(--rw-motion-responsive) var(--rw-ease-out)}
  .settings-nav button.selected{color:var(--rw-text);background:color-mix(in srgb,var(--rw-text) 10%,transparent);box-shadow:inset 0 .5px rgba(255,255,255,.48)}
  .settings-nav button :global(svg){width:16px;height:16px;flex:0 0 16px;color:var(--rw-accent);stroke-width:1.8}.compact-category{display:none}
  .settings-nav-spacer{height:8px;margin:2px 4px 0;border-top:1px solid var(--rw-border-subtle)}
  .settings-content{min-width:0;background:var(--rw-content)}
  .settings-panel{animation:settings-panel-reveal var(--rw-motion-fluid) var(--rw-ease-fluid) both}
  .settings-content header{padding:2px 2px 14px}
  .settings-content h2{margin:0;font-size:20px;line-height:25px;font-weight:680}
  .settings-content header p,.setting-copy p{color:var(--rw-text-secondary);font-size:11px;line-height:16px}.settings-content header p{margin:3px 0 0}
  .settings-group{display:grid;grid-template-columns:minmax(180px,1fr) minmax(250px,1.15fr);gap:24px;padding:18px;border:1px solid var(--rw-border-subtle);border-bottom:0;background:var(--rw-surface-muted)}.settings-group:first-of-type{border-radius:9px 9px 0 0}.settings-group:last-of-type{border-bottom:1px solid var(--rw-border-subtle);border-radius:0 0 9px 9px}.settings-group:only-of-type{border-bottom:1px solid var(--rw-border-subtle);border-radius:9px}
  .setting-copy h3{margin:0;font-size:13px;line-height:17px;font-weight:650}.setting-copy p{margin:4px 0 0}.setting-control{min-width:0;align-self:start}.setting-control :global(.popup-button){width:100%}
  .language-row{display:grid;grid-template-columns:minmax(0,1fr) 36px;gap:6px;align-items:center}.language-row :global(.popup-button){margin-top:0}
  .icon-button{display:grid;place-items:center;width:36px;height:36px;min-height:36px;padding:0;border:.5px solid var(--rw-glass-border);border-radius:18px;color:var(--rw-text-secondary);background:transparent}
  .control-hint{margin:7px 0 0;color:var(--rw-muted);font-size:10px;line-height:14px}.settings-error{margin:7px 0 0;color:#c24848;font-size:11px;line-height:15px}
  .font-preview,.caption-sample{margin-top:10px;padding:11px 12px;border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-content);font-size:12px;line-height:18px}
  .font-preview small{color:var(--rw-muted);font-size:10px}.caption-sample{display:flex;justify-content:space-between;align-items:center;gap:16px;color:#fff;background:#17191d}
  .caption-sample span{font-family:"Rounded M+ 1m for ARIB","Hiragino Sans","Yu Gothic UI",sans-serif;font-size:18px}.caption-sample b{color:#a9b2bd;font-size:10px;text-align:right}
  .runtime-status{display:grid;gap:7px;margin:0}.runtime-status div{display:grid;grid-template-columns:112px minmax(0,1fr);gap:10px}.runtime-status dt{color:var(--rw-muted);font-size:10px}.runtime-status dd{margin:0;color:var(--rw-warning);font-size:10px;line-height:14px;word-break:break-word}.runtime-status dd.available{color:var(--rw-success)}
  footer{display:flex;align-items:center;justify-content:flex-end;gap:10px;min-height:49px;padding:9px 0}footer>span{margin-right:auto;color:var(--rw-muted);font-size:10px;line-height:14px}footer>span.error{color:#c24848}.reset{display:flex;align-items:center;justify-content:center;gap:6px;height:32px;padding:0 12px;border:.5px solid var(--rw-glass-border);border-radius:8px;color:var(--rw-text);background:transparent;box-shadow:var(--rw-control-shadow);font-size:11px}
  .theme-control :global(.mac-segmented){width:100%}
  @keyframes settings-panel-reveal{from{opacity:0;transform:translate3d(0,5px,0)}to{opacity:1;transform:none}}
  @media(prefers-reduced-motion:reduce){.settings-panel{animation:none}}
  @container content (max-width:820px){.settings-shell{grid-template-columns:1fr;gap:14px;margin-top:0}.settings-nav{position:static;display:flex;overflow-x:auto}.settings-nav button{flex:0 0 auto}.settings-nav-spacer{width:1px;height:26px;margin:5px 2px;border:0;border-left:1px solid var(--rw-border-subtle)}.settings-content{max-width:none}.settings-group{grid-template-columns:minmax(160px,.85fr) minmax(240px,1.15fr)}.runtime-status div{grid-template-columns:1fr}}
  @container content (max-width:560px){.settings-nav{display:none}.compact-category{display:block}.settings-shell{gap:12px}.settings-group{grid-template-columns:1fr;gap:12px;padding:15px}.caption-sample{align-items:flex-start;flex-direction:column}.theme-control :global(.mac-segmented){width:100%;min-width:0}}
</style>
