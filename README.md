# Alkane Monkey

monkey banana

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled WASM binary will be available in `target/wasm32-unknown-unknown/release/alkanes_monkey.wasm`. 

## Deployment

```bash
oyl alkane new-contract -c ./target/alkanes/wasm32-unknown-unknown/release/alkanes_monkey.wasm -data 1,0 -p oylnet
```

## Tracing

```bash
oyl provider alkanes --method trace -params '{"txid":"6f028f97a67f74ffedbc7daabe0ae01c43f17eebcad1721cd5b0eebac61bb9da", "vout":5}' -p oylnet
``` 

