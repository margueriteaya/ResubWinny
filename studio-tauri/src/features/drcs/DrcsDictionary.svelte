<script lang="ts">
import { FileType2, Filter, Grid3X3, Image, Maximize2, Minus, Plus, RotateCcw, Save, Search, TextCursorInput, X } from '@lucide/svelte'
  import { t } from '../../i18n'
  import PopupButton from '../../components/PopupButton.svelte'
  import MacSegmentedControl from '../../components/MacSegmentedControl.svelte'

  type Glyph = { id: string; width: number; height: number; alternativeText: string; image: string }
  type Mapping = { text: string; action: 'image' | 'character' | 'font' }

  export let glyphs: Glyph[] = []
  export let message = ''
  export let getMapping: (id: string) => Mapping | undefined
  export let saveMapping: (id: string, text: string, action: Mapping['action']) => void
  export let refresh: () => void = () => {}

  let selected: Glyph | null = null
  let search = ''
  let tab: 'auto' | 'user' = 'auto'
  let mapping = ''
  let action: Mapping['action'] = 'image'
  let status: 'all' | 'mapped' | 'review' = 'all'
  let zoom = 400
  let grid = true

  $: visible = glyphs.filter((glyph) => {
    const saved = getMapping(glyph.id)
    const needsReview = !saved?.text && (saved?.action ?? 'image') === 'image'
    return (tab === 'auto' || saved)
      && (status === 'all' || (status === 'mapped' ? !needsReview : needsReview))
      && (glyph.id.toLowerCase().includes(search.toLowerCase()) || glyph.alternativeText.includes(search))
  })
  $: if (visible.length && (!selected || !visible.some((glyph) => glyph.id === selected?.id))) selectGlyph(visible[0])
  $: if (!visible.length) selected = null

  function selectGlyph(glyph: Glyph) {
    selected = glyph
    const saved = getMapping(glyph.id)
    mapping = saved?.text ?? glyph.alternativeText ?? ''
    action = saved?.action === 'character' ? 'character' : saved?.action === 'font' ? 'font' : 'image'
  }
</script>

