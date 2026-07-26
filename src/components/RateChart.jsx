// 速率 SVG 图（迷你折线图）
export function RateChart({ incoming, outgoing }) {
  const W = 640;
  const H = 160;
  const pad = 10;
  const n = Math.max(incoming?.length || 0, outgoing?.length || 0, 2);
  const all = [...(incoming || []), ...(outgoing || [])];
  const max = Math.max(...all, 1) * 1.2;
  const x = (i) => pad + (i * (W - 2 * pad)) / Math.max(n - 1, 1);
  const y = (v) => H - pad - (v / max) * (H - 2 * pad);
  const line = (arr, color) =>
    arr && arr.length > 0
      ? `<polyline fill="none" stroke="${color}" stroke-width="2" points="${arr
          .map((v, i) => `${x(i)},${y(v)}`)
          .join(" ")}"/>`
      : "";
  const lastIdx = (arr) => (arr ? arr.length - 1 : -1);
  const inLast = lastIdx(incoming);
  const outLast = lastIdx(outgoing);
  const grid = [0, 1, 2, 3]
    .map((g) => {
      const gy = pad + (g * (H - 2 * pad)) / 3;
      return `<line x1="${pad}" y1="${gy}" x2="${W - pad}" y2="${gy}" stroke="rgba(27,34,48,0.07)" stroke-width="1"/>`;
    })
    .join("");
  const inCircle = inLast >= 0 ? `<circle cx="${x(inLast)}" cy="${y(incoming[inLast])}" r="3.5" fill="#2f7ff2"/>` : "";
  const outCircle =
    outLast >= 0 ? `<circle cx="${x(outLast)}" cy="${y(outgoing[outLast])}" r="3.5" fill="#12b5a6"/>` : "";
  const svg = `${grid}${line(incoming, "#2f7ff2")}${line(outgoing, "#12b5a6")}${inCircle}${outCircle}`;

  return (
    <div class="chart-wrap">
      <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="xMidYMid meet" dangerouslySetInnerHTML={{ __html: svg }} />
      <div class="chart-legend">
        <div class="lg-item">
          <div class="swatch" style="background:#2f7ff2" />
          进入速率
        </div>
        <div class="lg-item">
          <div class="swatch" style="background:#12b5a6" />
          消费速率
        </div>
      </div>
    </div>
  );
}
