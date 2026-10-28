import type { Burgy } from "./Burgy";

export class TestService {
	constructor(private id: object) {}

  double(value: Burgy): Promise<number> {
		return window.webkit.messageHandlers.ui.postMessage({
			__mty: "rpc",
			__id: this.id,
		  __type: "double", 
      value, 
    });
  }
}