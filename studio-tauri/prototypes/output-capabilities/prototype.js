import { mountConditionList } from "./variants/condition-list.js";
import { mountStatusMatrix } from "./variants/status-matrix.js";
import { mountProgressiveDetail } from "./variants/progressive-detail.js";

const variants = [mountConditionList, mountStatusMatrix, mountProgressiveDetail];
const stage = document.getElementById("stage");
const picker = document.querySelector(".proto-picker");
const highlight = picker.querySelector(".proto-picker-highlight");
const items = [...picker.querySelectorAll(".proto-picker-item:not(.proto-picker-replay)")];
let current = 0;

function moveHighlight() {
  const el = items[current];
  highlight.style.width = `${el.offsetWidth}px`;
  highlight.style.transform = `translateX(${el.offsetLeft}px)`;
}

function mount(i) {
  stage.replaceChildren();
  requestAnimationFrame(() => variants[i](stage));
}

function setActive(i) {
  if (i < 0 || i >= variants.length) return;
  current = i;
  items.forEach((el, j) => {
    el.toggleAttribute("data-active", j === i);
    if (j === i) el.setAttribute("aria-current", "true");
    else el.removeAttribute("aria-current");
  });
  moveHighlight();
  const url = new URL(location.href);
  url.searchParams.set("v", String(i + 1));
  history.replaceState(null, "", url);
  mount(i);
}

items.forEach((el, i) => el.addEventListener("click", () => setActive(i)));
window.addEventListener("resize", moveHighlight);

document.addEventListener("keydown", (event) => {
  if (/^(INPUT|TEXTAREA|SELECT)$/.test(event.target.tagName) || event.target.isContentEditable) return;
  if (event.metaKey || event.ctrlKey || event.altKey) return;
  const num = Number.parseInt(event.key, 10);
  if (num >= 1 && num <= variants.length) setActive(num - 1);
  else if (event.key === "ArrowRight") setActive((current + 1) % variants.length);
  else if (event.key === "ArrowLeft") setActive((current - 1 + variants.length) % variants.length);
});

setActive((Number.parseInt(new URLSearchParams(location.search).get("v") ?? "1", 10) || 1) - 1);
requestAnimationFrame(() => requestAnimationFrame(() => picker.setAttribute("data-ready", "")));
