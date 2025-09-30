use soroban_sdk::{contract, contractimpl, Address, Env, Vec, Map, String, Bytes};

#[contract]
pub struct AccessControlManager;

#[contractimpl]
impl AccessControlManager {
    /// Initialize access control
    pub fn initialize(
        env: Env,
        super_admin: Address
    ) -> Result<(), AccessControlError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(AccessControlError::AlreadyInitialized);
        }

        // Set up default roles
        Self::setup_default_roles(&env)?;

        // Grant super admin role
        Self::grant_role(&env, &SUPER_ADMIN_ROLE, &super_admin, &super_admin)?;

        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    /// Grant role to account
    pub fn grant_role(
        env: &Env,
        role: &Bytes,
        account: &Address,
        granter: &Address
    ) -> Result<(), AccessControlError> {
        granter.require_auth();

        // Check if granter has permission to grant this role
        if !Self::can_grant_role(env, granter, role) {
            return Err(AccessControlError::UnauthorizedGrant);
        }

        // Check role exists
        if !Self::role_exists(env, role) {
            return Err(AccessControlError::RoleDoesNotExist);
        }

        // Grant role
        env.storage().persistent().set(&DataKey::UserRole(account.clone(), role.clone()), &true);

        emit_role_granted(env, role.clone(), account.clone(), granter.clone());
        Ok(())
    }

    /// Revoke role from account
    pub fn revoke_role(
        env: &Env,
        role: &Bytes,
        account: &Address,
        revoker: &Address
    ) -> Result<(), AccessControlError> {
        revoker.require_auth();

        if !Self::can_revoke_role(env, revoker, role) {
            return Err(AccessControlError::UnauthorizedRevoke);
        }

        env.storage().persistent().remove(&DataKey::UserRole(account.clone(), role.clone()));

        emit_role_revoked(env, role.clone(), account.clone(), revoker.clone());
        Ok(())
    }

    /// Check if account has role
    pub fn has_role(env: Env, role: Bytes, account: Address) -> bool {
        env.storage().persistent().has(&DataKey::UserRole(account, role))
    }

    /// Check if account can perform action on contract
    pub fn can_perform_action(
        env: Env,
        account: Address,
        contract: Address,
        action: String
    ) -> bool {
        // Check if account has specific permission
        if env.storage().persistent().has(&DataKey::Permission(account.clone(), contract.clone(), action.clone())) {
            return true;
        }

        // Check role-based permissions
        let required_roles = Self::get_required_roles(&env, &contract, &action);

        for role in required_roles {
            if Self::has_role(env.clone(), role, account.clone()) {
                return true;
            }
        }

        false
    }

    /// Set action permission requirements
    pub fn set_action_roles(
        env: Env,
        admin: Address,
        contract: Address,
        action: String,
        required_roles: Vec<Bytes>
    ) -> Result<(), AccessControlError> {
        admin.require_auth();
        Self::require_role(&env, &admin, &ADMIN_ROLE)?;

        env.storage().persistent().set(
            &DataKey::ActionRoles(contract.clone(), action.clone()),
            &required_roles
        );

        emit_action_roles_updated(&env, contract, action, required_roles);
        Ok(())
    }

    /// Emergency role assignment (super admin only)
    pub fn emergency_grant_role(
        env: Env,
        super_admin: Address,
        role: Bytes,
        account: Address,
        duration: u64 // Temporary role duration in seconds
    ) -> Result<(), AccessControlError> {
        super_admin.require_auth();
        Self::require_role(&env, &super_admin, &SUPER_ADMIN_ROLE)?;

        let expiry = env.ledger().timestamp() + duration;
        env.storage().persistent().set(
            &DataKey::TemporaryRole(account.clone(), role.clone()),
            &expiry
        );

        emit_emergency_role_granted(&env, role, account, expiry);
        Ok(())
    }

    fn setup_default_roles(env: &Env) -> Result<(), AccessControlError> {
        // Define default roles
        let roles = vec![
            SUPER_ADMIN_ROLE,
            ADMIN_ROLE,
            ORACLE_ADMIN_ROLE,
            POOL_ADMIN_ROLE,
            EMERGENCY_GUARDIAN_ROLE,
            PAUSER_ROLE,
            UPGRADER_ROLE,
        ];

        for role in roles {
            env.storage().persistent().set(&DataKey::Role(role.clone()), &true);
        }

        Ok(())
    }
}

// Role definitions
const SUPER_ADMIN_ROLE: Bytes = Bytes::from_array(&[0x00]);
const ADMIN_ROLE: Bytes = Bytes::from_array(&[0x01]);
const ORACLE_ADMIN_ROLE: Bytes = Bytes::from_array(&[0x02]);
const POOL_ADMIN_ROLE: Bytes = Bytes::from_array(&[0x03]);
const EMERGENCY_GUARDIAN_ROLE: Bytes = Bytes::from_array(&[0x04]);
const PAUSER_ROLE: Bytes = Bytes::from_array(&[0x05]);
const UPGRADER_ROLE: Bytes = Bytes::from_array(&[0x06]);