// Vitest 测试环境设置
import "@testing-library/preact";
import "@testing-library/jest-dom/vitest";

// jsdom 不支持的 API polyfill
if (!globalThis.matchMedia) {
  globalThis.matchMedia = (query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  });
}
