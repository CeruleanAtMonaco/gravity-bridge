use clarity::Address as EthAddress;
use deep_space::address::Address;
use gravity_proto::gravity::query_client::QueryClient as GravityQueryClient;
use gravity_proto::gravity::BatchTxEthereumSignaturesRequest;
use gravity_proto::gravity::SignerSetTxRequest;
use gravity_proto::gravity::LastSubmittedEthereumEventRequest;
use gravity_proto::gravity::PendingBatchTxEthereumSignaturesRequest;
use gravity_proto::gravity::PendingContractCallTxEthereumSignaturesRequest;
use gravity_proto::gravity::PendingSignerSetTxEthereumSignaturesRequest;
use gravity_proto::gravity::SignerSetTxsRequest;
use gravity_proto::gravity::ContractCallTxEthereumSignaturesRequest;
use gravity_proto::gravity::ContractCallTxsRequest;
use gravity_proto::gravity::BatchTxsRequest;
use gravity_proto::gravity::QueryValsetConfirmsByNonceRequest;
use gravity_utils::error::GravityError;
use gravity_utils::types::*;
use tonic::transport::Channel;

/// get the valset for a given nonce (block) height
pub async fn get_valset(
    client: &mut GravityQueryClient<Channel>,
    nonce: u64,
) -> Result<Option<Valset>, GravityError> {
    let request = client
        .signer_set_tx(SignerSetTxRequest { nonce })
        .await?;
    let valset = request.into_inner().signer_set;
    let valset = match valset {
        Some(v) => Some(v.into()),
        None => None,
    };
    Ok(valset)
}

/// get the current valset. You should never sign this valset
/// valset requests create a consensus point around the block height
/// that transaction got in. Without that consensus point everyone trying
/// to sign the 'current' valset would run into slight differences and fail
/// to produce a viable update.
pub async fn get_current_valset(
    client: &mut GravityQueryClient<Channel>,
) -> Result<Valset, GravityError> {
    let request = client.signer_set_tx(SignerSetTxRequest { nonce: 0 }).await?;
    let valset = request.into_inner().signer_set;
    if let Some(valset) = valset {
        Ok(valset.into())
    } else {
        error!("Current valset returned None? This should be impossible");
        Err(GravityError::InvalidBridgeStateError(
            "Must have a current valset!".to_string(),
        ))
    }
}

/// This hits the /pending_valset_requests endpoint and will provide
/// an array of validator sets we have not already signed
pub async fn get_oldest_unsigned_valsets(
    client: &mut GravityQueryClient<Channel>,
    address: Address,
) -> Result<Vec<Valset>, GravityError> {
    let request = client
        .pending_signer_set_tx_ethereum_signatures(PendingSignerSetTxEthereumSignaturesRequest {
            address: address.to_string(),
        })
        .await?;
    let valsets = request.into_inner().signer_sets;
    // convert from proto valset type to rust valset type
    let valsets = valsets.iter().map(|v| v.into()).collect();
    Ok(valsets)
}

/// this input views the last five valset requests that have been made, useful if you're
/// a relayer looking to ferry confirmations
pub async fn get_latest_valsets(
    client: &mut GravityQueryClient<Channel>,
) -> Result<Vec<Valset>, GravityError> {
    let request = client
        .update_signer_set_txs(SignerSetTxsRequest { count: 5 })
        .await?;
    let valsets = request.into_inner().signer_sets;
    Ok(valsets.iter().map(|v| v.into()).collect())
}

/// get all valset confirmations for a given nonce
pub async fn get_all_valset_confirms(
    client: &mut GravityQueryClient<Channel>,
    nonce: u64,
) -> Result<Vec<ValsetConfirmResponse>, GravityError> {
    let request = client
        .valset_confirms_by_nonce(QueryValsetConfirmsByNonceRequest { nonce })
        .await?;
    let confirms = request.into_inner().confirms;
    let mut parsed_confirms = Vec::new();
    for item in confirms {
        parsed_confirms.push(ValsetConfirmResponse::from_proto(item)?)
    }
    Ok(parsed_confirms)
}

