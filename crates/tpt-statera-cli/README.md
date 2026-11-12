# tpt-statera-cli

The `statera` command. It runs the same library code as the crates it wraps, and adds no validation logic.

```sh
cargo run -p tpt-statera-cli -- validate --sender 1000 --recipient 0 --amount 500 --fee-bps 100
cargo run -p tpt-statera-cli -- validate --sender 1000 --recipient 0 --amount 1000 --fee-bps 100
cargo run -p tpt-statera-cli -- policy lint crates/tpt-statera-cli/tests/fixtures/policy_ok.toml
cargo run -p tpt-statera-cli -- simulate-fees --policy crates/tpt-statera-cli/tests/fixtures/policy_ok.toml 500 2000
cargo run -p tpt-statera-cli -- cert verify proofs/certificate.json --root .
```

## Commands

| Command | What it does | Exit 0 | Exit 1 |
| :--- | :--- | :--- | :--- |
| `validate` | Validates one transfer against a balance snapshot, with `validate_transfer_explained`. | Allow | Deny, with the reason and its code |
| `simulate-fees` | For each amount: the tier rate, the fee and how the fee is divided. | always | never (an error exits 2) |
| `policy lint` | Checks a policy file. Every section is checked and each problem is listed. | valid | any problem |
| `cert verify` | Checks a certificate against the files it names. Solver-free. | consistent | any failed check |

Exit code `2` means a usage, file or parse error. `validate` has no separate `explain` command: on a
denial it prints the reason, which is the explanation.

`cert verify` checks what `tpt-statera-proof` checks: the schema, the caveat, every input hash, the
proof-ID binding and that transcripts exist. It does **not** re-run the solver. Use `scripts/verify_proofs.py`
for that.

## Policy file

```toml
[rounding]
mode = "floor"          # only "floor" is permitted in this release
dust = "sender_retains" # "sender_retains", "treasury" or "burn"

[[tiers]]               # the first tier must start at 0; thresholds strictly increase
min_amount = 0
fee_rate_bps = 100

[[tiers]]
min_amount = 1000
fee_rate_bps = 50

[split]
treasury_bps = 6000     # treasury share plus every "others" share must sum to 10000

[[split.others]]
account = 7
bps = 4000
```

Unknown keys are refused, so a misspelt option fails the load rather than being ignored.
See `tests/fixtures/` for a valid file and files with problems.

## Testing

```sh
cargo test -p tpt-statera-cli
```
