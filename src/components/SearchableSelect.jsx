import { useMemo } from "preact/hooks";

/**
 * 可检索下拉框
 *
 * 保持原生 select 的受控行为，但用 <input list="..."> + <datalist>
 * 实现输入即检索、仍可点击下拉选择。适用于选项较多或需要快速定位的场景。
 */
export function SearchableSelect({
  value,
  options,
  onChange,
  placeholder = "请选择…",
  disabled = false,
  class: className = "input",
  id,
  ...rest
}) {
  const listId = useMemo(() => id || `searchable-select-${Math.random().toString(36).slice(2, 9)}`, [id]);

  return (
    <>
      <input
        {...rest}
        type="text"
        list={listId}
        class={className}
        value={value}
        onInput={(e) => onChange(e.currentTarget.value)}
        placeholder={placeholder}
        disabled={disabled}
        autoComplete="off"
      />
      <datalist id={listId}>
        {options.map((opt) => (
          <option key={opt} value={opt}>
            {opt}
          </option>
        ))}
      </datalist>
    </>
  );
}
