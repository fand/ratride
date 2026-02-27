# Build WASM package
build-wasm:
    cd ratride-web && npm run build

# Serve demo site with live reload
web: build-wasm
    cd docs && npm install && npx vite

# Pack npm package (dry-run)
pack:
    cd ratride-web && npm pack --dry-run
