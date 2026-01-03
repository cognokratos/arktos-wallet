# Regional Compliance Guide: Arktos Wallet

Arktos Wallet is designed with **compliance-first architecture** to support adaptation for regional and industry-specific regulations. This guide provides frameworks, patterns, and technical implementations for ensuring your Arktos deployment meets regulatory requirements.

## Overview: Compliance-Ready Architecture

Arktos's core design decisions support compliance requirements:

| Requirement | Arktos Support | Evidence |
|------------|----------------|----------|
| **Data Encryption at Rest** | ✅ AES-256 via SQLCipher | All sensitive data encrypted by default |
| **Audit Logging** | ✅ Comprehensive logging | Every wallet operation logged with timestamp, actor, action |
| **Access Control** | ✅ API key authentication + ownership model | Role-based access via MCP tokens |
| **Data Minimization** | ✅ Stateless design | No unnecessary data retention in memory |
| **Secure Communication** | ✅ TLS 1.2+ ready | HTTPS enforced in production |
| **Key Management** | ✅ Non-custodial model | System owner controls encryption keys |
| **Immutable Records** | ✅ SQLite ACID properties | Database transactions ensure consistency |

See [Architecture Document](./architecture.md) for technical details.

---

## 1. GDPR Compliance (Europe)

### Key GDPR Requirements

- **Right to be Forgotten** (Article 17): Ability to delete personal data
- **Data Portability** (Article 20): Export data in standard format
- **Lawful Basis**: Demonstrated justification for data processing
- **Privacy by Design**: Data protection integrated from start
- **Data Processing Agreement** (DPA): Contracts with data processors

### Arktos Implementation Patterns

#### 1.1 Data Minimization

Arktos collects only essential data for wallet management:

```
Stored Data:
- Wallet ID (user-generated)
- Encrypted mnemonic (necessary for wallet recovery)
- Account index (necessary for multi-account support)
- Public addresses (non-personal, derived data)

NOT Stored:
- User names or email addresses
- IP addresses or connection logs (unless audit required)
- Transaction history (not in scope of this service)
- Personal identification data
```

**Configuration for Data Minimization:**

```rust
// src/config/gdpr.rs
pub struct GDPRConfig {
    pub collect_ip_logs: bool,          // Default: false (disable unless needed)
    pub audit_log_retention_days: u32,  // Default: 90
    pub allow_data_export: bool,        // Default: true
    pub allow_deletion: bool,           // Default: true
}

impl GDPRConfig {
    pub fn strict_mode() -> Self {
        Self {
            collect_ip_logs: false,
            audit_log_retention_days: 30,
            allow_data_export: true,
            allow_deletion: true,
        }
    }
}
```

#### 1.2 Right to be Forgotten Implementation

Create a secure deletion mechanism:

```rust
// src/db/deletion.rs
pub async fn delete_wallet_data(wallet_id: &str, owner_id: &str) -> Result<(), Error> {
    // Verify ownership
    let wallet = db::get_wallet(wallet_id).await?;
    if wallet.owner_id != owner_id {
        return Err(Error::Unauthorized);
    }
    
    // Log deletion request for audit
    audit_log::log_deletion_request(wallet_id, owner_id).await?;
    
    // Securely overwrite mnemonic (multiple passes)
    db::securely_delete_mnemonic(wallet_id).await?;
    
    // Delete wallet record
    db::delete_wallet(wallet_id).await?;
    
    // Delete associated accounts
    db::delete_accounts(wallet_id).await?;
    
    // Log successful deletion
    audit_log::log_deletion_complete(wallet_id).await?;
    
    Ok(())
}

async fn securely_delete_mnemonic(wallet_id: &str) -> Result<(), Error> {
    // Overwrite with random data (Gutmann method)
    for _ in 0..35 {
        db::update_mnemonic(wallet_id, &random_data()).await?;
    }
    
    // Final overwrite with zeros
    db::update_mnemonic(wallet_id, &vec![0; 32]).await?;
}
```

#### 1.3 Data Portability

Implement secure data export:

```rust
// src/api/data_export.rs
pub async fn export_wallet_data(
    wallet_id: &str,
    owner_id: &str,
) -> Result<JsonValue, Error> {
    // Verify ownership
    let wallet = db::get_wallet(wallet_id).await?;
    if wallet.owner_id != owner_id {
        return Err(Error::Unauthorized);
    }
    
    // Prepare export (unencrypted - returned over HTTPS only)
    let export = json!({
        "wallet_id": wallet.id,
        "created_at": wallet.created_at,
        "accounts": db::get_accounts(wallet_id).await?,
        "public_addresses": {
            "bitcoin": wallet.bitcoin_address,
            "ethereum": wallet.ethereum_address,
        },
        "export_date": Utc::now().to_rfc3339(),
    });
    
    // Log export request
    audit_log::log_data_export(wallet_id, owner_id).await?;
    
    Ok(export)
}
```

