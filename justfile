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

# Get current version from Cargo.toml
_version:
    @cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="ratride") | .version'

# Release dry-run
release-dry level:
    cargo release version {{ level }}
    @echo "npm: would bump ratride-web to {{ level }}"

# Release cargo crate + npm package
release level:
    cargo release version {{ level }} --execute --no-confirm
    cd ratride-web && npm version {{ level }} --no-git-tag-version
    git add -A
    git commit -m "chore: release v$(just _version)"
    git tag "v$(just _version)"
    cargo publish -p ratride
    cd ratride-web && npm publish
    git push && git push --tags
