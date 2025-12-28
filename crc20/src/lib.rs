//! CRC-20 Fungible Token Standard
//!
//! A standard interface for fungible tokens on Chert Coin blockchain,
//! similar to ERC-20 on Ethereum.
//!
//! ## Features
//! - Transfer tokens between accounts
//! - Approve spending allowances
//! - Delegated transfers via allowances
//! - Query balances and total supply
//! - Event emission for indexing

#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(target_arch = "wasm32", no_main)]

extern crate alloc;

use silica_contract_sdk::event;
use silica_contract_sdk::prelude::*;
use serde::de::DeserializeOwned;

const METADATA_KEY: &str = "metadata";
const BALANCES_PREFIX: &str = "balances";
const ALLOWANCES_PREFIX: &str = "allowances";
const ZERO_ADDRESS: &str = "0x0";
const MAX_CALL_DATA_BYTES: usize = 4096;
const MAX_RETURN_BYTES: usize = 4096;

/// Token metadata stored once at initialization
#[derive(Serialize, Deserialize)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub owner: String,
}

#[derive(Serialize, Deserialize)]
struct InitializeArgs {
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
}

#[derive(Serialize, Deserialize)]
struct TransferArgs {
    to: String,
    amount: u64,
}

#[derive(Serialize, Deserialize)]
struct ApproveArgs {
    spender: String,
    amount: u64,
}

#[derive(Serialize, Deserialize)]
struct TransferFromArgs {
    from: String,
    to: String,
    amount: u64,
}

#[derive(Serialize, Deserialize)]
struct BalanceOfArgs {
    account: String,
}

#[derive(Serialize, Deserialize)]
struct MintArgs {
    to: String,
    amount: u64,
}

fn read_args<T>() -> ContractResult<T>
where
    T: DeserializeOwned,
{
    let payload = context().call_data()?;
    assert!(
        payload.len() <= MAX_CALL_DATA_BYTES,
        "Call data exceeds static bound"
    );
    if payload.is_empty() {
        return Err(ContractError::CallDataUnavailable);
    }

    postcard::from_bytes(&payload).map_err(|_| ContractError::DeserializationFailed)
}

fn try_respond<T: Serialize>(value: &T) -> ContractResult<()> {
    let data = postcard::to_allocvec(value).map_err(|_| ContractError::SerializationFailed)?;
    assert!(
        data.len() <= MAX_RETURN_BYTES,
        "Return payload exceeds static bound"
    );
    context().return_bytes(&data)
}

fn load_metadata() -> ContractResult<TokenMetadata> {
    storage()
        .get::<TokenMetadata>(METADATA_KEY)?
        .ok_or_else(|| ContractError::InvalidArgument("Token not initialized".to_string()))
}

fn save_metadata(metadata: &TokenMetadata) -> ContractResult<()> {
    let mut store = storage();
    store.set(METADATA_KEY, metadata)
}

fn read_balance(address: &str) -> ContractResult<u64> {
    assert!(
        !address.is_empty(),
        "Balance lookup requires non-empty address"
    );
    let balances: Map<String, u64> = Map::new(BALANCES_PREFIX);
    Ok(balances.get(&address.to_string())?.unwrap_or(0))
}

fn write_balance(address: &str, amount: u64) -> ContractResult<()> {
    assert!(
        !address.is_empty(),
        "Balance write requires non-empty address"
    );
    let mut balances: Map<String, u64> = Map::new(BALANCES_PREFIX);
    balances.set(&address.to_string(), &amount)?;
    let stored = balances.get(&address.to_string())?.unwrap_or(0);
    assert_eq!(stored, amount, "Balance write verification failed");
    Ok(())
}

fn read_allowance(owner: &str, spender: &str) -> ContractResult<u64> {
    assert!(!owner.is_empty(), "Allowance owner cannot be empty");
    assert!(!spender.is_empty(), "Allowance spender cannot be empty");
    let allowances: Map<(String, String), u64> = Map::new(ALLOWANCES_PREFIX);
    Ok(allowances
        .get(&(owner.to_string(), spender.to_string()))?
        .unwrap_or(0))
}

fn write_allowance(owner: &str, spender: &str, amount: u64) -> ContractResult<()> {
    assert!(!owner.is_empty(), "Allowance owner cannot be empty");
    assert!(!spender.is_empty(), "Allowance spender cannot be empty");
    let mut allowances: Map<(String, String), u64> = Map::new(ALLOWANCES_PREFIX);
    let key = (owner.to_string(), spender.to_string());
    allowances.set(&key, &amount)?;
    let stored = allowances.get(&key)?.unwrap_or(0);
    assert_eq!(stored, amount, "Allowance write verification failed");
    Ok(())
}