#### 1.4 Privacy by Design Checklist

- [x] Data encrypted at rest (SQLCipher AES-256)
- [x] Minimal data collection
- [x] No tracking or profiling
- [x] Secure deletion implemented
- [x] Data export capability
- [x] TLS encryption in transit
- [x] Access control (ownership model)
- [x] Audit logging for accountability

### GDPR Deployment Checklist

- [ ] Privacy policy document addressing Article 13/14 requirements
- [ ] Data Processing Agreement (DPA) with any cloud providers
- [ ] Documented lawful basis for data processing
- [ ] Data retention policy (e.g., audit logs: 90 days)
- [ ] Incident response plan for data breaches
- [ ] Regular security audits and penetration testing
- [ ] DPIA (Data Protection Impact Assessment) if high risk
- [ ] Staff training on GDPR and data protection

---

## 2. HIPAA Compliance (Healthcare - USA)

### Key HIPAA Requirements

- **PHI Protection**: Safeguard Protected Health Information
- **Encryption**: Encryption for data at rest and in transit
- **Audit Controls**: Track all access and modifications
- **Access Controls**: Role-based access with authentication
- **Integrity Controls**: Ensure data hasn't been modified
- **Business Associate Agreements (BAAs)**: Contracts required

### Arktos Implementation Patterns

#### 2.1 Encryption Standards

HIPAA requires specific encryption standards:

```rust
// src/crypto/hipaa.rs
use aes_gcm::{Aes256Gcm, Key, Nonce};

pub struct HIPAAEncryption {
    // AES-256-GCM for HIPAA compliance
    cipher: Aes256Gcm,
}

impl HIPAAEncryption {
    pub fn new(key_material: &[u8; 32]) -> Self {
        let key = Key::<Aes256Gcm>::from_slice(key_material);
        Self {
            cipher: Aes256Gcm::new(key),
        }
    }
    
    pub fn encrypt_phi(&self, plaintext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>, Error> {
        // Use GCM for authenticated encryption
        self.cipher.encrypt(Nonce::from_slice(nonce), plaintext)
            .map_err(|_| Error::EncryptionFailed)
    }
    
    pub fn decrypt_phi(&self, ciphertext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>, Error> {
        // Verify authenticity during decryption
        self.cipher.decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| Error::DecryptionFailed)
    }
}
```

#### 2.2 Audit Logging (HIPAA-Compliant)

Comprehensive logging of all access to sensitive data:

```rust
// src/audit/hipaa_audit.rs
pub struct HIPAAAuditLog {
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub action: String,           // "CREATE_WALLET", "RETRIEVE_ADDRESS", etc.
    pub resource_id: String,       // Wallet ID or account ID
    pub result: String,            // "SUCCESS" or "FAILURE"
    pub ip_address: String,        // For tracking
    pub user_agent: String,        // Device/application info
}

pub async fn log_hipaa_access(
    user_id: &str,
    action: &str,
    resource_id: &str,
    ip_address: &str,
) -> Result<(), Error> {
    let log_entry = HIPAAAuditLog {
        timestamp: Utc::now(),
        user_id: user_id.to_string(),
        action: action.to_string(),
        resource_id: resource_id.to_string(),
        result: "SUCCESS".to_string(),
        ip_address: ip_address.to_string(),
        user_agent: "arktos-service".to_string(),
    };
    
    // Store in tamper-proof audit log
    db::insert_audit_log(&log_entry).await?;
    
    // Also stream to external SIEM if configured
    if let Some(siem_endpoint) = env::var("HIPAA_SIEM_ENDPOINT").ok() {
        stream_to_siem(&siem_endpoint, &log_entry).await?;
    }
    
    Ok(())
}
```

#### 2.3 Access Control (Role-Based)

Implement HIPAA role-based access control:

```rust
// src/auth/hipaa_roles.rs
pub enum HIPAARole {
    SystemOwner,     // Full access to all data
    Auditor,         // Read-only access to logs
    Operator,        // Limited operational access
    Technician,      // Maintenance access only
}

pub struct HIPAAAuthz {
    role: HIPAARole,
}

impl HIPAAAuthz {
    pub fn can_access_wallet(&self, wallet_id: &str) -> bool {
        match self.role {
            HIPAARole::SystemOwner => true,
            HIPAARole::Operator => true,      // With ownership check
            HIPAARole::Auditor => false,      // Read-only, no wallet access
            HIPAARole::Technician => false,   // No data access
        }
    }
    
    pub fn can_delete_wallet(&self) -> bool {
        matches!(self.role, HIPAARole::SystemOwner)
    }
}
```

