//! Security utilities for smart contracts
//!
//! Provides reentrancy protection, access control mechanisms, safe math, input
//! validation, and constant-time comparison helpers.

use crate::error::{ContractError, ContractResult};
use crate::storage::{Map, storage};
use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicBool, Ordering};

#[inline(always)]
fn invalid_argument(message: &'static str) -> ContractError {
    ContractError::InvalidArgument(String::from(message))
}

const OWNER_KEY: &str = "__ac_owner";
const ROLE_BUCKET: &str = "__ac_roles";

fn roles_map() -> Map<String, bool> {
    Map::new(ROLE_BUCKET)
}

fn role_storage_key(role: &str, address: &str) -> String {
    alloc::format!("{}:{}", role, address)
}

static ENTERED_FLAG: AtomicBool = AtomicBool::new(false);

/// Reentrancy guard API.
pub struct ReentrancyGuard;

/// RAII guard returned by [`ReentrancyGuard::enter`].
pub struct ReentrancyGuardGuard;

impl ReentrancyGuard {
    /// Attempt to enter the protected section, returning a guard on success.
    pub fn enter() -> ContractResult<ReentrancyGuardGuard> {
        if ENTERED_FLAG
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(ContractError::ReentrancyDetected);
        }
        Ok(ReentrancyGuardGuard)
    }

    /// Manually release the guard. Prefer relying on the RAII drop implementation.
    pub fn exit() {
        ENTERED_FLAG.store(false, Ordering::Release);
    }

    /// Execute a closure with reentrancy protection.
    pub fn execute<F, R>(f: F) -> ContractResult<R>
    where
        F: FnOnce() -> ContractResult<R>,
    {
        let guard = Self::enter()?;
        let result = f();
        drop(guard);
        result
    }
}

impl Drop for ReentrancyGuardGuard {
    fn drop(&mut self) {
        ReentrancyGuard::exit();
    }
}

/// Role-based access control manager.
pub struct AccessControl;

impl AccessControl {
    /// Initialise access control with the given owner.
    pub fn initialize(owner: &str) -> ContractResult<()> {
        let mut store = storage();
        store.set(OWNER_KEY, &owner.to_string())?;
        AccessControl::grant_role_internal(owner, "admin")
    }

    /// Retrieve the stored contract owner (if any).
    pub fn owner() -> Option<String> {
        AccessControl::read_owner().ok().flatten()
    }

    /// Check if an address currently holds a role.
    pub fn has_role(address: &str, role: &str) -> bool {
        AccessControl::has_role_internal(address, role).unwrap_or(false)
    }

    /// Grant a role to an address. Only the owner or an admin may grant roles.
    pub fn grant_role(granter: &str, address: &str, role: &str) -> ContractResult<()> {
        if !AccessControl::is_owner_or_admin(granter)? {
            return Err(ContractError::Unauthorized);
        }
        AccessControl::grant_role_internal(address, role)
    }

    /// Revoke a role from an address. Only the owner or an admin may revoke roles.
    pub fn revoke_role(revoker: &str, address: &str, role: &str) -> ContractResult<()> {
        if !AccessControl::is_owner_or_admin(revoker)? {
            return Err(ContractError::Unauthorized);
        }
        AccessControl::revoke_role_internal(address, role)
    }

    /// Ensure the caller is authorised, optionally requiring a specific role.
    pub fn authorize(caller: &str, required_role: Option<&str>) -> ContractResult<()> {
        if AccessControl::is_owner(caller)? {
            return Ok(());
        }

        if let Some(role) = required_role {
            if AccessControl::has_role_internal(caller, role)? {
                return Ok(());
            }
        }

        Err(ContractError::Unauthorized)
    }

    /// Transfer ownership to a new address, updating the admin role accordingly.
    pub fn transfer_ownership(current_owner: &str, new_owner: &str) -> ContractResult<()> {
        if !AccessControl::is_owner(current_owner)? {
            return Err(ContractError::Unauthorized);
        }

        let mut store = storage();
        store.set(OWNER_KEY, &new_owner.to_string())?;
        AccessControl::grant_role_internal(new_owner, "admin")?;
        AccessControl::revoke_role_internal(current_owner, "admin")
    }

    fn read_owner() -> ContractResult<Option<String>> {
        storage().get::<String>(OWNER_KEY)
    }

    fn is_owner(address: &str) -> ContractResult<bool> {
        Ok(AccessControl::read_owner()?.map_or(false, |owner| owner == address))
    }

    fn is_owner_or_admin(address: &str) -> ContractResult<bool> {
        if AccessControl::is_owner(address)? {
            return Ok(true);
        }
        AccessControl::has_role_internal(address, "admin")
    }

    fn grant_role_internal(address: &str, role: &str) -> ContractResult<()> {
        let mut roles = roles_map();
        roles.set(&role_storage_key(role, address), &true)
    }

    fn revoke_role_internal(address: &str, role: &str) -> ContractResult<()> {
        let mut roles = roles_map();
        roles.remove(&role_storage_key(role, address))
    }

    fn has_role_internal(address: &str, role: &str) -> ContractResult<bool> {
        Ok(roles_map()
            .get(&role_storage_key(role, address))?
            .unwrap_or(false))
    }
}

/// Safe arithmetic helpers with overflow checking.
pub mod safe_math {
    use crate::error::{ContractError, ContractResult};

    #[inline(always)]
    pub fn add(a: u64, b: u64) -> ContractResult<u64> {
        match a.checked_add(b) {
            Some(result) => Ok(result),
            None => Err(ContractError::Overflow),
        }
    }

