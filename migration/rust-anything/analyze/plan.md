# Rust migration plan: v2 (Python)

1 packages, 0 source files (0 lines), 0 test files, 0 UI files, grouped into 0 units.

| Ready | Caution | Blocked | Keep in source language |
| ---: | ---: | ---: | ---: |
| 0 | 0 | 0 | 0 |

## Start here

No unit is a safe first port yet. See the blocked units and dependencies below.

## Order by package

Packages come after the workspace packages they use. Inside a package, port by wave: a unit's imports are in earlier waves.

## Risk guide


## Whole app

Shape: **library**.

| Entry point | Kind | Found from |
| --- | --- | --- |

### Every dependency

None.

### Pieces, in port order


### Decisions for the user

**D1** (shape): The app is a library, entered through . Port every entry point, in that shape?

> Yes: the Rust crate's public API becomes the library's front door

