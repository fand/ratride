# Build WASM package
build-wasm:
    cd ratride-web && npm run build

# Watch Rust sources and rebuild WASM
watch:
    cargo watch -w ratride/src -w ratride-web/src -s "cd ratride-web && npm run build"

# Serve docs with Vite dev server
_vite:
    cd docs && npm run dev

# Serve docs site with Vite + WASM watch
web:
    just watch & just _vite & wait

# Build docs for deployment
build-docs:
    cargo run -- docs/slides.md --export dist

# Pack npm package (dry-run)
pack:
    cd ratride-web && npm pack --dry-run

# Release dry-run
release-dry level:
    cargo release {{ level }}
    cd ratride-web && npm publish --dry-run

# Release cargo crate + npm package
release level:
    cargo release {{ level }} --execute
    cd ratride-web && npm version {{ level }} && npm publish
