<script setup lang="ts">
/**
 * Access token dialog
 *
 * Shows the owner token of this browser (the HttpOnly cookie) so it can be
 * used with the AI-DB CLI or the HTTP API – jobs created in the web interface
 * and from the command line then belong to the same owner.
 *
 * The token is only fetched while the dialog is open, masked by default and
 * dropped from memory again when the dialog closes.
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { createAccessToken, getAccessToken, TokenNotExportableError } from '../api/auth.ts'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

type State = 'idle' | 'loading' | 'ready' | 'none' | 'creating' | 'error' | 'not-exportable'

const dialogEl = ref<HTMLDialogElement | null>(null)
const state = ref<State>('idle')
const token = ref<string | null>(null)
const errorMessage = ref('')
const revealed = ref(false)
const copiedKey = ref<string | null>(null)
let copiedTimer: ReturnType<typeof setTimeout> | undefined
/** Bumped on every request and on close, so late responses are ignored. */
let requestId = 0

const origin = window.location.origin

// ── Display ───────────────────────────────────────────────────────────────────

/** Masked by default: only the last 4 characters stay visible. */
const displayToken = computed(() => {
  const t = token.value
  if (!t) return ''
  if (revealed.value) return t
  return `••••••••-••••-••••-••••-••••••••${t.slice(-4)}`
})

const cliSnippet = computed(() => `export AIDB_TOKEN=${displayToken.value}`)
const curlSnippet = computed(
    () => `curl -H "Authorization: Bearer ${displayToken.value}" ${origin}/api/jobs/`,
)

// ── Loading ───────────────────────────────────────────────────────────────────

async function load() {
  const id = ++requestId
  state.value = 'loading'
  errorMessage.value = ''
  try {
    const t = await getAccessToken()
    if (id !== requestId) return
    token.value = t
    state.value = t ? 'ready' : 'none'
  } catch (e) {
    if (id === requestId) handleError(e)
  }
}

async function create() {
  const id = ++requestId
  state.value = 'creating'
  errorMessage.value = ''
  try {
    const t = await createAccessToken()
    if (id !== requestId) return
    token.value = t
    state.value = 'ready'
  } catch (e) {
    if (id === requestId) handleError(e)
  }
}

function handleError(e: unknown) {
  token.value = null
  errorMessage.value = e instanceof Error ? e.message : 'Something went wrong'
  state.value = e instanceof TokenNotExportableError ? 'not-exportable' : 'error'
}

function reset() {
  requestId++
  token.value = null
  revealed.value = false
  copiedKey.value = null
  errorMessage.value = ''
  state.value = 'idle'
}

// ── Open / close ──────────────────────────────────────────────────────────────

function show() {
  const el = dialogEl.value
  if (el && !el.open) el.showModal()
  load()
}

function close() {
  const el = dialogEl.value
  if (el?.open) el.close() // fires the native "close" event -> onNativeClose
  else onNativeClose()
}

/** Esc, the close button and backdrop clicks all end up here. */
function onNativeClose() {
  reset()
  emit('close')
}

function onBackdropClick(event: MouseEvent) {
  // Clicks inside the panel have a child element as target
  if (event.target === dialogEl.value) close()
}

watch(
    () => props.open,
    (isOpen) => {
      if (isOpen) nextTick(show)
      else if (dialogEl.value?.open) dialogEl.value.close()
    },
)

onMounted(() => {
  if (props.open) show()
})

onBeforeUnmount(() => {
  if (copiedTimer) clearTimeout(copiedTimer)
})

// ── Copy & export ─────────────────────────────────────────────────────────────

/** Clipboard API with a fallback for non-secure contexts (plain HTTP). */
async function writeClipboard(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text)
      return true
    }
  } catch {
    /* fall through to the legacy path */
  }
  const area = document.createElement('textarea')
  area.value = text
  area.setAttribute('readonly', '')
  area.style.position = 'fixed'
  area.style.opacity = '0'
  // Inside the modal dialog: elements outside it are inert and cannot be selected
  ;(dialogEl.value ?? document.body).appendChild(area)
  area.select()
  let ok = false
  try {
    ok = document.execCommand('copy')
  } catch {
    ok = false
  }
  area.remove()
  return ok
}

async function copy(key: 'token' | 'cli' | 'curl') {
  const t = token.value
  if (!t) return
  const text = {
    token: t,
    cli: `export AIDB_TOKEN=${t}`,
    curl: `curl -H "Authorization: Bearer ${t}" ${origin}/api/jobs/`,
  }[key]

  if (await writeClipboard(text)) {
    copiedKey.value = key
    if (copiedTimer) clearTimeout(copiedTimer)
    copiedTimer = setTimeout(() => (copiedKey.value = null), 2000)
  } else {
    // Last resort: show the token so it can be selected manually
    revealed.value = true
    errorMessage.value = 'Copying is blocked by your browser – please select the token and copy it manually.'
  }
}

