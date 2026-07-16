use uuid::Uuid;

use crate::domain::ids::PrincipalId;

/// Provisioning authorization configuration.
#[derive(Debug, Clone)]
pub struct ProvisioningConfig {
    allowlist: Vec<PrincipalId>,
}

impl ProvisioningConfig {
    /// Create a configuration with an explicit allow-list (primarily tests).
    pub fn new(allowlist: Vec<PrincipalId>) -> Self {
        Self { allowlist }
    }

    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var("WORKFLOW_PROVISIONING_PRINCIPAL_IDS").unwrap_or_default();
        if raw.is_empty() {
            return Err("WORKFLOW_PROVISIONING_PRINCIPAL_IDS is required".to_string());
        }
        let allowlist = raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                Uuid::parse_str(value)
                    .map(PrincipalId::from_uuid)
                    .map_err(|_| {
                        format!("invalid UUID in WORKFLOW_PROVISIONING_PRINCIPAL_IDS: {value}")
                    })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if allowlist.is_empty() {
            return Err(
                "WORKFLOW_PROVISIONING_PRINCIPAL_IDS must contain at least one UUID".to_string(),
            );
        }
        Ok(Self { allowlist })
    }

    pub fn is_allowed(&self, principal_id: &PrincipalId) -> bool {
        self.allowlist.contains(principal_id)
    }
}
