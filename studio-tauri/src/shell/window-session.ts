import {
  beginWindowDrag,
  beginWindowResize,
  performWindowAction,
  type ResizeDirection,
  type WindowAction,
} from "./desktop";

type WindowSessionBindings = {
  onError: (message: string) => void;
  formatError: (kind: "action" | "drag" | "resize", message: string) => string;
};

/** Owns desktop window interactions and turns native failures into UI messages. */
export class WindowSession {
  constructor(private readonly bindings: WindowSessionBindings) {}

  async action(action: WindowAction) {
    try {
      await performWindowAction(action);
    } catch (reason) {
      this.bindings.onError(this.bindings.formatError("action", String(reason)));
    }
  }

  async beginDrag() {
    try {
      await beginWindowDrag();
    } catch (reason) {
      this.bindings.onError(this.bindings.formatError("drag", String(reason)));
    }
  }

  async beginResize(direction: ResizeDirection) {
    try {
      await beginWindowResize(direction);
    } catch (reason) {
      this.bindings.onError(this.bindings.formatError("resize", String(reason)));
    }
  }
}
