# Build & serve ratride-web with live reload
web:
    wasm-pack build ratride-web --target web --out-dir pkg
    cd ratride-web && npx vite
