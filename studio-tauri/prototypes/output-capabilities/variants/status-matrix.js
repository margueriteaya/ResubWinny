import { capabilityRows, formatTabs, surroundingContext, wireFormatTabs } from "../data.js";

export function mountStatusMatrix(stage) {
  let selected = "ASS";
  const render = (format = selected) => {
    selected = format;
    const rows = capabilityRows(selected);
    stage.innerHTML = surroundingContext(`<section class="format-card status-matrix" aria-label="输出格式">
      <div class="format-heading"><h2>输出格式</h2><span>状态会在载入录制文件后复核</span></div>
      ${formatTabs(selected)}
      <div class="matrix-grid">
        ${rows.map((row) => `<div class="matrix-cell level-${row.level}"><span>${row.label}</span><b><i>${row.icon}</i>${row.short}</b>${row.level === "conditional" ? `<small>找到字形或映射后解锁</small>` : ""}</div>`).join("")}
      </div>
      <div class="matrix-legend"><span><i>✓</i>完整保留</span><span><i>△</i>兼容转换</span><span><i>◇</i>需字形资源</span><span><i>×</i>无法保留</span></div>
    </section>`);
    wireFormatTabs(stage, render, selected);
  };
  render();
}
