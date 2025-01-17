use yew::{function_component, Html, html};
use yew::prelude::*;
use super::app_drawer::App_drawer;
use super::gen_components::{UseScrollToTop, Search_nav, empty_message, episode_item, on_shownotes_click};
use crate::requests::pod_req::{self, EpisodeDownloadResponse, DownloadEpisodeRequest, call_remove_downloaded_episode};
use yewdux::prelude::*;
use crate::components::context::{AppState, UIState};
use yew_router::history::BrowserHistory;
use crate::components::audio::AudioPlayer;
use crate::components::gen_funcs::{sanitize_html_with_blank_target, truncate_description, parse_date, format_datetime, match_date_format};
use crate::components::audio::on_play_click;
use crate::components::context::AppStateMsg;
// use crate::components::gen_funcs::check_auth;
use crate::components::episodes_layout::UIStateMsg;
use wasm_bindgen::closure::Closure;
use web_sys::window;
use wasm_bindgen::JsCast;
use std::borrow::Borrow;
use crate::requests::login_requests::use_check_authentication;

#[function_component(Downloads)]
pub fn downloads() -> Html {
    let (state, dispatch) = use_store::<AppState>();
    let effect_dispatch = dispatch.clone();
    let history = BrowserHistory::new();

    let session_dispatch = effect_dispatch.clone();
    let session_state = state.clone();

    use_effect_with((), move |_| {
        // Check if the page reload action has already occurred to prevent redundant execution
        if session_state.reload_occured.unwrap_or(false) {
            // Logic for the case where reload has already been processed
        } else {
            // Normal effect logic for handling page reload
            let window = web_sys::window().expect("no global `window` exists");
            let performance = window.performance().expect("should have performance");
            let navigation_type = performance.navigation().type_();
            
            if navigation_type == 1 { // 1 stands for reload
                let session_storage = window.session_storage().unwrap().unwrap();
                session_storage.set_item("isAuthenticated", "false").unwrap();
            }
    
            // Always check authentication status
            let current_route = window.location().href().unwrap_or_default();
            use_check_authentication(session_dispatch.clone(), &current_route);
    
            // Mark that the page reload handling has occurred
            session_dispatch.reduce_mut(|state| {
                state.reload_occured = Some(true);
                state.clone() // Return the modified state
            });
        }
    
        || ()
    });

    let error = use_state(|| None);
    let (post_state, _post_dispatch) = use_store::<AppState>();
    let (audio_state, audio_dispatch) = use_store::<UIState>();
    let error_message = audio_state.error_message.clone();
    let info_message = audio_state.info_message.clone();
    let page_state = use_state(|| PageState::Normal);
    let api_key = post_state.auth_details.as_ref().map(|ud| ud.api_key.clone());
    let user_id = post_state.user_details.as_ref().map(|ud| ud.UserID.clone());
    let server_name = post_state.auth_details.as_ref().map(|ud| ud.server_name.clone());
    let loading = use_state(|| true);

    {
        let ui_dispatch = audio_dispatch.clone();
        use_effect(move || {
            let window = window().unwrap();
            let document = window.document().unwrap();

            let closure = Closure::wrap(Box::new(move |_event: Event| {
                ui_dispatch.apply(UIStateMsg::ClearErrorMessage);
                ui_dispatch.apply(UIStateMsg::ClearInfoMessage);
            }) as Box<dyn Fn(_)>);

            document.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();

            // Return cleanup function
            move || {
                document.remove_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
                closure.forget(); // Prevents the closure from being dropped
            }
        });
    }


    // Fetch episodes on component mount
    let loading_ep = loading.clone();
    {
        // let episodes = episodes.clone();
        let error = error.clone();
        let api_key = post_state.auth_details.as_ref().map(|ud| ud.api_key.clone());
        let user_id = post_state.user_details.as_ref().map(|ud| ud.UserID.clone());
        let server_name = post_state.auth_details.as_ref().map(|ud| ud.server_name.clone());

        let effect_dispatch = dispatch.clone();

        // fetch_episodes(api_key.flatten(), user_id, server_name, dispatch, error, pod_req::call_get_recent_eps);

        use_effect_with(
            (api_key.clone(), user_id.clone(), server_name.clone()),
            move |_| {
                let error_clone = error.clone();
                if let (Some(api_key), Some(user_id), Some(server_name)) = (api_key.clone(), user_id.clone(), server_name.clone()) {
                    let dispatch = effect_dispatch.clone();
    
                    wasm_bindgen_futures::spawn_local(async move {
                        match pod_req::call_get_episode_downloads(&server_name, &api_key, &user_id).await {
                            Ok(fetched_episodes) => {
                                dispatch.reduce_mut(move |state| {
                                    state.downloaded_episodes = Some(EpisodeDownloadResponse { episodes: fetched_episodes });
                                });
                                loading_ep.set(false);
                                // web_sys::console::log_1(&format!("State after update: {:?}", state).into()); // Log state after update
                            },
                            Err(e) => {
                                error_clone.set(Some(e.to_string()));
                                loading_ep.set(false);
                            },
                        }
                    });
                }
                || ()
            },
        );
    }

    // Define the state of the application
    #[derive(Clone, PartialEq)]
    enum PageState {
        Delete,
        Normal,
    }

    // Define the function to Enter Delete Mode
    let delete_mode_enable = {
        let page_state = page_state.clone();
        Callback::from(move |_: MouseEvent| {
            page_state.set(PageState::Delete);
        })
    };

    // Define the function to Exit Delete Mode
    let delete_mode_disable = {
        let page_state = page_state.clone();
        Callback::from(move |_: MouseEvent| {
            page_state.set(PageState::Normal);
        })
    };

    let on_checkbox_change = {
        let dispatch = dispatch.clone();
        Callback::from(move |episode_id: i32| {
            dispatch.reduce_mut(move |state| {
                // Update the state of the selected episodes for deletion
                state.selected_episodes_for_deletion.insert(episode_id);
            });
        })
    };

    let delete_selected_episodes = {
        let dispatch = dispatch.clone();
        let page_state = page_state.clone();
        let server_name = server_name.clone();
        let api_key = api_key.clone();
        let user_id = user_id.clone(); // Make sure this is cloned from a state or props where it's guaranteed to exist.
    
        Callback::from(move |_: MouseEvent| {
            // Clone values for use inside the async block
            let dispatch_cloned = dispatch.clone();
            let page_state_cloned = page_state.clone();
            let server_name_cloned = server_name.clone().unwrap(); // Assuming you've ensured these are present
            let api_key_cloned = api_key.clone().unwrap();
            let user_id_cloned = user_id.unwrap();
    
            dispatch.reduce_mut(move |state| {
                let selected_episodes = state.selected_episodes_for_deletion.clone();
                // Clear the selected episodes for deletion right away to prevent re-deletion in case of re-render
                state.selected_episodes_for_deletion.clear();
    
                for &episode_id in &selected_episodes {
                    let request = DownloadEpisodeRequest {
                        episode_id,
                        user_id: user_id_cloned,
                    };
                    let server_name_cloned = server_name_cloned.clone();
                    let api_key_cloned = api_key_cloned.clone();
                    let future = async move {
                        match call_remove_downloaded_episode(&server_name_cloned, &api_key_cloned, &request).await {
                            Ok(success_message) => Some((success_message, episode_id)),
                            Err(_) => None,
                        }
                    };
    
                    let dispatch_for_future = dispatch_cloned.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some((success_message, episode_id)) = future.await {
                            dispatch_for_future.reduce_mut(|state| {
                                if let Some(downloaded_episodes) = &mut state.downloaded_episodes {
                                    downloaded_episodes.episodes.retain(|ep| ep.EpisodeID != episode_id);
                                }
                                state.info_message = Some(success_message);
                            });
                        }
                    });
                }
    
                page_state_cloned.set(PageState::Normal); // Return to normal state after operations
            });
        })
    };
    
    let is_delete_mode = **page_state.borrow() == PageState::Delete; // Add this line

    html! {
        <>
        <div class="main-container">
            <Search_nav />
            <UseScrollToTop />
                if *loading { // If loading is true, display the loading animation
                    {
                        html! {
                            <div class="loading-animation">
                                <div class="frame1"></div>
                                <div class="frame2"></div>
                                <div class="frame3"></div>
                                <div class="frame4"></div>
                                <div class="frame5"></div>
                                <div class="frame6"></div>
                            </div>
                        }
                    }
                } else {
                    {
                        html! {
                            <div>
                                <h1 class="text-2xl item_container-text font-bold text-center mb-6">{"Downloaded Episodes"}</h1>
                                <div class="flex justify-between">
                                    {
                                        if **page_state.borrow() == PageState::Normal {
                                            html! {
                                                <button class="download-button font-bold py-2 px-4 rounded inline-flex items-center"
                                                    onclick={delete_mode_enable.clone()}>
                                                    <span class="material-icons icon-space">{"check_box"}</span>
                                                    <span class="text-lg">{"Select Multiple"}</span>
                                                </button>
                                            }
                                        } else {
                                            html! {
                                                <>
                                                <button class="download-button font-bold py-2 px-4 rounded inline-flex items-center"
                                                    onclick={delete_mode_disable.clone()}>
                                                    <span class="material-icons icon-space">{"cancel"}</span>
                                                    <span class="text-lg">{"Cancel"}</span>
                                                </button>
                                                <button class="download-button font-bold py-2 px-4 rounded inline-flex items-center"
                                                    onclick={delete_selected_episodes.clone()}>
                                                    <span class="material-icons icon-space">{"delete"}</span>
                                                    <span class="text-lg">{"Delete"}</span>
                                                </button>
                                                </>
                                            }
                                        }
                                    }
                                </div>
                            </div>
                        }
                    }
                    
                    {
                    if let Some(download_eps) = state.downloaded_episodes.clone() {
                        let int_download_eps = download_eps.clone();
                            let api_key = post_state.auth_details.as_ref().map(|ud| ud.api_key.clone());
                            let user_id = post_state.user_details.as_ref().map(|ud| ud.UserID.clone());
                            let server_name = post_state.auth_details.as_ref().map(|ud| ud.server_name.clone());
                            let history_clone = history.clone();

                            if int_download_eps.episodes.is_empty() {
                                // Render "No Recent Episodes Found" if episodes list is empty
                                empty_message(
                                    "No Downloaded Episodes Found",
                                    "This is where episode downloads will appear. To download an episode you can open the context menu on an episode and select Download Episode. It will then download the the server and show up here!"
                                )
                            } else {
                                int_download_eps.episodes.into_iter().map(|episode| {

                                let id_string = &episode.EpisodeID.to_string();
        
                                let is_expanded = state.expanded_descriptions.contains(id_string);
        
                                let dispatch = dispatch.clone();
        
                                let episode_url_clone = episode.EpisodeURL.clone();
                                let episode_title_clone = episode.EpisodeTitle.clone();
                                let episode_artwork_clone = episode.EpisodeArtwork.clone();
                                let episode_duration_clone = episode.EpisodeDuration.clone();
                                let episode_id_clone = episode.EpisodeID.clone();
                                let episode_listened_clone = episode.ListenDuration.clone();

                                let sanitized_description = sanitize_html_with_blank_target(&episode.EpisodeDescription.clone());

                                let (description, _is_truncated) = if is_expanded {
                                    (sanitized_description, false)
                                } else {
                                    truncate_description(sanitized_description, 300)
                                };
        
                                let toggle_expanded = {
                                    let search_dispatch_clone = dispatch.clone();
                                    let state_clone = state.clone();
                                    let episode_guid = episode.EpisodeID.clone();
        
                                    Callback::from(move |_: MouseEvent| {
                                        let guid_clone = episode_guid.to_string().clone();
                                        let search_dispatch_call = search_dispatch_clone.clone();
        
                                        if state_clone.expanded_descriptions.contains(&guid_clone) {
                                            search_dispatch_call.apply(AppStateMsg::CollapseEpisode(guid_clone));
                                        } else {
                                            search_dispatch_call.apply(AppStateMsg::ExpandEpisode(guid_clone));
                                        }
                                    })
                                };

                                let episode_url_for_closure = episode_url_clone.clone();
                                let episode_title_for_closure = episode_title_clone.clone();
                                let episode_artwork_for_closure = episode_artwork_clone.clone();
                                let episode_duration_for_closure = episode_duration_clone.clone();
                                let listener_duration_for_closure = episode_listened_clone.clone();
                                let episode_id_for_closure = episode_id_clone.clone();
                                let user_id_play = user_id.clone();
                                let server_name_play = server_name.clone();
                                let api_key_play = api_key.clone();
                                let audio_dispatch = audio_dispatch.clone();
                                let is_local = Option::from(true);
                                
                                let on_play_click = on_play_click(
                                    episode_url_for_closure.clone(),
                                    episode_title_for_closure.clone(),
                                    episode_artwork_for_closure.clone(),
                                    episode_duration_for_closure.clone(),
                                    episode_id_for_closure.clone(),
                                    listener_duration_for_closure.clone(),
                                    api_key_play.unwrap().unwrap(),
                                    user_id_play.unwrap(),
                                    server_name_play.unwrap(),
                                    audio_dispatch.clone(),
                                    audio_state.clone(),
                                    is_local,
                                );

                                let on_shownotes_click = on_shownotes_click(
                                    history_clone.clone(),
                                    dispatch.clone(),
                                    episode_id_for_closure.clone(),
                                );

                                let date_format = match_date_format(state.date_format.as_deref());
                                let datetime = parse_date(&episode.EpisodePubDate, &state.user_tz);
                                let format_release = format!("{}", format_datetime(&datetime, &state.hour_preference, date_format));
    
                                let on_checkbox_change_cloned = on_checkbox_change.clone();
                                let episode_url_for_ep_item = episode_url_clone.clone();
                                let item = episode_item(
                                    Box::new(episode),
                                    description.clone(),
                                    is_expanded,
                                    &format_release,
                                    on_play_click,
                                    on_shownotes_click,
                                    toggle_expanded,
                                    episode_duration_clone,
                                    episode_listened_clone,
                                    "downloads",
                                    on_checkbox_change_cloned, // Add this line
                                    is_delete_mode, // Add this line
                                    episode_url_for_ep_item
                                );

                                item
                            }).collect::<Html>()
                            }
                        

                        } else {
                            empty_message(
                                "No Episode Downloads Found",
                                "This is where episode downloads will appear. To download an episode you can open the context menu on an episode and select Download Episode. It will then download to the server and show up here!"
                            )
                        }
                    }
            }
        {
            if let Some(audio_props) = &audio_state.currently_playing {
                html! { <AudioPlayer src={audio_props.src.clone()} title={audio_props.title.clone()} artwork_url={audio_props.artwork_url.clone()} duration={audio_props.duration.clone()} episode_id={audio_props.episode_id.clone()} duration_sec={audio_props.duration_sec.clone()} start_pos_sec={audio_props.start_pos_sec.clone()} /> }
            } else {
                html! {}
            }
        }
        // Conditional rendering for the error banner
        if let Some(error) = error_message {
            <div class="error-snackbar">{ error }</div>
        }
        if let Some(info) = info_message {
            <div class="info-snackbar">{ info }</div>
        }
        </div>
        <App_drawer />
        </>
    }
}

