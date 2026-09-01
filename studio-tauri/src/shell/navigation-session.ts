import type { Page } from "../components/navigation";

/** Provides monotonic route generations for async page activation. */
export class NavigationSession {
  private generation = 0;
  private current: Page;

  constructor(initial: Page) { this.current = initial; }

  get page() { return this.current; }

  navigate(target: Page) {
    if (target === this.current) return null;
    this.current = target;
    return ++this.generation;
  }

  isCurrent(generation: number, page: Page) {
    return generation === this.generation && this.current === page;
  }
}
