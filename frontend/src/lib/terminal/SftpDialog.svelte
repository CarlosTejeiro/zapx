<script lang="ts">
  import { onMount } from 'svelte'
  import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
  import {
    sftpCanonicalize,
    sftpListDir,
    sftpMkdir,
    sftpRemoveDir,
    sftpRemoveFile,
    sftpRename,
    sftpDownloadFile,
    sftpUploadFile,
  } from '$lib/bridge/commands'
  import type { SftpEntry } from '$lib/bridge/types'

  interface Props {
    sessionId: string
    onClose: () => void
  }

  let { sessionId, onClose }: Props = $props()

  let path = $state('/')
  let entries = $state<SftpEntry[]>([])
  let selected = $state<string | null>(null)
  let loading = $state(true)
  let error = $state('')
  let busy = $state('') // 'mkdir' | 'download' | 'upload' | ...

  // Inline-form state
  let mkdirName = $state('')
  let renameTo = $state('')
  let downloadTo = $state('')
  let uploadFrom = $state('')
  let uploadAs = $state('')

  let showMkdir = $state(false)
  let showRename = $state(false)
  let showDownload = $state(false)
  let showUpload = $state(false)

  const selectedEntry = $derived(entries.find((e) => e.name === selected) ?? null)

  onMount(async () => {
    try {
      path = await sftpCanonicalize(sessionId, '.')
    } catch (e) {
      error = fmt(e)
    }
    await refresh()
  })

  async function refresh() {
    loading = true
    error = ''
    try {
      entries = await sftpListDir(sessionId, path)
      selected = null
    } catch (e) {
      error = fmt(e)
      entries = []
    } finally {
      loading = false
    }
  }

  function joinPath(base: string, name: string): string {
    if (base === '/') return `/${name}`
    return `${base.replace(/\/+$/, '')}/${name}`
  }

  function parentOf(p: string): string {
    if (p === '/' || p === '') return '/'
    const trimmed = p.replace(/\/+$/, '')
    const i = trimmed.lastIndexOf('/')
    if (i <= 0) return '/'
    return trimmed.slice(0, i)
  }

  async function goTo(newPath: string) {
    path = newPath || '/'
    await refresh()
  }

  function onEntryClick(entry: SftpEntry) {
    selected = entry.name
  }

  async function onEntryActivate(entry: SftpEntry) {
    const full = joinPath(path, entry.name)
    if (entry.kind === 'dir') {
      await goTo(full)
    } else if (entry.kind === 'symlink') {
      // Symlinks could point anywhere; try to enter as a directory, fall back to download prompt.
      await goTo(full).catch(() => {
        downloadTo = ''
        selected = entry.name
        showDownload = true
      })
    } else {
      selected = entry.name
      downloadTo = ''
      showDownload = true
    }
  }

  async function doMkdir() {
    busy = 'mkdir'
    error = ''
    try {
      await sftpMkdir(sessionId, joinPath(path, mkdirName.trim()))
      mkdirName = ''
      showMkdir = false
      await refresh()
    } catch (e) {
      error = fmt(e)
    } finally {
      busy = ''
    }
  }

  async function doRename() {
    if (!selectedEntry) return
    busy = 'rename'
    error = ''
    try {
      const from = joinPath(path, selectedEntry.name)
      const to = joinPath(path, renameTo.trim())
      await sftpRename(sessionId, from, to)
      renameTo = ''
      showRename = false
      await refresh()
    } catch (e) {
      error = fmt(e)
    } finally {
      busy = ''
    }
  }

  async function doDelete() {
    if (!selectedEntry) return
    const full = joinPath(path, selectedEntry.name)
    const isDir = selectedEntry.kind === 'dir'
    if (!confirm(`Delete ${isDir ? 'directory' : 'file'} "${selectedEntry.name}"?`)) return
    busy = 'delete'
    error = ''
    try {
      if (isDir) {
        await sftpRemoveDir(sessionId, full)
      } else {
        await sftpRemoveFile(sessionId, full)
      }
      await refresh()
    } catch (e) {
      error = fmt(e)
    } finally {
      busy = ''
    }
  }

  async function doDownload() {
    if (!selectedEntry || !downloadTo.trim()) return
    busy = 'download'
    error = ''
    try {
      const n = await sftpDownloadFile(
        sessionId,
        joinPath(path, selectedEntry.name),
        downloadTo.trim(),
      )
      showDownload = false
      downloadTo = ''
      error = `Downloaded ${n} bytes.`
    } catch (e) {
      error = fmt(e)
    } finally {
      busy = ''
    }
  }

  async function doUpload() {
    if (!uploadFrom.trim()) return
    const remoteName = uploadAs.trim() || baseName(uploadFrom.trim())
    if (!remoteName) {
      error = 'Remote name required.'
      return
    }
    busy = 'upload'
    error = ''
    try {
      const n = await sftpUploadFile(
        sessionId,
        uploadFrom.trim(),
        joinPath(path, remoteName),
      )
      showUpload = false
      uploadFrom = ''
      uploadAs = ''
      error = `Uploaded ${n} bytes.`
      await refresh()
    } catch (e) {
      error = fmt(e)
    } finally {
      busy = ''
    }
  }

  function baseName(p: string): string {
    return p.split(/[\\/]/).filter(Boolean).pop() ?? ''
  }

  function fmt(e: unknown): string {
    return e instanceof Error ? e.message : String(e)
  }

  /// Native picker for "where do I save this remote file locally?".
  async function pickDownloadDestination() {
    if (!selectedEntry) return
    try {
      const target = await saveDialog({
        title: 'Save remote file as…',
        defaultPath: selectedEntry.name,
      })
      if (target) downloadTo = target
    } catch (e) {
      error = fmt(e)
    }
  }

  /// Native picker for "which local file should I upload?".
  async function pickUploadSource() {
    try {
      const selected = await openDialog({
        title: 'Choose file to upload',
        multiple: false,
        directory: false,
      })
      if (typeof selected === 'string' && selected.length > 0) {
        uploadFrom = selected
        if (!uploadAs.trim()) uploadAs = baseName(selected)
      }
    } catch (e) {
      error = fmt(e)
    }
  }

  function fmtSize(n: number | null): string {
    if (n == null) return '–'
    if (n < 1024) return `${n} B`
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
    if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`
    return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`
  }

  function fmtMode(perm: number | null, kind: string): string {
    const prefix = kind === 'dir' ? 'd' : kind === 'symlink' ? 'l' : '-'
    if (perm == null) return `${prefix}---------`
    const bit = (b: number) => (perm & b ? 1 : 0)
    const triplet = (r: number, w: number, x: number) =>
      `${bit(r) ? 'r' : '-'}${bit(w) ? 'w' : '-'}${bit(x) ? 'x' : '-'}`
    return (
      prefix +
      triplet(0o400, 0o200, 0o100) +
      triplet(0o040, 0o020, 0o010) +
      triplet(0o004, 0o002, 0o001)
    )
  }

  function fmtMtime(s: number | null): string {
    if (s == null) return '–'
    const d = new Date(s * 1000)
    return d.toISOString().slice(0, 16).replace('T', ' ')
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose()
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
  <div class="dialog">
    <div class="header">
      <h2>SFTP browser</h2>
      <div class="header-right">
        <button class="btn" onclick={refresh} disabled={loading} title="Refresh">↻</button>
        <button class="btn" onclick={onClose} title="Close">✕</button>
      </div>
    </div>

    <div class="pathbar">
      <button class="btn" onclick={() => goTo(parentOf(path))} title="Up">↑</button>
      <input
        class="path"
        bind:value={path}
        onkeydown={(e) => { if (e.key === 'Enter') refresh() }}
        spellcheck="false"
      />
      <button class="btn primary" onclick={refresh} disabled={loading}>Go</button>
    </div>

    <div class="actions">
      <button class="btn" onclick={() => { showMkdir = true; showRename = false; showDownload = false; showUpload = false }}>
        + Folder
      </button>
      <button class="btn" onclick={() => { showUpload = true; showMkdir = false; showRename = false; showDownload = false; uploadAs = '' }}>
        ⬆ Upload
      </button>
      {#if selectedEntry}
        {#if selectedEntry.kind === 'file' || selectedEntry.kind === 'symlink'}
          <button class="btn" onclick={() => { showDownload = true; showMkdir = false; showRename = false; showUpload = false; downloadTo = '' }}>
            ⬇ Download
          </button>
        {/if}
        <button class="btn" onclick={() => { showRename = true; showMkdir = false; showDownload = false; showUpload = false; renameTo = selectedEntry?.name ?? '' }}>
          ✎ Rename
        </button>
        <button class="btn danger" onclick={doDelete} disabled={busy === 'delete'}>
          ✕ Delete
        </button>
      {/if}
    </div>

    {#if showMkdir}
      <form class="inline-form" onsubmit={(e) => { e.preventDefault(); doMkdir() }}>
        <input bind:value={mkdirName} placeholder="New folder name" spellcheck="false" autofocus />
        <button class="btn primary" type="submit" disabled={!mkdirName.trim() || busy === 'mkdir'}>Create</button>
        <button class="btn" type="button" onclick={() => (showMkdir = false)}>Cancel</button>
      </form>
    {/if}
    {#if showRename && selectedEntry}
      <form class="inline-form" onsubmit={(e) => { e.preventDefault(); doRename() }}>
        <input bind:value={renameTo} placeholder="New name" spellcheck="false" autofocus />
        <button class="btn primary" type="submit" disabled={!renameTo.trim() || busy === 'rename'}>Rename</button>
        <button class="btn" type="button" onclick={() => (showRename = false)}>Cancel</button>
      </form>
    {/if}
    {#if showDownload && selectedEntry}
      <form class="inline-form" onsubmit={(e) => { e.preventDefault(); doDownload() }}>
        <span class="form-label">Local path:</span>
        <input bind:value={downloadTo} placeholder="/abs/path/to/save/{selectedEntry.name}" spellcheck="false" autofocus />
        <button class="btn" type="button" onclick={pickDownloadDestination}>Browse…</button>
        <button class="btn primary" type="submit" disabled={!downloadTo.trim() || busy === 'download'}>
          {busy === 'download' ? 'Downloading…' : 'Download'}
        </button>
        <button class="btn" type="button" onclick={() => (showDownload = false)}>Cancel</button>
      </form>
    {/if}
    {#if showUpload}
      <form class="inline-form col" onsubmit={(e) => { e.preventDefault(); doUpload() }}>
        <div class="form-row">
          <span class="form-label">Local file:</span>
          <input bind:value={uploadFrom} placeholder="/abs/path/to/local/file" spellcheck="false" autofocus />
          <button class="btn" type="button" onclick={pickUploadSource}>Browse…</button>
        </div>
        <div class="form-row">
          <span class="form-label">Save as:</span>
          <input bind:value={uploadAs} placeholder="(defaults to local file name)" spellcheck="false" />
          <button class="btn primary" type="submit" disabled={!uploadFrom.trim() || busy === 'upload'}>
            {busy === 'upload' ? 'Uploading…' : 'Upload'}
          </button>
          <button class="btn" type="button" onclick={() => (showUpload = false)}>Cancel</button>
        </div>
      </form>
    {/if}

    {#if error}<p class="msg">{error}</p>{/if}

    <div class="listing">
      {#if loading}
        <p class="muted center">Loading…</p>
      {:else if entries.length === 0}
        <p class="muted center">Empty directory.</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th class="col-name">Name</th>
              <th class="col-size">Size</th>
              <th class="col-mode">Mode</th>
              <th class="col-mtime">Modified</th>
            </tr>
          </thead>
          <tbody>
            {#each entries as e (e.name)}
              <tr
                class:selected={selected === e.name}
                onclick={() => onEntryClick(e)}
                ondblclick={() => onEntryActivate(e)}
              >
                <td class="col-name">
                  <span class="icon">
                    {#if e.kind === 'dir'}📁{:else if e.kind === 'symlink'}🔗{:else}📄{/if}
                  </span>
                  {e.name}
                </td>
                <td class="col-size num">{e.kind === 'dir' ? '–' : fmtSize(e.size)}</td>
                <td class="col-mode num">{fmtMode(e.permissions, e.kind)}</td>
                <td class="col-mtime num">{fmtMtime(e.mtime)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
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
    z-index: 115;
  }

  .dialog {
    background: #18181b;
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    padding: 1rem 1.2rem 1.2rem;
    width: 48rem;
    max-width: 95vw;
    height: 36rem;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .header-right {
    display: flex;
    gap: 0.35rem;
  }

  h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: #e4e4e7;
  }

  .pathbar,
  .actions {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }

  .actions {
    flex-wrap: wrap;
  }

  .path {
    flex: 1;
    background: #09090b;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    color: #e4e4e7;
    font-size: 0.85rem;
    padding: 0.35rem 0.55rem;
    outline: none;
    font-family: monospace;
  }

  .path:focus {
    border-color: #3b82f6;
  }

  .btn {
    padding: 0.35rem 0.7rem;
    font-size: 0.8rem;
    border-radius: 0.25rem;
    border: 1px solid #3f3f46;
    background: #27272a;
    color: #e4e4e7;
    cursor: pointer;
    font-family: inherit;
    flex-shrink: 0;
  }

  .btn:hover:not(:disabled) {
    background: #3f3f46;
  }

  .btn.primary {
    background: #3b82f6;
    border-color: #3b82f6;
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .btn.danger {
    border-color: #b91c1c;
    color: #f87171;
  }

  .btn.danger:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.15);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .inline-form {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    background: #0a0a0b;
    border: 1px solid #27272a;
    border-radius: 0.3rem;
    padding: 0.5rem 0.6rem;
  }

  .inline-form.col {
    flex-direction: column;
    align-items: stretch;
  }

  .form-row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }

  .form-label {
    font-size: 0.75rem;
    color: #71717a;
    min-width: 5rem;
    flex-shrink: 0;
  }

  .inline-form input {
    flex: 1;
    background: #09090b;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    color: #e4e4e7;
    font-size: 0.82rem;
    padding: 0.3rem 0.5rem;
    outline: none;
    font-family: monospace;
  }

  .inline-form input:focus {
    border-color: #3b82f6;
  }

  .msg {
    font-size: 0.8rem;
    color: #f87171;
    margin: 0;
  }

  .listing {
    flex: 1;
    overflow: auto;
    background: #09090b;
    border: 1px solid #27272a;
    border-radius: 0.3rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
  }

  th {
    position: sticky;
    top: 0;
    background: #09090b;
    color: #71717a;
    text-align: left;
    font-weight: 500;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0.4rem 0.55rem;
    border-bottom: 1px solid #27272a;
  }

  td {
    padding: 0.35rem 0.55rem;
    color: #e4e4e7;
    border-bottom: 1px solid #18181b;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  tr {
    cursor: pointer;
  }

  tr:hover td {
    background: #18181b;
  }

  tr.selected td {
    background: #1e3a8a;
    color: #fff;
  }

  .col-name {
    width: 45%;
  }
  .col-size,
  .col-mode,
  .col-mtime {
    width: 18%;
  }
  .icon {
    margin-right: 0.35rem;
  }
  .num {
    font-family: monospace;
    font-size: 0.78rem;
  }

  .muted {
    color: #71717a;
    font-size: 0.82rem;
  }

  .center {
    text-align: center;
    padding: 1.5rem 0;
  }
</style>