fn ensure_initialized() -> ContractResult<()> {
    if !storage().has(METADATA_KEY) {
        return Err(ContractError::InvalidArgument(
            "Token contract not initialized".to_string(),
        ));
    }
    Ok(())
}

fn transfer_impl(from: &str, to: &str, amount: u64) -> ContractResult<()> {
    // Input validation
    validation::validate_address(from)?;
    validation::validate_address(to)?;
    validation::validate_positive_amount(amount)?;

    let from_balance = read_balance(from)?;
    if from_balance < amount {
        return Err(ContractError::InsufficientBalance {
            required: amount,
            available: from_balance,
        });
    }

    // Safe arithmetic operations
    let new_from_balance = safe_math::sub(from_balance, amount)?;
    let to_balance = read_balance(to)?;
    let new_to_balance = safe_math::add(to_balance, amount)?;

    write_balance(from, new_from_balance)?;
    write_balance(to, new_to_balance)?;

    Ok(())
}

fn execute_initialize() -> ContractResult<()> {
    let args: InitializeArgs = read_args()?;
    validation::validate_non_empty(&args.name, "name")?;
    validation::validate_non_empty(&args.symbol, "symbol")?;

    if storage().has(METADATA_KEY) {
        return Err(ContractError::InvalidArgument(
            "Token already initialized".to_string(),
        ));
    }

    let ctx = context();
    let deployer = ctx.sender();
    validation::validate_address(deployer)?;

    let metadata = TokenMetadata {
        name: args.name.clone(),
        symbol: args.symbol.clone(),
        decimals: args.decimals,
        total_supply: args.initial_supply,
        owner: deployer.to_string(),
    };

    save_metadata(&metadata)?;
    write_balance(deployer, args.initial_supply)?;

    event!("Transfer", from: ZERO_ADDRESS, to: deployer, amount: args.initial_supply);
    Ok(())
}

fn execute_transfer() -> ContractResult<()> {
    ensure_initialized()?;
    let ctx = context();
    let sender = ctx.sender().to_string();
    let args: TransferArgs = read_args()?;
    validation::validate_positive_amount(args.amount)?;

    transfer_impl(&sender, &args.to, args.amount)?;
    event!("Transfer", from: sender, to: args.to, amount: args.amount);
    Ok(())
}

fn execute_approve() -> ContractResult<()> {
    ensure_initialized()?;
    let ctx = context();
    let owner = ctx.sender().to_string();
    let args: ApproveArgs = read_args()?;

    write_allowance(&owner, &args.spender, args.amount)?;
    event!("Approval", owner: owner, spender: args.spender, amount: args.amount);
    Ok(())
}

fn execute_transfer_from() -> ContractResult<()> {
    ensure_initialized()?;
    let ctx = context();
    let spender = ctx.sender().to_string();
    let args: TransferFromArgs = read_args()?;
    validation::validate_positive_amount(args.amount)?;

    let allowance = read_allowance(&args.from, &spender)?;
    if allowance < args.amount {
        return Err(ContractError::InsufficientBalance {
            required: args.amount,
            available: allowance,
        });
    }

    transfer_impl(&args.from, &args.to, args.amount)?;
    let new_allowance = safe_math::sub(allowance, args.amount)?;
    write_allowance(&args.from, &spender, new_allowance)?;

    event!("Transfer", from: args.from, to: args.to, amount: args.amount);
    Ok(())
}

fn execute_balance_of() -> ContractResult<u64> {
    ensure_initialized()?;
    let args: BalanceOfArgs = read_args()?;
    let balance = read_balance(&args.account)?;
    try_respond(&balance)?;
    Ok(balance)
}

fn execute_total_supply() -> ContractResult<u64> {
    ensure_initialized()?;
    let metadata = load_metadata()?;
    try_respond(&metadata.total_supply)?;
    Ok(metadata.total_supply)
}

fn execute_decimals() -> ContractResult<u8> {
    ensure_initialized()?;
    let metadata = load_metadata()?;
    try_respond(&metadata.decimals)?;
    Ok(metadata.decimals)
}

fn execute_name() -> ContractResult<()> {
    ensure_initialized()?;
    let metadata = load_metadata()?;
    try_respond(&metadata.name)
}

fn execute_symbol() -> ContractResult<()> {
    ensure_initialized()?;
    let metadata = load_metadata()?;
    try_respond(&metadata.symbol)
}

