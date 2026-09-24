import { attentionCount, featureRows, formats, preservationPanel, selectedSummary } from "./data.js";

export const quiet = {
  name: "静默提醒",
  mount(root) {
    let focused = "ASS";
    const selected = new Set(["ASS"]);

    const render = () => {
      const active = formats.find((format) => format.name === focused);
      const summary = selectedSummary(active);
      root.innerHTML = `
        <section class="workbench-grid quiet-variant">
          <div class="panel output-panel">
            <div class="panel-title"><span><b>输出格式</b><small>可多选；限制直接标在格式上</small></span></div>
            <div class="quiet-format-list">
              ${formats.map((format) => {
                const count = attentionCount(format);
                const safe = count === 0;
                return `<button class="quiet-format ${selected.has(format.name) ? "selected" : ""} ${focused === format.name ? "focused" : ""}" data-format="${format.name}" aria-pressed="${selected.has(format.name)}">
                  <span class="format-icon">${format.icon}</span>
                  <span class="format-copy"><b>${format.name}</b><small>${format.ext}</small></span>
                  <span class="format-risk ${safe ? "safe" : "attention"}"><i></i>${safe ? "完整保留" : `${count} 项需处理`}</span>
                </button>`;
              }).join("")}
            </div>
            <div class="selection-summary"><b>${active.name}</b><span><em class="blue">${summary.preserved} 项保留</em><em class="yellow">${summary.converted} 项转换</em><em class="red">${summary.attention} 项处理</em></span></div>
            <div class="feature-chips">
              ${featureRows(active).map((row) => `<button class="feature-chip tone-${row.status.tone}" title="${row.status.short}：${row.status.detail}"><span>${row.icon}</span>${row.label}</button>`).join("")}
            </div>
            <div class="anchored-detail tone-${featureRows(active)[3].status.tone}">
              <b>${featureRows(active)[3].icon} ${featureRows(active)[3].label} · ${featureRows(active)[3].status.short}</b>
              <p>${featureRows(active)[3].status.detail}</p>
            </div>
          </div>
          ${preservationPanel()}
        </section>`;

      root.querySelectorAll(".quiet-format").forEach((button) => button.addEventListener("click", () => {
        const name = button.dataset.format;
        focused = name;
        if (selected.has(name) && selected.size > 1) selected.delete(name); else selected.add(name);
        render();
      }));
    };

    render();
  },
};
