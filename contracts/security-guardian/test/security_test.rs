#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_flash_loan_attack_prevention() {
        let env = TestEnvironment::new();
        let contracts = deploy_all_contracts(&env);

        // Simulate flash loan attack
        let attack_result = simulate_flash_loan_attack(&env, &contracts);
        assert!(attack_result.is_err());
    }

    #[test]
    fn test_oracle_manipulation_protection() {
        let env = TestEnvironment::new();
        let oracle = deploy_oracle_aggregator(&env);

        // Try to manipulate single oracle
        let manipulation_result = attempt_oracle_manipulation(&env, &oracle);
        assert!(manipulation_result.is_err());
    }

    #[test]
    fn test_emergency_procedures() {
        let env = TestEnvironment::new();
        let guardian = deploy_security_guardian(&env);

        // Test emergency pause
        guardian.emergency_pause_all(&"test emergency".into());
        assert!(all_contracts_paused(&env));
    }
}