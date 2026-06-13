import { useEffect, useRef } from 'react'
import { STALE_AFTER_MS, type PlatformView, type Threat } from './types'

interface TacticalMapProps {
  threats: Threat[]
  platforms: PlatformView[]
}

const WORLD_RADIUS = 5_000

const COLOR_BG = '#04070b'
const COLOR_GRID = '#0b141d'
const COLOR_RING = '#12222e'
const COLOR_ASSET = '#39d5ff'
const COLOR_PLATFORM = '#35f0a8'
const COLOR_TEXT = '#56788c'

function threatColor(level: number): string {
  if (level >= 5) return '#ff3b4d'
  if (level >= 4) return '#ff6b35'
  if (level >= 3) return '#ffa02e'
  return '#ffd23e'
}

function shortId(id: string): string {
  return id.slice(0, 8)
}

export function TacticalMap({ threats, platforms }: TacticalMapProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  // Latest data, readable from the draw loop without re-creating it.
  const dataRef = useRef({ threats, platforms })
  dataRef.current = { threats, platforms }

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const context = canvas.getContext('2d')
    if (!context) return

    const view = { scale: 0, offsetX: 0, offsetY: 0 } // scale: px per metre
    let mouse: { x: number; y: number } | null = null
    let dragging: { x: number; y: number; offsetX: number; offsetY: number } | null = null
    let frame = 0

    const cssSize = () => ({
      w: canvas.clientWidth,
      h: canvas.clientHeight,
    })

    const fitScale = () => {
      const { w, h } = cssSize()
      return Math.min(w, h) / (2 * (WORLD_RADIUS + 600))
    }

    const resize = () => {
      const parent = canvas.parentElement
      if (!parent) return
      const dpr = window.devicePixelRatio || 1
      canvas.width = parent.clientWidth * dpr
      canvas.height = parent.clientHeight * dpr
    }
    resize()
    const observer = new ResizeObserver(resize)
    if (canvas.parentElement) observer.observe(canvas.parentElement)

    const toScreen = (wx: number, wy: number) => {
      const { w, h } = cssSize()
      return {
        x: w / 2 + view.offsetX + wx * view.scale,
        y: h / 2 + view.offsetY - wy * view.scale,
      }
    }

    const draw = (now: number) => {
      const dpr = window.devicePixelRatio || 1
      context.setTransform(dpr, 0, 0, dpr, 0, 0)
      const { w, h } = cssSize()
      if (view.scale === 0) view.scale = fitScale()

      context.fillStyle = COLOR_BG
      context.fillRect(0, 0, w, h)

      // Cartesian grid, one line every 1 km.
      context.strokeStyle = COLOR_GRID
      context.lineWidth = 1
      for (let m = -WORLD_RADIUS - 1000; m <= WORLD_RADIUS + 1000; m += 1000) {
        const v = toScreen(m, 0)
        const hl = toScreen(0, m)
        context.beginPath()
        context.moveTo(v.x, 0)
        context.lineTo(v.x, h)
        context.moveTo(0, hl.y)
        context.lineTo(w, hl.y)
        context.stroke()
      }

      // Distance rings around the defended asset.
      const center = toScreen(0, 0)
      context.fillStyle = COLOR_TEXT
      context.font = '10px ui-monospace, monospace'
      for (let r = 1000; r <= WORLD_RADIUS; r += 1000) {
        context.strokeStyle = COLOR_RING
        context.beginPath()
        context.arc(center.x, center.y, r * view.scale, 0, Math.PI * 2)
        context.stroke()
        context.fillText(`${r / 1000} km`, center.x + r * view.scale * 0.7071 + 4, center.y - r * view.scale * 0.7071 - 4)
      }

      const { threats, platforms } = dataRef.current
      const nowMs = Date.now()
      const trackedIds = new Set<string>()

      // Detection links + range bubbles, under the markers.
      for (const { report, lastSeen } of platforms) {
        const stale = nowMs - lastSeen > STALE_AFTER_MS
        const p = toScreen(report.position.x, report.position.y)

        context.strokeStyle = stale ? 'rgba(86, 120, 140, 0.25)' : 'rgba(53, 240, 168, 0.3)'
        context.fillStyle = stale ? 'rgba(86, 120, 140, 0.03)' : 'rgba(53, 240, 168, 0.05)'
        context.setLineDash(stale ? [4, 6] : [])
        context.beginPath()
        context.arc(p.x, p.y, report.range * view.scale, 0, Math.PI * 2)
        context.fill()
        context.stroke()
        context.setLineDash([])

        if (!stale) {
          context.strokeStyle = 'rgba(57, 213, 255, 0.35)'
          context.setLineDash([3, 5])
          for (const contact of report.threats) {
            trackedIds.add(contact.id)
            const c = toScreen(contact.position.x, contact.position.y)
            context.beginPath()
            context.moveTo(p.x, p.y)
            context.lineTo(c.x, c.y)
            context.stroke()
          }
          context.setLineDash([])
        }
      }

      // Platforms.
      for (const { report, lastSeen } of platforms) {
        const stale = nowMs - lastSeen > STALE_AFTER_MS
        const p = toScreen(report.position.x, report.position.y)
        context.globalAlpha = stale ? 0.4 : 1

        context.fillStyle = COLOR_PLATFORM
        context.beginPath()
        context.moveTo(p.x, p.y - 8)
        context.lineTo(p.x + 7, p.y + 6)
        context.lineTo(p.x - 7, p.y + 6)
        context.closePath()
        context.fill()

        context.fillStyle = COLOR_PLATFORM
        context.font = 'bold 11px ui-monospace, monospace'
        context.fillText(report.name.toUpperCase(), p.x + 10, p.y - 6)
        context.globalAlpha = 1
      }

      // Ground-truth threats.
      context.font = '10px ui-monospace, monospace'
      for (const threat of threats) {
        const t = toScreen(threat.position.x, threat.position.y)
        const color = threatColor(threat.threat_level)

        // Velocity vector: threats fly straight at the defended asset,
        // drawn as a 10 s projection of their course.
        const distance = Math.hypot(threat.position.x, threat.position.y)
        if (distance > 1) {
          const proj = 10 * threat.speed * view.scale
          context.strokeStyle = color
          context.globalAlpha = 0.7
          context.beginPath()
          context.moveTo(t.x, t.y)
          context.lineTo(
            t.x - (threat.position.x / distance) * proj,
            t.y + (threat.position.y / distance) * proj,
          )
          context.stroke()
          context.globalAlpha = 1
        }

        context.shadowColor = color
        context.shadowBlur = 12
        context.fillStyle = color
        context.beginPath()
        context.arc(t.x, t.y, 3 + threat.threat_level * 0.7, 0, Math.PI * 2)
        context.fill()
        context.shadowBlur = 0

        if (trackedIds.has(threat.id)) {
          // Tracked by at least one radar: targeting brackets.
          const s = 9 + threat.threat_level
          context.strokeStyle = COLOR_ASSET
          context.lineWidth = 1.5
          context.beginPath()
          for (const [dx, dy] of [[-1, -1], [1, -1], [1, 1], [-1, 1]] as const) {
            context.moveTo(t.x + dx * s, t.y + dy * s - dy * 5)
            context.lineTo(t.x + dx * s, t.y + dy * s)
            context.lineTo(t.x + dx * s - dx * 5, t.y + dy * s)
          }
          context.stroke()
          context.lineWidth = 1
        }

        context.fillStyle = COLOR_TEXT
        context.fillText(shortId(threat.id), t.x + 12, t.y + 4)
      }

      // Defended asset: diamond + sonar pulse.
      const pulse = ((now / 1500) % 1) * 40
      context.strokeStyle = `rgba(57, 213, 255, ${1 - pulse / 40})`
      context.beginPath()
      context.arc(center.x, center.y, 8 + pulse, 0, Math.PI * 2)
      context.stroke()

      context.fillStyle = COLOR_ASSET
      context.beginPath()
      context.moveTo(center.x, center.y - 8)
      context.lineTo(center.x + 8, center.y)
      context.lineTo(center.x, center.y + 8)
      context.lineTo(center.x - 8, center.y)
      context.closePath()
      context.fill()
      context.fillStyle = COLOR_ASSET
      context.font = 'bold 10px ui-monospace, monospace'
      context.fillText('DEFENDED ASSET', center.x + 12, center.y + 14)

      // Scale bar.
      context.strokeStyle = COLOR_TEXT
      context.fillStyle = COLOR_TEXT
      context.beginPath()
      context.moveTo(16, h - 20)
      context.lineTo(16 + 1000 * view.scale, h - 20)
      context.stroke()
      context.font = '10px ui-monospace, monospace'
      context.fillText('1 km', 16, h - 26)
      context.fillText('SCROLL: ZOOM — DRAG: PAN', w - 170, h - 14)

      drawTooltip(w)
    }

    const drawTooltip = (w: number) => {
      if (!mouse) return
      const { threats, platforms } = dataRef.current

      let lines: string[] | null = null
      let best = 16 // px hit radius

      for (const threat of threats) {
        const t = toScreen(threat.position.x, threat.position.y)
        const d = Math.hypot(t.x - mouse.x, t.y - mouse.y)
        if (d < best) {
          best = d
          lines = [
            `HOSTILE ${shortId(threat.id)}`,
            `LVL ${threat.threat_level}  ${threat.speed.toFixed(0)} m/s`,
            `DIST ${(Math.hypot(threat.position.x, threat.position.y) / 1000).toFixed(2)} km`,
          ]
        }
      }
      for (const { report } of platforms) {
        const p = toScreen(report.position.x, report.position.y)
        const d = Math.hypot(p.x - mouse.x, p.y - mouse.y)
        if (d < best) {
          best = d
          lines = [
            `PLATFORM ${report.name.toUpperCase()}`,
            `INTERCEPTORS ${report.interceptors_remaining}`,
            `RANGE ${(report.range / 1000).toFixed(1)} km — ${report.threats.length} contact(s)`,
          ]
        }
      }

      if (!lines) return
      const boxW = 200
      const boxH = 14 * lines.length + 12
      const x = Math.min(mouse.x + 14, w - boxW - 8)
      const y = mouse.y - boxH - 10
      context.fillStyle = 'rgba(4, 11, 16, 0.92)'
      context.strokeStyle = COLOR_ASSET
      context.fillRect(x, y, boxW, boxH)
      context.strokeRect(x, y, boxW, boxH)
      context.fillStyle = '#c9d8e2'
      context.font = '11px ui-monospace, monospace'
      lines.forEach((line, i) => context.fillText(line, x + 8, y + 17 + i * 14))
    }

    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      const factor = event.deltaY < 0 ? 1.15 : 1 / 1.15
      const next = Math.min(Math.max(view.scale * factor, fitScale() * 0.3), fitScale() * 40)
      const { w, h } = cssSize()
      // Keep the world point under the cursor fixed while zooming.
      const wx = (event.offsetX - w / 2 - view.offsetX) / view.scale
      const wy = (h / 2 + view.offsetY - event.offsetY) / view.scale
      view.scale = next
      view.offsetX = event.offsetX - w / 2 - wx * view.scale
      view.offsetY = event.offsetY - h / 2 + wy * view.scale
    }
    const onMouseDown = (event: MouseEvent) => {
      dragging = { x: event.offsetX, y: event.offsetY, offsetX: view.offsetX, offsetY: view.offsetY }
    }
    const onMouseMove = (event: MouseEvent) => {
      mouse = { x: event.offsetX, y: event.offsetY }
      if (dragging) {
        view.offsetX = dragging.offsetX + event.offsetX - dragging.x
        view.offsetY = dragging.offsetY + event.offsetY - dragging.y
      }
    }
    const onMouseUp = () => {
      dragging = null
    }
    const onMouseLeave = () => {
      mouse = null
      dragging = null
    }

    canvas.addEventListener('wheel', onWheel, { passive: false })
    canvas.addEventListener('mousedown', onMouseDown)
    canvas.addEventListener('mousemove', onMouseMove)
    canvas.addEventListener('mouseup', onMouseUp)
    canvas.addEventListener('mouseleave', onMouseLeave)

    const loop = (now: number) => {
      draw(now)
      frame = requestAnimationFrame(loop)
    }
    frame = requestAnimationFrame(loop)

    return () => {
      cancelAnimationFrame(frame)
      observer.disconnect()
      canvas.removeEventListener('wheel', onWheel)
      canvas.removeEventListener('mousedown', onMouseDown)
      canvas.removeEventListener('mousemove', onMouseMove)
      canvas.removeEventListener('mouseup', onMouseUp)
      canvas.removeEventListener('mouseleave', onMouseLeave)
    }
  }, [])

  return <canvas ref={canvasRef} className="block h-full w-full cursor-crosshair" />
}
