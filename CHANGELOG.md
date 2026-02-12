# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-12

### Added

#### Core Features
- **P2P Node Implementation** - Full iroh-based peer-to-peer networking
  - QUIC transport protocol with NAT traversal
  - Node discovery and connection management
  - Gossip protocol for pub/sub messaging

- **Model Inference Engine** (`src/inference.rs`)
  - Tensor snapshot and sparse update support
  - Top-K sparse updates with residual feedback
  - Model hash and dimension output
  - Memory pressure detection with auto-adjusting Top-K

- **Topology Management** (`src/topology.rs`)
  - Geo + embedding dual-metric scoring
  - Primary neighbors and backup pool
  - Failover and unreachable marking
  - Device-capability-based neighbor count adjustment

- **Device Adaptation** (`src/device.rs`)
  - Device capability detection (memory, CPU, network, battery)
  - Adaptive configuration based on device capabilities
  - Battery-aware scheduling
  - Network type detection (WiFi/4G/5G)

#### Web3 Integration
- **Dual Signature Support**
  - Ethereum (k256) signatures
  - Solana (ed25519) signatures
- **Stake/Reputation System**
- **Heartbeat, sparse, and dense message signing**

#### Privacy Protection (`src/privacy/`)
- Traffic obfuscation with random padding
- IP hiding via relay network
- Identity protection with periodic NodeId rotation
- Privacy-performance balance engine

#### Desktop Application
- **Tauri Desktop App** (v0.1.1)
  - Modern Web UI with P2P management
  - Real-time connection status monitoring
  - Node ID display and management

#### Additional Features
- Model persistence with checkpoint saving/loading
- PyTorch model conversion tools
- Multi-node testing scripts
- Android and iOS FFI integration
- P2P model distribution with sharding

### Known Issues

- Web3 staking system requires mainnet deployment
- Privacy protection advanced features need further testing
- Mobile integration verification pending

### Recommended for MVP (v0.1.0)

Include:
- P2P node startup and connection
- Model inference (128-dim)
- Topology neighbor management
- Tauri desktop application

Postpone to v0.2.0:
- Web3 staking system
- Advanced privacy features
- Mobile integration verification

---

## [Unreleased]

### Planned for v0.2.0
- Enhanced Web3 integration with mainnet
- Advanced privacy protection features
- Mobile platform verification (Android/iOS)
- Performance optimizations
