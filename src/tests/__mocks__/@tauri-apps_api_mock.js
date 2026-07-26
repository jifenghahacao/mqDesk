// Mock @tauri-apps/api 模块，让前端组件可在 Vitest 中测试

module.exports = {
  invoke: jest.fn(),
  event: {
    listen: jest.fn(),
    emit: jest.fn(),
  },
  window: {
    getCurrentWindow: () => ({
      minimize: jest.fn(),
      toggleMaximize: jest.fn(),
      close: jest.fn(),
      startDragging: jest.fn(),
    }),
  },
};
