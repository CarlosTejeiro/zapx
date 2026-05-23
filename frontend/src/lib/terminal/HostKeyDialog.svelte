<script lang="ts">
  interface Props {
    host: string
    port: number
    fingerprint: string
    /** true = key changed (mismatch, danger); false = first connection (TOFU). */
    changed: boolean
    onTrust: () => void
    onCancel: () => void
  }

  let { host, port, fingerprint, changed, onTrust, onCancel }: Props = $props()

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onCancel()
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
  <div class="dialog" class:danger={changed}>
    {#if changed}
      <h2 class="title danger-text">⚠ Host key changed</h2>
      <p class="msg">
        The host key for <strong>{host}:{port}</strong> does not match the one recorded in
        <code>known_hosts</code>. This could indicate a man-in-the-middle attack, or the server
        was legitimately reinstalled.
      </p>
      <p class="msg">
        For safety, the connection is refused. If you trust this change, remove the old key first:
      </p>
      <pre class="cmd">ssh-keygen -R {host}</pre>
    {:else}
      <h2 class="title">First connection to {host}:{port}</h2>
      <p class="msg">
        This host is not yet in your <code>known_hosts</code>. Verify the fingerprint below
        through a trusted channel before continuing.
      </p>
    {/if}

    <div class="fp">
      <span>Fingerprint</span>
      <code>{fingerprint}</code>
    </div>

    <div class="actions">
      <button class="btn-cancel" onclick={onCancel}>Cancel</button>
      {#if !changed}
        <button class="btn-trust" onclick={onTrust}>Trust &amp; connect</button>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 120;
  }

  .dialog {
    background: #18181b;
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    padding: 1.5rem;
    width: 30rem;
    max-width: 95vw;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .dialog.danger {
    border-color: #b91c1c;
  }

  .title {
    font-size: 0.95rem;
    font-weight: 600;
    color: #e4e4e7;
    margin: 0;
  }

  .danger-text {
    color: #f87171;
  }

  .msg {
    font-size: 0.82rem;
    color: #a1a1aa;
    line-height: 1.5;
    margin: 0;
  }

  .msg code,
  .fp code {
    color: #e4e4e7;
    background: #09090b;
    padding: 0 0.2rem;
    border-radius: 0.2rem;
  }

  .cmd {
    background: #09090b;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    padding: 0.5rem 0.6rem;
    font-size: 0.8rem;
    color: #e4e4e7;
    margin: 0;
    overflow-x: auto;
  }

  .fp {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: #71717a;
  }

  .fp code {
    font-size: 0.8rem;
    word-break: break-all;
    padding: 0.4rem 0.5rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  .btn-cancel,
  .btn-trust {
    padding: 0.35rem 0.9rem;
    font-size: 0.8rem;
    border-radius: 0.25rem;
    border: none;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-cancel {
    background: transparent;
    color: #a1a1aa;
    border: 1px solid #3f3f46;
  }

  .btn-cancel:hover {
    background: #27272a;
    color: #e4e4e7;
  }

  .btn-trust {
    background: #16a34a;
    color: #fff;
  }

  .btn-trust:hover {
    background: #15803d;
  }
</style>
