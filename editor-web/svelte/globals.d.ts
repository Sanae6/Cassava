declare global {
  interface Window {
    webkit: {
      messageHandlers: {
        ui: { postMessage: (value: any) => Promise<any> }
      }
    }
  }
}

export { };
