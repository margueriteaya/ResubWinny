<script lang="ts">
  import { onMount } from 'svelte'
  import { FileText, Search } from '@lucide/svelte'
  import { backend, type LegalDocumentContent, type LegalDocumentId, type LegalDocumentSummary } from '../../backend'
  import { t } from '../../i18n'
  import { isDesktopRuntime } from '../../shell/desktop'

  let documents: LegalDocumentSummary[] = []
  let selected: LegalDocumentContent | null = null
  let selectedId: LegalDocumentId | null = null
  let query = ''
  let loading = false
  let error = ''
  let request = 0
  $: normalizedQuery = query.trim().toLocaleLowerCase()
  $: visibleDocuments = documents.filter((document) => `${document.title} ${document.category} ${document.license}`.toLocaleLowerCase().includes(normalizedQuery))

  async function selectDocument(id: LegalDocumentId) {
    const currentRequest = ++request
    selectedId = id
    loading = true
    error = ''
    try {
      const document = await backend.getLegalDocument(id)
      if (request === currentRequest) selected = document
    } catch (reason) {
      if (request === currentRequest) error = String(reason)
    } finally {
      if (request === currentRequest) loading = false
    }
  }

  onMount(() => {
    if (!isDesktopRuntime()) return
    backend.listLegalDocuments().then((items) => {
      documents = items
      if (items[0]) void selectDocument(items[0].id)
    }).catch((reason) => error = String(reason))
  })
</script>

<header><h2>{t('settings.licenses')}</h2><p>{t('settings.licensesDescription')}</p></header>
<div class="license-layout">
  <section class="license-list" aria-label={t('settings.licenseDocuments')}>
    <label class="license-search"><Search size={15}/><input bind:value={query} type="search" placeholder={t('settings.licenseSearch')} aria-label={t('settings.licenseSearch')} /></label>
    {#if documents.length === 0 && !error}<p class="license-placeholder">{t('workspace.loading')}</p>
    {:else if visibleDocuments.length === 0}<p class="license-placeholder">{t('settings.legalNoMatches')}</p>
    {:else}<ol>{#each visibleDocuments as document (document.id)}<li><button class:selected={selectedId === document.id} aria-current={selectedId === document.id ? 'page' : undefined} onclick={() => void selectDocument(document.id)}><FileText size={15}/><span><b>{document.title}</b><small>{document.category} · {document.license}</small></span></button></li>{/each}</ol>{/if}
  </section>
  <section class="license-document" aria-live="polite">
    {#if error}<p class="license-error" role="alert">{t('settings.legalLoadFailed')} {error}</p>
    {:else if loading}<p class="license-placeholder">{t('workspace.loading')}</p>
    {:else if selected}<header><h3>{selected.title}</h3><span>{t('settings.licenseText')}</span></header><pre>{selected.content}</pre>
    {:else}<p class="license-placeholder">{t('settings.legalUnavailable')}</p>{/if}
  </section>
</div>

<style>
  header{padding:2px 2px 14px}h2{margin:0;font-size:20px;line-height:25px;font-weight:680}header p{margin:3px 0 0;color:var(--rw-text-secondary);font-size:11px;line-height:16px}
  .license-layout{display:grid;grid-template-columns:minmax(210px,.72fr) minmax(0,1.28fr);min-height:430px;border:1px solid var(--rw-border-subtle);border-radius:9px;background:var(--rw-border-subtle);gap:1px}.license-list,.license-document{min-width:0;background:var(--rw-content)}.license-list{padding:10px}.license-search{display:flex;align-items:center;gap:7px;height:34px;padding:0 9px;border:1px solid var(--rw-border);border-radius:7px;color:var(--rw-muted);background:var(--rw-surface-muted)}.license-search input{width:100%;min-width:0;border:0;outline:0;color:var(--rw-text);background:transparent;font-size:11px}.license-list ol{display:grid;gap:2px;margin:8px 0 0;padding:0;list-style:none}.license-list button{display:grid;grid-template-columns:16px minmax(0,1fr);align-items:center;gap:8px;width:100%;min-height:46px;padding:7px;border:0;border-radius:7px;color:var(--rw-text);background:transparent;text-align:left}.license-list button:hover,.license-list button.selected,.license-list button:focus-visible{background:color-mix(in srgb,var(--rw-accent) 8%,transparent)}.license-list button:focus-visible{outline:2px solid color-mix(in srgb,var(--rw-accent) 52%,transparent);outline-offset:-2px}.license-list button :global(svg){color:var(--rw-accent)}.license-list span{min-width:0}.license-list b,.license-list small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.license-list b{font-size:11px;line-height:15px}.license-list small{margin-top:1px;color:var(--rw-muted);font-size:9px;line-height:12px}
  .license-document{display:grid;grid-template-rows:auto minmax(0,1fr);min-height:0}.license-document header{display:flex;align-items:baseline;gap:8px;padding:13px 14px 10px;border-bottom:1px solid var(--rw-border-subtle)}.license-document h3{margin:0;font-size:13px}.license-document header span{color:var(--rw-muted);font-size:10px}.license-document pre{min-width:0;max-height:530px;margin:0;padding:14px;overflow:auto;color:var(--rw-text-secondary);background:var(--rw-content);font:10px/15px var(--rw-font-mono);white-space:pre-wrap;overflow-wrap:anywhere}.license-placeholder,.license-error{margin:0;padding:16px;color:var(--rw-muted);font-size:11px;line-height:16px}.license-error{color:#c24848}
  @container content (max-width:720px){.license-layout{grid-template-columns:1fr;min-height:0}.license-list{border-bottom:1px solid var(--rw-border-subtle)}.license-list ol{max-height:220px;overflow:auto}.license-document pre{max-height:400px}}

  .license-list button{min-height:50px}.license-list b{font-size:12px;line-height:16px}.license-list small,.license-document header span{font-size:11px;line-height:15px}.license-document pre{font:11px/16px var(--rw-font-mono)}
</style>
