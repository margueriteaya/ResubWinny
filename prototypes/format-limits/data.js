export const features = [
  { key: "position", label: "字幕位置", icon: "↔" },
  { key: "color", label: "颜色", icon: "◉" },
  { key: "ruby", label: "Ruby 注音", icon: "ア" },
  { key: "drcs", label: "DRCS 字形", icon: "▦" },
  { key: "gaiji", label: "ARIB 外字", icon: "✦" },
  { key: "accessibility", label: "无障碍标识", icon: "◖" },
];

export const formats = [
  { name: "ASS", ext: ".ass", icon: "▤", capabilities: { position: "preserved", color: "preserved", ruby: "converted", drcs: "conditional", gaiji: "preserved", accessibility: "converted" } },
  { name: "TTML", ext: ".ttml", icon: "⌘", capabilities: { position: "preserved", color: "preserved", ruby: "preserved", drcs: "conditional", gaiji: "converted", accessibility: "preserved" } },
  { name: "SRT", ext: ".srt", icon: "☷", capabilities: { position: "unsupported", color: "unsupported", ruby: "unsupported", drcs: "conditional", gaiji: "converted", accessibility: "converted" } },
  { name: "WebVTT", ext: ".vtt", icon: "▣", capabilities: { position: "unsupported", color: "unsupported", ruby: "unsupported", drcs: "conditional", gaiji: "converted", accessibility: "converted" } },
  { name: "JSON", ext: ".json", icon: "{ }", capabilities: { position: "preserved", color: "preserved", ruby: "preserved", drcs: "preserved", gaiji: "preserved", accessibility: "preserved" } },
  { name: "Raw Data", ext: ".bin", icon: "01", capabilities: { position: "preserved", color: "preserved", ruby: "preserved", drcs: "preserved", gaiji: "preserved", accessibility: "preserved" } },
];

export const statusCopy = {
  preserved: { short: "完整保留", detail: "导出后仍可按原字幕语义还原。", tone: "blue" },
  converted: { short: "兼容转换", detail: "转换成目标格式可表达的近似形式，部分细节可能简化。", tone: "yellow" },
  conditional: { short: "需满足条件", detail: "找到对应的字形或映射资源后可保留。", tone: "red" },
  unsupported: { short: "无法保留", detail: "目标格式没有对应表达方式，导出时会移除。", tone: "red" },
};

export function attentionCount(format) {
  return Object.values(format.capabilities).filter((level) => level === "conditional" || level === "unsupported").length;
}

export function selectedSummary(format) {
  const values = Object.values(format.capabilities);
  return {
    preserved: values.filter((value) => value === "preserved").length,
    converted: values.filter((value) => value === "converted").length,
    attention: values.filter((value) => value === "conditional" || value === "unsupported").length,
  };
}

export function featureRows(format) {
  return features.map((feature) => ({ ...feature, level: format.capabilities[feature.key], status: statusCopy[format.capabilities[feature.key]] }));
}

export function preservationPanel() {
  return `
    <aside class="panel preservation-panel">
      <h2>保留内容</h2>
      <div class="preservation-grid">
        ${features.map((feature) => `<label><input type="checkbox" checked><span class="feature-symbol">${feature.icon}</span><span>${feature.label}</span></label>`).join("")}
      </div>
      <div class="legend">
        <h3>状态说明</h3>
        <p><i class="dot blue"></i><span><b>完整保留</b><small>原字幕语义保持完整</small></span></p>
        <p><i class="dot yellow"></i><span><b>兼容转换</b><small>转换为目标格式支持的表达</small></span></p>
        <p><i class="dot red"></i><span><b>需要处理</b><small>需要资源，或导出时会移除</small></span></p>
      </div>
    </aside>`;
}
