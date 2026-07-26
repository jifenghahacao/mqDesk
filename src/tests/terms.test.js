// Terms 模块单元测试
import { describe, expect, it } from "vitest";
import { TERMS, getTerm } from "../lib/terms.js";

describe("terms", () => {
  it("包含 PRD §6.1 全部 12 个术语", () => {
    const requiredKeys = [
      "broker",
      "vhost",
      "exchange",
      "binding",
      "routing_key",
      "queue",
      "ready",
      "unacked",
      "message",
      "ack",
      "consumer",
      "dead_letter",
    ];
    for (const key of requiredKeys) {
      expect(TERMS[key], `术语 ${key} 应存在`).toBeDefined();
      expect(TERMS[key].label, `术语 ${key} 应有中文 label`).toBeTruthy();
      expect(TERMS[key].tip, `术语 ${key} 应有大白话解释`).toBeTruthy();
    }
  });

  it("getTerm 返回指定术语，不存在返回 null", () => {
    expect(getTerm("queue").label).toBe("队列");
    expect(getTerm("not_exist")).toBeNull();
  });
});
