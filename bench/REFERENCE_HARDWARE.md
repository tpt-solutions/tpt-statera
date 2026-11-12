# bench/REFERENCE_HARDWARE.md — reference hardware (todo 1E.01)

**Status: FROZEN, PROVISIONAL.** Per the recommendation in `docs/ARCHITECTURE.md` AD-9, this
workstation is the reference host until a dedicated machine is provided. The settings below are the
frozen configuration. Any change to a frozen field requires a new record, and results taken under
the old record are not comparable.

| Field | Frozen value (read 2026-11-12) |
| :--- | :--- |
| CPU | Intel Core i7-12700 (12th Gen), 12 cores / 20 logical processors |
| Reported max clock | 2100 MHz (WMI `MaxClockSpeed`, the base clock). Boost is not pinned (see below) |
| RAM | 16 GB |
| GPU | Intel UHD Graphics 770 (not used by the benchmark) |
| OS | Microsoft Windows 11 Pro, 10.0.26200 (build 26200) |
| Power scheme | `75fb4723-d650-4cbc-a415-ff4076b309d1` ("Lenovo Default"), read with `powercfg -getactivescheme` |
| Rust toolchain | rustc 1.99.1 (`58cf8f9c3`, 2026-10-13), stable, host `x86_64-pc-windows-msvc`, `--release` profile |
| Solver | `tpt-telos` 0.2.0, git `23d53eb`, library path dependency `tpt-telos-verifier` (built-in Fourier–Motzkin, no Z3) |
| Benchmark crate | `bench/fm_netting`, outside the product workspace |
| Benchmark inputs | Constraint set built in code from the 10-party netting obligations (no RNG). Fixture values match `tpt-statera-core::fixtures` |
| Warm-up | 50 calls, discarded |
| Measured calls | 5 000 per run (`FM_CALLS=5000`), timed per call with `std::time::Instant` around the solver calls only |
| RNG seeding | Property and differential tests use fixed xorshift seeds (`tests/common/mod.rs`) |

## Limits that still apply

These are recorded so that a reader can judge the numbers. They do not change the frozen values.

- **Boost is not pinned.** The power scheme is the vendor default, so the CPU may boost above 2.1 GHz
  under load. Pinning requires an administrator change to the power plan, which is outside this repo.
- **Background load is not controlled.** No isolation procedure has been run. Other processes on this
  workstation can add tail latency, which is visible in p99.
- **Not a dedicated machine.** The host is a daily-use workstation. A third party reproducing the
  result on other hardware will see different absolute numbers.

## Benchmark result (todo 1E.02 / 1E.03)

Raw data: `bench/results/fm_netting_raw.csv` (5 000 rows). Summary: `bench/results/fm_netting_summary.txt`.

| Statistic | Value |
| :--- | ---: |
| median | 0.519 ms |
| p99 | 0.876 ms |
| mean | 0.530 ms |
| min / max | 0.452 ms / 0.967 ms |
| gate (median < 10 ms and p99 < 50 ms) | **PASS** |

**Caveat on what this measures.** Each call discharges the 11 obligations of the 10-party settlement
(conservation, and non-negativity for each party) over linear constraints with box-shaped solvency
bounds. That is the shape proven in `proofs/netting_10party.telos`. The benchmark does not parse the
`.telos` file, so it does not measure the telos front end. The result is a lower bound on latency for
this constraint shape, not evidence about other shapes. Settlements with coupled solvency bounds
(see `proofs/README.md`, the orthogonality limit) have not been measured.
