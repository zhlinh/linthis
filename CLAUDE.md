# Linthis Development Instructions

## Build & Install

Every time code is modified, rebuild and reinstall:

```bash
cargo build --release && cargo install --path . --force
```

This is **mandatory** after any code change to ensure the installed binary is up to date.
