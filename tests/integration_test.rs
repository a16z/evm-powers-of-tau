use ark_bn254::{G1Projective as G1, G2Projective as G2};
use ethers::utils::AnvilInstance;
use ethers::{middleware::SignerMiddleware, prelude::*, utils::Anvil};
use std::{sync::Arc, time::Duration};

use rust_pot::utils::IntoBytes;
use rust_pot::{pot_update, query, utils};

const NUM_G1: usize = 255;
const NUM_G2: usize = 1;

abigen!(KZG, "contracts/out/KZG.sol/KZG.json",);

#[tokio::test(flavor = "multi_thread")]
async fn primary() {
    let (contract, anvil, (init_g1s, init_g2s)) = launch_integration().await;
    let proof1 = pot_update::run_update(&init_g1s.as_slice(), &init_g2s.as_slice());

    pot_update::verify_proof(&proof1, &init_g1s.as_slice());

    let updated_g1s: Vec<Bytes> = proof1.g1s.iter().map(|item| item.into_bytes()).collect();
    let updated_g2s: Vec<Bytes> = proof1.g2s.iter().map(|item| item.into_bytes()).collect();

    let tx = contract.pot_update(
        updated_g1s,
        updated_g2s,
        proof1.pi1.into_bytes(),
        utils::fr_to_u256(proof1.pi2),
    );
    let pending_tx = tx.send().await;
    // Unwrap will panic if tx reverts
    let wait = pending_tx.unwrap().await;
    let receipt = wait.unwrap().unwrap();
    assert_eq!(receipt.status.unwrap().as_usize(), 1);
    println!("Update 1 gas usage: {}gwei", receipt.cumulative_gas_used);

    let provider = Provider::<Http>::try_from(anvil.endpoint())
        .expect("Failed to create provider")
        .interval(Duration::from_millis(10u64));
    let query_result = query::query_most_recent_kzg(&provider, contract.address()).await;
    let (queried_g1s, queried_g2s) = query_result.unwrap();
    assert_eq!(queried_g1s, proof1.g1s);
    assert_eq!(queried_g2s, proof1.g2s);

    // Second update
    let proof2 = pot_update::run_update(&proof1.g1s.as_slice(), &proof1.g2s.as_slice());
    pot_update::verify_proof(&proof2, &proof1.g1s.as_slice());
    let updated_g1s: Vec<Bytes> = proof2.g1s.iter().map(|item| item.into_bytes()).collect();
    let updated_g2s = proof2.g2s.iter().map(|item| item.into_bytes()).collect();
    let tx = contract.pot_update(
        updated_g1s,
        updated_g2s,
        proof2.pi1.into_bytes(),
        utils::fr_to_u256(proof2.pi2),
    );
    let pending_tx = tx.send().await;
    // Unwrap will panic if tx reverts
    let wait = pending_tx.unwrap().await;
    let receipt = wait.unwrap().unwrap();
    println!("Update 2 gas usage: {}gwei", receipt.cumulative_gas_used);
    let query_result = query::query_most_recent_kzg(&provider, contract.address()).await;
    let (queried_g1s, queried_g2s) = query_result.unwrap();
    assert_eq!(queried_g1s, proof2.g1s);
    assert_eq!(queried_g2s, proof2.g2s);

    // Clean
    drop(anvil);
}

#[tokio::test(flavor = "multi_thread")]
async fn hash_randomness() {
    let (contract, anvil, _) = launch_integration().await;

    let p1 = utils::rand_g1();
    let p2 = utils::rand_g1();
    let p3 = utils::rand_g1();

    let local_hash = pot_update::hash_randomness(p1, p2, p3);
    let contract_hash = contract
        .hash_randomness(p1.into_bytes(), p2.into_bytes(), p3.into_bytes())
        .call()
        .await
        .unwrap();

    assert_eq!(local_hash, contract_hash);

    // Clean
    drop(anvil);
}

async fn launch_integration() -> (
    kzg::KZG<SignerMiddleware<ethers::providers::Provider<ethers::providers::Http>, LocalWallet>>,
    AnvilInstance,
    (Vec<G1>, Vec<G2>),
) {
    let anvil = Anvil::new().spawn();
    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let provider = Provider::<Http>::try_from(anvil.endpoint())
        .expect("Failed to create provider")
        .interval(Duration::from_millis(10u64));

    let anvil_chain_id = provider.get_chainid().await.unwrap();
    let client = SignerMiddleware::new(provider, wallet.with_chain_id(anvil_chain_id.as_u64()));
    let client = Arc::new(client);

    let (init_g1s, init_g2s) = pot_update::init_params(NUM_G1, NUM_G2);

    let init_g1s_serial: Vec<Bytes> = init_g1s.iter().map(|item| item.into_bytes()).collect();
    let init_g2s_serial: Vec<Bytes> = init_g2s.iter().map(|item| item.into_bytes()).collect();

    let constructor_params: (Vec<Bytes>, Vec<Bytes>) = (init_g1s_serial, init_g2s_serial);
    (
        KZG::deploy(client, constructor_params)
            .unwrap()
            .legacy()
            .send()
            .await
            .unwrap(),
        anvil,
        (init_g1s, init_g2s),
    )
}
