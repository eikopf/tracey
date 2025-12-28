# tracey

A CLI tool to measure spec coverage in Rust codebases.

## What it does

tracey parses Rust source files to find references to specification rules (in the format `[rule.id]` in comments) and compares them against a spec manifest to produce coverage reports.

This enables **traceability** between your spec documents and implementation code.

## Example

In your Rust code, reference spec rules in comments:

```rust
/// Allocates the next channel ID for this peer.
/// 
/// See [channel.id.parity] - initiators use odd IDs, acceptors use even.
/// See [channel.id.no-reuse] - IDs are never recycled.
fn allocate_channel_id(&mut self) -> u32 {
    let id = self.next_channel_id;
    self.next_channel_id += 2;  // Skip to next ID with same parity
    id
}
```

tracey will match these against your spec's rule manifest and tell you:
- Which rules are covered (referenced in code)
- Which rules are orphaned (never referenced)
- Which references are invalid (rule doesn't exist in spec)

## Configuration

Create `.config/tracey/config.kdl`:

```kdl
spec {
    name "rapace"
    rules_url "https://rapace.dev/_rules.json"
}
```

You can define multiple specs:

```kdl
spec {
    name "rapace"
    rules_url "https://rapace.dev/_rules.json"
}

spec {
    name "other-spec"
    rules_url "https://example.com/_rules.json"
    include "src/other/**/*.rs"
    exclude "src/other/tests/**"
}
```

## Usage

```bash
# Run coverage report
tracey

# Check mode (exit 1 if failing)
tracey --check --threshold 50

# Verbose output
tracey -v

# Custom config file
tracey -c path/to/config.kdl
```

## Spec Format

tracey expects your spec to publish a `_rules.json` manifest file with this structure:

```json
{
  "rules": {
    "channel.id.allocation": {
      "url": "/spec/#r-channel.id.allocation"
    },
    "channel.id.parity": {
      "url": "/spec/#r-channel.id.parity"
    }
  }
}
```

This is the format produced by [dodeca](https://github.com/bearcove/dodeca) when using rule identifiers (`r[rule.id]` syntax in markdown).

## License

MIT OR Apache-2.0
