<script lang="ts">
  import { ask, open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
  import {
    BACKUP_EXTENSION,
    BACKUP_MIN_PASSPHRASE,
    backupExport,
    backupInspect,
    backupRestore,
    restartApp,
    type BundleInfo,
  } from '$lib/bridge/commands'
  import { showToast } from '$lib/ui/toast-store.svelte'
  import PassphraseDialog from '$lib/ui/PassphraseDialog.svelte'

  let busy = $state(false)

  /// Which passphrase prompt is open, and the file it applies to.
  type Prompt = { mode: 'export' | 'restore'; path: string }
  let prompt = $state<Prompt | null>(null)

  function defaultFileName(): string {
    const d = new Date()
    const pad = (n: number) => String(n).padStart(2, '0')
    return `zapx-backup-${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}.${BACKUP_EXTENSION}`
  }

  function describe(i: BundleInfo): string {
    return (
      `${i.sessions} sessions, ${i.folders} folders, ${i.groups} groups, ` +
      `${i.snippets} snippets, ${i.rules} rules, ${i.vault_entries} vault entries, ` +
      `${i.session_credentials} session credentials, ${i.settings} settings, ` +
      `${i.known_hosts_lines} known hosts`
    )
  }

  function warn(title: string, warnings: string[]) {
    if (warnings.length === 0) return
    showToast({
      kind: 'warning',
      title: `${title}: ${warnings.length} warning(s)`,
      detail: warnings.slice(0, 3).join(' · ') + (warnings.length > 3 ? ' …' : ''),
      ttl: 0,
    })
  }

  async function chooseExportFile() {
    if (busy) return
    const path = await saveDialog({
      title: 'Save encrypted backup',
      defaultPath: defaultFileName(),
      filters: [{ name: 'ZAPX backup', extensions: [BACKUP_EXTENSION] }],
    })
    if (!path) return
    prompt = { mode: 'export', path }
  }

  async function chooseRestoreFile() {
    if (busy) return
    const path = await openDialog({
      title: 'Open encrypted backup',
      multiple: false,
      directory: false,
      filters: [{ name: 'ZAPX backup', extensions: [BACKUP_EXTENSION] }],
    })
    if (typeof path !== 'string' || !path) return
    prompt = { mode: 'restore', path }
  }

  async function onPassphrase(passphrase: string) {
    const p = prompt
    prompt = null
    if (!p) return
    busy = true
    try {
      if (p.mode === 'export') await doExport(p.path, passphrase)
      else await doRestore(p.path, passphrase)
    } finally {
      busy = false
    }
  }

  async function doExport(path: string, passphrase: string) {
    try {
      const s = await backupExport(path, passphrase)
      showToast({
        kind: 'success',
        title: 'Backup saved',
        detail: `${describe(s.info)} → ${s.path}`,
      })
      warn('Backup', s.warnings)
    } catch (e) {
      showToast({ kind: 'error', title: 'Backup failed', detail: String(e) })
    }
  }

  async function doRestore(path: string, passphrase: string) {
    let info: BundleInfo
    try {
      info = await backupInspect(path, passphrase)
    } catch (e) {
      showToast({ kind: 'error', title: 'Cannot open backup', detail: String(e) })
      return
    }
    const when = info.created_at ? new Date(info.created_at).toLocaleString() : 'unknown date'
    const proceed = await ask(
      `Backup from "${info.device_name}" (ZAPX ${info.app_version}, ${when}):\n\n` +
        `${describe(info)}.\n\n` +
        'Sessions that already exist here are kept as they are. Vault entries with the same ' +
        'name are updated from the backup. Settings from the backup replace the current ones.\n\n' +
        'Restore now?',
      { title: 'Restore backup', kind: 'warning' },
    )
    if (!proceed) return
    try {
      const s = await backupRestore(path, passphrase)
      const env = s.environment
      showToast({
        kind: 'success',
        title: 'Backup restored',
        detail:
          `${env.sessions_added} new sessions (${env.sessions_skipped} already existed), ` +
          `${env.folders_added} folders, ${env.groups_added} groups, ${env.snippets_added} snippets, ` +
          `${env.rules_added} rules · vault: ${s.vault_added} added, ${s.vault_updated} updated · ` +
          `${s.credentials_linked} credentials attached · ${s.settings_applied} settings · ` +
          `${s.known_hosts_added} known hosts`,
        ttl: 0,
      })
      warn('Restore', [...env.warnings, ...s.warnings])
      const yes = await ask(
        'Restart ZAPX now so the restored sessions and settings are loaded?\n\nOpen sessions will be closed.',
        { title: 'Restart ZAPX', kind: 'info' },
      )
      if (yes) await restartApp()
    } catch (e) {
      showToast({ kind: 'error', title: 'Restore failed', detail: String(e) })
    }
  }
</script>

<div class="panel">
  <section>
    <h3>Encrypted backup</h3>
    <p class="hint">
      One file with <strong>everything</strong>: sessions, folders, groups, snippets and macros,
      highlight rules, settings, trusted host keys — and the passwords and vault entries that the
      plain sessions export leaves out. It is sealed with a passphrase you choose (Argon2id +
      AES-256-GCM), so it is safe to keep in any cloud-synced folder: the provider only ever sees
      ciphertext.
    </p>
    <div class="actions">
      <button type="button" class="ok-btn" disabled={busy} onclick={chooseExportFile}>
        Create backup…
      </button>
      <button type="button" class="ghost-btn" disabled={busy} onclick={chooseRestoreFile}>
        Restore from backup…
      </button>
    </div>
    <p class="hint">
      Restoring merges: sessions that already exist here are left untouched (including their
      passwords), vault entries with the same name are updated, settings are replaced, host keys are
      added. Nothing is deleted.
    </p>
  </section>

  <section>
    <h3>Moving to another device</h3>
    <p class="hint">
      Create a backup here, let your cloud folder (Dropbox, Drive, iCloud, OneDrive, a NAS…) carry
      it, install ZAPX on the other device and restore it with the same passphrase. The passphrase
      is never stored anywhere — if you forget it, the backup cannot be opened.
    </p>
  </section>
</div>

{#if prompt}
  <PassphraseDialog
    title={prompt.mode === 'export' ? 'Protect the backup' : 'Open the backup'}
    message={prompt.mode === 'export'
      ? `Choose a passphrase (at least ${BACKUP_MIN_PASSPHRASE} characters). You will need it to restore the backup — it cannot be recovered.`
      : 'Enter the passphrase the backup was created with.'}
    confirm={prompt.mode === 'export'}
    minLength={prompt.mode === 'export' ? BACKUP_MIN_PASSPHRASE : 0}
    submitLabel={prompt.mode === 'export' ? 'Create backup' : 'Open'}
    onSubmit={onPassphrase}
    onCancel={() => (prompt = null)}
  />
{/if}

<style>
  .panel {
    padding: 0.75rem 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    color: var(--zx-text);
    font-family: var(--zx-font-ui);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  h3 {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--zx-text);
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .hint {
    font-size: 0.72rem;
    color: var(--zx-text-muted);
    margin: 0;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .ok-btn {
    background: var(--zx-accent);
    color: var(--zx-on-accent);
    border: 0;
    border-radius: 4px;
    font-size: 0.78rem;
    font-family: inherit;
    font-weight: 600;
    padding: 0.35rem 0.85rem;
    cursor: pointer;
  }
  .ok-btn:hover {
    filter: brightness(1.1);
  }
  .ok-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .ghost-btn {
    background: transparent;
    color: var(--zx-text-muted);
    border: 1px solid var(--zx-border);
    border-radius: 4px;
    font-size: 0.78rem;
    font-family: inherit;
    padding: 0.35rem 0.85rem;
    cursor: pointer;
  }
  .ghost-btn:hover {
    background: var(--zx-hover-bg);
    color: var(--zx-text);
  }
  .ghost-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
