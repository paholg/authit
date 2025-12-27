use jiff::Timestamp;
use reqwest::{Client, Method, RequestBuilder, Url};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde_json::json;
use types::{
    ResetLink, Result,
    kanidm::{Group, Person, RawGroup, RawPerson},
};
use uuid::Uuid;

trait ReqwestExt {
    async fn try_send<T: DeserializeOwned>(self) -> Result<T>;
}

impl ReqwestExt for RequestBuilder {
    async fn try_send<T: DeserializeOwned>(self) -> Result<T> {
        let response = self.send().await?.error_for_status()?;
        let body = response.bytes().await?;

        match serde_json::from_slice(&body) {
            Ok(r) => Ok(r),
            Err(error) => {
                let body = String::from_utf8_lossy(&body);
                // NOTE: We don't want to log these responses in production, but
                // they can be useful for debugging.
                // tracing::debug!(?error, ?body, "failed to parse response");
                Err(error.into())
            }
        }
    }
}

#[derive(Clone)]
pub struct KanidmClient {
    client: Client,
    base_url: Url,
    token: SecretString,
}

impl KanidmClient {
    pub fn new(base_url: Url, token: SecretString) -> Self {
        Self {
            client: Client::new(),
            base_url,
            token,
        }
    }

    fn request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let url = self.base_url.join(path)?;

        Ok(self
            .client
            .request(method, url)
            .bearer_auth(self.token.expose_secret()))
    }

    fn get(&self, path: impl AsRef<str>) -> Result<RequestBuilder> {
        self.request(Method::GET, path.as_ref())
    }

    fn post(&self, path: impl AsRef<str>) -> Result<RequestBuilder> {
        self.request(Method::POST, path.as_ref())
    }

    fn delete(&self, path: impl AsRef<str>) -> Result<RequestBuilder> {
        self.request(Method::DELETE, path.as_ref())
    }

    pub async fn list_persons(&self) -> Result<Vec<Person>> {
        self.get("/v1/person")?
            .try_send::<Vec<RawPerson>>()
            .await?
            .into_iter()
            .map(Person::try_from)
            .collect()
    }

    pub async fn get_person(&self, id_or_name: &str) -> Result<Person> {
        self.get(format!("/v1/person/{}", id_or_name))?
            .try_send::<RawPerson>()
            .await?
            .try_into()
    }

    pub async fn list_groups(&self) -> Result<Vec<Group>> {
        self.get("/v1/group")?
            .try_send::<Vec<RawGroup>>()
            .await?
            .into_iter()
            .map(Group::try_from)
            .collect()
    }

    pub async fn add_user_to_group(&self, group_id: &Uuid, user_id: &Uuid) -> Result<()> {
        self.post(format!("/v1/group/{group_id}/_attr/member"))?
            .json(&vec![user_id])
            .try_send()
            .await
    }

    pub async fn remove_user_from_group(&self, group_id: &Uuid, user_id: &Uuid) -> Result<()> {
        self.delete(format!("/v1/group/{group_id}/_attr/member"))?
            .json(&vec![user_id])
            .try_send()
            .await
    }

    pub async fn delete_person(&self, user_id: &Uuid) -> Result<()> {
        self.delete(format!("/v1/person/{user_id}"))?
            .try_send()
            .await
    }

    pub async fn create_person(
        &self,
        user_name: &str,
        display_name: &str,
        email_address: &str,
    ) -> Result<()> {
        self.post("/v1/person")?
            .json(&json!({
                "attrs": {
                    "name": [user_name],
                    "displayname": [display_name],
                    "mail": [email_address]
                }
            }))
            .try_send()
            .await
    }

    pub async fn generate_credential_reset_link(&self, user_id: &Uuid) -> Result<ResetLink> {
        #[derive(serde::Deserialize)]
        struct TokenResponse {
            token: String,
            expiry_time: i64,
        }

        let response: TokenResponse = self
            .get(format!("/v1/person/{user_id}/_credential/_update_intent"))?
            .try_send()
            .await?;

        let mut url = self.base_url.join("/ui/reset")?;
        url.query_pairs_mut().append_pair("token", &response.token);

        Ok(ResetLink {
            url,
            expires_at: Timestamp::new(response.expiry_time, 0)?,
        })
    }
}