    #[inline(always)]
    pub fn sub(a: u64, b: u64) -> ContractResult<u64> {
        match a.checked_sub(b) {
            Some(result) => Ok(result),
            None => Err(ContractError::Underflow),
        }
    }

    #[inline(always)]
    pub fn mul(a: u64, b: u64) -> ContractResult<u64> {
        match a.checked_mul(b) {
            Some(result) => Ok(result),
            None => Err(ContractError::Overflow),
        }
    }

    #[inline(always)]
    pub fn div(a: u64, b: u64) -> ContractResult<u64> {
        if b == 0 {
            return Err(super::invalid_argument("Division by zero"));
        }
        Ok(a / b)
    }

    #[inline(always)]
    pub fn pow(base: u64, exp: u32) -> ContractResult<u64> {
        base.checked_pow(exp).ok_or(ContractError::Overflow)
    }

    #[inline(always)]
    pub const fn saturating_add(a: u64, b: u64) -> u64 {
        a.saturating_add(b)
    }

    #[inline(always)]
    pub const fn saturating_sub(a: u64, b: u64) -> u64 {
        a.saturating_sub(b)
    }
}

/// Input validation helpers.
pub mod validation {
    use crate::error::{ContractError, ContractResult};
    use alloc::string::ToString;

    #[inline(always)]
    pub fn validate_address(address: &str) -> ContractResult<()> {
        if address.is_empty() {
            return Err(ContractError::InvalidArgument(
                "Address cannot be empty".to_string(),
            ));
        }

        let len = address.len();
        if len < 10 || len > 100 {
            return Err(ContractError::InvalidArgument(
                "Invalid address length".to_string(),
            ));
        }

        Ok(())
    }

    #[inline(always)]
    pub fn validate_non_empty(value: &str, field_name: &str) -> ContractResult<()> {
        if value.trim().is_empty() {
            return Err(ContractError::InvalidArgument(alloc::format!(
                "{} cannot be empty",
                field_name
            )));
        }
        Ok(())
    }

    #[inline(always)]
    pub fn validate_positive_amount(amount: u64) -> ContractResult<()> {
        if amount == 0 {
            return Err(super::invalid_argument("Amount must be positive"));
        }
        Ok(())
    }

    #[inline(always)]
    pub fn validate_token_id(token_id: u64) -> ContractResult<()> {
        if token_id == 0 {
            return Err(super::invalid_argument("Token ID cannot be zero"));
        }
        Ok(())
    }

    pub fn validate_addresses(addresses: &[&str]) -> ContractResult<()> {
        for &address in addresses {
            validate_address(address)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn validate_range(value: u64, min: u64, max: u64) -> ContractResult<()> {
        if value < min || value > max {
            return Err(super::invalid_argument("Value out of range"));
        }
        Ok(())
    }
}

/// Constant-time comparison utilities for sensitive data.
pub mod constant_time {
    use subtle::ConstantTimeEq;

    #[inline(always)]
    pub fn eq_str(a: &str, b: &str) -> bool {
        eq_bytes(a.as_bytes(), b.as_bytes())
    }

    #[inline(always)]
    pub fn eq_bytes(a: &[u8], b: &[u8]) -> bool {
        a.ct_eq(b).into()
    }

    #[inline(always)]
    pub fn eq_array<const N: usize>(a: &[u8; N], b: &[u8; N]) -> bool {
        a.ct_eq(b).into()
    }

    #[inline(always)]
    pub fn secure_eq(a: &[u8], b: &[u8]) -> bool {
        eq_bytes(a, b)
    }
}

/// Errors specific to the security module.
#[derive(Debug, Clone)]
pub enum SecurityError {
    ReentrancyDetected,
    Unauthorized,
    InvalidRole,
    Overflow,
    Underflow,
}

impl From<SecurityError> for ContractError {
    fn from(error: SecurityError) -> Self {
        match error {
            SecurityError::ReentrancyDetected => {
                ContractError::Custom("Reentrancy detected".to_string())
            }
            SecurityError::Unauthorized => ContractError::Unauthorized,
            SecurityError::InvalidRole => {
                ContractError::InvalidArgument("Invalid role".to_string())
            }
            SecurityError::Overflow => ContractError::Overflow,
            SecurityError::Underflow => ContractError::Underflow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::ffi::mock;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn test_reentrancy_guard() {
        let guard = ReentrancyGuard::enter().expect("first entry");
        assert!(ReentrancyGuard::enter().is_err());
        drop(guard);

        let guard = ReentrancyGuard::enter().expect("re-entry after drop");
        drop(guard);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_access_control() {
        mock::reset();

        let owner = "owner_address";
        AccessControl::initialize(owner).expect("init owner");

        assert_eq!(AccessControl::owner(), Some(owner.to_string()));
        assert!(AccessControl::authorize(owner, None).is_ok());

        let user = "user_address";
        assert!(!AccessControl::has_role(user, "admin"));
        assert!(AccessControl::authorize(user, Some("admin")).is_err());

        AccessControl::grant_role(owner, user, "admin").expect("grant admin");
        assert!(AccessControl::has_role(user, "admin"));
        AccessControl::authorize(user, Some("admin")).expect("user authorised");

        AccessControl::revoke_role(owner, user, "admin").expect("revoke admin");
        assert!(!AccessControl::has_role(user, "admin"));

        AccessControl::transfer_ownership(owner, "new_owner").expect("transfer ownership");
        assert_eq!(AccessControl::owner(), Some("new_owner".to_string()));
        assert!(AccessControl::authorize(owner, Some("admin")).is_err());
        AccessControl::authorize("new_owner", Some("admin")).expect("new owner admin");
    }
}
