// 术语映射表（PRD §6.1）
// 复用 .term 组件，所有界面共享

export const TERMS = {
  broker: { label: "消息服务节点", tip: "跑着 RabbitMQ 的那台服务器" },
  vhost: { label: "虚拟空间", tip: '一块隔离的"房间"，不同项目用不同房间互不干扰' },
  exchange: { label: "交换机", tip: '消息的"分拣中心"，按规则把消息送到对应队列' },
  binding: { label: "绑定规则", tip: '交换机和队列之间的"连线规则"' },
  routing_key: { label: "路由键", tip: '贴在消息上的"地址标签"，决定它去哪个队列' },
  queue: { label: "队列", tip: '存消息的"桶"，消费者从这里取' },
  ready: { label: "待消费", tip: "已经躺在桶里、等着被取的消息数" },
  unacked: { label: "处理中", tip: "已被取走、但还没确认处理完的消息数" },
  message: { label: "消息", tip: "你要传递的那段数据（通常是 JSON）" },
  ack: { label: "消费确认", tip: '消费者说"我处理完了"的回执' },
  consumer: { label: "消费者", tip: "从队列取消息来处理的程序" },
  dead_letter: { label: "死信", tip: '处理失败/过期被扔到"垃圾箱队列"的消息' },
};

export function getTerm(key) {
  return TERMS[key] || null;
}
