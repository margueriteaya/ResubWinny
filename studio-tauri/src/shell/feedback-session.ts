import { formatMessage } from "../i18n";

/** Keeps user-visible error and bounded notice-log policy out of the app shell. */
export class FeedbackSession {
  error(reason: unknown) {
    const message = reason instanceof Error ? reason.message : String(reason);
    return formatMessage("error.backend", { message });
  }

  append(logs: string[], code: string, parameters: Record<string, unknown> = {}) {
    return [...logs, formatMessage(code, parameters)].slice(-1000);
  }
}
