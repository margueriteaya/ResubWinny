import { capabilityRows, formatTabs, surroundingContext, wireFormatTabs } from "../data.js";

export function mountConditionList(stage) {
  let selected = "ASS";
  const render = (format = selected) => {
    selected = format;
    const rows = capabilityRows(selected);
    const conditional = rows.find((row) => row.level === "conditional");
    stage.innerHTML = surroundingContext(`<section class="format-card condition-list" aria-label="输出格式">
      <h2>输出格式</h2>
      ${formatTabs(selected)}
      <div class="format-summary"><b>${selected}</b><span>${rows.filter((row) => row.level === "preserved").length} 项完整保留 · ${rows.filter((row) => row.level === "approximated").length} 项兼容转换</span></div>
      <div class="capability-list">
        ${rows.map((row) => `<span class="capability-item level-${row.level}"><i>${row.icon}</i>${row.label}<small>${row.short}</small></span>`).join("")}
      </div>
      <div class="condition-slot ${conditional ? "has-condition" : "resolved"}">
        ${conditional ? `<span class="condition-icon">◇</span><p><b>${conditional.label}：需要字形资源</b><small>找到录制文件内的 DRCS 字形或已有映射后自动解锁；缺少资源时标记为待处理。</small></p>` : `<span class="condition-icon">✓</span><p><b>无需额外条件</b><small>${selected} 可直接保留当前列出的全部字幕特征。</small></p>`}
      </div>
    </section>`);
    wireFormatTabs(stage, render, selected);
  };
  render();
}
