# tpt-statera-testkit

Test helpers for code that calls `tpt-statera-core`. Use it in your own tests to check your wrappers
against the same invariants the core crate is tested with.

- `Rng`: a seeded, dependency-free PRNG with boundary-biased values.
- `ideal_allows`: an exact-rational oracle for the Allow/Deny decision.
- `invariants::all`: checks an outcome for conservation, exact debit, direction and Floor rounding.
- `strategies` (feature `proptest`): proptest strategies for states and proposals.

```rust
use tpt_statera_core::validate_transfer;
use tpt_statera_testkit::{invariants, Rng};

let mut rng = Rng(42);
for _ in 0..10_000 {
    let (state, proposal) = (rng.state(), rng.proposal());
    let outcome = validate_transfer(state, proposal);
    invariants::all(state, proposal, outcome).unwrap();
}
```

Published only inside this workspace (`publish = false`).
