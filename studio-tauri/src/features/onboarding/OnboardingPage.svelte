<script lang="ts">
  import { onMount } from "svelte";
  import { Archive, Check, ChevronRight, FileSearch, FileUp, TriangleAlert } from "@lucide/svelte";
  import { t } from "../../i18n";
  import type { UserMode } from "../../backend";
  import OnboardingColorBackground from "./OnboardingColorBackground.svelte";
  import XmbWaveBackground from "./XmbWaveBackground.svelte";

  export let saving = false;
  export let error = "";
  export let userMode: UserMode = "normie";
  export let onComplete: (mode: UserMode) => void = () => {};
  export let onOpenAbout: () => void = () => {};
  let motionPaused = false;

  onMount(() => {
    const syncMotionState = () => motionPaused = document.visibilityState === "hidden";
    syncMotionState();
    document.addEventListener("visibilitychange", syncMotionState);
    return () => document.removeEventListener("visibilitychange", syncMotionState);
  });

  const titleElements = [
    { src: "/onboarding/06-title-subtitle.png", light: "/onboarding/06-title-subtitle-light.png", label: "字幕", left: 495, top: 690, right: 741, bottom: 842 },
    { src: "/onboarding/07-title-wo.png", light: "/onboarding/07-title-wo-light.png", label: "を", left: 741, top: 690, right: 851, bottom: 842 },
    { src: "/onboarding/08-title-comma.png", light: "/onboarding/08-title-comma-light.png", label: "、", left: 851, top: 760, right: 919, bottom: 842 },
    { src: "/onboarding/09-title-shape-ruby.png", light: "/onboarding/09-title-shape-ruby-light.png", label: "形（ファイル）", left: 919, top: 635, right: 1128, bottom: 842 },
    { src: "/onboarding/10-title-ni.png", light: "/onboarding/10-title-ni-light.png", label: "に", left: 1128, top: 705, right: 1204, bottom: 842 },
    { src: "/onboarding/11-title-suru.png", light: "/onboarding/11-title-suru-light.png", label: "する", left: 1204, top: 690, right: 1441, bottom: 842 },
    { src: "/onboarding/12-title-drcs.png", light: "/onboarding/12-title-drcs-light.png", label: "DRCS", left: 1441, top: 690, right: 1550, bottom: 842 },
  ];

  function titleElementStyle(element: (typeof titleElements)[number], index: number) {
    return [
      `--element-delay:${680 + index * 167}ms`,
      `--element-left:${element.left / 19.2}%`,
      `--element-top:${element.top / 10.8}%`,
      `--element-width:${(element.right - element.left) / 19.2}%`,
      `--element-height:${(element.bottom - element.top) / 10.8}%`,
      `--element-mask:url('${element.light}')`,
    ].join(";");
  }

</script>