function downloadEnvFile() {
  const t = token.value
  if (!t) return
  const content =
      '# AI-DB access token – keep this file private\n' +
      `# Created ${new Date().toISOString()} for ${origin}\n` +
      `AIDB_TOKEN=${t}\n`
  const url = URL.createObjectURL(new Blob([content], { type: 'text/plain' }))
  const link = document.createElement('a')
  link.href = url
  link.download = 'aidb-token.env'
  link.click()
  URL.revokeObjectURL(url)
}
</script>

<template>
  <dialog
      ref="dialogEl"
      class="token-dialog"
      aria-labelledby="token-dialog-title"
      aria-describedby="token-dialog-intro"
      @close="onNativeClose"
      @click="onBackdropClick"
  >
    <div class="panel">
      <!-- Header -->
      <div class="panel-header">
        <div class="title-row">
          <span class="title-icon" aria-hidden="true">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="7.5" cy="15.5" r="5.5"/><path d="M21 2l-9.6 9.6"/><path d="M15.5 7.5l3 3L22 7l-3-3"/>
            </svg>
          </span>
          <h3 id="token-dialog-title">Access token</h3>
        </div>
        <button class="icon-btn" aria-label="Close" @click="close">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <p id="token-dialog-intro" class="intro">
        This token identifies the jobs created in this browser. Use it with the AI-DB CLI or the
        HTTP API to manage the same jobs from the command line or a pipeline.
      </p>

      <!-- Loading -->
      <div v-if="state === 'loading' || state === 'idle'" class="status-row">
        <span class="spinner" aria-hidden="true"></span>
        <span>Loading token…</span>
      </div>

      <!-- No identity yet -->
      <div v-else-if="state === 'none' || state === 'creating'" class="empty">
        <p>
          This browser has no token yet. It is created automatically with your first job –
          or create one now to start with the CLI first.
        </p>
        <button class="btn btn-primary" :disabled="state === 'creating'" @click="create">
          {{ state === 'creating' ? 'Creating…' : 'Create token' }}
        </button>
      </div>

      <!-- Identity not usable as API token -->
      <div v-else-if="state === 'not-exportable'" class="callout callout--error" role="alert">
        <strong>This browser's identity cannot be used as an API token.</strong>
        <span>{{ errorMessage }}</span>
      </div>

      <!-- Error -->
      <div v-else-if="state === 'error'" class="callout callout--error" role="alert">
        <strong>Could not load the token.</strong>
        <span>{{ errorMessage }}</span>
        <button class="btn btn-secondary btn-small" @click="load">Try again</button>
      </div>

      <!-- Token -->
      <template v-else-if="state === 'ready' && token">
        <span id="token-label" class="field-label">Your token</span>
        <div class="token-field">
          <code class="token-value" :class="{ masked: !revealed }" aria-labelledby="token-label">{{ displayToken }}</code>
          <button
              class="icon-btn"
              :aria-pressed="revealed"
              :aria-label="revealed ? 'Hide token' : 'Show token'"
              :title="revealed ? 'Hide token' : 'Show token'"
              @click="revealed = !revealed"
          >
            <svg v-if="!revealed" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>
            </svg>
            <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
              <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
              <line x1="1" y1="1" x2="23" y2="23"/>
            </svg>
          </button>
          <button class="copy-btn" :class="{ copied: copiedKey === 'token' }" @click="copy('token')">
            {{ copiedKey === 'token' ? 'Copied' : 'Copy' }}
          </button>
        </div>
        <p v-if="errorMessage" class="hint hint--error">{{ errorMessage }}</p>

        <div class="callout callout--warning">
          <strong>Keep it secret.</strong>
          <span>
            Anyone with this token can download, rename and delete your jobs. Saving a copy is
            also your backup: if you clear your browser data, you can still reach your jobs via
            the CLI or API with it.
          </span>
        </div>

        <h4 class="section-title">Use it</h4>

        <div class="snippet">
          <div class="snippet-header">
            <span>AI-DB CLI <span class="muted">(environment variable)</span></span>
            <button class="copy-btn copy-btn--small" :class="{ copied: copiedKey === 'cli' }" @click="copy('cli')">
              {{ copiedKey === 'cli' ? 'Copied' : 'Copy' }}
            </button>
          </div>
          <pre><code>{{ cliSnippet }}</code></pre>
        </div>

        <div class="snippet">
          <div class="snippet-header">
            <span>HTTP API <span class="muted">(curl)</span></span>
            <button class="copy-btn copy-btn--small" :class="{ copied: copiedKey === 'curl' }" @click="copy('curl')">
              {{ copiedKey === 'curl' ? 'Copied' : 'Copy' }}
            </button>
          </div>
          <pre><code>{{ curlSnippet }}</code></pre>
        </div>

        <div class="panel-footer">
          <button class="btn btn-secondary" @click="downloadEnvFile">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            Download .env file
          </button>
          <button class="btn btn-primary" @click="close">Done</button>
        </div>
      </template>

      <!-- Screen reader announcement for copy feedback -->
      <span class="sr-only" aria-live="polite">{{ copiedKey ? 'Copied to clipboard' : '' }}</span>
    </div>
  </dialog>
