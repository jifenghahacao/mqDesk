export function ChannelRow({ channel }) {
  return (
    <tr class="channel-row">
      <td class="mono">{channel.number}</td>
      <td class="mono">{channel.consumer_count}</td>
      <td class="mono">{channel.prefetch_count}</td>
      <td class="mono">{channel.unacked.toLocaleString()}</td>
      <td class="mono">{channel.publish_rate.toFixed(1)}</td>
      <td class="mono">{channel.deliver_rate.toFixed(1)}</td>
      <td class="mono">{channel.ack_rate.toFixed(1)}</td>
    </tr>
  );
}
