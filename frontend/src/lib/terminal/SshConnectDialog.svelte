<script lang="ts">
  interface Props {
    onConnect: (params: SshParams) => void
    onCancel: () => void
  }

  export interface SshParams {
    host: string
    port: number
    user: string
    password: string
  }

  let { onConnect, onCancel }: Props = $props()

  let host = $state('')
  let port = $state(22)
  let user = $state('')
  let password = $state('')
  let error = $state('')

  function submit() {
    error = ''
    if (!host.trim()) { error = 'Host is required'; return }
    if (!user.trim()) { error = 'Username is required'; return }
    onConnect({ host: host.trim(), port, user: user.trim(), password })
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') submit()
    if (e.key === 'Escape') onCancel()
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
  <div class="dialog">
    <h2 class="title">New SSH session</h2>

    <label class="field">
      <span>Host</span>
      <input bind:value={host} placeholder="192.168.1.1" autocomplete="off" spellcheck="false" />
    </label>

    <div class="row">
      <label class="field" style="flex:3">
        <span>Username</span>
        <input bind:value={user} placeholder="admin" autocomplete="off" spellcheck="false" />
      </label>
      <label class="field" style="flex:1">
        <span>Port</span>
        <input type="number" bind:value={port} min="1" max="65535" />
      </label>
    </div>

    <label class="field">
      <span>Password</span>
      <input type="password" bind:value={password} placeholder="(empty = key auth later)" autocomplete="off" />
    </label>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <div class="actions">
      <button class="btn-cancel" onclick={onCancel}>Cancel</button>
      <button class="btn-connect" onclick={submit}>Connect</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: #18181b;
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    padding: 1.5rem;
    width: 380px;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .title {
    font-size: 0.9rem;
    font-weight: 600;
    color: #e4e4e7;
    margin: 0 0 0.25rem;
  }

  .row {
    display: flex;
    gap: 0.5rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    flex: 1;
  }

  .field span {
    font-size: 0.75rem;
    color: #71717a;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .field input {
    background: #09090b;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    color: #e4e4e7;
    font-size: 0.875rem;
    padding: 0.4rem 0.6rem;
    outline: none;
    font-family: inherit;
    width: 100%;
    box-sizing: border-box;
  }

  .field input:focus {
    border-color: #3b82f6;
  }

  .error {
    font-size: 0.8rem;
    color: #ef4444;
    margin: 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  .btn-cancel,
  .btn-connect {
    padding: 0.35rem 0.9rem;
    font-size: 0.8rem;
    border-radius: 0.25rem;
    border: none;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-cancel {
    background: transparent;
    color: #71717a;
    border: 1px solid #3f3f46;
  }

  .btn-cancel:hover { background: #27272a; color: #e4e4e7; }

  .btn-connect {
    background: #3b82f6;
    color: #fff;
  }

  .btn-connect:hover { background: #2563eb; }
</style>