<section class="onboarding-page" class:motion-paused={motionPaused} aria-labelledby="onboarding-title">
  <div class="ambient-hero" role="img" aria-label={t("onboarding.sceneDescription")}>
    <div class="ambient-depth" aria-hidden="true"></div>
    <OnboardingColorBackground />
    <XmbWaveBackground />
    <div class="caption-stage" aria-hidden="true">
      <div class="caption-cells">
        <img class="caption-cell main-cell" src="/onboarding/00-main-cell.png" alt="" />
        <img class="caption-cell ruby-cell" src="/onboarding/00-ruby-cell.png" alt="" />
      </div>
      <div class="overlay-frames">
        {#each titleElements as element, index}
          <div class="title-element" style={titleElementStyle(element, index)} data-element={element.label}>
            <span class="scatter-window">
              <span class="screen-scatter scatter-reflection"><span class="scatter-source"></span></span>
              <span class="screen-scatter scatter-diffusion"><span class="scatter-source"></span></span>
            </span>
            <img class="title-ink" src={element.src} alt="" />
          </div>
        {/each}
        <div class="unified-scatter">
          <span class="screen-scatter unified-reflection"><span class="unified-scatter-source"></span></span>
          <span class="screen-scatter unified-interpolation"><span class="unified-scatter-source"></span></span>
          <span class="screen-scatter unified-diffusion"><span class="unified-scatter-source"></span></span>
          <span class="screen-scatter unified-overflow"><span class="unified-scatter-source"></span></span>
        </div>
        <img class="final-fallback" src="/onboarding/04-drcs.png" alt="" />
      </div>
      <span class="title-axis" data-wave-axis></span>
    </div>
  </div>

  <div class="onboarding-content">
    <header class="welcome-copy sequence-copy">
      <h1 id="onboarding-title">{t("onboarding.title")}</h1>
      <p>{t("onboarding.subtitle")}</p>
    </header>
    <ol class="welcome-steps sequence-steps">
      <li><span><FileUp size={18}/></span><div><b>{t("onboarding.step1Title")}</b><p>{t("onboarding.step1Body")}</p></div></li>
      <li><span><FileSearch size={18}/></span><div><b>{t("onboarding.step2Title")}</b><p>{t("onboarding.step2Body")}</p></div></li>
      <li><span><Archive size={18}/></span><div><b>{t("onboarding.step3Title")}</b><p>{t("onboarding.step3Body")}</p></div></li>
    </ol>
    <section class="mode-choice sequence-steps" aria-label="使用模式">
      <button type="button" class:selected={userMode === "normie"} onclick={() => userMode = "normie"}><b>工作模式</b><span>选择常用项并自动保存，快速载入文件并输出字幕。适合不求了解内部结构，潜心工作的时候。</span></button>
      <button type="button" class:selected={userMode === "nerd"} onclick={() => userMode = "nerd"}><b>狂热模式</b><span>展开查看录制文件中的服务、字幕轨道、特殊字形、时间结构和广播信息。适合充满兴趣到想把每个细节都看明白的时候。</span></button>
    </section>
    <aside class="use-notice sequence-notice" aria-labelledby="use-notice-title">
      <TriangleAlert size={19}/><div><b id="use-notice-title">{t("onboarding.noticeTitle")}</b><p>{t("onboarding.noticeBody")}</p></div>
    </aside>
    {#if error}<p class="onboarding-error" role="alert">{error}</p>{/if}
    <footer class="sequence-actions">
      <button class="open-about liquid-control" type="button" onclick={onOpenAbout}>{t("onboarding.about")}<ChevronRight size={15}/></button>
      <button class="complete liquid-control" type="button" disabled={saving} onclick={() => onComplete(userMode)}><Check size={16}/>{saving ? t("onboarding.saving") : t("onboarding.complete")}</button>
    </footer>
  </div>
</section>

<style>
  .onboarding-page{--scatter-color:#fff3e0;--diffusion-rgb:255,243,224;grid-column:1/-1;grid-row:2/-1;align-self:start;width:100%;min-width:0;min-height:calc(100dvh - 56px);margin:0;color:var(--rw-text);background:#fff}
  .ambient-hero{--ambient-x:50%;--ambient-y:42%;--ambient-shift-x:0px;--ambient-shift-y:0px;position:relative;width:100%;height:calc((100dvh - 56px) * .444444);overflow:hidden;background:#fff;isolation:isolate;animation:ambient-arrive 300ms var(--rw-ease-fluid) both}
  .ambient-hero::before,.ambient-hero::after,.ambient-depth{position:absolute;inset:-12%;content:"";pointer-events:none;will-change:transform}
  .ambient-hero::before{z-index:-2;background:radial-gradient(ellipse 36% 48% at 13% 38%,rgba(93,157,169,.22),transparent 74%),radial-gradient(ellipse 31% 50% at 86% 28%,rgba(193,128,139,.14),transparent 76%),radial-gradient(ellipse 40% 34% at 60% 78%,rgba(118,157,165,.15),transparent 74%);filter:blur(32px);transform:translate3d(var(--ambient-shift-x),var(--ambient-shift-y),0) scale(1.04);transition:transform var(--rw-motion-fluid) var(--rw-ease-fluid)}
  .ambient-hero::after{z-index:-1;background:radial-gradient(circle at var(--ambient-x) var(--ambient-y),rgba(255,255,255,.5),transparent 24%),linear-gradient(108deg,transparent 23%,rgba(255,255,255,.2) 46%,transparent 64%),linear-gradient(180deg,rgba(255,255,255,.08),transparent 54%,var(--rw-content) 96%);opacity:.9;transition:background-position var(--rw-motion-fluid) var(--rw-ease-fluid)}
  .ambient-depth{z-index:-1;background:linear-gradient(94deg,transparent 8%,rgba(255,255,255,.24) 31%,transparent 49%,rgba(182,96,105,.08) 72%,transparent 92%);filter:blur(12px);transform:translate3d(calc(var(--ambient-shift-x) * -.55),calc(var(--ambient-shift-y) * -.55),0);transition:transform var(--rw-motion-fluid) var(--rw-ease-fluid)}
  .caption-stage{--scatter-detail-envelope:2.695cqw;--scatter-envelope:3.126cqw;position:absolute;z-index:2;top:50%;left:50%;width:min(1080px,106vw);aspect-ratio:16/9;container-type:inline-size;pointer-events:none;transform:translate(-50%,-68.38%)}
  .title-axis{position:absolute;top:70.925926%;left:50%;width:0;height:0;pointer-events:none}
  .caption-cells,.overlay-frames{position:absolute;inset:0}.caption-cells{z-index:2}.caption-cell,.final-fallback{position:absolute;inset:0;display:block;width:100%;height:100%;object-fit:contain}.caption-cell{opacity:0}.main-cell{clip-path:inset(0 100% 0 0);animation:main-cell-enter 300ms 420ms var(--rw-ease-fluid) both}.ruby-cell{clip-path:inset(100% 0 0);transform:translateY(5px);animation:ruby-cell-enter 160ms 560ms var(--rw-ease-fluid) both}
  .overlay-frames{z-index:3}.title-element{position:absolute;left:var(--element-left);top:var(--element-top);width:var(--element-width);height:var(--element-height);overflow:hidden;pointer-events:none}.title-ink,.scatter-window,.screen-scatter,.scatter-source,.unified-scatter,.unified-scatter-source{position:absolute;inset:0;display:block;width:100%;height:100%}.title-ink{z-index:2;object-fit:fill;opacity:0;filter:blur(3px) brightness(1.12);animation:element-reveal 433ms var(--element-delay) var(--rw-ease-fluid) both}.scatter-window{z-index:3;overflow:hidden;mask-image:linear-gradient(to bottom,transparent 0,#000 8px,#000 calc(100% - 8px),transparent 100%);-webkit-mask-image:linear-gradient(to bottom,transparent 0,#000 8px,#000 calc(100% - 8px),transparent 100%)}.screen-scatter{opacity:0;mix-blend-mode:screen;will-change:filter,opacity}.scatter-source{background:var(--scatter-color);mask:var(--element-mask) center/100% 100% no-repeat;-webkit-mask:var(--element-mask) center/100% 100% no-repeat}.scatter-reflection{animation:reflection-propagate 880ms 2.14s linear both}.scatter-diffusion{animation:diffusion-propagate 880ms 2.14s linear both}.unified-scatter{z-index:5;pointer-events:none}.unified-scatter-source{background:var(--scatter-color);mask:url('/onboarding/13-title-unified-light.png') center/contain no-repeat;-webkit-mask:url('/onboarding/13-title-unified-light.png') center/contain no-repeat}.unified-reflection{z-index:3;animation:unified-reflection 880ms 2.14s linear both}.unified-interpolation{z-index:2;animation:unified-interpolation 880ms 2.14s linear both}.unified-diffusion{z-index:1;animation:unified-diffusion 880ms 2.14s linear both}.unified-overflow{z-index:0;animation:unified-overflow 880ms 2.14s linear both}.final-fallback{display:none}
  .onboarding-content{width:min(920px,calc(100% - 40px));margin:-8px auto 24px;position:relative;z-index:2}.welcome-copy{text-align:center;margin:0 auto}.welcome-copy h1{margin:0;font-size:27px;line-height:33px;letter-spacing:-.025em}.welcome-copy p{margin:5px auto 0;max-width:640px;color:var(--rw-text-secondary);font-size:13px;line-height:19px}
  .mode-choice{display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-top:10px}.mode-choice button{display:grid;gap:5px;padding:13px 14px;border:1px solid var(--rw-border);border-radius:9px;color:var(--rw-text);background:var(--rw-content);text-align:left}.mode-choice button.selected{border-color:var(--rw-accent);box-shadow:0 0 0 1px color-mix(in srgb,var(--rw-accent) 45%,transparent)}.mode-choice b{font-size:13px}.mode-choice span{color:var(--rw-text-secondary);font-size:11px;line-height:17px}
  .welcome-steps{display:grid;grid-template-columns:repeat(3,1fr);gap:10px;margin:17px 0 0;padding:0;list-style:none}.welcome-steps li{display:grid;grid-template-columns:34px 1fr;gap:10px;padding:13px;border:1px solid var(--rw-border-subtle);border-radius:10px;background:var(--rw-surface-muted)}.welcome-steps li>span{display:grid;place-items:center;width:32px;height:32px;border:.5px solid var(--rw-glass-border);border-radius:9px;color:var(--rw-accent);background:var(--rw-glass-control);box-shadow:var(--rw-control-shadow)}.welcome-steps b{font-size:12px}.welcome-steps p{margin:4px 0 0;color:var(--rw-text-secondary);font-size:11px;line-height:16px}
  .use-notice{display:flex;align-items:flex-start;gap:11px;margin-top:10px;padding:13px 15px;border:1px solid color-mix(in srgb,#c73c3c 58%,var(--rw-border));border-left:4px solid #c73c3c;border-radius:10px;color:color-mix(in srgb,#9f2020 88%,var(--rw-text));background:color-mix(in srgb,#d83e3e 10%,var(--rw-content))}.use-notice :global(svg){flex:0 0 19px;color:#c93636}.use-notice b{font-size:12px}.use-notice p{margin:3px 0 0;color:color-mix(in srgb,#972626 50%,var(--rw-text-secondary));font-size:11px;line-height:17px}.onboarding-error{margin:9px 2px 0;color:#c24848;font-size:12px}
  footer{display:flex;align-items:center;justify-content:flex-end;gap:9px;margin-top:13px}footer button{display:inline-flex;align-items:center;justify-content:center;gap:7px;height:36px;padding:0 14px;border:.5px solid var(--rw-glass-border);border-radius:18px;color:var(--rw-text);background:var(--rw-glass-control);box-shadow:var(--rw-control-shadow);font-size:12px}.complete{color:#fff!important;background:var(--rw-accent-control)!important;border-color:color-mix(in srgb,var(--rw-accent) 76%,white)!important}.complete:disabled{opacity:.65}
  .sequence-copy{opacity:0;animation:content-in 240ms 2.72s var(--rw-ease-fluid) both}.sequence-steps{opacity:0;animation:content-in 220ms 2.8s var(--rw-ease-fluid) both}.sequence-notice{opacity:0;animation:content-in 200ms 2.88s var(--rw-ease-fluid) both}.sequence-actions{opacity:0;animation:content-in 160ms 2.94s var(--rw-ease-fluid) both}
  .motion-paused .ambient-hero,.motion-paused .caption-cell,.motion-paused .title-ink,.motion-paused .screen-scatter,.motion-paused .sequence-copy,.motion-paused .sequence-steps,.motion-paused .sequence-notice,.motion-paused .sequence-actions{animation-play-state:paused}
  @keyframes ambient-arrive{from{opacity:.45;filter:blur(16px)}to{opacity:1;filter:none}}@keyframes main-cell-enter{from{opacity:0;clip-path:inset(0 100% 0 0)}to{opacity:1;clip-path:inset(0)}}@keyframes ruby-cell-enter{from{opacity:0;clip-path:inset(100% 0 0);transform:translateY(5px)}to{opacity:1;clip-path:inset(0);transform:none}}@keyframes element-reveal{from{opacity:0;filter:blur(3px) brightness(1.12)}to{opacity:1;filter:none}}@keyframes reflection-propagate{0%{opacity:0;filter:blur(0)}5%{opacity:.0703125;filter:blur(.6px)}35%{opacity:.225;filter:blur(2.5px)}62%{opacity:.45;filter:blur(5px)}100%{opacity:0;filter:blur(5px)}}@keyframes diffusion-propagate{0%{opacity:0;filter:blur(0)}5%{opacity:.125;filter:blur(2px)}35%{opacity:.4;filter:blur(13px)}62%{opacity:.8;filter:blur(26px)}100%{opacity:0;filter:blur(26px)}}@keyframes unified-reflection{0%,28%{opacity:0;filter:blur(0)}35%{opacity:.08;filter:blur(6px)}62%{opacity:.54;filter:blur(8px)}100%{opacity:.36;filter:blur(8px)}}@keyframes unified-diffusion{0%,28%{opacity:0;filter:blur(0)}35%{opacity:.14;filter:blur(20px)}62%{opacity:.96;filter:blur(42px)}100%{opacity:.64;filter:blur(42px)}}@keyframes content-in{from{opacity:0;transform:translateY(7px)}to{opacity:1;transform:none}}
  /* The element glows hand off to one continuous light field at completion. */
  @keyframes unified-reflection{0%,28%{opacity:0;filter:blur(0)}35%{opacity:.12;filter:blur(calc(var(--scatter-detail-envelope) * .1))}62%{opacity:.82;filter:blur(calc(var(--scatter-detail-envelope) * .22))}100%{opacity:.72;filter:blur(calc(var(--scatter-detail-envelope) * .22))}}
  @keyframes unified-interpolation{0%,28%{opacity:0;filter:blur(0)}35%{opacity:.06;filter:blur(calc(var(--scatter-detail-envelope) * .14))}62%{opacity:.34;filter:blur(calc(var(--scatter-detail-envelope) * .31))}100%{opacity:.3;filter:blur(calc(var(--scatter-detail-envelope) * .31))}}
  @keyframes unified-diffusion{0%,28%{opacity:0;filter:blur(0)}35%{opacity:.16;filter:blur(calc(var(--scatter-detail-envelope) * .18))}62%{opacity:.88;filter:drop-shadow(calc(var(--scatter-detail-envelope) * -.08) calc(var(--scatter-detail-envelope) * .05) calc(var(--scatter-detail-envelope) * .28) rgba(var(--diffusion-rgb),.62)) drop-shadow(calc(var(--scatter-detail-envelope) * .07) calc(var(--scatter-detail-envelope) * -.06) calc(var(--scatter-detail-envelope) * .36) rgba(var(--diffusion-rgb),.44)) blur(calc(var(--scatter-detail-envelope) * .48))}100%{opacity:.82;filter:drop-shadow(calc(var(--scatter-detail-envelope) * -.08) calc(var(--scatter-detail-envelope) * .05) calc(var(--scatter-detail-envelope) * .28) rgba(var(--diffusion-rgb),.62)) drop-shadow(calc(var(--scatter-detail-envelope) * .07) calc(var(--scatter-detail-envelope) * -.06) calc(var(--scatter-detail-envelope) * .36) rgba(var(--diffusion-rgb),.44)) blur(calc(var(--scatter-detail-envelope) * .48))}}
  @keyframes unified-overflow{0%,28%{opacity:0;filter:blur(0)}35%{opacity:.025;filter:blur(calc(var(--scatter-envelope) * .34))}62%{opacity:.2;filter:drop-shadow(calc(var(--scatter-envelope) * -.04) calc(var(--scatter-envelope) * .07) calc(var(--scatter-envelope) * .34) rgba(var(--diffusion-rgb),.34)) blur(calc(var(--scatter-envelope) * .58))}100%{opacity:.16;filter:drop-shadow(calc(var(--scatter-envelope) * -.04) calc(var(--scatter-envelope) * .07) calc(var(--scatter-envelope) * .34) rgba(var(--diffusion-rgb),.34)) blur(calc(var(--scatter-envelope) * .58))}}
  :global([data-theme="dark"]) .onboarding-page{--scatter-color:#ffd9bb;--diffusion-rgb:255,217,187;background:#000}
  :global([data-theme="dark"]) .ambient-hero{background:#000}
  :global([data-theme="dark"]) .ambient-hero::before{background:radial-gradient(ellipse 46% 72% at 10% 40%,rgba(65,130,145,.13),transparent 76%),radial-gradient(ellipse 42% 68% at 92% 34%,rgba(126,79,91,.085),transparent 78%);filter:blur(42px);opacity:.66}
  :global([data-theme="dark"]) .ambient-hero::after{background:linear-gradient(180deg,transparent 48%,color-mix(in srgb,var(--rw-content) 24%,transparent) 74%,var(--rw-content) 100%);opacity:1}
  :global([data-theme="dark"]) .ambient-depth{display:none}
  :global([data-theme="dark"]) .use-notice{color:#ffaaaa;background:color-mix(in srgb,#8d1717 28%,var(--rw-content));border-color:rgba(255,105,105,.48);border-left-color:#ff6868}:global([data-theme="dark"]) .use-notice p{color:#efb6b6}
  @media(max-width:700px){.ambient-hero{height:calc((100dvh - 56px) * .444444)}.caption-stage{width:124vw;transform:translate(-50%,-68.38%)}.onboarding-content{width:calc(100% - 24px);margin-top:-2px}.welcome-steps{grid-template-columns:1fr}.welcome-copy{text-align:left}.welcome-copy h1{font-size:23px}footer{align-items:stretch;flex-direction:column-reverse}footer button{width:100%}}
  :global(html[data-glass-static="true"]) .ambient-hero,:global(html[data-glass-static="true"]) .caption-cell,:global(html[data-glass-static="true"]) .sequence-copy,:global(html[data-glass-static="true"]) .sequence-steps,:global(html[data-glass-static="true"]) .sequence-notice,:global(html[data-glass-static="true"]) .sequence-actions{animation:none!important;filter:none}:global(html[data-glass-static="true"]) .caption-cell{opacity:1;clip-path:inset(0);transform:none}:global(html[data-glass-static="true"]) .title-element,:global(html[data-glass-static="true"]) .unified-scatter{display:none}:global(html[data-glass-static="true"]) .final-fallback{display:block}:global(html[data-glass-static="true"]) .sequence-copy,:global(html[data-glass-static="true"]) .sequence-steps,:global(html[data-glass-static="true"]) .sequence-notice,:global(html[data-glass-static="true"]) .sequence-actions{opacity:1}
  @media(prefers-reduced-motion:reduce){.ambient-hero,.caption-cell,.sequence-copy,.sequence-steps,.sequence-notice,.sequence-actions{animation:none!important;filter:none}.caption-cell{opacity:1;clip-path:inset(0);transform:none}.title-element,.unified-scatter{display:none}.final-fallback{display:block}.sequence-copy,.sequence-steps,.sequence-notice,.sequence-actions{opacity:1}.ambient-hero::before,.ambient-depth{transform:none!important;transition:none}}
  @media(forced-colors:active){.ambient-hero{background:Canvas;border-bottom:1px solid CanvasText}.ambient-hero::before,.ambient-hero::after,.ambient-depth{display:none}.caption-cell{animation:none!important;opacity:1;clip-path:inset(0);transform:none}.title-element,.unified-scatter{display:none}.final-fallback{display:block}.welcome-steps li,.use-notice,footer button{border:1px solid CanvasText}.use-notice{border-left:4px solid CanvasText;color:CanvasText;background:Canvas}.use-notice p{color:CanvasText}}
</style>
