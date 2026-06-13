import { connect } from 'nats.ws'
const nc = await connect({ servers: 'ws://127.0.0.1:8080' })
nc.publish('control.map.config', new TextEncoder().encode(JSON.stringify(
  {decoy_ratio:0.8, swarm_min:10, swarm_max:10, spawn_interval_s:8, zone_radius:6000, max_active:60})))
await nc.flush(); await nc.close(); console.log('config sent')
