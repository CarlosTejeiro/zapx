<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { respondKeyboardInteractive } from '$lib/bridge/commands'
  import type { KiPromptEvent } from '$lib/bridge/types'

  // Queue of pending prompts; we render the head and pop on submit/cancel so
  // multiple concurrent connections don't lose interactions.
  let queue = $state<KiPromptEvent[]>([])
  let answers = $state<string[]>([])

  const current = $derived(queue[0] ?? null)

  let unlisten: UnlistenFn | null = null

  onMount(async () => {
    unlisten = await listen<KiPromptEvent>('ssh-ki-prompt', (e) => {
      queue.push(e.payload)
      // Initialise the answer buffer for the head of the queue if this is it.
      if (queue.length === 1) {
        answers = e.payload.prompts.map(() => '')
      }
    })
  })

  onDestroy(() => {
    unlisten?.()
  })

  // Whenever we move to the next pending request, reset the answer buffer.
  $effect(() => {
    if (current) {
      // Re-initialise only when the underlying prompt set actually changed.
      if (answers.length !== current.prompts.length) {
        answers = current.prompts.map(() => '')
      }
    }
  })

  async function submit() {
    if (!current) return
    const id = current.interactionId
    const toSend = [...answers]
    queue.shift()
    answers = []
    await respondKeyboardInteractive(id, toSend)
  }

  async function cancel() {
    if (!current) return
    const id = current.interactionId
    queue.shift()
    answers = []
    // Empty responses → russh sends back zero answers and auth fails cleanly.
    await respondKeyboardInteractive(id, [])
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') cancel()
  }
</script>

{#if current}
  <div class="overlay" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
    <form class="dialog" onsubmit={(e) => { e.preventDefault(); submit() }}>
      <h2 class="title">{current.name || 'Authentication required'}</h2>

      {#if current.instructions}
        <p class="instructions">{current.instructions}</p>
      {/if}

      {#each current.prompts as p, i (i)}
        <label class="field">
          <span>{p.prompt}</span>
          <input
            type={p.echo ? 'text' : 'password'}
            bind:value={answers[i]}
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
          />
        </label>
      {/each}

      <div class="actions">
        <button type="button" class="btn-cancel" onclick={cancel}>Cancel</button>
        <button type="submit" class="btn-submit">Submit</button>
      </div>
    </form>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 130;
  }

  .dialog {
    background: #18181b;
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    padding: 1.5rem;
    width: 26rem;
    max-width: 95vw;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .title {
    font-size: 0.95rem;
    font-weight: 600;
    color: #e4e4e7;
    margin: 0;
  }

  .instructions {
    font-size: 0.8rem;
    color: #a1a1aa;
    line-height: 1.5;
    margin: 0;
    white-space: pre-wrap;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .field span {
    font-size: 0.75rem;
    color: #71717a;
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
  }

  .field input:focus {
    border-color: #3b82f6;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  .btn-cancel,
  .btn-submit {
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

  .btn-submit {
    background: #3b82f6;
    color: #fff;
  }

  .btn-submit:hover {
    background: #2563eb;
  }
</style>
