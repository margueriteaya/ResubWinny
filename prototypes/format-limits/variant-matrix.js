import { features, formats, preservationPanel, statusCopy } from "./data.js";

export const matrix = {
  name: "能力矩阵",
  mount(root) {
    const selected = new Set(["ASS"]);

    const render = () => {
      root.innerHTML = `
        <section class="workbench-grid matrix-variant">
          <div class="panel output-panel matrix-panel">
            <div class="panel-title"><span><b>格式能力比较</b><small>先比较每项字幕能力，再勾选输出格式</small></span></div>
            <div class="matrix-scroll">
              <table class="capability-matrix">
                <thead><tr><th scope="col">字幕特征</th>${formats.map((format) => `<th scope="col"><button data-format="${format.name}" aria-pressed="${selected.has(format.name)}"><span class="matrix-check">${selected.has(format.name) ? "✓" : ""}</span><b>${format.name}</b><small>${format.ext}</small></button></th>`).join("")}</tr></thead>
                <tbody>${features.map((feature) => `<tr><th scope="row"><span class="feature-symbol">${feature.icon}</span>${feature.label}</th>${formats.map((format) => { const status = statusCopy[format.capabilities[feature.key]]; return `<td><span class="matrix-state ${status.tone}" title="${status.short}：${status.detail}"><i></i><span>${status.short}</span></span></td>`; }).join("")}</tr>`).join("")}</tbody>
              </table>
            </div>
            <div class="matrix-footer"><span>已选择 <b>${selected.size}</b> 种格式</span><span class="matrix-legend"><i class="dot blue"></i>保留 <i class="dot yellow"></i>转换 <i class="dot red"></i>处理</span></div>
          </div>
          ${preservationPanel()}
        </section>`;

      root.querySelectorAll(".capability-matrix thead button").forEach((button) => button.addEventListener("click", () => {
        const name = button.dataset.format;
        if (selected.has(name) && selected.size > 1) selected.delete(name); else selected.add(name);
        render();
      }));
    };

    render();
  },
};