fn execute_mint() -> ContractResult<()> {
    ensure_initialized()?;
    let ctx = context();
    let caller = ctx.sender().to_string();
    let args: MintArgs = read_args()?;
    validation::validate_positive_amount(args.amount)?;

    let mut metadata = load_metadata()?;
    if caller != metadata.owner {
        return Err(ContractError::Unauthorized);
    }

    let new_total = safe_math::add(metadata.total_supply, args.amount)?;
    metadata.total_supply = new_total;
    save_metadata(&metadata)?;

    let current_balance = read_balance(&args.to)?;
    let new_balance = safe_math::add(current_balance, args.amount)?;
    write_balance(&args.to, new_balance)?;

    event!("Transfer", from: ZERO_ADDRESS, to: args.to, amount: args.amount);
    Ok(())
}

/// Initialize the token contract
///
/// # Arguments (should be parsed from transaction data)
/// * `name` - Token name (e.g., "Chert Token")
/// * `symbol` - Token symbol (e.g., "CHT")
/// * `decimals` - Number of decimal places (e.g., 18)
/// * `initial_supply` - Initial token supply (will be minted to deployer)
#[unsafe(no_mangle)]
pub extern "C" fn initialize() {
    if let Err(err) = execute_initialize() {
        log(&format!("Initialize failed: {}", err));
    }
}

/// Transfer tokens from sender to recipient
///
/// # Arguments (from transaction data)
/// * `to` - Recipient address
/// * `amount` - Amount to transfer
#[unsafe(no_mangle)]
pub extern "C" fn transfer() {
    if let Err(err) = execute_transfer() {
        log(&format!("Transfer failed: {}", err));
    }
}

/// Approve a spender to transfer tokens on behalf of the sender
///
/// # Arguments
/// * `spender` - Address allowed to spend
/// * `amount` - Maximum amount they can spend
#[unsafe(no_mangle)]
pub extern "C" fn approve() {
    if let Err(err) = execute_approve() {
        log(&format!("Approve failed: {}", err));
    }
}

/// Transfer tokens on behalf of another account (requires prior approval)
///
/// # Arguments
/// * `from` - Account to transfer from
/// * `to` - Recipient address
/// * `amount` - Amount to transfer
#[unsafe(no_mangle)]
pub extern "C" fn transfer_from() {
    if let Err(err) = execute_transfer_from() {
        log(&format!("TransferFrom failed: {}", err));
    }
}

/// Query balance of an account
///
/// # Arguments
/// * `account` - Address to query
///
/// # Returns
/// Balance of the account
#[unsafe(no_mangle)]
pub extern "C" fn balance_of() -> u64 {
    match execute_balance_of() {
        Ok(value) => value,
        Err(err) => {
            log(&format!("balance_of failed: {}", err));
            0
        }
    }
}

/// Get total token supply
///
/// # Returns
/// Total supply of tokens
#[unsafe(no_mangle)]
pub extern "C" fn total_supply() -> u64 {
    match execute_total_supply() {
        Ok(value) => value,
        Err(err) => {
            log(&format!("total_supply failed: {}", err));
            0
        }
    }
}

/// Get token decimals
#[unsafe(no_mangle)]
pub extern "C" fn decimals() -> u8 {
    match execute_decimals() {
        Ok(value) => value,
        Err(err) => {
            log(&format!("decimals failed: {}", err));
            0
        }
    }
}

/// Get token name
#[unsafe(no_mangle)]
pub extern "C" fn name() {
    if let Err(err) = execute_name() {
        log(&format!("name failed: {}", err));
    }
}

/// Get token symbol
#[unsafe(no_mangle)]
pub extern "C" fn symbol() {
    if let Err(err) = execute_symbol() {
        log(&format!("symbol failed: {}", err));
    }
}

