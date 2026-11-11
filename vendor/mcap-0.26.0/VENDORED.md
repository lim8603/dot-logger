# Vendored `mcap` 0.26.0

- **Source:** crates.io `mcap` 0.26.0 (<https://github.com/foxglove/mcap>), `src/` and `README.md` as published.
- **License:** MIT (crate manifest `license = "MIT"`). The crates.io package carries no LICENSE file; the upstream license text is to be added in the pre-publication license review (backlog T2).
- **Manifest:** the published `Cargo.toml` without its example/test/bench targets and dev-dependencies (those files are not vendored). Everything else is unchanged.
- **Patch (dot-logger ADR-010 amendment, 2026-11-11, EV-068):** `src/write.rs` `ChunkWriter` — for an uncompressed chunk the compressed-bytes CRC layer is not created, and `finish` reuses the uncompressed CRC as the compressed CRC for the data section. Upstream hashed the same bytes twice. Output bytes, including every CRC value, are identical.
- **Check:** upstream tests on the patched full package gave the same results as the original (lib 130 Pass; 7 integration tests fail identically in both for fixture files absent from the package).
- **Security:** `cargo audit` does not check path packages, so MCAP advisories must be checked by hand (GitHub/RustSec for `mcap`) when dependencies are reviewed. Drop this patch once upstream hashes uncompressed chunks once.
