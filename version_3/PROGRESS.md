# version_3: plan and progress log

Crash-recovery file. Read **Status** and **Next step** first. version_2 is frozen; its log is `../version_2/PROGRESS.md`.

## Goal

Make one definite, provable improvement to version_2, judged the way Joel's ecdsafail-v2 benchmark will judge it (`jtaroreh/ecdsafail-v2-reference`). That profile (`secp256k1-controlled-exact-v1`) forbids measurement-based uncomputation, so the Toffoli count without measurement ("coherent") is the number that matters. The measured-profile score must not get worse.

## Ideas considered

| Idea | Verdict |
| --- | --- |
| 1. Kaliski integer cofactors in the target pass (version_1 Suggestions, item 1, "about −27%") | Rejected. Integer cofactors stay bounded only from the start (0, 1), so they give 2^k/x, not y/x. Recovering y/x needs a full modular multiplication by y, its uncomputation, and a 2^-k correction, which likely costs more than the saving. The −27% estimate missed this. |
| 2. Port Joel's arithmetic tricks (controlled CDKM ripple, constant-carry-aware reduction, 3n − 15 doubler) | Deferred. Real savings, but the constant tricks depend on the sparsity of secp256k1's constants, which breaks our field-generic rule. Joel's INVARIANTS notes they do not carry to a windowed (coherent-addend) kernel. |
| **3. Mask-free controlled addition** | **Built.** |

## The change

version_2 did every controlled subtraction t −= ctl·a by first ANDing ctl·a into a fresh n-qubit register, then subtracting it (`with_masked`). With measurement the mask costs n Toffolis, and without measurement it costs 2n. The mask register also sat at the qubit peak during the target pass.

version_3 adds `arith::ctl_add`. It computes the carries of a + t unconditionally, using the existing Gidney-style `carries`. On the way back down it restores each t_i and writes t_i ^= ctl·(a_i ^ c_{i−1}) with one Toffoli per bit. The carry-out, when requested, is z ^= ctl·c_{n−1}. Costs for a quantum addend:

| | measured | coherent | scratch |
| --- | --- | --- | --- |
| version_2 masked controlled add | 2n − 1 | 4n − 2 | 2n − 1 |
| version_3 `ctl_add` | 2n − 1 | 3n − 2 | n − 1 |

`Field::ctl_sub` is the inverse of a controlled modular add. It runs the controlled add with carry-out z, then the unchanged p-subtract and p·z add-back. It clears z with a controlled compare, `less_than(a, t, z, Some(ctl))` and then X. The argument: z = [ctl·a + t < p]. For ctl = 0 this is 1, because t < p. For ctl = 1 it is NOT [result < a]. So z = NOT(ctl·[result < a]). The controlled carry-out and the controlled compare each cost one more Toffoli than version_2's, so the field ctl_sub costs 2 more measured Toffolis (1278 to 1280). Public signatures did not change, so `divide.rs` and `point.rs` are untouched.

## Status

All numbers are measured in this sandbox with the offline twin. Raw outputs are in `results/`.

| Build | Qubits | Toffolis | Q×T | v1 harness (9,024 shots) |
| --- | --- | --- | --- | --- |
| version_2c, measured | 1,855 | 4,379,378 | 8.124e9 | all OK |
| version_2c, coherent | 1,855 | 8,100,836 | 1.503e10 | builder count only |
| **version_3, measured** | **1,600** | **4,381,938** | **7.011e9 (−13.7%)** | **all OK** |
| **version_3, coherent** | **1,600** | **7,381,988 (−8.9%)** | **1.181e10 (−21.4%)** | **all OK** |
| Joel's reference (controlled, complete, coherent) | 2,311 | 7,209,419 | 1.666e10 | his receipts |

Every count is deterministic. The executed Toffoli count per shot equals the builder count exactly in both lowerings, so the version_2 coherent row, taken from the builder, is the same quantity.

The comparison with Joel's reference is under different contracts. His circuit is controlled (|c>|R> → |c>|R + cA>) and complete (handles infinity and R in {A, −A, −2A}). Ours is uncontrolled and fails R = −2A. On the Toffoli count alone ours is still 2.4% above his, with 689 fewer qubits.

