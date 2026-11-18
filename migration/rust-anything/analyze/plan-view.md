# Rust migration plan: analyze-view (Python)

1 packages, 3 source files (727 lines), 0 test files, 0 UI files, grouped into 1 units.

| Ready | Caution | Blocked | Keep in source language |
| ---: | ---: | ---: | ---: |
| 0 | 1 | 0 | 0 |

## Start here

Best first ports: safe to port (readiness) and worth porting (loops and string work, tests, size).

| # | Unit | Lines | Priority | Readiness | Tests | Bridge | Main risks |
| ---: | --- | ---: | ---: | ---: | ---: | --- | --- |
| 1 | `factory/bin/` | 727 | 30 | 18 | 0 | PyO3 | Hash iteration order (19); String offsets and slicing (13); Regex features the `regex` crate lacks (1) |

### `factory/bin/`

727 lines in 3 files, wave 0, priority 30, readiness 18, bridge **PyO3**.

- Leaf: imports no other unit.
- No tests found: write characterization tests against the original code first.
- Hash iteration order x19: `factory/bin/job.py:42`, `factory/bin/job.py:43`, `factory/bin/job.py:44`
- String offsets and slicing x13: `factory/bin/gate.py:73`, `factory/bin/gate.py:77`, `factory/bin/gate.py:102`
- Regex features the `regex` crate lacks x1: `factory/bin/job.py:169`
- Regexes built at runtime x1: `factory/bin/job.py:169`
- Dynamic typing x51: `factory/bin/gate.py:29`, `factory/bin/gate.py:34`, `factory/bin/gate.py:41`
- Division and modulo x3: `factory/bin/job.py:665`, `factory/bin/job.py:666`, `factory/bin/job.py:666`
- Class inheritance x1: `factory/bin/job.py:59`
- Exceptions x5: `factory/bin/job.py:75`, `factory/bin/job.py:336`, `factory/bin/job.py:521`
- Number semantics x4: `factory/bin/job.py:134`, `factory/bin/job.py:169`, `factory/bin/job.py:505`

## Order by package

Packages come after the workspace packages they use. Inside a package, port by wave: a unit's imports are in earlier waves.

### 1. analyze-view (`.`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `factory/bin/` | caution | 727 | 18 | PyO3 | Hash iteration order (19); String offsets and slicing (13) |

## Risk guide

- **String offsets and slicing** (High): JS indexes strings in UTF-16 code units, Rust in UTF-8 bytes. Offsets that cross the boundary must be converted, or non-ASCII text gets wrong ranges silently.
- **Regex features the `regex` crate lacks** (High): Lookaround, backreferences and the `y` flag are not in the `regex` crate. Use `fancy-regex` for those patterns, or rewrite them, and test them on non-ASCII text: `\w`, `\b` and case-insensitive matching differ between engines even where a pattern compiles.
- **Regexes built at runtime** (High): Patterns come from data or config in JS syntax. Rust regex syntax differs, so user-visible behavior can change. Pin it with tests on real patterns.
- **Dynamic typing** (Medium): Values whose shape is checked at runtime need an explicit Rust enum or `serde_json::Value`.
- **Exceptions** (Low): Thrown errors become `Result`. Errors crossing the bridge must map back to the same JS error shape.
- **Class inheritance** (Medium): Class hierarchies become traits plus composition; `instanceof` checks become enums.
- **Number semantics** (Low): JS numbers are f64; bitwise ops truncate to i32; parseInt is lenient. Pick Rust integer types deliberately.
- **Division and modulo** (Medium): Python `//` and `%` floor toward negative infinity; Rust `/` and `%` truncate: use `div_euclid`/`rem_euclid` or explicit floor math. C division by zero is undefined.
- **Hash iteration order** (High): Set and map iteration order comes from the source language's hash function. A port cannot reproduce it: sort before output, or make the order part of the contract.

## Whole app

Shape: **cli**.

| Entry point | Kind | Found from |
| --- | --- | --- |
| `factory/bin/gate.py` | cli | `if __name__ == "__main__"` at factory/bin/gate.py:120 |
| `factory/bin/job.py` | cli | `if __name__ == "__main__"` at factory/bin/job.py:717 |
| `factory/bin/route.py` | cli | `if __name__ == "__main__"` at factory/bin/route.py:50 |

### Every dependency

None.

### Pieces, in port order

- wave 0: `factory/bin/` (727 lines)

### Decisions for the user

**D1** (shape): The app is a cli, entered through factory/bin/gate.py, factory/bin/job.py, factory/bin/route.py. Port every entry point, in that shape?

> Yes: a clap front door (`app front-door`) takes every command over first

