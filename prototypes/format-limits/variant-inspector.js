import { attentionCount, featureRows, formats, preservationPanel } from "./data.js";

export const inspector = {
  name: "检查器",
  mount(root) {
    let focused = "ASS";
    const selected = new Set(["ASS"]);

    const render = () => {
      const active = formats.find((format) => format.name === focused);
      root.innerHTML = `
        <section class="workbench-grid inspector-variant">
          <div class="panel output-panel inspector-layout">
            <div class="inspector-formats">
              <div class="panel-title"><span><b>输出格式</b><small>选择格式，在右侧核对限制</small></span></div>
              <div class="compact-format-grid">
                ${formats.map((format) => {
                  const count = attentionCount(format);
                  return `<button class="compact-format ${selected.has(format.name) ? "selected" : ""} ${focused === format.name ? "focused" : ""}" data-format="${format.name}" aria-pressed="${selected.has(format.name)}">
                    <span class="format-icon">${format.icon}</span><span><b>${format.name}</b><small>${format.ext}</small></span>
                    ${count ? `<i class="risk-badge">${count}</i>` : `<i class="safe-badge">✓</i>`}
                  </button>`;
                }).join("")}
              </div>
              <p class="inspector-hint">数字表示需要补充资源或无法保留的项目数。</p>
            </div>
            <aside class="format-inspector" aria-live="polite">
              <header><span class="large-format-icon">${active.icon}</span><span><b>${active.name}</b><small>${active.ext}</small></span><em>${attentionCount(active) ? `${attentionCount(active)} 项需处理` : "完整保留"}</em></header>
              <div class="inspector-feature-list">
                ${featureRows(active).map((row) => `<div class="inspector-feature tone-${row.status.tone}"><span class="feature-symbol">${row.icon}</span><span><b>${row.label}</b><small>${row.status.short}</small></span><i></i></div>`).join("")}
              </div>
              <p class="inspector-note">选择具体项目后，任务页会给出对应资源或转换说明。</p>
            </aside>
          </div>
          ${preservationPanel()}
        </section>`;

      root.querySelectorAll(".compact-format").forEach((button) => button.addEventListener("click", () => {
        const name = button.dataset.format;
        focused = name;
        if (selected.has(name) && selected.size > 1) selected.delete(name); else selected.add(name);
        render();
      }));
    };

    render();
  },
};
