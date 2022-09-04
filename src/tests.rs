use anyhow::Ok;
use near_sdk::{json_types::U128, AccountId};
use workspaces::{Contract, prelude::DevAccountDeployer, Worker, network::Sandbox, Account};
use near_units::parse_near;
use serde_json::{json, Value};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{Balance, Timestamp};



const PAYMENT_CONTRACT_PATH: &str = "./wasm/contract.wasm";
const FT_TOKEN_PATH: &str = "./wasm/vbi-ft.wasm";

#[derive( Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct OrderDetail {
    pub order_id: AccountId,
    pub payer_id: AccountId,
    pub amount: Balance,
    pub received_amount: Balance,
    pub is_completed: bool,
    pub is_refund: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // async function(){}
    // Tao worker -> start worker
    let worker = workspaces::sandbox().await?;

    // Dev-Deploy ecommerce payment contract len worker
    let payment_wasm = std::fs::read(PAYMENT_CONTRACT_PATH)?;
    let payment_contract: Contract = worker.dev_deploy(&payment_wasm).await?;

    // Dev-deploy fungible token contract
    let ft_token_wasm = std::fs::read(FT_TOKEN_PATH)?;
    let ft_contract: Contract = worker.dev_deploy(&ft_token_wasm).await?;

    // Create account
    // mainnet -> root account = near ex abc.near, xyz.near
    // testnet -> root account = testnet, ex: vbidev.testnet
    let owner = worker.root_account().unwrap();

    // tao account alice, vbidev.testnet, uit-payment-contract.vbidev.testnet
    let alice = owner.create_subaccount(&worker, "alice")
                                            .initial_balance(parse_near!("30 N"))
                                            .transact()
                                            .await?
                                            .into_result()?;

    // Init contract
    ft_contract
        .call(&worker, "new_default_meta")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("1,000,000,000 N").to_string(),
        }))?
        .transact()
        .await?;

    // Init contract
    payment_contract
        .call(&worker, "new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "ft_contract_id": ft_contract.id(),
        }))?
        .transact()
        .await?;


    // Begin test
    test_pay_order(&alice, &payment_contract, &worker).await?;
    Ok(())
}

async fn test_pay_order(user: &Account, payment_contract: &Contract, worker: &Worker<Sandbox>) -> anyhow::Result<()> {
    let order_amount = parse_near!("1 N");

    user.
        call(&worker, payment_contract.id(), "pay_order")
        .args_json(json!({
            "order_id": "order_1",
            "order_amount": U128(order_amount)
        }))?
        .deposit(order_amount)
        .transact()
        .await?;

    println!("      Passed ✅  pay_order");

    let res_order: OrderDetail = user.call(worker, payment_contract.id(), "get_order_detail")
                                        .args_json(json!({
                                            "order_id": "order_1"
                                        }))?
                                        .transact()
                                        .await?
                                        .json()?;

    assert_eq!(res_order.payer_id.to_string(), user.id().to_string());
    assert_eq!(res_order.amount, order_amount);

    println!("      Passed ✅  get_order_detail");

    Ok(())
}