</template>

<style scoped>
.token-dialog {
  /* The global reset in base.css (`* { margin: 0 }`) removes the browser's
     default `margin: auto` that centres a modal dialog – restore it. */
  position: fixed;
  inset: 0;
  margin: auto;
  height: fit-content;
  width: min(560px, calc(100vw - 2rem));
  max-height: calc(100vh - 2rem);
  padding: 0;
  border: 1px solid var(--color-border);
  border-radius: 12px;
  background: var(--color-background);
  color: var(--color-text);
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.25);
}
.token-dialog::backdrop {
  background: rgba(0, 0, 0, 0.45);
}

.panel {
  padding: 1.25rem 1.5rem 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}
.title-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.title-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: hsla(160, 100%, 37%, 0.12);
  color: hsla(160, 100%, 37%, 1);
}
.panel-header h3 {
  margin: 0;
  font-size: 1.15rem;
  color: var(--color-heading);
}

.intro {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.5;
  opacity: 0.85;
}

.status-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.9rem;
  padding: 0.75rem 0;
}
.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--color-border);
  border-top-color: hsla(160, 100%, 37%, 1);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.75rem;
}
.empty p {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.5;
}

.field-label {
  font-size: 0.8rem;
  font-weight: 600;
  margin-bottom: -0.5rem;
}

.token-field {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.35rem 0.35rem 0.35rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-background-soft);
}
.token-value {
  flex: 1;
  min-width: 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem;
  overflow-x: auto;
  white-space: nowrap;
  user-select: all;
}
.token-value.masked {
  user-select: none;
  letter-spacing: 0.02em;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border: none;
  border-radius: 6px;
  background: none;
  color: var(--color-text);
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 0.15s, background 0.15s;
}
.icon-btn:hover {
  opacity: 1;
  background: var(--color-background-mute);
}

.copy-btn {
  flex-shrink: 0;
  min-width: 70px;
  padding: 0.4rem 0.8rem;
  font-size: 0.82rem;
  font-weight: 500;
  border: none;
  border-radius: 6px;
  background: hsla(160, 100%, 37%, 1);
  color: #fff;
  cursor: pointer;
  transition: background 0.15s;
}
.copy-btn:hover {
  background: hsla(160, 100%, 32%, 1);
}
.copy-btn.copied {
  background: hsla(160, 60%, 30%, 1);
}
.copy-btn--small {
  min-width: 58px;
  padding: 0.25rem 0.6rem;
  font-size: 0.75rem;
}

.hint {
  margin: -0.4rem 0 0;
  font-size: 0.8rem;
}
.hint--error {
  color: #f44336;
}

.callout {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.25rem;
  padding: 0.7rem 0.9rem;
  border-radius: 8px;
  font-size: 0.85rem;
  line-height: 1.5;
}
.callout--warning {
  background: hsla(38, 100%, 50%, 0.1);
  border: 1px solid hsla(38, 100%, 50%, 0.35);
}
.callout--error {
  background: hsla(0, 80%, 55%, 0.08);
  border: 1px solid hsla(0, 80%, 55%, 0.3);
}
.callout .btn {
  margin-top: 0.35rem;
}

.section-title {
  margin: 0.25rem 0 -0.3rem;
  font-size: 0.85rem;
  color: var(--color-heading);
}

.snippet {
  border: 1px solid var(--color-border);
  border-radius: 8px;
  overflow: hidden;
}
.snippet-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.35rem 0.4rem 0.35rem 0.75rem;
  font-size: 0.8rem;
  font-weight: 500;
  background: var(--color-background-soft);
  border-bottom: 1px solid var(--color-border);
}
.muted {
  font-weight: 400;
  opacity: 0.6;
}
.snippet pre {
  margin: 0;
  padding: 0.6rem 0.75rem;
  overflow-x: auto;
  font-size: 0.8rem;
}
.snippet code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  white-space: pre;
}

.panel-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
  margin-top: 0.25rem;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.6rem 1.1rem;
  font-size: 0.9rem;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
}
.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.btn-primary {
  background: hsla(160, 100%, 37%, 1);
  color: #fff;
}
.btn-primary:hover:not(:disabled) {
  background: hsla(160, 100%, 32%, 1);
}
.btn-secondary {
  background: transparent;
  color: hsla(160, 100%, 37%, 1);
  border: 1px solid hsla(160, 100%, 37%, 0.5);
}
.btn-secondary:hover {
  background: hsla(160, 100%, 37%, 0.08);
}
.btn-small {
  padding: 0.35rem 0.75rem;
  font-size: 0.8rem;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

@media (max-width: 480px) {
  .panel {
    padding: 1rem;
  }
  .panel-footer .btn {
    flex: 1;
    justify-content: center;
  }
}
</style>