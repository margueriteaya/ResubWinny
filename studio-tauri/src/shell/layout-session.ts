export type LayoutState = {
  sidebarCollapsed: boolean;
  sidebarAutoCollapsed: boolean;
  compactTaskViewport: boolean;
  compactSourceOpen: boolean;
  compactOutputOpen: boolean;
};

/** Encapsulates responsive shell layout transitions; callers re-project state for Svelte. */
export class LayoutSession {
  state: LayoutState;

  constructor(sidebarCollapsed: boolean) {
    this.state = {
      sidebarCollapsed,
      sidebarAutoCollapsed: sidebarCollapsed,
      compactTaskViewport: false,
      compactSourceOpen: false,
      compactOutputOpen: false,
    };
  }

  setSidebarCollapsed(collapsed: boolean, automatic = false) {
    this.state = {
      ...this.state,
      sidebarCollapsed: collapsed,
      sidebarAutoCollapsed: automatic && collapsed,
    };
  }

  toggleSidebar() { this.setSidebarCollapsed(!this.state.sidebarCollapsed); }

  setCompactViewport(compactTaskViewport: boolean) {
    this.state = { ...this.state, compactTaskViewport, compactSourceOpen: false, compactOutputOpen: false };
  }

  toggleInspector(kind: "source" | "output") {
    const opening = kind === "source" ? !this.state.compactSourceOpen : !this.state.compactOutputOpen;
    this.state = {
      ...this.state,
      compactSourceOpen: kind === "source" ? opening : opening ? false : this.state.compactSourceOpen,
      compactOutputOpen: kind === "output" ? opening : opening ? false : this.state.compactOutputOpen,
    };
  }
}