#### 2.4 Business Associate Agreement (BAA) Requirement

If using third-party services:

```rust
// Config for HIPAA-compliant deployment
pub struct HIPAADeploymentConfig {
    pub storage_provider: String,           // "self-hosted", "aws-kms", etc.
    pub backup_provider: String,
    pub logging_provider: String,           // Must have BAA
    pub baa_signed: bool,                   // Enforce BAA check
}

impl HIPAADeploymentConfig {
    pub fn validate(&self) -> Result<(), Error> {
        if self.baa_signed {
            // Verify BAA is in place for all providers
            // This could call an external service to verify
            println!("⚠️ Ensure BAAs are signed with all service providers");
        }
        Ok(())
    }
}
```

### HIPAA Deployment Checklist

- [ ] AES-256-GCM encryption enabled for all data
- [ ] TLS 1.2+ for all communications (HTTPS enforced)
- [ ] Comprehensive audit logging implemented
- [ ] HIPAA-compliant access control (role-based)
- [ ] Disaster recovery and backup plan documented
- [ ] Security incident response procedure documented
- [ ] Business Associate Agreements (BAAs) signed
- [ ] Staff HIPAA training completed
- [ ] Penetration testing and security audit completed
- [ ] Breach notification procedure established

---

## 3. PCI DSS Compliance (Payment Card Industry)

### Key PCI DSS Requirements

- **Encryption**: Protect cardholder data in transit and at rest
- **Access Control**: Limit access by need-to-know
- **Monitoring**: Log all access to cardholder data
- **Vulnerability Management**: Regular assessments and patches
- **Testing**: Annual penetration testing required

### Note: Arktos is Non-Custodial

**Arktos does NOT store, process, or transmit payment card data.** Wallets are non-custodial—the system owner maintains complete control of private keys and funds.

However, if Arktos is integrated with systems that process payments, ensure:

```rust
// If integrating with payment systems, ensure separation:
pub mod payment_isolation {
    // Wallets and payment processing are completely isolated
    // No mixing of concerns
    
    pub struct PaymentGateway {
        // For payment processing - use PCI-compliant provider
    }
    
    pub struct NonCustodialWallet {
        // For Arktos wallet management - no payment data
    }
}
```

### PCI DSS Checklist for Integrated Systems

- [ ] Arktos wallet data is NOT mixed with payment card data
- [ ] If using payment processors: Use certified, PCI-compliant providers
- [ ] Encryption for any network traffic
- [ ] Access logs for compliance auditing
- [ ] Annual PCI DSS compliance assessment

---

## 4. SOC 2 Compliance (Service Organization Control)

### Key SOC 2 Requirements

- **Security**: Access controls, encryption, threat prevention
- **Availability**: System reliability and uptime
- **Processing Integrity**: Accurate and complete processing
- **Confidentiality**: Protection of confidential information
- **Privacy**: Personal information handling

### Arktos SOC 2 Implementation

#### 4.1 Security Controls

```rust
// src/compliance/soc2.rs
pub struct SOC2SecurityControls {
    // Control 1: Encryption at rest
    pub data_encryption_enabled: bool,
    
    // Control 2: Encryption in transit
    pub tls_minimum_version: String,
    
    // Control 3: Access controls
    pub mfa_enabled: bool,
    pub role_based_access: bool,
    
    // Control 4: Audit logging
    pub audit_logging_enabled: bool,
    pub log_retention_days: u32,
    
    // Control 5: Incident response
    pub incident_response_plan: String,
}

impl SOC2SecurityControls {
    pub fn production_defaults() -> Self {
        Self {
            data_encryption_enabled: true,
            tls_minimum_version: "1.2".to_string(),
            mfa_enabled: true,
            role_based_access: true,
            audit_logging_enabled: true,
            log_retention_days: 365,
            incident_response_plan: "in-place".to_string(),
        }
    }
}
```

#### 4.2 Availability Monitoring

```rust
// src/monitoring/soc2_availability.rs
pub async fn monitor_availability() {
    loop {
        let uptime = calculate_uptime().await;
        let target = 0.999; // 99.9% uptime
        
        if uptime < target {
            log::warn!("SLA breach: Uptime {:.3} below target", uptime);
            trigger_incident_response();
        }
        
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
```

#### 4.3 Processing Integrity Verification

