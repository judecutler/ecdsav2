# version_4 progress

## Goal

Build the cheapest circuit we can for Joel's ecdsafail-v2 challenge: exact, complete, controlled point addition on secp256k1, measurement-free, gates X/CX/CCX/SWAP only, at most 4096 wires. The work lives on branch `version_4` of our fork, judecutler/ecdsafail-v2-reference. `version_4.patch` here is the same change as a diff against Joel's reference.

## Result

| Stage | CCX at G | Wires | Change |
|---|---|---|---|
| Joel's reference | 7,209,419 | 2,311 | |
| Euclid compare (our submitted PR) | 6,421,963 | 2,311 | -10.9% |
| Integer square folded by 2^n - q | 5,736,546 | 2,311 | -10.7% |
| Fused Bezout double-and-add | 5,084,258 | 2,347 | -11.4% |

Total so far is -29.5% against the reference, at +36 wires.

Where the CCX go now, per point addition at G:

| Component | CCX | Share |
|---|---|---|
| Euclid walk, 4 passes of 512 steps | 2,364,416 | 46.5% |
| Bezout replay, 2 passes of 512 steps | about 2,477,600 | 48.7% |
| lambda^2 subtract | 224,663 | 4.4% |
| Everything else | about 17,600 | 0.3% |

## The three changes

**Euclid compare.** Each Euclid step wrote its swap bit by comparing into a temporary, ANDing with the parity bit and comparing again. `gt_into` already takes a control, so one controlled comparison writes the bit. Each step drops from 8w + 1 to 6w + 1 CCX.

**Integer square.** The lambda^2 subtract used a modular multiplier. It now builds lambda^2 as a 2n-bit integer from symmetric partial products, folds the high half down with 2^n = c mod q (c = 2^n - q), reduces once and subtracts with one controlled modular subtract. The whole computation is then undone, so the folds may leave garbage that the uncompute clears. Fold widths are derived from exact integer bounds, so the same code is correct for every toy prime the gate tests.

**Fused Bezout cell.** Each replay step did a modular doubling (3n) and a controlled modular add (8n), two separate reductions. It now builds T = 2S + g0*R < 3q as an integer and subtracts kappa*q, kappa = floor(T/q) in {0, 1, 2}, in one pass. T >= q and T >= 2q are the carries out of T + c and T + 2c. Above bit K = bitlen(2c) both carries need the same run of ones in T, so one AND chain serves both thresholds and the increment. For secp256k1, K = 34, so the reduction costs about 3n + 3K instead of 6n. The two flags clear by parity and one controlled comparison against R. The step costs about 2,420 CCX instead of 3,056.

## What we ruled out, and why

The research reports (literature and the ecdsa.fail v1 leaderboard) agree that the big leaps elsewhere came from measurement-based uncomputation, approximate compares and empirical iteration counts. Joel's contract excludes all three. What transfers exactly:

- **Comparison-free walks** (ping-pong, leapfrog) are exact per step but have no proven round bound. The proven bound for ping-pong is 197,376 rounds.
- **Bernstein-Yang hddivsteps** have a proven 590-step bound. The walk step would drop from 6w to about 4w, but the Bezout replay grows by 78 steps. Without a proven per-step width schedule the walk saves nothing, and with one the net is about -2%.
- **Fewer than four walk passes.** Both in-place operations (divide by dx, multiply by a - x3) need their own transcript, and no published circuit avoids that.
- **Swap-free Euclid steps** using signed values or borrow-based compares all came out at 6w or more.

So the remaining cost is about evenly split between a walk step at 6w and a replay step at about 9.5n, and both are close to the floor for these algorithms without measurement.

## Verification

`python3 scripts/accept.py` on the fork branch, full mode. See `accept_full.txt`. The new cell also has an exhaustive gate-level test over 14 toy primes (every S, R and control bit, all ancillas checked clean).

## Decision trail

`decisions.tsv` has one row per decision with its evidence.
