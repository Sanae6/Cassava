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
