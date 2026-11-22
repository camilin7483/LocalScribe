import { useEffect, useRef, useState } from 'react'
import { errorText, request } from './api'
import type { Detail, Segment } from './types'

type Patch = Pick<Segment, 'id' | 'text' | 'speaker' | 'start_ms' | 'end_ms'>
type PendingEdit = Pick<Patch, 'id' | 'text' | 'speaker'> & Partial<Pick<Patch, 'start_ms' | 'end_ms'>>

function sameTiming(a: Patch, b: Patch) {
  return a.start_ms === b.start_ms && a.end_ms === b.end_ms
}

type Change = { before: Patch; after: Patch }

export function useEdits(detail: Detail) {
  const [segments, setSegments] = useState(detail.segments)
  const [state, setState] = useState<'saved' | 'saving' | 'unsaved'>('saved')
  const [error, setError] = useState('')
  const current = useRef(detail.segments)
  const revision = useRef(detail.job.revision)
  const pending = useRef(new Map<number, PendingEdit>())
  const requestInFlight = useRef<Promise<boolean> | null>(null)
  const timer = useRef<number | undefined>(undefined)
  const undoStack = useRef<Change[]>([])
  const redoStack = useRef<Change[]>([])
  const id = detail.job.id

  async function save(): Promise<boolean> {
    window.clearTimeout(timer.current)
    if (requestInFlight.current) return requestInFlight.current
    if (!pending.current.size) return true
    setState('saving')
    const operation = async () => {
      while (pending.current.size) {
        const [segmentId, patch] = pending.current.entries().next().value as [number, PendingEdit]
        pending.current.delete(segmentId)
        try {
          const response = await request<{ revision: number }>('/jobs/' + id + '/segments', {
            method: 'PATCH', body: JSON.stringify({ ...patch, revision: revision.current }),
          })
          revision.current = response.revision
        } catch (err) {
          pending.current.set(segmentId, { ...patch, ...pending.current.get(segmentId) })
          setError(errorText(err))
          setState('unsaved')
          return false
        }
      }
      setError('')
      setState('saved')
      return true
    }
    requestInFlight.current = operation().finally(() => { requestInFlight.current = null })
    return requestInFlight.current
  }

  useEffect(() => {
    const unload = (event: BeforeUnloadEvent) => {
      if (pending.current.size || requestInFlight.current) {
        event.preventDefault()
        event.returnValue = ''
      }
    }
    window.addEventListener('beforeunload', unload)
    return () => {
      window.removeEventListener('beforeunload', unload)
      window.clearTimeout(timer.current)
      if (pending.current.size) void save()
    }
    // A fresh editor instance is mounted for each transcript ID.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id])

  function apply(patch: Patch) {
    const previous = current.current.find((segment) => segment.id === patch.id)
    if (!previous) return
    const timingChanged = !sameTiming(previous, patch)
    current.current = current.current.map((segment) => segment.id === patch.id ? { ...segment, ...patch } : segment)
    if (timingChanged) current.current.sort((a, b) => a.start_ms - b.start_ms || a.id - b.id)
    setSegments(current.current)
    const { id, text, speaker, start_ms, end_ms } = patch
    const timing = timingChanged || pending.current.get(id)?.start_ms !== undefined ? { start_ms, end_ms } : {}
    pending.current.set(id, { id, text, speaker, ...timing })
    setState('saving')
    window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => void save(), 550)
  }

  function change(id: number, patch: Partial<Omit<Patch, 'id'>>) {
    const segment = current.current.find((item) => item.id === id)
    if (!segment) return
    const { text, speaker, start_ms, end_ms } = segment
    const before = { id, text, speaker, start_ms, end_ms }
    const after = { ...before, ...patch }
    if (before.text === after.text && before.speaker === after.speaker && sameTiming(before, after)) return
    const last = undoStack.current.at(-1)
    const newTyping = pending.current.has(id) && last?.after.id === id &&
      sameTiming(before, after) && sameTiming(last.before, last.after) &&
      ((patch.text !== undefined && last.before.speaker === last.after.speaker) ||
      (patch.speaker !== undefined && last.before.text === last.after.text))
    if (newTyping && last) last.after = after
    else undoStack.current = [...undoStack.current.slice(-99), { before, after }]
    redoStack.current = []
    apply(after)
  }

  function undo() {
    const edit = undoStack.current.pop()
    if (edit) { redoStack.current.push(edit); apply(edit.before) }
  }
  function redo() {
    const edit = redoStack.current.pop()
    if (edit) { undoStack.current.push(edit); apply(edit.after) }
  }

  return { segments, change, save, state, error, undo, redo, canUndo: !!undoStack.current.length, canRedo: !!redoStack.current.length }
}
