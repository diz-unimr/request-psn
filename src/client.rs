use crate::config::{Gpas, Ttp};
use anyhow::anyhow;
use log::{error, info};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct TtpClient {
    client: Client,
    gpas: Gpas,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "resourceType", rename = "Parameters")]
struct Parameters {
    parameter: Vec<StringParameter>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StringParameter {
    name: String,
    value_string: String,
}

#[derive(Serialize, Deserialize)]
struct PsnResponse {
    parameter: Vec<Parameter>,
}

#[derive(Serialize, Deserialize)]
struct Parameter {
    name: String,
    part: Vec<IdentifierParameter>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentifierParameter {
    name: String,
    value_identifier: Identifier,
}

#[derive(Serialize, Deserialize)]
struct Identifier {
    system: String,
    value: String,
}

impl TtpClient {
    pub(crate) async fn is_healthy(&self) -> bool {
        self.test_connection().await.is_ok()
    }

    pub(crate) async fn new(config: &Ttp) -> Result<Self, anyhow::Error> {
        // default headers
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/fhir+json"),
        );

        // http client
        let client = Client::builder()
            .default_headers(headers.clone())
            .timeout(Duration::from_secs(config.timeout))
            .build()?;

        Ok(TtpClient {
            client,
            gpas: config.gpas.clone(),
        })
    }

    pub(crate) async fn test_connection(&self) -> anyhow::Result<()> {
        // test gpas
        self.get_metadata(format!("{}/ttp-fhir/fhir/gpas", self.gpas.base_url).as_str())
            .await?;
        info!("Connection test to gPAS successful");

        Ok(())
    }

    async fn get_metadata(&self, base_url: &str) -> anyhow::Result<()> {
        let metadata = format!("{}/metadata", base_url);
        match self.client.get(metadata.as_str()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow!(
                        "Metadata response returned error code: {}",
                        resp.status()
                    ))
                }
            }

            Err(e) => {
                error!("Connection failed, {e}");
                Err(e.into())
            }
        }
    }

    pub(crate) async fn pseudonymize(
        &self,
        domain: String,
        original: String,
    ) -> anyhow::Result<String> {
        // pseudonymize
        let body = Parameters {
            parameter: vec![
                StringParameter {
                    name: "target".to_string(),
                    value_string: domain,
                },
                StringParameter {
                    name: "original".to_string(),
                    value_string: original,
                },
            ],
        };

        let request = self
            .client
            .post(
                format!(
                    "{}/ttp-fhir/fhir/gpas/$pseudonymizeAllowCreate",
                    self.gpas.base_url
                )
                .as_str(),
            )
            .body(serde_json::to_string(&body)?);

        let response = request.send().await?;

        let bla = response.text().await?;

        let params: PsnResponse = serde_json::from_str(bla.as_str())
            .map_err(|e| anyhow!("Failed to parse gPAS response: {e}"))?;
        parse_pseudonym(params)
    }
}

fn parse_pseudonym(params: PsnResponse) -> anyhow::Result<String> {
    params
        .parameter
        .into_iter()
        .flat_map(|p| p.part)
        .find_map(|param| {
            if param.name == "pseudonym" {
                Some(param.value_identifier.value)
            } else {
                None
            }
        })
        .ok_or(anyhow!("Failed to parse pseudonym response"))
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::client::{Parameters, StringParameter, TtpClient};
    use crate::config::{AppConfig, Gpas, Ttp};
    use reqwest::header::CONTENT_TYPE;
    use serde_json::json;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    pub(crate) fn setup_config(base_url: String) -> AppConfig {
        AppConfig {
            ttp: Ttp {
                gpas: Gpas { base_url },
                timeout: 5,
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_connection_ok() {
        use httpmock::prelude::*;

        init();
        let server = MockServer::start();
        let gpas_metadata = server.mock(|when, then| {
            when.method(GET)
                .path("/ttp-fhir/fhir/gpas/metadata")
                .header_exists(CONTENT_TYPE.as_str());
            then.status(200).body("OK");
        });

        let config = setup_config(server.base_url());
        // create new client
        let client = TtpClient::new(&config.ttp).await;

        // connection test
        let test_result = client.unwrap().test_connection().await;

        // mocks were called once
        gpas_metadata.assert();

        // assert client is created and initialized
        assert!(test_result.is_ok());
    }

    #[tokio::test]
    async fn test_connection_error() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        let metadata_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/ttp-fhir/fhir/gpas/metadata")
                .header_exists(CONTENT_TYPE.as_str());
            then.status(404);
        });

        let config = setup_config(server.base_url());
        // create new client
        let client = TtpClient::new(&config.ttp).await;

        // connection test
        let test_result = client.unwrap().test_connection().await;

        // mock was called once
        metadata_mock.assert();

        // assert client connection test failed
        assert!(test_result.is_err());
    }

    #[test]
    fn test_parameters_serde() {
        let target = "test-domain";
        let original = "test-id";

        let params = Parameters {
            parameter: vec![
                StringParameter {
                    name: "target".to_string(),
                    value_string: target.to_string(),
                },
                StringParameter {
                    name: "original".to_string(),
                    value_string: original.to_string(),
                },
            ],
        };

        let expected = json!(
        {
          "resourceType": "Parameters",
          "parameter": [
            {
              "name": "target",
              "valueString": target
            },
            {
              "name": "original",
              "valueString": original
            }
          ]
        });

        let actual = serde_json::to_string(&params)
            .unwrap()
            .parse::<serde_json::Value>()
            .unwrap();

        assert_eq!(expected, actual);
    }
}
