# DynoWatt ⚡

**DynoWatt** is a fast, modern desktop analytics platform built for data-driven cyclists and coaches. Designed as an ergonomic, local-first alternative to Golden Cheetah, it delivers deep power-duration modeling, granular 1Hz telemetry inspection, and physiological metrics ($CP$, $W'$ balance, MMP) inside a clean, responsive interface.

## Key Features

- **Fast .FIT Telemetry:** High-performance local parsing and synchronized multi-metric charts (Power, HR, Cadence, Elevation).
- **Physiological Modeling:** Power Duration Curves (PDC), Critical Power ($CP$), and real-time $W'$ balance tracking.
- **Modern & Local-First:** Full control over your training data with zero cloud lock-in and a lightweight desktop footprint.

## Tech Stack

- **Runtime & Shell:** [Tauri](https://tauri.app/) — Native desktop performance with minimal memory usage and small binary size.
- **Core Engine & Backend:** **Rust** — High-throughput 1Hz `.FIT` file decoding, multi-threaded telemetry processing, and numerical modeling.
- **Frontend:** **Vue 3** + **TypeScript** — Reactive, modular UI paired with Tailwind CSS for an ergonomic dashboard experience.
- **Data Layer:** **SQLite** — Embedded, zero-configuration local database for indexed activity storage and fast historical querying.
- **Data Visualization:** High-performance canvas-based time-series charting designed for smooth panning and zooming across long endurance rides.
