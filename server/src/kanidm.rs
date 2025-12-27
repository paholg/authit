use eyre::{Result, WrapErr};
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use types::{Entry, Error, Group, Person, ResetLink};

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

    pub async fn list_persons(&self) -> Result<Vec<Person>, Error> {
        let url = format!("{}/v1/person", self.base_url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(self.token.expose_secret())
            .send()
            .await
            .wrap_err("failed to send request to Kanidm")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Kanidm API error ({}): {}", status, body);
            return Err(eyre::eyre!("Kanidm API error ({}): {}", status, body).into());
        }

        let entries: Vec<Entry> = response
            .json()
            .await
            .wrap_err("failed to parse Kanidm response")?;
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

    pub async fn get_person(&self, id: &str) -> Result<Person, Error> {
        let response = self
            .client
            .get(format!("{}/v1/person/{}", self.base_url, id))
            .bearer_auth(self.token.expose_secret())
            .send()
            .await
            .wrap_err("failed to send request to Kanidm")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(eyre::eyre!("Kanidm API error ({}): {}", status, body).into());
        }

        let entry: Entry = response
            .json()
            .await
            .wrap_err("failed to parse Kanidm response")?;
        Person::try_from(entry)
            .map_err(|e| eyre::eyre!("failed to parse person: {}", e).into())
    }

    pub async fn list_groups(&self) -> Result<Vec<Group>, Error> {
        let response = self
            .client
            .get(format!("{}/v1/group", self.base_url))
            .bearer_auth(self.token.expose_secret())
            .send()
            .await
            .wrap_err("failed to send request to Kanidm")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(eyre::eyre!("Kanidm API error ({}): {}", status, body).into());
        }

        let entries: Vec<Entry> = response
            .json()
            .await
            .wrap_err("failed to parse Kanidm response")?;
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

    pub async fn add_user_to_group(&self, group_id: &str, user_id: &str) -> Result<(), Error> {
        let response = self
            .client
            .post(format!(
                "{}/v1/group/{}/_attr/member",
                self.base_url, group_id
            ))
            .bearer_auth(self.token.expose_secret())
            .json(&vec![user_id])
            .send()
            .await
            .wrap_err("failed to send request to Kanidm")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(eyre::eyre!("Kanidm API error ({}): {}", status, body).into());
        }

        Ok(())
    }

    pub async fn remove_user_from_group(&self, group_id: &str, user_id: &str) -> Result<(), Error> {
        let response = self
            .client
            .delete(format!(
                "{}/v1/group/{}/_attr/member",
                self.base_url, group_id
            ))
            .bearer_auth(self.token.expose_secret())
            .json(&vec![user_id])
            .send()
            .await
            .wrap_err("failed to send request to Kanidm")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(eyre::eyre!("Kanidm API error ({}): {}", status, body).into());
        }

        Ok(())
    }

    pub async fn delete_person(&self, id: &str) -> Result<(), Error> {
        let response = self
            .client
            .delete(format!("{}/v1/person/{}", self.base_url, id))
            .bearer_auth(self.token.expose_secret())
            .send()
            .await
            .wrap_err("failed to send request to Kanidm")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Delete person failed ({}): {}", status, body);
            return Err(eyre::eyre!("Kanidm API error ({}): {}", status, body).into());
        }

        Ok(())
    }

    pub async fn create_person(
        &self,
        name: &str,
        display_name: &str,
        mail: Option<&str>,
    ) -> Result<(), Error> {
        let mut attrs = serde_json::json!({
            "attrs": {
                "name": [name],
                "displayname": [display_name],
            }
        });

        if let Some(email) = mail {
            attrs["attrs"]["mail"] = serde_json::json!([email]);
        }

        let response = self
            .client
            .post(format!("{}/v1/person", self.base_url))
            .bearer_auth(self.token.expose_secret())
            .json(&attrs)
            .send()
            .await
            .wrap_err("failed to send request to Kanidm")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Create person failed ({}): {}", status, body);
            return Err(eyre::eyre!("Kanidm API error ({}): {}", status, body).into());
        }

        Ok(())
    }

    pub async fn generate_credential_reset_link(&self, user_id: &str) -> Result<ResetLink, Error> {
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
            .await
            .wrap_err("failed to send request to Kanidm")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Credential reset failed ({}): {}", status, body);
            return Err(eyre::eyre!("Kanidm API error ({}): {}", status, body).into());
        }

        let body = response.text().await.wrap_err("failed to read response")?;
        tracing::info!("Credential reset response: {}", body);

        #[derive(serde::Deserialize)]
        struct TokenResponse {
            token: SecretString,
            expiry_time: u64,
        }

        let token_response: TokenResponse = serde_json::from_str(&body)
            .wrap_err_with(|| format!("failed to parse token response: {}", body))?;

        Ok(ResetLink {
            url: format!(
                "{}/ui/reset?token={}",
                self.base_url,
                token_response.token.expose_secret()
            ),
            expires_at: token_response.expiry_time,
        })
    }
}