```rust
// src/integrity/soc2_integrity.rs
pub async fn verify_processing_integrity(wallet_id: &str) -> Result<(), Error> {
    // Verify wallet data consistency
    let wallet = db::get_wallet(wallet_id).await?;
    
    // Checksum verification
    if wallet.checksum != calculate_checksum(&wallet) {
        log::error!("Data integrity check failed for wallet {}", wallet_id);
        return Err(Error::IntegrityViolation);
    }
    
    Ok(())
}
```

### SOC 2 Audit Preparation

- [ ] Complete security controls documentation
- [ ] Evidence collection (logs, configurations, policies)
- [ ] Access control testing
- [ ] Encryption verification
- [ ] Audit log completeness verification
- [ ] Incident response testing
- [ ] Staff access review
- [ ] Third-party vendor assessment

---

## 5. Custom Regional Compliance

### Framework: Building Custom Compliance Adapters

Arktos supports building custom compliance modules for local regulations:

```rust
// src/compliance/mod.rs
pub trait ComplianceAdapter {
    fn validate_data_handling(&self) -> Result<(), Error>;
    fn validate_encryption(&self) -> Result<(), Error>;
    fn validate_audit_logging(&self) -> Result<(), Error>;
    fn get_compliance_report(&self) -> ComplianceReport;
}

// Example: Hong Kong (Personal Data Privacy Ordinance - PDPO)
pub struct PDPOAdapter {
    // PDPO-specific configuration
}

impl ComplianceAdapter for PDPOAdapter {
    fn validate_data_handling(&self) -> Result<(), Error> {
        // Verify PDPO requirements
        // - Lawfulness and fairness
        // - Accuracy and retention
        // - Use limitation
        Ok(())
    }
    
    fn validate_encryption(&self) -> Result<(), Error> {
        // PDPO doesn't mandate encryption, but recommends strong measures
        Ok(())
    }
    
    fn validate_audit_logging(&self) -> Result<(), Error> {
        // PDPO requires logging of data access
        Ok(())
    }
    
    fn get_compliance_report(&self) -> ComplianceReport {
        // Generate compliance certificate
        ComplianceReport::default()
    }
}
```

### Regional Compliance Examples

| Region | Regulation | Key Requirements |
|--------|-----------|------------------|
| **EU** | GDPR | Data minimization, deletion rights, portability |
| **USA/Healthcare** | HIPAA | Encryption, audit logs, access controls |
| **USA/Finance** | PCI DSS | Cardholder data protection (if integrated) |
| **Singapore** | PDPA | Consent, accuracy, protection, correction, transfer |
| **UK** | Data Protection Act 2018 | Similar to GDPR |
| **Canada** | PIPEDA | Consent, accuracy, security, access, retention |
| **Australia** | Privacy Act | APPs (Australian Privacy Principles) |
| **Brazil** | LGPD | Similar to GDPR, strict consent requirements |

---

## 6. Encryption Key Management

### Arktos Key Management Architecture

```
┌─────────────────────────────────────────────┐
│ System Owner (Outside Arktos)               │
│ - Generates and manages master key         │
│ - Controls key storage (HSM/Vault)         │
│ - Performs key rotation                    │
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ Arktos Wallet Service                       │
│ - Receives encrypted data                   │
│ - Uses temporary session keys               │
│ - Never stores master key                   │
│ - Performs encryption/decryption operations│
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ Encrypted Database (SQLCipher)              │
│ - All data at rest encrypted with AES-256  │
│ - Encryption key managed by system owner   │
└─────────────────────────────────────────────┘
```

### Implementation: Key Rotation

```rust
// src/crypto/key_rotation.rs
pub async fn rotate_encryption_key(
    old_key: &[u8; 32],
    new_key: &[u8; 32],
) -> Result<(), Error> {
    // 1. Verify you have proper authorization
    verify_rotation_authorization().await?;
    
    // 2. Create backup of encrypted data
    db::backup_database().await?;
    
    // 3. Re-encrypt all wallet data with new key
    let wallets = db::get_all_wallets().await?;
    for wallet in wallets {
        let decrypted = decrypt_mnemonic(&wallet.encrypted_mnemonic, old_key)?;
        let re_encrypted = encrypt_mnemonic(&decrypted, new_key)?;
        db::update_encrypted_mnemonic(&wallet.id, &re_encrypted).await?;
    }
    
    // 4. Verify all data is correctly re-encrypted
    verify_encryption_integrity(new_key).await?;
    
    // 5. Log key rotation for audit
    audit_log::log_key_rotation().await?;
    
    Ok(())
}
```

---

## 7. Compliance Validation & Testing

### Automated Compliance Checks

