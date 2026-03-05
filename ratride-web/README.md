# ratride

WASM-based markdown slide presenter for the browser.

## Install

```
npm install ratride
```

## Usage

```js
import { run } from "ratride";

const instance = await run(markdownString, {
  parent: document.getElementById("app"),
  theme: "mocha",
  fontSize: 16,
});

// Cleanup
instance.destroy();
```

## Options

| Option     | Type          | Default         | Description          |
| ---------- | ------------- | --------------- | -------------------- |
| `parent`   | `HTMLElement`  | `document.body` | Container element    |
| `theme`    | `string`      | `"mocha"`       | Catppuccin theme     |
| `fontSize` | `number`      | `16`            | Base font size in px |

## License

MIT
