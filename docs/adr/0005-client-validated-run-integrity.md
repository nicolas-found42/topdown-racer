# Client-validated Run integrity, no accounts

Runs are validated against checkpoint gates on the client; the server accepts and ranks them without re-simulation. With no accounts, a determined cheat can post fake times; we accept this for v1 rather than pay for server-side replay verification. Mitigation if trust ever breaks: Leaderboards are rebuildable, since any Run is a deterministic replay from its recorded inputs and its Track is reproducible from its Track Code.
