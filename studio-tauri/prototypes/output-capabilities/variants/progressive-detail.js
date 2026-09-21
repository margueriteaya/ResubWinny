import { capabilityRows, formatTabs, surroundingContext, wireFormatTabs } from "../data.js";

export function mountProgressiveDetail(stage) {
  let selected = "ASS";
  let activeFeature = "drcs";
  const render = (format = selected, feature = activeFeature) => {
    selected = format;
    const rows = capabilityRows(selected);
    const active = rows.find((row) => row.key === feature) ?? rows[0];
    activeFeature = active.key;
    const counts = Object.groupBy ? Object.groupBy(rows, (row) => row.level) : rows.reduce((all, row) => ((all[row.level] ??= []).push(row), all), {});
    stage.innerHTML = surroundingContext(`<section class="format-card progressive-detail" aria-label="输出格式">
      <h2>输出格式</h2>
      ${formatTabs(selected)}
      <p class="decision-summary"><b>${selected}</b>：${counts.preserved?.length ?? 0} 项完整保留${counts.approximated?.length ? `，${counts.approximated.length} 项会兼容转换` : ""}${counts.conditional?.length ? `，${counts.conditional.length} 项需满足条件` : ""}${counts.unsupported?.length ? `，${counts.unsupported.length} 项无法保留` : ""}。</p>
      <div class="capability-chips" role="list" aria-label="字幕特征">
        ${rows.map((row) => `<button type="button" data-feature="${row.key}" class="level-${row.level}" aria-pressed="${row.key === active.key}"><i>${row.icon}</i>${row.label}</button>`).join("")}
      </div>
      <div class="detail-slot level-${active.level}" aria-live="polite">
        <span class="detail-status"><i>${active.icon}</i>${active.label} · ${active.label === "DRCS 字形" && active.level === "conditional" ? "需字形资源" : active.statusLabel}</span>
        <p>${active.detail}</p>
      </div>
    </section>`);
    wireFormatTabs(stage, (next) => {
      const nextRows = capabilityRows(next);
      const priority = nextRows.find((row) => row.level === "conditional") ?? nextRows.find((row) => row.level === "unsupported") ?? nextRows[0];
      render(next, priority.key);
    }, selected);
    stage.querySelectorAll("[data-feature]").forEach((button) => button.addEventListener("click", () => render(selected, button.dataset.feature)));
  };
  render();
}
