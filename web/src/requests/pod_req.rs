use std::collections::HashMap;
use anyhow::Error;
use gloo_net::http::Request;
use serde::{Deserialize, Deserializer, Serialize};
use web_sys::console;


fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
{
    let value: i32 = Deserialize::deserialize(deserializer)?;
    Ok(value != 0)
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct VerifyResponse {
    status: String,
    retrieved_id: i32,
}
pub async fn call_verify_pinepods(server_name: String, api_key: Option<String>) -> Result<String, anyhow::Error> {
    let url = format!("{}/api/data/get_user/", server_name);
    let api_key_ref = api_key.as_deref().ok_or_else(|| Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if response.ok() {
        let response_body = response.json::<VerifyResponse>().await?;
        Ok(response_body.status)
    } else {
        console::log_1(&format!("Error adding podcast: {}", response.status_text()).into());
        Err(Error::msg(format!("Error logging in. Is the server reachable? Server Response: {}", response.status_text())))
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[allow(non_snake_case)]
pub struct Episode {
    pub PodcastName: String,
    pub EpisodeTitle: String,
    pub EpisodePubDate: String,
    pub EpisodeDescription: String,
    pub EpisodeArtwork: String,
    pub EpisodeURL: String,
    pub EpisodeDuration: i32,
    pub ListenDuration: Option<String>,
    pub EpisodeID: i32,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct RecentEps {
    pub episodes: Option<Vec<Episode>>,
}

pub async fn call_get_recent_eps(server_name: &String, api_key: &Option<String>, user_id: &i32) -> Result<Vec<Episode>, anyhow::Error> {
    let url = format!("{}/api/data/return_episodes/{}", server_name, user_id);

    console::log_1(&format!("URL: {}", url).into());

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key.as_deref().ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;
    if !response.ok() {
        return Err(anyhow::Error::msg(format!("Failed to fetch episodes: {}", response.status_text())));
    }

    console::log_1(&format!("HTTP Response Status: {}", response.status()).into());
    
    // First, capture the response text for diagnostic purposes
    let response_text = response.text().await.unwrap_or_else(|_| "Failed to get response text".to_string());
    console::log_1(&format!("HTTP Response Body: {}", response_text).into());

    // Try to deserialize the response text
    match serde_json::from_str::<RecentEps>(&response_text) {
        Ok(response_body) => {
            console::log_1(&format!("Deserialized Response Body: {:?}", response_body).into());
            Ok(response_body.episodes.unwrap_or_else(Vec::new))
        }
        Err(e) => {
            console::log_1(&format!("Deserialization Error: {:?}", e).into());
            Err(anyhow::Error::msg("Failed to deserialize response"))
        }
    }
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PodcastValues {
    pub pod_title: String,
    pub pod_artwork: String,
    pub pod_author: String,
    pub categories: HashMap<String, String>,
    pub pod_description: String,
    pub pod_episode_count: i32,
    pub pod_feed_url: String,
    pub pod_website: String,
    pub pod_explicit: bool,
    pub user_id: i32
}

#[derive(serde::Deserialize)]
struct PodcastStatusResponse {
    success: bool,
    // Include other fields if your response contains more data
}

pub async fn call_add_podcast(server_name: &str, api_key: &Option<String>, _user_id: i32, added_podcast: &PodcastValues) -> Result<bool, Error> {
    let url = format!("{}/api/data/add_podcast/", server_name);
    let api_key_ref = api_key.as_deref().ok_or_else(|| Error::msg("API key is missing"))?;

    // Serialize `added_podcast` into JSON
    let json_body = serde_json::to_string(added_podcast)?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(json_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body = response.json::<PodcastStatusResponse>().await?;
        Ok(response_body.success)
    } else {
        console::log_1(&format!("Error adding podcast: {}", response.status_text()).into());
        Err(Error::msg(format!("Error adding podcast: {}", response.status_text())))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemovePodcastValues {
    pub podcast_id: i32,
    pub user_id: i32
}

pub async fn call_remove_podcasts(server_name: &String, api_key: &Option<String>, remove_podcast: &RemovePodcastValues) -> Result<bool, Error> {
    let url = format!("{}/api/data/remove_podcast_id", server_name);

    console::log_1(&format!("URL: {}", url).into());

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key.as_deref().ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    // Serialize `added_podcast` into JSON
    let json_body = serde_json::to_string(remove_podcast)?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(json_body)?
        .send()
        .await?;

    let response_text = response.text().await.unwrap_or_else(|_| "Failed to get response text".to_string());
    console::log_1(&format!("Response Text: {}", response_text).into());


    if response.ok() {
        match serde_json::from_str::<PodcastStatusResponse>(&response_text) {
            Ok(parsed_response) => Ok(parsed_response.success),
            Err(parse_error) => {
                console::log_1(&format!("Error parsing response: {:?}", parse_error).into());
                Err(anyhow::Error::msg("Failed to parse response"))
            }
        }
    } else {
        console::log_1(&format!("Error removing podcast: {}", response.status_text()).into());
        Err(anyhow::Error::msg(format!("Error removing podcast: {}", response.status_text())))
    }
}


#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PodcastResponse {
    pub pods: Option<Vec<Podcast>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[allow(non_snake_case)]
pub struct Podcast {
    pub PodcastID: i32,
    pub PodcastName: String,
    pub ArtworkURL: String,
    pub Description: String,
    pub EpisodeCount: i32,
    pub WebsiteURL: String,
    pub FeedURL: String,
    pub Author: String,
    pub Categories: String, // Assuming categories are key-value pairs
    #[serde(deserialize_with = "bool_from_int")]
    pub Explicit: bool,
}

pub async fn call_get_podcasts(server_name: &String, api_key: &Option<String>, user_id: &i32) -> Result<Vec<Podcast>, anyhow::Error> {
    let url = format!("{}/api/data/return_pods/{}", server_name, user_id);

    console::log_1(&format!("URL: {}", url).into());

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key.as_deref().ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;
    if !response.ok() {
        return Err(anyhow::Error::msg(format!("Failed to fetch podcasts: {}", response.status_text())));
    }

    console::log_1(&format!("HTTP Response Status: {}", response.status()).into());
    
    // First, capture the response text for diagnostic purposes
    let response_text = response.text().await.unwrap_or_else(|_| "Failed to get response text".to_string());
    console::log_1(&format!("HTTP Response Body: {}", response_text).into());

    // Try to deserialize the response text
    match serde_json::from_str::<PodcastResponse>(&response_text) {
        Ok(response_body) => {
            console::log_1(&format!("Deserialized Response Body: {:?}", response_body).into());
            Ok(response_body.pods.unwrap_or_else(Vec::new))
        }
        Err(e) => {
            console::log_1(&format!("Deserialization Error: {:?}", e).into());
            Err(anyhow::Error::msg("Failed to deserialize response"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueuePodcastRequest {
    pub episode_title: String,
    pub ep_url: String,
    pub user_id: i32,
}

pub async fn call_queue_episode(
    server_name: &String, 
    api_key: &Option<String>, 
    request_data: &QueuePodcastRequest
) -> Result<(), Error> {
    let url = format!("{}/api/data/queue_pod", server_name);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key.as_deref().ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data).map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!("Failed to queue episode: {}", response.status_text())));
    }

    Ok(())
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct QueuePodcastResponse {
//     pub data: Vec<String>,
// }

// #[derive(Debug, Deserialize, PartialEq, Clone)]
// pub struct DataResponse {
//     pub data: Option<QueuedEpisodesResponse>,
// }

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct QueuedEpisodesResponse {
    pub episodes: Vec<QueuedEpisode>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[allow(non_snake_case)]
pub struct QueuedEpisode {
    pub EpisodeTitle: String,
    pub PodcastName: String,
    pub EpisodePubDate: String,
    pub EpisodeDescription: String,
    pub EpisodeArtwork: String,
    pub EpisodeURL: String,
    pub QueuePosition: i32,
    pub EpisodeDuration: i32,
    pub QueueDate: String,
    pub ListenDuration: Option<i32>,
    pub EpisodeID: i32,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct DataResponse {
    pub data: Vec<QueuedEpisode>,
}

pub async fn call_get_queued_episodes(
    server_name: &str, 
    api_key: &Option<String>, 
    user_id: &i32
) -> Result<Vec<QueuedEpisode>, anyhow::Error> {
    // Append the user_id as a query parameter
    let url = format!("{}/api/data/get_queued_episodes?user_id={}", server_name, user_id);

    console::log_1(&format!("URL: {}", url).into());

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key.as_deref().ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!("Failed to fetch queued episodes: {}", response.status_text())));
    }

    console::log_1(&format!("HTTP Response Status: {}", response.status()).into());
    let response_text = response.text().await?;

    console::log_1(&format!("HTTP Response Body: {}", &response_text).into());
    
    let response_data: DataResponse = serde_json::from_str(&response_text)?;
    Ok(response_data.data)
}