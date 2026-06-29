use super::DatabaseAdapter;
use crate::types::{MetadataRequest, MetadataType, QueryResult};
use anyhow::Result;
use async_trait::async_trait;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::{DigitallySignedStruct, SignatureScheme};
use serde_json::{Map, Value};
use std::sync::Arc;
use tokio_postgres::config::SslMode;
use tokio_postgres::{Client, Config as PgConfig, NoTls, Row};

#[derive(Debug, PartialEq, Eq)]
enum SslPlan {
    Disable,
    Encrypt { mode: SslMode, verify: bool },
}

pub struct PostgresAdapter {
    url: String,
    client: Option<Client>,
}

impl PostgresAdapter {
    pub fn new(url: String) -> Self {
        Self { url, client: None }
    }

    async fn query(&mut self, command: &str) -> Result<QueryResult> {
        self.connect().await?;
        let rows = self.client.as_ref().unwrap().query(command, &[]).await?;
        let fields = rows
            .first()
            .map(|row| {
                row.columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let values = rows.iter().map(row_to_json).collect::<Vec<_>>();
        Ok(QueryResult {
            row_count: Some(values.len() as u64),
            rows: values,
            fields: Some(fields),
        })
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    async fn connect(&mut self) -> Result<()> {
        if self.client.is_some() {
            return Ok(());
        }

        let (plan, url) = plan_tls(&self.url)?;
        let client = match plan {
            SslPlan::Disable => {
                let (client, connection) = tokio_postgres::connect(&url, NoTls).await?;
                tokio::spawn(async move {
                    let _ = connection.await;
                });
                client
            }
            SslPlan::Encrypt { mode, verify } => {
                let mut config: PgConfig = url.parse()?;
                config.ssl_mode(mode);
                let (client, connection) = config.connect(make_tls(verify)?).await?;
                tokio::spawn(async move {
                    let _ = connection.await;
                });
                client
            }
        };
        self.client = Some(client);
        Ok(())
    }
    async fn disconnect(&mut self) -> Result<()> {
        self.client = None;
        Ok(())
    }
    async fn test(&mut self) -> Result<()> {
        self.execute("select 1").await.map(|_| ())
    }
    async fn execute(&mut self, command: &str) -> Result<QueryResult> {
        self.query(command).await
    }
    async fn metadata(&mut self, request: MetadataRequest) -> Result<QueryResult> {
        match request.request_type {
            MetadataType::Tables => self.query("select table_schema, table_name from information_schema.tables where table_type = 'BASE TABLE' and table_schema not in ('pg_catalog', 'information_schema') order by table_schema, table_name").await,
            MetadataType::Columns => {
                let table = request.table.ok_or_else(|| anyhow::anyhow!("columns 元信息查询必须提供 --table"))?.replace('\'', "''");
                self.query(&format!("select table_schema, table_name, column_name, data_type from information_schema.columns where table_name = '{}' order by ordinal_position", table)).await
            }
            _ => anyhow::bail!("当前数据库不支持元信息类型: {:?}", request.request_type),
        }
    }
}

fn plan_tls(url: &str) -> Result<(SslPlan, String)> {
    let mut parsed = url::Url::parse(url)?;
    let mut sslmode = None;
    let kept = parsed
        .query_pairs()
        .filter_map(|(key, value)| {
            if key.eq_ignore_ascii_case("sslmode") {
                sslmode = Some(value.to_string());
                None
            } else {
                Some((key.to_string(), value.to_string()))
            }
        })
        .collect::<Vec<_>>();

    parsed.set_query(None);
    if !kept.is_empty() {
        let mut pairs = parsed.query_pairs_mut();
        for (key, value) in kept {
            pairs.append_pair(&key, &value);
        }
    }

    // sslmode 兼容 libpq：默认 prefer；require 只加密不校验证书；verify-* 才校验证书。
    let plan = match sslmode.as_deref().map(str::to_ascii_lowercase).as_deref() {
        Some("disable") => SslPlan::Disable,
        Some("require") => SslPlan::Encrypt {
            mode: SslMode::Require,
            verify: false,
        },
        Some("verify-ca") | Some("verify-full") => SslPlan::Encrypt {
            mode: SslMode::Require,
            verify: true,
        },
        _ => SslPlan::Encrypt {
            mode: SslMode::Prefer,
            verify: false,
        },
    };
    Ok((plan, parsed.to_string()))
}

fn make_tls(verify: bool) -> Result<tokio_postgres_rustls::MakeRustlsConnect> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|error| anyhow::anyhow!("构建 PostgreSQL TLS 配置失败: {error}"))?;
    let config = if verify {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        builder.with_root_certificates(roots).with_no_client_auth()
    } else {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertVerification(provider)))
            .with_no_client_auth()
    };
    Ok(tokio_postgres_rustls::MakeRustlsConnect::new(config))
}

