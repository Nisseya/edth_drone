import { connect, type NatsConnection } from 'nats.ws'
import { useEffect, useState } from 'react'
import type { InterceptorReport, PlatformView, Threat, ThreatClassification } from './types'

export type ConnectionStatus = 'connecting' | 'connected' | 'offline'

const NATS_WS_URL = 'ws://127.0.0.1:8080'

/**
 * Connects to the NATS WebSocket listener and keeps the live picture:
 * ground-truth threats from `map.threats`, latest radar report per platform
 * from `platform.*.report`.
 */
export function useNats(url: string = NATS_WS_URL) {
  const [status, setStatus] = useState<ConnectionStatus>('connecting')
  const [threats, setThreats] = useState<Threat[]>([])
  const [platforms, setPlatforms] = useState<Map<string, PlatformView>>(new Map())
  // Operator picture: best classification known per track id, fused across
  // platform reports. A track stays out of this map until a platform resolves
  // it within its classification range.
  const [classifications, setClassifications] = useState<Map<string, ThreatClassification>>(
    new Map(),
  )

  useEffect(() => {
    let connection: NatsConnection | undefined
    let cancelled = false
    const decoder = new TextDecoder()

    ;(async () => {
      try {
        connection = await connect({
          servers: url,
          maxReconnectAttempts: -1,
          waitOnFirstConnect: true,
        })
      } catch {
        if (!cancelled) setStatus('offline')
        return
      }
      if (cancelled) {
        void connection.close()
        return
      }
      setStatus('connected')

      void (async () => {
        for await (const event of connection.status()) {
          if (cancelled) return
          if (event.type === 'disconnect') setStatus('connecting')
          if (event.type === 'reconnect') setStatus('connected')
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe('map.threats')) {
          const live = JSON.parse(decoder.decode(message.data)) as Threat[]
          setThreats(live)
          // Drop classifications for tracks that no longer exist.
          const liveIds = new Set(live.map((t) => t.id))
          setClassifications((previous) => {
            const next = new Map(previous)
            for (const id of next.keys()) if (!liveIds.has(id)) next.delete(id)
            return next
          })
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe('platform.*.report')) {
          const report = JSON.parse(decoder.decode(message.data)) as InterceptorReport
          setPlatforms((previous) => {
            const next = new Map(previous)
            next.set(report.platform_id, { report, lastSeen: Date.now() })
            return next
          })
          // Record any definitive classification (a closer platform resolves it).
          setClassifications((previous) => {
            const next = new Map(previous)
            for (const contact of report.threats) {
              if (contact.classification !== 'Unknown') {
                next.set(contact.id, contact.classification)
              }
            }
            return next
          })
        }
      })()
    })()

    return () => {
      cancelled = true
      void connection?.close()
    }
  }, [url])

  return { status, threats, platforms, classifications }
}
