# EDTH Drone — Coordination distribuée d'intercepteurs

Système de défense aérienne temps réel : plusieurs plateformes d'interception partagent leurs détections, et un orchestrateur central fusionne les pistes, priorise les menaces et assigne les cibles de façon optimale — pour vaincre les attaques saturantes qu'une défense mono-intercepteur ne peut pas absorber.

## Le problème

Les menaces modernes arrivent de toutes les directions à la fois. Une attaque coordonnée (4 drones simultanés sur des vecteurs différents + essaims de leurres) dépasse la capacité d'un intercepteur seul, et une coordination manuelle prend 15–20 secondes par décision d'engagement — bien trop lent.

Le système doit :

- **Fusionner les capteurs distribués** (radar, optique) des 3 plateformes d'interception en une image de situation unifiée
- **Prioriser les menaces automatiquement** (vitesse, proximité, dangerosité)
- **Assigner chaque intercepteur à sa cible optimale** selon la portée, le temps de rechargement et la probabilité d'engagement
- **Suivre l'état du réseau** : munitions restantes et statut de chaque intercepteur
- **Recalculer les assignations toutes les 1–2 s** à mesure que les menaces se déplacent
- **Émettre des recommandations de tir** avec score de confiance pour chaque intercepteur

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Interceptor 1│     │ Interceptor 2│     │ Interceptor 3│
│ (capteurs +  │     │ (capteurs +  │     │ (capteurs +  │
│  effecteur)  │     │  effecteur)  │     │  effecteur)  │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │  rapports (position, menaces détectées, munitions)
       ▼                    ▼                    ▼
╔══════════════════════ NATS (broker pub/sub) ══════════════════════╗
╚════════════════════════════════╤═══════════════════════════════════╝
                                 ▼
                      ┌─────────────────────┐
                      │    Orchestrator     │
                      │ fusion des pistes   │
                      │ priorisation        │
                      │ assignation optimale│
                      └──────────┬──────────┘
                                 │  ordres (Intercept / MoveTo / Idle)
                                 ▼
                       retour aux intercepteurs
```

- Chaque **intercepteur** publie périodiquement un `InterceptorReport` (sa position, ses menaces détectées, ses munitions) sur NATS.
- L'**orchestrateur** s'abonne à ces rapports, maintient l'état global (`OrchestratorState`), et à chaque `tick` fusionne les détections puis recalcule les assignations.
- Les **ordres** (`InterceptorOrder::Intercept(threat_id)`, `MoveTo`, `Idle`) ne sont republiés que s'ils changent, pour minimiser le trafic.

## Structure du dépôt

```
edth_drone/
├── src/                  # binaire principal : l'orchestrateur
│   ├── main.rs
│   ├── orchestrator.rs   # OrchestratorState : fusion + assignation
│   └── models.rs         # Position, DetectedThreat, Interceptor, ordres, rapports
├── interceptor/          # binaire intercepteur (simulation d'une plateforme)
└── common/               # types partagés sérialisables (serde)
```

Workspace Cargo (édition 2024) avec deux binaires (`edth_2026`, `interceptor`) et une lib partagée (`common`).

## Stack

- **Rust** (tokio pour l'async, futures)
- **NATS** ([`async-nats`](https://crates.io/crates/async-nats)) comme broker publish-subscribe
- **serde / serde_json** pour la sérialisation des messages

## Lancer le projet

Prérequis : Rust (édition 2024) et un serveur NATS.

```bash
# 1. Démarrer NATS (ex. via Docker)
docker run -p 4222:4222 nats:latest

# 2. Lancer l'orchestrateur
cargo run

# 3. Lancer un ou plusieurs intercepteurs
cargo run -p interceptor
```

## État d'avancement

- [x] Modèles de données (menaces, intercepteurs, ordres, rapports)
- [x] Boucle d'orchestration : fusion des rapports + assignation (heuristique : menace de plus haut niveau)
- [ ] Crate `common` : extraire les modèles partagés sérialisables
- [ ] Transport NATS entre intercepteurs et orchestrateur
- [ ] Simulation de capteurs côté intercepteur
- [ ] Assignation optimale (algorithme hongrois / max-flow, contraintes de portée et munitions)
- [ ] Scores de confiance sur les recommandations de tir
- [ ] Re-tasking dynamique en cours d'engagement
