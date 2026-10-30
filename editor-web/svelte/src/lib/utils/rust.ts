import type { Version } from "$bindings/Version";
import type { Unsubscriber } from "svelte/store";

export function formatVersion(version: Version): `v${number}.${number}.${number}` {
  switch (version) {
    case "v100": return "v1.0.0";
    case "v101": return "v1.0.1";
    case "v110": return "v1.1.0";
    case "v120": return "v1.2.0";
    case "v130": return "v1.3.0";
    // default: throw new Error("unexpected version");
  }
}

interface Service<T> {
  new(id: object): T;
}
export async function fetchSingleton<T>(value: Service<T>, name: string): Promise<T> {
  let objectId = await window.webkit.messageHandlers.ui.postMessage({
    __mty: "get_singleton",
    name
  });
  console.log("got id", objectId);
  return new value(objectId);
}

export type ProgressReportCallback = (...args: any) => void;
class EventManager {
  private progressReporters = new Map<string, ProgressReportCallback>;
  registerProgressReporter(callback: ProgressReportCallback): [string, Unsubscriber] {
    if (!callback) throw new Error("a callback must be provided");
    let uuid = crypto.randomUUID();
    this.progressReporters.set(uuid, callback);
    return [
      uuid,
      () => this.progressReporters.delete(uuid),
    ];
  }

  on(event: string, callback: () => void) {

  }

  private invokeProgressReporter(uuid: string, ...args: any) {
    this.progressReporters.get(uuid)?.(...args);
  }
}
declare global {
  var __crescentEventManager: EventManager;
}
globalThis.__crescentEventManager = new EventManager();
