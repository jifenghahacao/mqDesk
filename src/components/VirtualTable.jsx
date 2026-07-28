import { useEffect, useRef, useState } from "preact/hooks";

/**
 * 固定行高的虚拟滚动表格。
 *
 * Props:
 * - data: 行数据数组
 * - rowHeight: 每行高度（px），默认 44
 * - buffer: 上下额外渲染的行数，默认 5
 * - columns: 列数，用于占位 td 的 colspan
 * - renderHeader: () => <thead>...</thead>
 * - renderRow: (item, index) => <tr>...</tr>
 * - className: 额外类名
 */
export function VirtualTable({
  data,
  rowHeight = 44,
  buffer = 5,
  columns,
  renderHeader,
  renderRow,
  className = "",
  tableClass = "",
}) {
  const wrapRef = useRef(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewHeight, setViewHeight] = useState(0);

  useEffect(() => {
    if (!wrapRef.current) return;
    const el = wrapRef.current;
    setViewHeight(el.clientHeight);

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setViewHeight(entry.contentRect.height);
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const totalHeight = data.length * rowHeight;
  const startIndex = Math.max(0, Math.floor(scrollTop / rowHeight) - buffer);
  const visibleCount = Math.ceil(viewHeight / rowHeight) + buffer * 2;
  const endIndex = Math.min(data.length, startIndex + visibleCount);

  const topSpacerHeight = startIndex * rowHeight;
  const bottomSpacerHeight = (data.length - endIndex) * rowHeight;
  const visibleData = data.slice(startIndex, endIndex);

  function handleScroll(e) {
    setScrollTop(e.currentTarget.scrollTop);
  }

  return (
    <div ref={wrapRef} class={`virtual-table-wrap ${className}`} onScroll={handleScroll}>
      <table class={`tbl virtual-table ${tableClass}`}>
        {renderHeader?.()}
        <tbody>
          {topSpacerHeight > 0 ? (
            <tr class="virtual-spacer" aria-hidden="true">
              <td colspan={columns} style={{ height: `${topSpacerHeight}px`, padding: 0, border: 0 }} />
            </tr>
          ) : null}
          {visibleData.map((item, i) => renderRow(item, startIndex + i))}
          {bottomSpacerHeight > 0 ? (
            <tr class="virtual-spacer" aria-hidden="true">
              <td colspan={columns} style={{ height: `${bottomSpacerHeight}px`, padding: 0, border: 0 }} />
            </tr>
          ) : null}
        </tbody>
      </table>
    </div>
  );
}
