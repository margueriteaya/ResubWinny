<script lang="ts">
import { ChevronRight, Filter, Grid3X3, Maximize2, Minus, Plus, RotateCcw, Save, Search, X } from '@lucide/svelte'
  import { t } from './i18n'

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
  $: if (!selected && glyphs.length) selectGlyph(glyphs[0])
  $: if (selected && !glyphs.some((glyph) => glyph.id === selected?.id)) selected = null

  function selectGlyph(glyph: Glyph) {
    selected = glyph
    const saved = getMapping(glyph.id)
    mapping = saved?.text ?? glyph.alternativeText ?? ''
    action = saved?.action === 'character' ? 'character' : 'image'
  }
</script>

<section class="dictionary-shell">
  <div class="dictionary-tabs"><button class:selected={tab === 'auto'} onclick={() => tab = 'auto'}>{t('drcs.auto')}</button><button class:selected={tab === 'user'} onclick={() => tab = 'user'}>{t('drcs.user')}</button></div>
  <div class="dictionary-tools">
    <label class="search"><Search size={17} /><input bind:value={search} placeholder={t('drcs.search')} /><button aria-label={t('drcs.clearSearch')} onclick={() => search = ''}><X size={15} /></button></label>
    <button class="select" onclick={() => status = status === 'all' ? 'mapped' : status === 'mapped' ? 'review' : 'all'}>{t('drcs.status')}: {status === 'all' ? t('drcs.all') : status === 'mapped' ? t('drcs.mapped') : t('drcs.review')} <span class="select-chevron"><ChevronRight size={15} /></span></button>
    <button class="tool-button" onclick={() => status = status === 'review' ? 'all' : 'review'}><Filter size={17} /> {status === 'review' ? t('drcs.allGlyphs') : t('drcs.needsReview')}</button><button class="tool-button" aria-label={t('drcs.refresh')} onclick={refresh}><RotateCcw size={18} /></button>
  </div>
  {#if glyphs.length}
    <div class="dictionary-content">
      <section class="glyph-table" aria-label={t('drcs.glyphs')}>
        <div class="glyph-heading"><span>{t('drcs.preview')}</span><span>{t('drcs.mapping')}</span><span>{t('drcs.status')}</span></div>
        {#each visible as glyph}
          {@const saved = getMapping(glyph.id)}
          {@const currentAction = saved?.action === 'character' ? 'character' : 'image'}
          <button class:selected={selected?.id === glyph.id} class="glyph-row" onclick={() => selectGlyph(glyph)}>
            <span class="mini-glyph"><img src={glyph.image} alt={`${t('drcs.glyph')} ${glyph.id}`} /></span>
            <span class="mapping"><b>{saved?.text || glyph.alternativeText || t('drcs.unmapped')}</b><small>{glyph.id} · {glyph.width}×{glyph.height}</small></span>
            <span class:needs-review={currentAction === 'image' && !saved?.text} class="mapping-status"><i></i>{currentAction === 'image' && !saved?.text ? t('drcs.review') : t('drcs.mapped')}<small>{currentAction === 'character' ? t('drcs.character') : t('drcs.image')}</small></span>
          </button>
        {/each}
        <footer class="table-footer"><span>{t('drcs.totalGlyphs')}: {glyphs.length}</span><span>{visible.length} {t('drcs.shown')}</span></footer>
      </section>
      {#if selected}
        <aside class="dictionary-inspector">
          <header><div><h2>{t('drcs.inspector')}</h2><p>{t('drcs.glyph')} ({selected.id})</p></div></header>
          <div class:no-grid={!grid} class="large-glyph"><img src={selected.image} style={`width:${Math.min(94, 18 + zoom / 6)}%;height:${Math.min(94, 18 + zoom / 6)}%`} alt={`${t('drcs.enlargedGlyph')} ${selected.id}`} /></div>
          <div class="zoom-row"><button onclick={() => zoom = Math.max(100, zoom - 100)} aria-label={t('drcs.zoomOut')}><Minus size={16} /></button><b>{zoom}%</b><button onclick={() => zoom = Math.min(800, zoom + 100)} aria-label={t('drcs.zoomIn')}><Plus size={16} /></button><span></span><button aria-label={t('drcs.fitGlyph')} onclick={() => zoom = 400}><Maximize2 size={16} /></button><button class:selected={grid} aria-label={t('drcs.toggleGrid')} onclick={() => grid = !grid}><Grid3X3 size={16} /></button></div>
          <label>{t('drcs.mapping')}<input type="text" bind:value={mapping} placeholder={t('drcs.noUnicodeMapping')} /></label>
          <small class="mapping-hint">{mapping || t('drcs.mappingHint')}</small>
          <fieldset><legend>{t('drcs.mappingAction')}</legend><label><input type="radio" bind:group={action} value="character" /> {t('drcs.replaceCharacter')} <small>{t('drcs.mappingCharacter')}</small></label><label><input type="radio" bind:group={action} value="image" /> {t('drcs.keepImage')} <small>{t('drcs.mappingImage')}</small></label></fieldset>
          <button class="save-mapping" onclick={() => saveMapping(selected!.id, mapping, action)}><Save size={18} /> {t('drcs.save')}</button><button class="reset" onclick={() => selectGlyph(selected!)}><RotateCcw size={17} /> {t('drcs.reset')}</button>
        </aside>
      {/if}
    </div>
  {:else}
    <div class="dictionary-empty"><Search size={36} /><h2>{t('drcs.empty')}</h2><p>{message}</p></div>
  {/if}
</section>

<style>
  .dictionary-shell { min-height: 650px; border-top: 1px solid #2c3946; }
  .dictionary-tabs { display: flex; gap: 16px; padding: 0 23px; border-bottom: 1px solid #2c3946; }
  .dictionary-tabs button { height: 54px; padding: 0 12px; color: #aeb9c7; background: transparent; border-bottom: 2px solid transparent; }
  .dictionary-tabs .selected { color: #4ba1ff; border-color: #1876ed; }
  .dictionary-tools { display: flex; align-items: center; gap: 10px; padding: 17px 22px; }
  .search { display: flex; align-items: center; gap: 9px; width: 270px; padding: 0 10px; color: #91a0b0; border: 1px solid #34414e; border-radius: 6px; background: #171f28; }
  .search input { min-width: 0; flex: 1; padding: 9px 0; color: inherit; border: 0; outline: 0; background: transparent; }
  .search button { display:grid; place-items:center; padding:0; color:inherit; background:transparent; }
  .select, .tool-button { display: inline-flex; align-items: center; gap: 8px; padding: 10px 12px; color: #dbe4ee; border: 1px solid #34414e; border-radius: 6px; background: #19232d; }
  .select-chevron { margin-left: auto; transform: rotate(90deg); }
  .tool-button:first-of-type { margin-left: auto; }
  .dictionary-content { display: grid; grid-template-columns: minmax(0, 1fr) 335px; min-height: 560px; border-top: 1px solid #2c3946; }
  .glyph-table { min-width: 0; }
  .glyph-heading, .glyph-row { display: grid; grid-template-columns: 128px minmax(155px, 1fr) minmax(115px, .55fr); align-items: center; gap: 10px; }
  .glyph-heading { min-height: 49px; padding: 0 17px; color: #aab6c4; border-bottom: 1px solid #2d3946; font-size: 12px; }
  .glyph-row { width: 100%; min-height: 90px; padding: 8px 17px; color: #dce5ee; border-bottom: 1px solid #2a3541; background: #151d25; text-align: left; }
  .glyph-row:hover, .glyph-row.selected { background: linear-gradient(90deg, #15375c, #182431); box-shadow: inset 0 0 0 1px #2177d5; }
  .dictionary-inspector input[type="radio"] { accent-color: #1c79ef; }
  .mini-glyph { display: grid; place-items: center; width: 64px; height: 64px; background: #080d12; }
  .mini-glyph img { width: 62px; height: 62px; image-rendering: pixelated; }
  .mapping b, .mapping small, .mapping-status small { display: block; }
  .mapping small, .mapping-status small { margin-top: 5px; color: #91a0b0; font-size: 11px; }
  .mapping b { font-size: 17px; }
  .mapping-status { position: relative; padding-left: 14px; font-size: 13px; }
  .mapping-status i { position: absolute; left: 0; top: 3px; width: 7px; height: 7px; border-radius: 50%; background: #37c46d; }
  .mapping-status.needs-review i { background: #f3a51e; }
  .table-footer { display: flex; gap: 24px; align-items: center; padding: 17px 21px; color: #9facba; font-size: 12px; }
  .dictionary-inspector { padding: 18px; border-left: 1px solid #2b3845; background: #111820; }
  .dictionary-inspector header { display: flex; justify-content: space-between; align-items: flex-start; }
  .dictionary-inspector h2 { margin: 0; font-size: 16px; }
  .dictionary-inspector header p { margin: 12px 0; color: #cbd6e1; font-size: 13px; }
  .large-glyph { display: grid; place-items: center; aspect-ratio: 1; margin-top: 15px; border: 1px solid #3a4652; background-color: #0a0e13; background-image: linear-gradient(#26313a 1px, transparent 1px), linear-gradient(90deg, #26313a 1px, transparent 1px); background-size: 11px 11px; }
  .large-glyph.no-grid { background-image: none; }.zoom-row button.selected { color:#fff; border-color:#2f84e5; background:#176ce7; }
  .large-glyph img { width: 76%; height: 76%; image-rendering: pixelated; }
  .zoom-row { display: flex; gap: 8px; align-items: center; margin: 12px 0 22px; }
  .zoom-row button { width: 34px; height: 34px; color: #dbe5ef; border: 1px solid #364350; border-radius: 5px; background: #1a242e; }
  .zoom-row b { padding: 0 7px; font-size: 13px; }.zoom-row span { flex: 1; }
  .dictionary-inspector > label { display: block; color: #d4dee8; font-size: 12px; font-weight: 600; }
  .dictionary-inspector input[type="text"] { width: 100%; margin-top: 8px; padding: 10px; color: #e7eff7; border: 1px solid #35424e; border-radius: 6px; background: #19232c; }
  .mapping-hint { display: block; margin-top: 8px; color: #98a7b6; font-size: 11px; line-height: 1.4; }
  .dictionary-inspector fieldset { margin: 22px 0; padding: 0; border: 0; }.dictionary-inspector legend { margin-bottom: 11px; color: #d6e0eb; font-size: 13px; font-weight: 650; }
  .dictionary-inspector fieldset label { display: block; margin: 12px 0; color: #dce5ee; font-size: 13px; }.dictionary-inspector fieldset small { display: block; margin: 5px 0 0 23px; color: #8d9baa; font-size: 11px; }
  .save-mapping, .reset { display: flex; justify-content: center; align-items: center; gap: 8px; width: 100%; padding: 12px; border-radius: 6px; }.save-mapping { color: #fff; background: #176ce7; }.reset { margin-top: 10px; color: #ccd7e1; border: 1px solid #35424e; background: #19232c; }
  .dictionary-empty { display: grid; place-items: center; gap: 10px; min-height: 500px; color: #a1afbd; text-align: center; }.dictionary-empty h2, .dictionary-empty p { margin: 0; }.dictionary-empty h2 { color: #e1e8ef; }
  @media (max-width: 1150px) { .glyph-heading, .glyph-row { grid-template-columns: 102px minmax(145px, 1fr) minmax(95px, .6fr); }.dictionary-content { grid-template-columns: minmax(0, 1fr) 300px; } }
  @media (prefers-color-scheme: light) { .dictionary-shell, .dictionary-tabs, .dictionary-content { border-color: #dce5ee; }.dictionary-tabs button { color: #657488; }.dictionary-tabs .selected { color: #176ce7; }.search, .select, .tool-button { color: #344254; border-color: #d9e2ec; background: #fff; }.glyph-heading { color: #617085; border-color: #e0e7ef; }.glyph-row { color: #273344; border-color: #e7edf3; background: #fff; }.glyph-row:hover, .glyph-row.selected { background: #eef6ff; box-shadow: inset 0 0 0 1px #2b7ce1; }.mapping small, .mapping-status small, .dictionary-empty { color: #718093; }.dictionary-inspector { color: #273344; border-color: #dce5ee; background: #fafcff; }.dictionary-inspector h2, .dictionary-inspector header p, .dictionary-inspector > label, .dictionary-inspector legend, .dictionary-inspector fieldset label { color: #2c394b; }.large-glyph { border-color: #d9e2eb; background-color: #10151b; background-image: linear-gradient(#303940 1px, transparent 1px), linear-gradient(90deg, #303940 1px, transparent 1px); }.zoom-row button, .dictionary-inspector input[type="text"], .reset { color: #344254; border-color: #d8e1ea; background: #fff; }.mapping-hint, .dictionary-inspector fieldset small { color: #718093; }.dictionary-empty h2 { color: #283648; } }
</style>
