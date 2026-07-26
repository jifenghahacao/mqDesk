import { render, screen } from "@testing-library/preact";
// HealthBadge + StatusPill 单元测试
import { describe, expect, it } from "vitest";
import { HealthBadge, StatusPill } from "../components/Badges.jsx";

describe("HealthBadge", () => {
  it("正常状态显示绿色徽标", () => {
    render(<HealthBadge status="ok" />);
    expect(screen.getByText("正常")).toBeInTheDocument();
  });

  it("堆积预警状态显示黄色徽标", () => {
    render(<HealthBadge status="warn" />);
    expect(screen.getByText("堆积预警")).toBeInTheDocument();
  });

  it("无人消费状态显示红色徽标", () => {
    render(<HealthBadge status="danger" />);
    expect(screen.getByText("无人消费")).toBeInTheDocument();
  });

  it("空闲状态显示灰色徽标", () => {
    render(<HealthBadge status="idle" />);
    expect(screen.getByText("空闲")).toBeInTheDocument();
  });
});

describe("StatusPill", () => {
  it("已发送状态药丸", () => {
    render(<StatusPill status="sent" />);
    expect(screen.getByText("已发送")).toBeInTheDocument();
  });

  it("已被消费状态药丸", () => {
    render(<StatusPill status="consumed" />);
    expect(screen.getByText("已被消费")).toBeInTheDocument();
  });

  it("仍堆积状态药丸", () => {
    render(<StatusPill status="backlog" />);
    expect(screen.getByText("仍堆积")).toBeInTheDocument();
  });

  it("消费失败状态药丸", () => {
    render(<StatusPill status="failed" />);
    expect(screen.getByText("消费失败")).toBeInTheDocument();
  });
});
