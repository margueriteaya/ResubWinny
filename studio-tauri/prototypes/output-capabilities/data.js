export const formats = ["ASS", "TTML", "SRT", "WebVTT", "JSON", "Raw Data"];

export const features = [
  { key: "position", label: "字幕位置" },
  { key: "color", label: "颜色" },
  { key: "ruby", label: "Ruby 注音" },
  { key: "drcs", label: "DRCS 字形" },
  { key: "gaiji", label: "ARIB 外字" },
  { key: "accessibility", label: "无障碍标识" },
];

export const capabilities = {
  ASS: { position: "preserved", color: "preserved", ruby: "approximated", drcs: "conditional", gaiji: "preserved", accessibility: "approximated" },
  TTML: { position: "preserved", color: "preserved", ruby: "preserved", drcs: "conditional", gaiji: "approximated", accessibility: "preserved" },
  SRT: { position: "unsupported", color: "unsupported", ruby: "unsupported", drcs: "conditional", gaiji: "approximated", accessibility: "approximated" },
  WebVTT: { position: "unsupported", color: "unsupported", ruby: "unsupported", drcs: "conditional", gaiji: "approximated", accessibility: "approximated" },
  JSON: { position: "preserved", color: "preserved", ruby: "preserved", drcs: "preserved", gaiji: "preserved", accessibility: "preserved" },
  "Raw Data": { position: "preserved", color: "preserved", ruby: "preserved", drcs: "preserved", gaiji: "preserved", accessibility: "preserved" },
};

export const status = {
  preserved: { statusLabel: "完整保留", short: "保留", icon: "✓", detail: "导出后仍可按原字幕语义还原。" },
  approximated: { statusLabel: "兼容转换", short: "转换", icon: "△", detail: "会转换成目标格式可表达的近似形式，细节可能简化。" },
  conditional: { statusLabel: "满足条件后保留", short: "需资源", icon: "◇", detail: "载入录制文件并找到对应的 DRCS 字形或映射资源后可保留；缺少资源时会标记为待处理。" },
  unsupported: { statusLabel: "无法保留", short: "不支持", icon: "×", detail: "目标格式没有对应表达方式，导出时会移除这项信息。" },
};

export function capabilityRows(format) {
  return features.map((feature) => ({ ...feature, level: capabilities[format][feature.key], ...status[capabilities[format][feature.key]] }));
}

export function formatTabs(selected) {
  return `<div class="format-tabs" role="tablist" aria-label="输出格式">
    ${formats.map((format) => `<button type="button" role="tab" data-format="${format}" aria-selected="${format === selected}">${format}</button>`).join("")}
  </div>`;
}

export function surroundingContext(inner) {
  return `<div class="prototype-window">
    <aside class="prototype-sidebar" aria-label="工作区导航"><strong>ResubWinny</strong><small>J-contents Caption Toolkit</small><span class="nav-active">⌂　主页</span><span>▧　任务</span><span>⌘　设置</span></aside>
    <section class="prototype-content">
      <header><h1>ResubWinny</h1><p>日本广播字幕提取、检查与转换。</p></header>
      <div class="drop-placeholder"><b>将录制文件拖放到这里</b><small>TS · M2TS · TLV</small></div>
      <div class="preference-grid">
        ${inner}
        <section class="preserve-card" aria-label="保留内容">
          <h2>保留内容</h2>
          <label><input type="checkbox" checked />字幕位置</label><label><input type="checkbox" checked />颜色</label>
          <label><input type="checkbox" checked />Ruby 注音</label><label><input type="checkbox" checked />ARIB 外字</label>
          <label><input type="checkbox" checked />DRCS 字形</label><label><input type="checkbox" checked />无障碍标识</label>
        </section>
      </div>
      <div class="rights-row">△　请仅处理你合法持有或获准使用的录制内容。<b>为什么？⌄</b></div>
      <h2 class="recent-heading">最近任务</h2>
    </section>
  </div>`;
}

export function wireFormatTabs(root, render, selected) {
  root.querySelectorAll("[data-format]").forEach((button) => {
    button.addEventListener("click", () => render(button.dataset.format));
  });
  root.querySelector(`[data-format="${CSS.escape(selected)}"]`)?.focus({ preventScroll: true });
}
