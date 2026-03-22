# TonyTools-rs

TonyTools but blazingly fast!

## WASM Build

Run `make wasm`.

### Optimized WASM Build

To get an optimized version, run `make wasm-optimized`.

This requires dependencies listed in `package.json` to be installed. That can be done so with Yarn. If you haven't already, enable corepack globally for seamless yarn version management:

```
npm i -g corepack
corepack enable
```

Then just run `yarn` and it should take care of the rest.
