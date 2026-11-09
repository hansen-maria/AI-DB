/**
 * Thin wrapper around Plausible custom events.
 *
 * The tracker itself is loaded in index.html. All calls are fire-and-forget and
 * must never break the app (ad-blockers, script not loaded yet, …).
 *
 * Every event name used here has to be registered as a "Custom event" goal in
 * Plausible (Site Settings → Goals), and every prop key as a Custom Property,
 * otherwise it won't show up in the dashboard.
 *
 * Privacy: only coarse, bucketed values are sent – never sequence IDs,
 * job IDs, filenames or sequence content.
 */

type PlausibleProps = Record<string, string | number | boolean>

interface PlausibleOptions {
  props?: PlausibleProps
  callback?: () => void
}

declare global {
  interface Window {
    plausible?: (event: string, options?: PlausibleOptions) => void
  }
}

export function track(event: string, props?: PlausibleProps): void {
  try {
    window.plausible?.(event, props ? { props } : undefined)
  } catch {
    /* analytics must never break the app */
  }
}

/** Buckets counts so that individual jobs can't be fingerprinted. */
export function bucket(n: number): string {
  if (n <= 1)    return '1'
  if (n <= 10)   return '2-10'
  if (n <= 100)  return '11-100'
  if (n <= 1000) return '101-1000'
  if (n <= 10000) return '1001-10000'
  return '>10000'
}

/** Buckets a 0..1 ratio into 10 %-steps, e.g. 0.83 → "80-90%". */
export function percentBucket(ratio: number): string {
  if (!Number.isFinite(ratio)) return 'n/a'
  const p = Math.min(100, Math.max(0, Math.floor(ratio * 10) * 10))
  return p === 100 ? '100%' : `${p}-${p + 10}%`
}

// ── Domain events ────────────────────────────────────────────────────────────

export type SubmitSource = 'file' | 'paste' | 'example'

export const trackJobSubmitted = (source: SubmitSource, nSeqs: number) =>
  track('Job submitted', { source, n_seqs: bucket(nSeqs) })

/** Only fired when a job finishes while the user is watching (polling transition). */
export const trackJobFinished = (status: 'completed' | 'failed', nSeqs: number, matches: number) =>
  track('Job finished', {
    status,
    n_seqs: bucket(nSeqs),
    match_rate: nSeqs > 0 ? percentBucket(matches / nSeqs) : 'n/a',
  })

export const trackForwardToBakta = (workflow: string, nSeqs: number, autoSave: boolean) =>
  track('Forward to Bakta', { workflow, n_seqs: bucket(nSeqs), auto_save: autoSave })

export const trackForwardToPsos = (mode: 'api' | 'open' | 'fasta', nSeqs: number) =>
  track('Forward to Psos', { mode, n_seqs: bucket(nSeqs) })

export const trackAutoSaveToggle = (enabled: boolean) =>
  track(enabled ? 'Auto-save opt-in' : 'Auto-save opt-out')

export type ExportKind = 'results' | 'filtered' | 'stats' | 'psos'

/** Blob downloads are invisible to Plausible's automatic file-download tracking. */
export const trackExport = (kind: ExportKind, format: string) =>
  track('Export', { kind, format })

export const trackNotFound = (path: string) =>
  track('404', { path })
