<script lang="ts">
  import { onMount } from 'svelte'
  import { Code2, ExternalLink, ShieldAlert } from '@lucide/svelte'
  import { backend, type AboutInfo } from '../../backend'
  import { t } from '../../i18n'
  import { isDesktopRuntime } from '../../shell/desktop'

  const links = [
    { label: 'settings.sourceCode', target: 'source' },
    { label: 'settings.releases', target: 'releases' },
    { label: 'settings.reportIssue', target: 'issues' },
  ]
  let info: AboutInfo | null = null
  let loadError = ''

  onMount(() => {
    if (!isDesktopRuntime()) return
    backend.getAboutInfo().then((value) => info = value).catch((reason) => loadError = String(reason))
  })

  const shortCommit = (commit: string | null) => commit ? commit.slice(0, 12) : t('settings.notAvailable')
</script>

<header><h2>{t('settings.about')}</h2><p>{t('settings.aboutDescription')}</p></header>
<section class="about-identity">
  <div class="product-mark" aria-hidden="true">RW</div>
  <div><h3>ResubWinny</h3><p>{info?.description ?? t('app.tagline')}</p><span>{info ? `${info.version} · ${info.channel}` : t('workspace.loading')}</span></div>
</section>
{#if loadError}<p class="about-error" role="alert">{loadError}</p>{/if}
<section class="about-grid" aria-label={t('settings.buildInformation')}>
  <div><span>{t('settings.version')}</span><b>{info?.version ?? '—'}</b></div>
  <div><span>{t('settings.platform')}</span><b>{info ? `${info.platform} · ${info.architecture}` : '—'}</b></div>
  <div><span>{t('settings.releaseTier')}</span><b>{info?.releaseTier ?? 'Development'}</b></div>
  <div><span>{t('settings.buildTag')}</span><b>{info?.buildTag ?? t('settings.notAvailable')}</b></div>
  <div><span>{t('settings.buildCommit')}</span><b title={info?.buildCommit ?? undefined}>{shortCommit(info?.buildCommit ?? null)}</b></div>
  <div><span>{t('settings.signing')}</span><b class:warning={info?.signingDeclaration === 'unsigned-alpha'}>{info?.signingDeclaration === 'unsigned-alpha' ? t('settings.unsignedAlpha') : info?.signingDeclaration === 'declared-signed' ? t('settings.declaredSigned') : t('settings.developmentBuild')}</b></div>
</section>
<aside class="signing-note"><ShieldAlert size={17}/><p>{t('settings.signingDeclarationNote')}</p></aside>
<div class="about-actions">
  {#each links as link, index}
    <button class="liquid-control" onclick={() => void backend.openProjectLink(link.target as 'source' | 'releases' | 'issues')}>{#if index === 0}<Code2 size={16}/>{:else}<ExternalLink size={16}/>{/if}{t(link.label)}</button>
  {/each}
</div>

<style>
  header{padding:2px 2px 14px}h2{margin:0;font-size:20px;line-height:25px;font-weight:680}header p{margin:3px 0 0;color:var(--rw-text-secondary);font-size:11px;line-height:16px}
  .about-identity{display:flex;align-items:center;gap:14px;padding:18px;border:1px solid var(--rw-border-subtle);border-radius:9px;background:var(--rw-surface-muted)}.product-mark{display:grid;place-items:center;width:52px;height:52px;flex:0 0 52px;border-radius:12px;color:#fff;background:var(--rw-accent);font-size:16px;font-weight:750}.about-identity h3{margin:0;font-size:17px}.about-identity p{margin:3px 0;color:var(--rw-text-secondary);font-size:11px;line-height:15px}.about-identity span{color:var(--rw-muted);font-size:10px}
  .about-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));margin-top:12px;overflow:hidden;border:1px solid var(--rw-border-subtle);border-radius:9px;background:var(--rw-border-subtle);gap:1px}.about-grid div{min-width:0;padding:12px;background:var(--rw-content)}.about-grid span,.about-grid b{display:block}.about-grid span{color:var(--rw-muted);font-size:10px}.about-grid b{margin-top:3px;overflow:hidden;font-size:11px;text-overflow:ellipsis;white-space:nowrap}.about-grid b.warning{color:var(--rw-warning)}
  .signing-note{display:flex;align-items:flex-start;gap:8px;margin-top:12px;padding:11px 12px;border:1px solid var(--rw-border-subtle);border-radius:8px;background:var(--rw-surface-muted)}.signing-note :global(svg){flex:0 0 17px;color:var(--rw-warning)}.signing-note p{margin:0;color:var(--rw-text-secondary);font-size:10px;line-height:15px}.about-actions{display:flex;flex-wrap:wrap;gap:8px;margin-top:14px}.about-actions button{display:flex;align-items:center;gap:6px;height:32px;padding:0 11px;border:.5px solid var(--rw-glass-border);border-radius:8px;color:var(--rw-text);background:transparent;box-shadow:var(--rw-control-shadow);font-size:11px}.about-error{color:#c24848;font-size:11px}
  @container content (max-width:560px){.about-grid{grid-template-columns:1fr}.about-actions{display:grid}.about-actions button{justify-content:center}}

  .about-identity span,.about-grid span{font-size:11px}.signing-note p{font-size:12px;line-height:17px}.about-actions button,.about-error{font-size:12px}
</style>
