//! Run the standalone aggregate verifier on the claims and proof the native prover produced.

use aggregation::{AggregateClaims, AggregateProof};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

pub fn prepare(claims: &AggregateClaims, proof: &AggregateProof) -> TempDir {
    let contract = Path::new(env!("CARGO_MANIFEST_DIR")).join("ckb");
    let tx = tempfile::tempdir().expect("create debugger transaction directory");
    fs::write(tx.path().join("claims"), claims.to_bytes()).expect("write expected claims");
    fs::write(tx.path().join("proof"), proof.to_bytes()).expect("write aggregate proof");
    fs::write(tx.path().join("tx.json"), include_bytes!("../ckb/tx.json")).expect("write debugger transaction");
    let status = Command::new(contract.join("build"))
        .arg(tx.path().join("verifier"))
        .status()
        .expect("build CKB verifier");
    assert!(status.success(), "CKB verifier build failed");
    tx
}

pub fn verify(tx: &TempDir) {
    let status = Command::new("ckb-debugger")
        .current_dir(tx.path())
        .args(["--tx-file", "tx.json", "--script", "input.0.lock"])
        .status()
        .expect("run ckb-debugger");
    assert!(status.success(), "CKB aggregate verification failed ({status})");
}
