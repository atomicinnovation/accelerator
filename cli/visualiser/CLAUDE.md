# Visualiser

- `build:server:dev` builds the dev binary; release builds embed the frontend
  via the `embed-dist` feature.
- The frontend uses **Biome** (not ESLint/Prettier) for lint + format.
- The binary is distributed via GitHub Releases and downloaded on first use,
  verified against the signed `manifest.json` (SHA-256 + minisign, optional SLSA
  provenance).
