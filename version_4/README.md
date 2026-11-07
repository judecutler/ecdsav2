# version_4

This is our entry for Joel's ecdsafail-v2 challenge: an exact, complete, controlled secp256k1 point addition that uses no measurement. It costs 5,084,258 CCX at G on 2,347 wires. Joel's reference costs 7,209,419 CCX, so this is 29.5% cheaper.

The circuit is built on Joel's Python reference generator. The working code is on branch `version_4` of judecutler/ecdsafail-v2-reference. `version_4.patch` is the same `qec/` change as a diff against Joel's `main`.

## Verify

```sh
git clone -b version_4 https://github.com/judecutler/ecdsafail-v2-reference
cd ecdsafail-v2-reference
python3 scripts/accept.py          # full gate, about 5 minutes on 2 cores
python3 scripts/accept.py --quick  # about 90 seconds
python3 scripts/breakdown.py       # CCX per component
```

`accept_full.txt` is our full run: 13 pass, 0 fail, and I7.charge pending because the benchmark has not published its charge formula. Across 24 sampled addends the maximum T is 35,589,862, which is 14,876,127 below the reference.

## Files

- `PROGRESS.md` covers each change, where the cost goes now, and what we ruled out.
- `version_4.patch` is the generator change.
- `decisions.tsv` is the decision trail.
- `accept_full.txt` is the gate output.
