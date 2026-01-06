list:
    just --list

lint:
    (cd crates/tracey/dashboard && pnpm exec tsgo --noEmit) && echo "TypeScript's okay"
    cargo check && echo "Rust's okay"
