export function BindingRow({ binding, onDelete }) {
  return (
    <tr class="binding-row">
      <td class="mono">{binding.source || "-"}</td>
      <td class="mono">{binding.routing_key || "-"}</td>
      <td>{binding.destination_type || "-"}</td>
      <td>
        <button type="button" class="btn sm ghost" onClick={() => onDelete(binding)}>
          解绑
        </button>
      </td>
    </tr>
  );
}
