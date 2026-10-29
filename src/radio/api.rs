use anyhow::{Context, Result, bail};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone)]
pub struct Api {
    client: reqwest::Client,
    endpoint: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub username: String,
    pub display_name: String,
}

// Intentionally no Debug implementations on credential-bearing responses.
#[derive(Deserialize)]
pub struct Login {
    pub token: Option<String>,
    pub user: Option<User>,
    #[serde(default)]
    pub mfa: bool,
}

#[derive(Deserialize)]
struct Response<T> {
    data: Option<T>,
    errors: Option<Vec<ApiError>>,
}
#[derive(Deserialize)]
struct ApiError {
    message: String,
}

impl Api {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent(concat!("LISTEN.moe Desktop/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(20))
                .build()?,
            endpoint: "https://listen.moe/graphql".into(),
        })
    }

    async fn request<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: Value,
        token: Option<&str>,
    ) -> Result<T> {
        let mut request = self
            .client
            .post(&self.endpoint)
            .json(&json!({"query":query,"variables":variables}));
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response: Response<T> = request
            .send()
            .await
            .context("Cannot reach LISTEN.moe")?
            .error_for_status()
            .context("LISTEN.moe request failed")?
            .json()
            .await
            .context("Invalid API response")?;
        if let Some(error) = response.errors.and_then(|errors| errors.into_iter().next()) {
            bail!("{}", error.message);
        }
        response.data.context("The API returned no data")
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<Login> {
        #[derive(Deserialize)]
        struct Data {
            login: Login,
        }
        let result: Data = self.request(
            "mutation($username:String!,$password:String!){login(username:$username,password:$password){token mfa user{username displayName}}}",
            json!({"username":username,"password":password}), None).await?;
        Ok(result.login)
    }

    pub async fn verify_mfa(&self, temporary_token: &str, code: &str) -> Result<Login> {
        #[derive(Deserialize)]
        struct Data {
            #[serde(rename = "loginMFA")]
            login: Login,
        }
        // v7 sends the temporary token as Bearer auth and the TOTP code as token.
        let result: Data = self.request(
            "mutation($token:String!){loginMFA(token:$token){token user{username displayName}}}",
            json!({"token":code}), Some(temporary_token)).await?;
        Ok(result.login)
    }

    pub async fn user(&self, token: &str) -> Result<User> {
        #[derive(Deserialize)]
        struct Data {
            user: User,
        }
        let data: Data = self
            .request(
                "query{user(username:\"@me\"){username displayName}}",
                json!({}),
                Some(token),
            )
            .await?;
        Ok(data.user)
    }

    pub async fn is_favorite(&self, token: &str, song_id: u64) -> Result<bool> {
        #[derive(Deserialize)]
        struct Data {
            #[serde(rename = "checkFavorite")]
            favorites: Vec<u64>,
        }
        let data: Data = self
            .request(
                "query($songs:[Int!]!){checkFavorite(songs:$songs)}",
                json!({"songs":[song_id]}),
                Some(token),
            )
            .await?;
        Ok(data.favorites.contains(&song_id))
    }

    pub async fn toggle_favorite(&self, token: &str, song_id: u64) -> Result<()> {
        let _: Value = self
            .request(
                "mutation($id:Int!){favoriteSong(id:$id){id}}",
                json!({"id":song_id}),
                Some(token),
            )
            .await?;
        Ok(())
    }

    pub async fn cover(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?.error_for_status()?;
        const MAX_COVER_BYTES: usize = 8 * 1024 * 1024;
        let mut bytes = Vec::new();
        let mut response = response;
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > MAX_COVER_BYTES {
                bail!("Album artwork is too large");
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn server(response: &'static str) -> (Api, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/graphql", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let read = stream.read(&mut buffer).unwrap();
                assert!(read > 0);
                request.extend_from_slice(&buffer[..read]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(str::to_owned)
                        })
                        .unwrap()
                        .parse()
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
            String::from_utf8(request).unwrap()
        });
        let mut api = Api::new().unwrap();
        api.endpoint = endpoint;
        (api, worker)
    }

    #[tokio::test]
    async fn mfa_sends_temporary_session_as_bearer_and_code_as_variable() {
        let (api, request) = server(
            r#"{"data":{"loginMFA":{"token":"new-session","user":{"username":"listener","displayName":"Listener"}}}}"#,
        );
        let result = api.verify_mfa("temporary-session", "123456").await.unwrap();
        assert_eq!(result.token.as_deref(), Some("new-session"));
        let request = request.join().unwrap();
        let (headers, body) = request.split_once("\r\n\r\n").unwrap();
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("authorization: bearer temporary-session")
        );
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["variables"]["token"], "123456");
    }

    #[tokio::test]
    async fn failed_graphql_response_cannot_be_treated_as_success() {
        let (api, request) =
            server(r#"{"data":null,"errors":[{"message":"Invalid credentials"}]}"#);
        let result = api.login("listener", "test-password").await;
        assert_eq!(result.err().unwrap().to_string(), "Invalid credentials");
        request.join().unwrap();
    }

    #[tokio::test]
    async fn favorite_status_is_based_on_membership_not_nonempty_response() {
        let (api, request) = server(r#"{"data":{"checkFavorite":[7]}}"#);
        assert!(!api.is_favorite("test-session", 42).await.unwrap());
        request.join().unwrap();
    }
}