#[derive(Debug)]
struct NoCertVerification(Arc<rustls::crypto::CryptoProvider>);

impl ServerCertVerifier for NoCertVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn row_to_json(row: &Row) -> Value {
    let mut object = Map::new();
    for (index, column) in row.columns().iter().enumerate() {
        let value = cell_to_json(row, index);
        object.insert(column.name().to_string(), value);
    }
    Value::Object(object)
}

fn cell_to_json(row: &Row, index: usize) -> Value {
    if let Ok(value) = row.try_get::<_, Option<String>>(index) {
        return value.map(Value::String).unwrap_or(Value::Null);
    }
    if let Ok(value) = row.try_get::<_, Option<i64>>(index) {
        return value
            .map(|v| Value::Number(v.into()))
            .unwrap_or(Value::Null);
    }
    if let Ok(value) = row.try_get::<_, Option<i32>>(index) {
        return value
            .map(|v| Value::Number(v.into()))
            .unwrap_or(Value::Null);
    }
    if let Ok(value) = row.try_get::<_, Option<f64>>(index) {
        return value
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(value) = row.try_get::<_, Option<bool>>(index) {
        return value.map(Value::Bool).unwrap_or(Value::Null);
    }
    Value::String("<unsupported>".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "postgres://postgres@db.example.com:5432/app";

    #[test]
    fn default_prefers_tls_without_verification() {
        let (plan, url) = plan_tls(BASE).unwrap();
        assert_eq!(
            plan,
            SslPlan::Encrypt {
                mode: SslMode::Prefer,
                verify: false
            }
        );
        assert_eq!(url, BASE);
    }

    #[test]
    fn disable_uses_plaintext() {
        let (plan, url) = plan_tls(&format!("{BASE}?sslmode=disable")).unwrap();
        assert_eq!(plan, SslPlan::Disable);
        assert!(!url.contains("sslmode"));
    }

    #[test]
    fn require_forces_tls_without_verification() {
        let (plan, url) =
            plan_tls(&format!("{BASE}?sslmode=require&application_name=cli")).unwrap();
        assert_eq!(
            plan,
            SslPlan::Encrypt {
                mode: SslMode::Require,
                verify: false
            }
        );
        assert!(url.contains("application_name=cli"));
        assert!(!url.contains("sslmode"));
    }

    #[test]
    fn verify_modes_enable_certificate_verification() {
        for mode in ["verify-ca", "verify-full"] {
            let (plan, _) = plan_tls(&format!("{BASE}?sslmode={mode}")).unwrap();
            assert_eq!(
                plan,
                SslPlan::Encrypt {
                    mode: SslMode::Require,
                    verify: true
                }
            );
        }
    }

    #[test]
    fn sslmode_is_case_insensitive() {
        let (plan, _) = plan_tls(&format!("{BASE}?sslmode=DISABLE")).unwrap();
        assert_eq!(plan, SslPlan::Disable);
    }
}
