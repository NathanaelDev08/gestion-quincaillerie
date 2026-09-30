use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub company_name: String,
    pub company_siret: String,
    pub company_tva: String,
    pub company_address: String,
    pub company_city: String,
    pub company_postal: String,
    pub company_phone: String,
    pub company_email: String,
    pub default_tva: f64,
    pub currency: String,
    pub jwt_secret: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            company_name: "Ma Société".into(),
            company_siret: "".into(),
            company_tva: "".into(),
            company_address: "".into(),
            company_city: "".into(),
            company_postal: "".into(),
            company_phone: "".into(),
            company_email: "".into(),
            default_tva: 20.0,
            currency: "XOF".into(),
            jwt_secret: "change-me-in-production-32-chars-min".into(),
        }
    }
}
