use types::{Entry, Group, Person};
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};

#[derive(Clone)]
pub struct KanidmClient {
    client: Client,
    base_url: String,
    token: SecretString,
}

impl KanidmClient {
    pub fn new(base_url: String, token: SecretString) -> Self {
        Self {
            client: Client::new(),
            base_url,
            token,
        }
    }

    pub async fn list_persons(&self) -> Result<Vec<Person>, KanidmError> {
        let url = format!("{}/v1/person", self.base_url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(self.token.expose_secret())
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Kanidm API error ({}): {}", status, body);
            return Err(KanidmError::ApiError(format!("{}: {}", status, body)));
        }

        let entries: Vec<Entry> = response.json().await?;
        tracing::debug!("Raw person entries: {:?}", entries);

        let persons: Vec<Person> = entries
            .into_iter()
            .filter_map(|e| Person::try_from(e).ok())
            .collect();

        if let Some(first) = persons.first() {
            tracing::info!(
                "Sample person '{}' memberof groups: {:?}",
                first.name,
                first.groups
            );
        }

        Ok(persons)
    }

    pub async fn get_person(&self, id: &str) -> Result<Person, KanidmError> {
        let response = self
            .client
            .get(format!("{}/v1/person/{}", self.base_url, id))
            .bearer_auth(self.token.expose_secret())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(KanidmError::ApiError(format!("{}: {}", status, body)));
        }

        let entry: Entry = response.json().await?;
        Person::try_from(entry).map_err(|e| KanidmError::ParseError(e.to_string()))
    }

    pub async fn list_groups(&self) -> Result<Vec<Group>, KanidmError> {
        let response = self
            .client
            .get(format!("{}/v1/group", self.base_url))
            .bearer_auth(self.token.expose_secret())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(KanidmError::ApiError(format!("{}: {}", status, body)));
        }

        let entries: Vec<Entry> = response.json().await?;
        tracing::debug!("Raw group entries: {:?}", entries);

        let groups: Vec<Group> = entries
            .into_iter()
            .filter_map(|e| Group::try_from(e).ok())
            .collect();

        tracing::info!("Loaded {} groups", groups.len());
        if let Some(first) = groups.first() {
            tracing::info!("Sample group: name='{}', uuid='{}'", first.name, first.uuid);
        }
        Ok(groups)
    }

    pub async fn add_user_to_group(
        &self,
        group_id: &str,
        user_id: &str,
    ) -> Result<(), KanidmError> {
        let response = self
            .client
            .post(format!(
                "{}/v1/group/{}/_attr/member",
                self.base_url, group_id
            ))
            .bearer_auth(self.token.expose_secret())
            .json(&vec![user_id])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(KanidmError::ApiError(format!("{}: {}", status, body)));
        }

        Ok(())
    }

    pub async fn remove_user_from_group(
        &self,
        group_id: &str,
        user_id: &str,
    ) -> Result<(), KanidmError> {
        let response = self
            .client
            .delete(format!(
                "{}/v1/group/{}/_attr/member",
                self.base_url, group_id
            ))
            .bearer_auth(self.token.expose_secret())
            .json(&vec![user_id])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(KanidmError::ApiError(format!("{}: {}", status, body)));
        }

        Ok(())
    }

    pub async fn generate_credential_reset_link(
        &self,
        user_id: &str,
    ) -> Result<String, KanidmError> {
        let url = format!(
            "{}/v1/person/{}/_credential/_update_intent",
            self.base_url, user_id
        );
        tracing::info!("Generating credential reset link for user: {}", user_id);

        let response = self
            .client
            .get(&url)
            .bearer_auth(self.token.expose_secret())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Credential reset failed ({}): {}", status, body);
            return Err(KanidmError::ApiError(format!("{}: {}", status, body)));
        }

        let body = response.text().await?;
        tracing::info!("Credential reset response: {}", body);

        #[derive(serde::Deserialize)]
        struct Token {
            token: SecretString,
            #[allow(dead_code)]
            expiry_time: u64,
        }

        let reset_token: Token = serde_json::from_str(&body).map_err(|e| {
            KanidmError::ParseError(format!("Failed to parse token: {} - body: {}", e, body))
        })?;

        Ok(format!(
            "{}/ui/reset?token={}",
            self.base_url,
            reset_token.token.expose_secret()
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KanidmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}