```rust
// src/compliance/validation.rs
pub async fn run_compliance_checks(config: &Config) -> Result<ComplianceReport, Error> {
    let mut report = ComplianceReport::default();
    
    // Encryption checks
    if !verify_encryption_enabled().await? {
        report.add_failure("Encryption not enabled");
    }
    
    // Access control checks
    if !verify_access_controls().await? {
        report.add_failure("Access controls not properly configured");
    }
    
    // Audit logging checks
    if !verify_audit_logging().await? {
        report.add_failure("Audit logging not functioning");
    }
    
    // TLS checks
    if !verify_tls_version("1.2").await? {
        report.add_failure("TLS version below 1.2");
    }
    
    // Backup/recovery checks
    if !verify_backup_integrity().await? {
        report.add_failure("Backup integrity issues detected");
    }
    
    Ok(report)
}

// Run on startup
#[tokio::main]
async fn main() {
    let config = Config::from_env();
    
    if let Err(e) = run_compliance_checks(&config).await {
        eprintln!("⚠️ Compliance check failed: {}", e);
        // Decide: log and continue, or fail startup
    }
    
    // Start server...
}
```

### Integration Testing for Compliance

```rust
#[cfg(test)]
mod compliance_tests {
    use super::*;

    #[tokio::test]
    async fn test_encryption_at_rest() {
        // Verify data stored in database is encrypted
        let plaintext = "sensitive data";
        db::store_encrypted(plaintext).await.unwrap();
        
        let raw_bytes = db::read_raw_bytes().await.unwrap();
        assert_ne!(raw_bytes, plaintext.as_bytes());
    }

    #[tokio::test]
    async fn test_audit_logging() {
        // Verify audit logs are created for sensitive operations
        let wallet_id = "test_wallet";
        
        // Perform operation
        wallet::create_wallet(wallet_id).await.unwrap();
        
        // Check audit log
        let logs = audit_log::get_logs(wallet_id).await.unwrap();
        assert!(!logs.is_empty());
        assert_eq!(logs[0].action, "CREATE_WALLET");
    }

    #[tokio::test]
    async fn test_gdpr_right_to_deletion() {
        let wallet_id = "test_wallet";
        
        // Create and then delete
        wallet::create_wallet(wallet_id).await.unwrap();
        wallet::delete_wallet(wallet_id).await.unwrap();
        
        // Verify complete deletion
        let result = db::get_wallet(wallet_id).await;
        assert!(result.is_err());
    }
}
```

---

## 8. Compliance Deployment Checklist

### Pre-Deployment

- [ ] Compliance requirements identified and documented
- [ ] Applicable regulations confirmed (GDPR, HIPAA, etc.)
- [ ] Compliance controls implemented in code
- [ ] Security audit completed
- [ ] Penetration testing performed
- [ ] Encryption key management plan documented
- [ ] Data retention policy established
- [ ] Incident response plan created

### Infrastructure

- [ ] TLS 1.2+ configured for all endpoints
- [ ] Database encryption enabled (SQLCipher configured)
- [ ] Audit logging system operational
- [ ] Backup and recovery procedures tested
- [ ] Monitoring and alerting configured
- [ ] Firewall and network security in place

### Operational

- [ ] Staff training on compliance requirements completed
- [ ] Compliance documentation stored securely
- [ ] Regular compliance audits scheduled
- [ ] Key rotation procedures established
- [ ] Incident response team identified
- [ ] Third-party vendor compliance verified (if applicable)

### Ongoing

- [ ] Monthly compliance validation runs
- [ ] Quarterly security reviews
- [ ] Annual compliance audit
- [ ] Continuous monitoring and alerting
- [ ] Regular security patches and updates

---

## Support & Resources

### Internal References

- [Architecture Document](./architecture.md) - Technical implementation details
- [Development Guide](./development-guide.md) - How to implement compliance features
- [Deployment Guide](./deployment-guide.md) - Production deployment standards

### External Resources

- **GDPR**: https://gdpr-info.eu/
- **HIPAA**: https://www.hhs.gov/hipaa/
- **PCI DSS**: https://www.pcisecuritystandards.org/
- **SOC 2**: https://www.aicpa.org/interestareas/informationmanagement/assurance/aicpasoc2report

### Compliance Consultation

For specific regional requirements not covered here:
1. Consult legal counsel on applicable regulations
2. Engage compliance specialists for your jurisdiction
3. Consider working with compliance auditors familiar with Arktos/Rust

---

## Conclusion

Arktos Wallet's architecture provides strong foundations for compliance. Customize and extend these patterns for your specific regional and organizational requirements. Remember: **compliance is ongoing, not a one-time implementation.**
