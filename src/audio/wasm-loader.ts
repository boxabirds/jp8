import wasmUrl from '@jp8-wasm/jp8_wasm_bg.wasm?url';

let modulePromise: Promise<WebAssembly.Module> | null = null;

export function loadJP8Wasm(): Promise<WebAssembly.Module> {
  if (modulePromise) return modulePromise;

  modulePromise = (async () => {
    try {
      const response = fetch(wasmUrl);
      return await WebAssembly.compileStreaming(response);
    } catch {
      const response = await fetch(wasmUrl);
      const bytes = await response.arrayBuffer();
      return WebAssembly.compile(bytes);
    }
  })();

  return modulePromise;
}
