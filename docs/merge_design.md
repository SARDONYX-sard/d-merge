# Merge Issues

## Separation of Path and Range

- Difficult to construct a HashMap:
  - Multiple `Range` objects may exist within a single array.
- HashMap (key) → Retrieve identical patch `Path`s.

### Path Specification Issues

- The current `Path` includes array indices as `Range`, but only the final segment is treated as a `Range` type.
- Consider revising the patch creation specification:
  - Support for parallelization.
- Remove `Path` from JSON Patch (since it already exists as a key in the HashMap).

## Merge Strategy

1. Sort patches by priority.
2. Resolve conflicts **in parallel** within each group.
3. Resolve conflicts **sequentially** using the winning patches from each group.
