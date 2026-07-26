import { getTerm } from "../lib/terms.js";

export function Term({ termKey, label }) {
  const term = getTerm(termKey);
  if (!term) return <span>{label}</span>;
  const display = label || term.label;
  return (
    <span class="term-wrapper">
      <span>{display}</span>
      <button type="button" class="term" aria-label={`术语解释：${term.label}`} tabindex="0">
        ?
        <span class="tip">
          <b>{term.label}</b>
          {term.tip}
        </span>
      </button>
    </span>
  );
}
