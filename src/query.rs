use ark_bn254::{G1Projective as G1, G2Projective as G2};
use ethers::abi::AbiDecode;
use ethers::prelude::{abigen, Filter, Http, Middleware, Provider, H160};
use std::error::Error;
use std::sync::Arc;

use anyhow::Result;

use crate::utils;

abigen!(KZG, "contracts/out/KZG.sol/KZG.json",);

pub async fn query_most_recent_kzg(
    provider: &Provider<Http>,
    contract_address: H160,
) -> Result<(Vec<G1>, Vec<G2>)> {
    let client = Arc::new(provider);

    let filter = Filter::new().address(contract_address);

    let mut logs = client.get_logs(&filter).await?;
    // Take the last log, check transaction execution, if success, decode input
    loop {
        let tx_hash = logs.pop().ok_or(QueryError)?.transaction_hash.ok_or(QueryError)?;

        let tx = client.get_transaction(tx_hash).await?.ok_or(QueryError)?;
        let receipt = client
            .get_transaction_receipt(tx_hash)
            .await?
            .ok_or(QueryError)?;

        if receipt.status.ok_or(QueryError)?.as_usize() == 0 {
            continue;
        }

        let decoded_input = PotUpdateCall::decode(&tx.input)?;
        let g1s: Vec<G1> = decoded_input
            .g_1s
            .iter()
            .map(|g1| utils::contract_bytes_to_g1(g1))
            .collect();
        let g2s: Vec<G2> = decoded_input
            .g_2s
            .iter()
            .map(|g2| utils::contract_bytes_to_g2(g2))
            .collect();

        return Ok((g1s, g2s));
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct QueryError;

impl Error for QueryError {
    fn description(&self) -> &str {
        "Query failure".as_ref()
    }
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