eval_v2 (fixed seed `qcb-v2-proxy-2026-10-31`, 2,000 random shots plus edge classes) gives the same profile as version_2 in both lowerings. Random inputs have 0 wrong. Every wrong shot is the documented T = −2O case (12 small-multiples, 3 near-order, 64 of 64 exceptional). The coherent build passes the forward∘reverse identity check on every shot. eval_v2 exits 1 because of the edge-class failures, as it did for version_2.

Cost breakdown (`cargo test --release --offline --bin build_circuit cost_breakdown -- --ignored --nocapture`, n = 256):

| Component | v2 measured | v3 measured | v2 coherent | v3 coherent | v2 width | v3 width |
| --- | --- | --- | --- | --- | --- | --- |
| gcd step (fwd, full width) | 1,024 | 1,024 | 1,791 | 1,535 | 1,025 | 770 |
| target step | 2,045 | 2,047 | 3,834 | 3,580 | 1,027 | 772 |
| divide | 1,891,712 | 1,892,736 | 3,454,464 | 3,127,552 | 1,855 | 1,600 |
| sub_square | 587,778 | 588,290 | 1,175,556 | 1,110,532 | 1,281 | 1,026 |
| point_add | 4,379,378 | 4,381,938 | 8,100,836 | 7,381,988 | 1,855 | 1,600 |

The measured count rises by 2,560 (+0.058%): 2 per field ctl_sub, times 1,280 calls (512 target steps in each of the two divisions, plus 256 in sub_square). Width falls by 255, so the measured Q×T still falls 13.7%.

Exactness evidence. Two new small-field tests run in both lowerings with clean-ancilla and zero-phase checks. `integer_ctl_add` covers n = 1..8, quantum and constant addends, with and without carry-out, and ctl in {0, 1}. `ctl_sub_small_fields` covers field and integer ctl_sub for p in {2, 3, 5, 7, 11, 13, 31, 61, 127, 131, 251}. All existing tests pass unchanged, including division by every divisor for 10 primes and the small-curve point addition in all four addend/lowering combinations. The new tests were checked to fail on three deliberate breaks: an uncontrolled compare, an uncontrolled carry-out, and an uncontrolled bit 0.

## Next step

version_3 is done. Where the coherent cost now sits: the two divisions are 6.26M of 7.38M (85%). Per step, the target pass is 3,580 against Joel's 3,056 for his Bezout step, and the square is 1.11M against his 910K. Candidates for version_4:

1. Merge the modular reduction's constant subtract and conditional add-back into one carry chain (Joel's trick, field-generic in its general form). Expected about −n per modular add.
2. A cheaper field halving without measurement (today 4n coherent).
3. Controlled and complete addition (Joel's flags-and-gen wrapper), which his contract requires and which we do not yet have.

## Log

- 2026-11-01 21:57 PT. Read Joel's v2 reference (`jtaroreh/ecdsafail-v2-reference@e3bf0c0`): exact, controlled, complete, no measurement, 2,311 wires, 7,209,419 CCX. Read version_2: 4,379,378 measured but 8,100,836 coherent, which is worse than Joel's.
- 2026-11-01 22:05 PT. Baseline scored: version_2c 9,024/9,024 OK on the unchanged upstream harness, 1,855 × 4,379,378 = 8.124e9. Coherent breakdown added to `cost_breakdown`. Masked copies account for about 1.44M coherent Toffolis (18%).
- 2026-11-01 22:10 PT. Three ideas weighed (above). Picked mask-free controlled addition.
- 2026-11-01 22:20 PT. Implemented `arith::ctl_add` and `Field::ctl_add`, removed `with_masked`. All 8 tests pass in both lowerings. Breakdown matches the prediction exactly for the integer ctl_sub (511 measured, 766 coherent).
- 2026-11-01 22:45 PT. Scored version_3 in both lowerings with the unchanged upstream harness: 9,024/9,024 OK each. Measured 1,600 × 4,381,938 = 7.011e9. Coherent 1,600 × 7,381,988 = 1.181e10. eval_v2: 0 wrong on random inputs, T = −2O failures unchanged, coherent forward∘reverse clean.
