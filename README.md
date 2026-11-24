# leanVM on CKB-VM

leanVM aggregates signature claims into a single proof. This port keeps signing and proof generation on the host and runs aggregate verification inside CKB-VM. The standalone Rust verifier uses `no_std` with `alloc` and targets little-endian RV64IMC with the B extensions. Disassembly of the benchmark binary confirmed it contains no atomic instructions.

The verifier reads the expected claims from the input cell data and the proof from its witness. It checks the proof against those claims and the aggregation guest compiled into the verifier binary. For the proof system, supported claims, and native benchmarks, see the [original leanVM README](README.original.md).

## Benchmark

The experimental prover aggregated 500 leanSPHINCS signatures on an Intel Xeon E5-2670 v2 host. With Rust 1.100.0 and `ckb-debugger` 1.1.1, the verifier accepted the proof with exit code `0`.

| Measurement | Result |
| --- | ---: |
| Native proving time | 7.747 s |
| Aggregate proof size | 285.6 KiB |
| CKB-VM verification cycles | 605,129,810 |
| Verification wall time, including debugger startup and transaction loading | 1.905 s |

## Reproduce the run

With `ckb-debugger` on your `PATH`, run the following from the repository root. The benchmark builds the verifier automatically and records one proving pass after warmup.

```bash
export RUSTUP_TOOLCHAIN=stable
rustup target add riscv64imac-unknown-none-elf
cargo leanvm-bench --prover experimental aggregate --sphincs 500 --log-inv-rate 1 --repeat 1
```

The `aggregate` command uses CKB-VM verification by default. Pass `--verifier native` to measure host verification.