<section class="dictionary-shell">
  <div class="dictionary-tabs"><MacSegmentedControl ariaLabel={t('nav.drcs')} value={tab} options={[{value:'auto',label:t('drcs.auto')},{value:'user',label:t('drcs.user')}]} onChange={(value) => tab = value as typeof tab} /></div>
  <div class="dictionary-tools">
    <label class="search"><Search size={17} /><input bind:value={search} placeholder={t('drcs.search')} /><button aria-label={t('drcs.clearSearch')} onclick={() => search = ''}><X size={15} /></button></label>
    <PopupButton label={t('drcs.status')} value={status} options={[{value:'all',label:t('drcs.all')},{value:'mapped',label:t('drcs.mapped')},{value:'review',label:t('drcs.review')}]} onChange={(value) => status = value as typeof status} />
    <button class="tool-button" onclick={() => status = status === 'review' ? 'all' : 'review'}><Filter size={17} /> {status === 'review' ? t('drcs.allGlyphs') : t('drcs.needsReview')}</button><button class="tool-button" aria-label={t('drcs.refresh')} onclick={refresh}><RotateCcw size={18} /></button>
  </div>
  {#if glyphs.length}
    <div class:no-inspector={!selected} class="dictionary-content">
      <section class="glyph-table" aria-label={t('drcs.glyphs')}>
        <div class="glyph-heading"><span>{t('drcs.preview')}</span><span>{t('drcs.mapping')}</span><span>{t('drcs.status')}</span></div>
        {#if visible.length}
          {#each visible as glyph (glyph.id)}
            {@const saved = getMapping(glyph.id)}
            {@const currentAction = saved?.action === 'character' ? 'character' : 'image'}
            <button class:selected={selected?.id === glyph.id} class="glyph-row" onclick={() => selectGlyph(glyph)}>
              <span class="mini-glyph"><img src={glyph.image} alt={`${t('drcs.glyph')} ${glyph.id}`} /></span>
              <span class="mapping"><b>{saved?.text || glyph.alternativeText || t('drcs.unmapped')}</b><small>{glyph.id} · {glyph.width}×{glyph.height}</small></span>
              <span class:needs-review={currentAction === 'image' && !saved?.text} class="mapping-status"><i></i>{currentAction === 'image' && !saved?.text ? t('drcs.review') : t('drcs.mapped')}<small>{currentAction === 'character' ? t('drcs.character') : t('drcs.image')}</small></span>
            </button>
          {/each}
        {:else}
          <div class="filter-empty"><Search size={30} /><b>{t('drcs.noMatches')}</b><p>{t('drcs.adjustFilters')}</p></div>
        {/if}
        <footer class="table-footer"><span>{t('drcs.totalGlyphs')}: {glyphs.length}</span><span>{visible.length} {t('drcs.shown')}</span></footer>
      </section>
      {#if selected}
        <aside class="dictionary-inspector">
          <header><div><h2>{t('drcs.inspector')}</h2><p>{t('drcs.glyph')} ({selected.id})</p></div></header>
          <div class:no-grid={!grid} class="large-glyph"><img src={selected.image} style={`width:${Math.min(94, 18 + zoom / 6)}%;height:${Math.min(94, 18 + zoom / 6)}%`} alt={`${t('drcs.enlargedGlyph')} ${selected.id}`} /></div>
          <div class="zoom-row"><button onclick={() => zoom = Math.max(100, zoom - 100)} aria-label={t('drcs.zoomOut')}><Minus size={16} /></button><b>{zoom}%</b><button onclick={() => zoom = Math.min(800, zoom + 100)} aria-label={t('drcs.zoomIn')}><Plus size={16} /></button><span></span><button aria-label={t('drcs.fitGlyph')} onclick={() => zoom = 400}><Maximize2 size={16} /></button><button class:selected={grid} aria-label={t('drcs.toggleGrid')} onclick={() => grid = !grid}><Grid3X3 size={16} /></button></div>
          <label>{t('drcs.mapping')}<input type="text" bind:value={mapping} placeholder={t('drcs.noUnicodeMapping')} /></label>
          <small class="mapping-hint">{mapping || t('drcs.mappingHint')}</small>
          <fieldset class="mapping-actions"><legend>{t('drcs.mappingAction')}</legend><MacSegmentedControl value={action} ariaLabel={t('drcs.mappingAction')} options={[{value:'character',label:t('drcs.character'),icon:TextCursorInput},{value:'image',label:t('drcs.image'),icon:Image},{value:'font',label:t('drcs.fontGlyph'),icon:FileType2}]} onChange={(value) => action = value as Mapping['action']} /></fieldset>
          <button class="save-mapping" onclick={() => saveMapping(selected!.id, mapping, action)}><Save size={18} /> {t('drcs.save')}</button><button class="reset" onclick={() => selectGlyph(selected!)}><RotateCcw size={17} /> {t('drcs.reset')}</button>
        </aside>
      {/if}
    </div>
  {:else}
    <div class="dictionary-empty"><Search size={36} /><h2>{t('drcs.empty')}</h2><p>{message}</p></div>
  {/if}
</section>

<style>
  .dictionary-shell{display:grid;grid-template-rows:40px 48px minmax(0,1fr);width:100%;height:100%;min-height:0;color:var(--rw-text);overflow:hidden;background:transparent}
  .dictionary-tabs{justify-self:start;display:flex;align-items:center;width:max-content;height:40px;margin:0}
  .dictionary-tabs :global(.mac-segmented button){min-width:88px}
  .dictionary-tools{justify-self:stretch;display:flex;align-items:center;gap:8px;width:100%;min-height:48px;padding:6px 0;border-bottom:1px solid var(--rw-border-subtle)}
  .search{display:flex;align-items:center;gap:7px;width:min(270px,32%);height:32px;padding:0 8px;color:var(--rw-muted);border:1px solid var(--rw-border);border-radius:7px;background:var(--rw-content)}
  .search input{min-width:0;min-height:0!important;flex:1;padding:0!important;border:0!important;outline:0!important;background:transparent!important;font-size:11px}.search button{display:grid;place-items:center;width:22px;height:22px;padding:0;border-radius:50%;color:inherit;background:transparent}
  .tool-button{display:inline-flex;align-items:center;justify-content:center;gap:6px;height:32px;padding:0 10px;border:.5px solid var(--rw-glass-border);border-radius:8px;color:var(--rw-text);background:transparent;font-size:11px}.tool-button:first-of-type{margin-left:auto}
  .dictionary-tools :global(.popup-button){width:152px;margin-top:0}
  .dictionary-content{justify-self:stretch;display:grid;grid-template-columns:minmax(0,1fr) 300px;width:100%;min-height:0;overflow:hidden}.dictionary-content.no-inspector{grid-template-columns:minmax(0,1fr)}.glyph-table{min-width:0;min-height:0;overflow:auto;border-right:1px solid var(--rw-border-subtle);background:var(--rw-content)}
  .glyph-heading,.glyph-row{display:grid;grid-template-columns:88px minmax(150px,1fr) minmax(108px,.45fr);align-items:center;gap:10px}.glyph-heading{height:32px;padding:0 12px;color:var(--rw-muted);border-bottom:1px solid var(--rw-border-subtle);background:var(--rw-surface-muted);font-size:9px;font-weight:650}
  .glyph-row{width:100%;min-height:70px;padding:6px 12px;color:var(--rw-text);border-bottom:1px solid var(--rw-border-subtle);background:transparent;text-align:left;content-visibility:auto;contain-intrinsic-size:auto 70px}.glyph-row:hover{background:color-mix(in srgb,var(--rw-text) 4%,transparent)}.glyph-row.selected{background:color-mix(in srgb,var(--rw-accent) 10%,var(--rw-content));box-shadow:inset 3px 0 var(--rw-accent)}
  .mini-glyph{display:grid;place-items:center;width:52px;height:52px;border:1px solid var(--rw-border);border-radius:6px;background:#101216}.mini-glyph img{width:48px;height:48px;image-rendering:pixelated}
  .mapping b,.mapping small,.mapping-status small{display:block}.mapping b{font-size:13px;line-height:17px}.mapping small,.mapping-status small{margin-top:3px;color:var(--rw-muted);font-size:9px;line-height:12px}.mapping-status{position:relative;padding-left:12px;color:var(--rw-text-secondary);font-size:11px}.mapping-status i{position:absolute;left:0;top:4px;width:6px;height:6px;border-radius:50%;background:var(--rw-success)}.mapping-status.needs-review i{background:var(--rw-warning)}
  .table-footer{display:flex;align-items:center;gap:20px;height:32px;padding:0 12px;color:var(--rw-muted);font-size:9px}
  .filter-empty{display:grid;place-items:center;align-content:center;min-height:240px;padding:24px;color:var(--rw-muted);text-align:center}.filter-empty b{margin-top:8px;color:var(--rw-text);font-size:13px}.filter-empty p{margin:3px 0 0;font-size:10px;line-height:14px}
  .dictionary-inspector{min-width:0;min-height:0;padding:14px;overflow:auto;background:var(--rw-surface-muted)}.dictionary-inspector h2{margin:0;font-size:13px;line-height:17px}.dictionary-inspector header p{margin:3px 0 0;color:var(--rw-muted);font-size:10px}
  .large-glyph{display:grid;place-items:center;aspect-ratio:1;margin-top:12px;overflow:hidden;border:1px solid var(--rw-border);border-radius:7px;background-color:#101216;background-image:linear-gradient(#2a2f35 1px,transparent 1px),linear-gradient(90deg,#2a2f35 1px,transparent 1px);background-size:11px 11px}.large-glyph.no-grid{background-image:none}.large-glyph img{width:76%;height:76%;image-rendering:pixelated}
  .zoom-row{display:flex;align-items:center;gap:5px;margin:8px 0 16px}.zoom-row button{display:grid;place-items:center;width:28px;height:28px;padding:0;border:.5px solid var(--rw-glass-border);border-radius:14px;color:var(--rw-text-secondary);background:transparent;box-shadow:var(--rw-control-shadow);backdrop-filter:blur(14px) saturate(1.26);-webkit-backdrop-filter:blur(14px) saturate(1.26)}.zoom-row button.selected{color:#fff;border-color:var(--rw-accent);background:var(--rw-accent)}.zoom-row b{padding:0 5px;font-size:10px}.zoom-row span{flex:1}
  .dictionary-inspector>label{display:block;color:var(--rw-text);font-size:10px;font-weight:600}.dictionary-inspector input[type="text"]{width:100%;margin-top:6px;padding:0 8px;font-size:11px}.mapping-hint{display:block;margin-top:6px;color:var(--rw-muted);font-size:9px;line-height:13px}.dictionary-inspector fieldset{margin:16px 0;padding:0;border:0}.dictionary-inspector legend{margin-bottom:7px;color:var(--rw-text);font-size:10px;font-weight:650}
  .mapping-actions :global(.mac-segmented){width:100%}.mapping-actions :global(.mac-segmented button){min-width:0;flex:1 1 0;padding-inline:4px}
  .save-mapping,.reset{display:flex;align-items:center;justify-content:center;gap:6px;width:100%;height:32px;padding:0;border-radius:7px;font-size:11px}.save-mapping{color:#fff;background:var(--rw-accent)}.reset{margin-top:7px;color:var(--rw-text);border:.5px solid var(--rw-glass-border);background:transparent;box-shadow:var(--rw-control-shadow)}
  .dictionary-empty{display:grid;place-items:center;gap:8px;min-height:0;color:var(--rw-muted);text-align:center}.dictionary-empty h2,.dictionary-empty p{margin:0}.dictionary-empty h2{color:var(--rw-text);font-size:15px}
  @media(max-width:980px){.dictionary-content{grid-template-columns:minmax(0,1fr) 270px}.glyph-heading,.glyph-row{grid-template-columns:72px minmax(130px,1fr) 96px}.dictionary-tools{flex-wrap:wrap}.search{width:min(240px,48%)}}
  @media(max-width:720px){.dictionary-shell{height:auto;overflow:visible}.dictionary-content{grid-template-columns:1fr;overflow:visible}.glyph-table{overflow:visible;border-right:0}.dictionary-inspector{overflow:visible;border-top:1px solid var(--rw-border-subtle)}.dictionary-tools .tool-button:first-of-type{margin-left:0}}
</style>
