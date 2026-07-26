// ConnectionsView 组件测试
// 重点验证：连接成功后/失败后都会调用 onConnectEnd，关闭 ConnectingOverlay

import { fireEvent, render, screen, waitFor } from "@testing-library/preact";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ConnectionsView } from "../views/ConnectionsView.jsx";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args) => invokeMock(...args),
}));

const toastOkMock = vi.fn();
const toastFailMock = vi.fn();
vi.mock("../lib/toast.js", () => ({
  toastOk: (msg) => toastOkMock(msg),
  toastFail: (msg) => toastFailMock(msg),
  toastInfo: () => {},
}));

const CONN = {
  id: "conn-1",
  name: "dev",
  host: "localhost",
  management_scheme: "http",
  management_port: 15672,
  amqp_port: 5672,
  vhost: "/",
  username: "guest",
};

describe("ConnectionsView", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    toastOkMock.mockReset();
    toastFailMock.mockReset();
  });

  it("点击连接卡片成功后应关闭连接中遮罩", async () => {
    invokeMock
      .mockResolvedValueOnce([CONN]) // list_connections
      .mockResolvedValueOnce({ is_active: false, is_reachable: true }) // get_connection_status
      .mockResolvedValueOnce(CONN); // connect_to

    const onConnectStart = vi.fn();
    const onConnectEnd = vi.fn();
    const onConnected = vi.fn();

    render(<ConnectionsView onConnectStart={onConnectStart} onConnectEnd={onConnectEnd} onConnected={onConnected} />);

    await waitFor(() => screen.getByText("dev"));

    fireEvent.click(screen.getByText("连接"));

    await waitFor(() => {
      expect(onConnectStart).toHaveBeenCalledWith("dev");
      expect(onConnected).toHaveBeenCalledWith(CONN);
      expect(onConnectEnd).toHaveBeenCalledTimes(1); // 关键：成功也要关闭遮罩
      expect(toastOkMock).toHaveBeenCalledWith("已连接到「dev」");
    });
  });

  it("连接失败后也应关闭连接中遮罩", async () => {
    invokeMock
      .mockResolvedValueOnce([CONN]) // list_connections
      .mockResolvedValueOnce({ is_active: false, is_reachable: true }) // get_connection_status
      .mockRejectedValueOnce(new Error("auth failed")); // connect_to

    const onConnectStart = vi.fn();
    const onConnectEnd = vi.fn();
    const onConnected = vi.fn();

    render(<ConnectionsView onConnectStart={onConnectStart} onConnectEnd={onConnectEnd} onConnected={onConnected} />);

    await waitFor(() => screen.getByText("dev"));

    fireEvent.click(screen.getByText("连接"));

    await waitFor(() => {
      expect(onConnectStart).toHaveBeenCalledWith("dev");
      expect(onConnected).not.toHaveBeenCalled();
      expect(onConnectEnd).toHaveBeenCalledTimes(1); // 关键：失败也要关闭遮罩
      expect(toastFailMock).toHaveBeenCalledWith(expect.stringContaining("auth failed"));
    });
  });
});