pub async fn get_oldest_unsigned_transaction_batch(
    client: &mut GravityQueryClient<Channel>,
    address: Address,
) -> Result<Option<TransactionBatch>, GravityError> {
    let request = client
        .pending_batch_tx_ethereum_signatures(PendingBatchTxEthereumSignaturesRequest {
            address: address.to_string(),
        })
        .await?;
    // TODO(levi) is this really getting the oldest; feels like newest?
    let batches = request.into_inner().batches;
    let batch = batches.get(0);
    match batch {
        Some(batch) => Ok(Some(TransactionBatch::from_proto(batch.clone())?)),
        None => Ok(None),
    }
}

/// gets the latest 100 transaction batches, regardless of token type
/// for relayers to consider relaying
pub async fn get_latest_transaction_batches(
    client: &mut GravityQueryClient<Channel>,
) -> Result<Vec<TransactionBatch>, GravityError> {
    let request = client
        .batch_txs(BatchTxsRequest {})
        .await?;
    let batches = request.into_inner().batches;
    let mut out = Vec::new();
    for batch in batches {
        out.push(TransactionBatch::from_proto(batch)?)
    }
    Ok(out)
}

/// get all batch confirmations for a given nonce and denom
pub async fn get_transaction_batch_signatures(
    client: &mut GravityQueryClient<Channel>,
    nonce: u64,
    contract_address: EthAddress,
) -> Result<Vec<BatchConfirmResponse>, GravityError> {
    let request = client
        .batch_confirms(BatchTxEthereumSignaturesRequest {
            nonce,
            contract_address: contract_address.to_string(),
        })
        .await?;
    let batch_confirms = request.into_inner().confirms;
    let mut out = Vec::new();
    for confirm in batch_confirms {
        out.push(BatchConfirmResponse::from_proto(confirm)?)
    }
    Ok(out)
}

/// Gets the last event nonce that a given validator has attested to, this lets us
/// catch up with what the current event nonce should be if a oracle is restarted
pub async fn get_last_event_nonce(
    client: &mut GravityQueryClient<Channel>,
    address: Address,
) -> Result<u64, GravityError> {
    let request = client
        .last_submitted_ethereum_event(LastSubmittedEthereumEventRequest {
            address: address.to_string(),
        })
        .await?;
    Ok(request.into_inner().event_nonce)
}

/// Gets the 100 latest logic calls for a relayer to consider relaying
pub async fn get_latest_logic_calls(
    client: &mut GravityQueryClient<Channel>,
) -> Result<Vec<LogicCall>, GravityError> {
    let request = client
        .contract_call_txs(ContractCallTxsRequest {})
        .await?;
    let calls = request.into_inner().calls;
    let mut out = Vec::new();
    for call in calls {
        out.push(LogicCall::from_proto(call)?);
    }
    Ok(out)
}

pub async fn get_logic_call_signatures(
    client: &mut GravityQueryClient<Channel>,
    invalidation_scope: Vec<u8>,
    invalidation_nonce: u64,
) -> Result<Vec<LogicCallConfirmResponse>, GravityError> {
    let request = client
        .contract_call_tx_ethereum_signatures(ContractCallTxEthereumSignaturesRequest {
            invalidation_scope,
            invalidation_nonce,
            address: String::from(""),
        })
        .await?;
    let call_confirms = request.into_inner().signature;
    let mut out = Vec::new();
    for confirm in call_confirms {
        out.push(LogicCallConfirmResponse::from_proto(confirm)?)
    }
    Ok(out)
}

pub async fn get_oldest_unsigned_logic_call(
    client: &mut GravityQueryClient<Channel>,
    address: Address,
) -> Result<Option<LogicCall>, GravityError> {
    let request = client
        .pending_contract_call_tx_ethereum_signatures(PendingContractCallTxEthereumSignaturesRequest {
            address: address.to_string(),
        })
        .await?;
    let calls = request.into_inner().calls;
    let call = calls.get(0);
    match call {
        Some(call) => Ok(Some(LogicCall::from_proto(call.clone())?)),
        None => Ok(None),
    }
}