/// Mint new tokens (only owner)
///
/// # Arguments
/// * `to` - Recipient address
/// * `amount` - Amount to mint
#[unsafe(no_mangle)]
pub extern "C" fn mint() {
    if let Err(err) = execute_mint() {
        log(&format!("Mint failed: {}", err));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use silica_contract_sdk::ffi::mock;
    use std::sync::{Mutex, OnceLock};

    const ADDR_DEPLOYER: &str = "0x0000000000000000000000000000000000000d01";
    const ADDR_BOB: &str = "0x0000000000000000000000000000000000000b02";
    const ADDR_CAROL: &str = "0x0000000000000000000000000000000000000c03";
    const ADDR_DAVE: &str = "0x0000000000000000000000000000000000000d04";
    const ADDR_EVE: &str = "0x0000000000000000000000000000000000000e05";

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn encode<T: Serialize>(value: &T) -> Vec<u8> {
        postcard::to_allocvec(value).expect("encode call arguments")
    }

    fn setup_runtime(sender: &str) {
        mock::reset();
        mock::set_sender(sender);
        mock::set_contract_address("crc20_contract");
        mock::set_block_height(1);
        mock::set_block_timestamp(1_736_000_000);
    }

    fn init_default() {
        setup_runtime(ADDR_DEPLOYER);
        let args = InitializeArgs {
            name: "Chert Token".to_string(),
            symbol: "CHT".to_string(),
            decimals: 18,
            initial_supply: 1_000,
        };
        mock::set_call_data(&encode(&args));
        initialize();
        mock::take_events(); // drain initialization event to avoid coupling across tests
    }

    #[test]
    fn initialize_sets_metadata_and_balance() {
        let _guard = test_lock().lock().expect("test mutex poisoned");
        init_default();

        let metadata = storage()
            .get::<TokenMetadata>(METADATA_KEY)
            .expect("metadata read")
            .expect("metadata exists");
        assert_eq!(metadata.name, "Chert Token");
        assert_eq!(metadata.symbol, "CHT");
        assert_eq!(metadata.total_supply, 1_000);

        let balance = read_balance(ADDR_DEPLOYER).expect("deployer balance");
        assert_eq!(balance, 1_000);
    }

    #[test]
    fn transfer_moves_balance_and_emits_event() {
        let _guard = test_lock().lock().expect("test mutex poisoned");
        init_default();

        mock::set_sender(ADDR_DEPLOYER);
        let args = TransferArgs {
            to: ADDR_BOB.to_string(),
            amount: 200,
        };
        mock::set_call_data(&encode(&args));
        transfer();

        let deployer_balance = read_balance(ADDR_DEPLOYER).expect("sender balance");
        let bob_balance = read_balance(ADDR_BOB).expect("recipient balance");
        assert_eq!(deployer_balance, 800);
        assert_eq!(bob_balance, 200);

        let events = mock::take_events();
        assert!(!events.is_empty(), "transfer should emit event");
    }

    #[test]
    fn approve_and_transfer_from_decrements_allowance() {
        let _guard = test_lock().lock().expect("test mutex poisoned");
        init_default();

        mock::set_sender(ADDR_DEPLOYER);
        let approve_args = ApproveArgs {
            spender: ADDR_CAROL.to_string(),
            amount: 300,
        };
        mock::set_call_data(&encode(&approve_args));
        approve();

        mock::set_sender(ADDR_CAROL);
        let transfer_from_args = TransferFromArgs {
            from: ADDR_DEPLOYER.to_string(),
            to: ADDR_DAVE.to_string(),
            amount: 150,
        };
        mock::set_call_data(&encode(&transfer_from_args));
        transfer_from();

        let allowance = read_allowance(ADDR_DEPLOYER, ADDR_CAROL).expect("allowance read");
        let dave_balance = read_balance(ADDR_DAVE).expect("recipient balance");
        assert_eq!(allowance, 150);
        assert_eq!(dave_balance, 150);
    }

    #[test]
    fn mint_increases_supply_and_balance() {
        let _guard = test_lock().lock().expect("test mutex poisoned");
        init_default();

        mock::set_sender(ADDR_DEPLOYER);
        let args = MintArgs {
            to: ADDR_EVE.to_string(),
            amount: 250,
        };
        mock::set_call_data(&encode(&args));
        mint();

        let metadata = load_metadata().expect("metadata");
        let eve_balance = read_balance(ADDR_EVE).expect("eve balance");
        assert_eq!(metadata.total_supply, 1_250);
        assert_eq!(eve_balance, 250);
    }

    #[test]
    fn metadata_queries_return_values() {
        let _guard = test_lock().lock().expect("test mutex poisoned");
        init_default();

        mock::set_call_data(&encode(&BalanceOfArgs {
            account: ADDR_DEPLOYER.to_string(),
        }));
        let balance = balance_of();
        let balance_bytes = mock::take_return_data();
        let decoded_balance: u64 = postcard::from_bytes(&balance_bytes).expect("decode balance");
        assert_eq!(balance, 1_000);
        assert_eq!(decoded_balance, 1_000);

        total_supply();
        let supply_bytes = mock::take_return_data();
        let supply: u64 = postcard::from_bytes(&supply_bytes).expect("decode supply");
        assert_eq!(supply, 1_000);

        decimals();
        let decimals_bytes = mock::take_return_data();
        let decimals_value: u8 = postcard::from_bytes(&decimals_bytes).expect("decode decimals");
        assert_eq!(decimals_value, 18);

        name();
        let name_bytes = mock::take_return_data();
        let name_value: String = postcard::from_bytes(&name_bytes).expect("decode name");
        assert_eq!(name_value, "Chert Token");

        symbol();
        let symbol_bytes = mock::take_return_data();
        let symbol_value: String = postcard::from_bytes(&symbol_bytes).expect("decode symbol");
        assert_eq!(symbol_value, "CHT");
    }
}
