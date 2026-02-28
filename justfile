# Build WASM package
build-wasm:
    cd ratride-web && npm run build

# Serve demo site with live reload
web: build-wasm
    cd docs && npm install && npx vite

# Pack npm package (dry-run)
pack:
    cd ratride-web && npm pack --dry-run

# Release dry-run
release-dry level:
    cargo release {{level}}
    cd ratride-web && npm publish --dry-run

# Release cargo crate + npm package
release level:
    cargo release {{level}} --execute
    cd ratride-web && npm version {{level}} && npm publish
