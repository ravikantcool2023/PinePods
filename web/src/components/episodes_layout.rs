use std::rc::Rc;
use js_sys::encode_uri_component;
use yew::{Callback, function_component, Html, html, TargetCast, use_effect, use_effect_with, use_node_ref};
use yew::prelude::*;
use web_sys::{Event, MouseEvent, window};
use yew_router::history::{BrowserHistory, History};
use yewdux::prelude::*;
use crate::components::context::{AppState, UIState};
use crate::components::audio::{AudioPlayer, on_play_click};
use super::gen_components::{UseScrollToTop, Search_nav, EpisodeTrait};
use super::app_drawer::App_drawer;
use crate::requests::pod_req::{call_add_podcast, PodcastValues, call_check_podcast, call_remove_podcasts_name, RemovePodcastValuesName};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use yew::Properties;
use super::gen_components::ContextButton;
use super::gen_funcs::{parse_date, format_datetime, match_date_format};
use crate::components::gen_funcs::format_time;
use crate::requests::login_requests::use_check_authentication;
use crate::components::gen_funcs::{sanitize_html_with_blank_target, truncate_description, convert_time_to_seconds};
use wasm_bindgen::prelude::*;
use htmlentity::entity::decode;
use htmlentity::entity::ICodedDataTrait;

fn add_icon() -> Html {
    html! {
        <span class="material-icons">{ "add_box" }</span>
    }
}

fn trash_icon() -> Html {
    html! {
        <span class="material-icons">{ "delete" }</span>

    }
}

#[allow(dead_code)]
fn play_icon() -> Html {
    html! {
<svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 -960 960 960" width="24"><path d="m380-300 280-180-280-180v360ZM480-80q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm0-80q134 0 227-93t93-227q0-134-93-227t-227-93q-134 0-227 93t-93 227q0 134 93 227t227 93Zm0-320Z"/></svg>
    }
}

#[allow(dead_code)]
fn pause_icon() -> Html {
    html! {
        <svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 -960 960 960" width="24"><path d="M360-320h80v-320h-80v320Zm160 0h80v-320h-80v320ZM480-80q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm0-80q134 0 227-93t93-227q0-134-93-227t-227-93q-134 0-227 93t-93 227q0 134 93 227t227 93Zm0-320Z"/></svg>
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub html: String,
}

#[function_component(SafeHtml)]
pub fn safe_html(props: &Props) -> Html {
    let div = gloo_utils::document().create_element("div").unwrap();
    div.set_inner_html(&props.html.clone());

    Html::VRef(div.into())
}

fn sanitize_html(html: &str) -> String {
    let cleaned_html = ammonia::clean(html);
    let decoded_data = decode(cleaned_html.as_bytes());
    match decoded_data.to_string() {
        Ok(decoded_html) => decoded_html,
        Err(e) => {
            // web_sys::console::error_1(&e.into());
            String::from("Invalid HTML content")
        }
    }
}


pub enum AppStateMsg {
    ExpandEpisode(String),
    CollapseEpisode(String),
}

impl Reducer<AppState> for AppStateMsg {
    fn apply(self, mut state: Rc<AppState>) -> Rc<AppState> {
        let state_mut = Rc::make_mut(&mut state);

        match self {
            AppStateMsg::ExpandEpisode(guid) => {
                state_mut.expanded_descriptions.insert(guid);
            },
            AppStateMsg::CollapseEpisode(guid) => {
                state_mut.expanded_descriptions.remove(&guid);
            },
        }

        // Return the Rc itself, not a reference to it
        state
    }
}

pub enum UIStateMsg {
    ClearErrorMessage,
    ClearInfoMessage,
}

impl Reducer<UIState> for UIStateMsg {
    fn apply(self, mut state: Rc<UIState>) -> Rc<UIState> {
        let state = Rc::make_mut(&mut state);

        match self {
            UIStateMsg::ClearErrorMessage => {
                state.error_message = None;
            },
            UIStateMsg::ClearInfoMessage => {
                state.info_message = None;
            },
        }

        (*state).clone().into()
    }
}




#[function_component(EpisodeLayout)]
pub fn episode_layout() -> Html {
    let is_added = use_state(|| false);
    let (state, _dispatch) = use_store::<UIState>();
    let (search_state, _search_dispatch) = use_store::<AppState>();
    let podcast_feed_results = search_state.podcast_feed_results.clone();
    let clicked_podcast_info = search_state.clicked_podcast_info.clone();
    let history = BrowserHistory::new();
    // let node_ref = use_node_ref();
    let user_id = search_state.user_details.as_ref().map(|ud| ud.UserID.clone());
    let api_key = search_state.auth_details.as_ref().map(|ud| ud.api_key.clone());
    let server_name = search_state.auth_details.as_ref().map(|ud| ud.server_name.clone());

    let session_dispatch = _search_dispatch.clone();
    let session_state = search_state.clone();
    let podcast_added = search_state.podcast_added.unwrap_or_default();


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

    // On mount, check if the podcast is in the database
    let effect_user_id = user_id.unwrap().clone();
    let effect_api_key = api_key.clone();

    {
        let is_added = is_added.clone();
        let podcast = clicked_podcast_info.clone();
        let user_id = effect_user_id.clone();
        let api_key = effect_api_key.clone();
        let server_name = server_name.clone();

        use_effect_with(
            &(),
            move |_| {
                let is_added = is_added.clone();
                let podcast = podcast.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let added = call_check_podcast(&server_name.unwrap(), &api_key.unwrap().unwrap(), user_id, podcast.clone().unwrap().podcast_title.as_str(), podcast.clone().unwrap().podcast_url.as_str()).await.unwrap_or_default().exists;
                    is_added.set(added);
                });
                || ()
            },
        );
    }

    // Function to handle link clicks
    let handle_click = Callback::from(move |event: MouseEvent| {
        if let Some(target) = event.target_dyn_into::<web_sys::HtmlElement>() {
            if let Some(href) = target.get_attribute("href") {
                event.prevent_default();
                if href.starts_with("http") {
                    // External link, open in a new tab
                    web_sys::window()
                        .unwrap()
                        .open_with_url_and_target(&href, "_blank")
                        .unwrap();
                } else {
                    // Internal link, use Yew Router to navigate
                    history.push(href);
                }
            }
        }
    });

    let node_ref = use_node_ref();

    use_effect_with((), move |_| {
        if let Some(container) = node_ref.cast::<web_sys::HtmlElement>() {
            if let Ok(links) = container.query_selector_all("a") {
                for i in 0..links.length() {
                    if let Some(link) = links.item(i) {
                        let link = link.dyn_into::<web_sys::HtmlElement>().unwrap();
                        let handle_click_clone = handle_click.clone();
                        let listener = gloo_events::EventListener::new(&link, "click", move |event| {
                            handle_click_clone.emit(event.clone().dyn_into::<web_sys::MouseEvent>().unwrap());
                        });
                        listener.forget(); // Prevent listener from being dropped
                    }
                }
            }
        }

        || ()
    });

    {
        let dispatch = _dispatch.clone();
        use_effect(move || {
            let window = window().unwrap();
            let document = window.document().unwrap();

            let closure = Closure::wrap(Box::new(move |_event: Event| {
                dispatch.apply(UIStateMsg::ClearErrorMessage);
                dispatch.apply(UIStateMsg::ClearInfoMessage);
            }) as Box<dyn Fn(_)>);

            document.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();

            // Return cleanup function
            move || {
                document.remove_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
                closure.forget(); // Prevents the closure from being dropped
            }
        });
    }


    let toggle_podcast = {
        let add_dispatch = _dispatch.clone();
        let pod_values = clicked_podcast_info.clone();

        let pod_title_og = pod_values.clone().unwrap().podcast_title.clone();
        let pod_artwork_og = pod_values.clone().unwrap().podcast_artwork.clone();
        let pod_author_og = pod_values.clone().unwrap().podcast_author.clone();
        let categories_og = pod_values.clone().unwrap().podcast_categories.unwrap().clone();
        let pod_description_og = pod_values.clone().unwrap().podcast_description.clone();
        let pod_episode_count_og = pod_values.clone().unwrap().podcast_episode_count.clone();
        let pod_feed_url_og = pod_values.clone().unwrap().podcast_url.clone();
        let pod_website_og = pod_values.clone().unwrap().podcast_link.clone();
        let pod_explicit_og = pod_values.clone().unwrap().podcast_explicit.clone();
        let user_id_og = user_id.unwrap().clone();

        let api_key_clone = api_key.clone();
        let server_name_clone = server_name.clone();
        let user_id_clone = user_id.clone();
        let dispatch = add_dispatch.clone();
        let app_dispatch = _search_dispatch.clone();

        let is_added = is_added.clone();

        if *is_added == true{
            Callback::from(move |_: MouseEvent| { 
            app_dispatch.reduce_mut(|state| state.is_loading = Some(true));
            let is_added_inner = is_added.clone();
            let call_dispatch = add_dispatch.clone();
            let pod_title = pod_title_og.clone();
            let pod_feed_url = pod_feed_url_og.clone();
            let user_id = user_id_og.clone();
            let podcast_values = RemovePodcastValuesName {
                podcast_name: pod_title,
                podcast_url: pod_feed_url,
                user_id: user_id
            };
            let api_key_call = api_key_clone.clone();
            let server_name_call = server_name_clone.clone();
            let app_dispatch = app_dispatch.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let dispatch_wasm = call_dispatch.clone();
                let api_key_wasm = api_key_call.clone().unwrap();
                let server_name_wasm = server_name_call.clone();
                let pod_values_clone = podcast_values.clone(); // Make sure you clone the podcast values
                match call_remove_podcasts_name(&server_name_wasm.unwrap(), &api_key_wasm, &pod_values_clone).await {
                    Ok(success) => {
                        if success {
                            dispatch_wasm.reduce_mut(|state| state.info_message = Option::from("Podcast successfully removed".to_string()));
                            app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                            is_added_inner.set(false);
                        } else {
                            dispatch_wasm.reduce_mut(|state| state.error_message = Option::from("Failed to add podcast".to_string()));
                            app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                        }
                    },
                    Err(e) => {
                        dispatch_wasm.reduce_mut(|state| state.error_message = Option::from(format!("Error adding podcast: {:?}", e)));
                        app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                    }
                }
            });
        })

        } else {        
            Callback::from(move |_: MouseEvent| { // Ensure this is triggered only by a MouseEvent
                let app_dispatch = app_dispatch.clone();
                app_dispatch.reduce_mut(|state| state.is_loading = Some(true));
                let is_added_inner = is_added.clone();
                let call_dispatch = add_dispatch.clone();
                let pod_title = pod_title_og.clone();
                let pod_artwork = pod_artwork_og.clone();
                let pod_author = pod_author_og.clone();
                let categories = categories_og.clone();
                let pod_description = pod_description_og.clone();
                let pod_episode_count = pod_episode_count_og.clone();
                let pod_feed_url = pod_feed_url_og.clone();
                let pod_website = pod_website_og.clone();
                let pod_explicit = pod_explicit_og.clone();
                let user_id = user_id_og.clone();
                let podcast_values = PodcastValues {
                    pod_title,
                    pod_artwork,
                    pod_author,
                    categories,
                    pod_description,
                    pod_episode_count,
                    pod_feed_url,
                    pod_website,
                    pod_explicit,
                    user_id
                };
                let api_key_call = api_key_clone.clone();
                let server_name_call = server_name_clone.clone();
                let user_id_call = user_id_clone.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let dispatch_wasm = call_dispatch.clone();
                    let api_key_wasm = api_key_call.clone().unwrap();
                    let user_id_wasm = user_id_call.clone().unwrap();
                    let server_name_wasm = server_name_call.clone();
                    let pod_values_clone = podcast_values.clone(); // Make sure you clone the podcast values

                    match call_add_podcast(&server_name_wasm.unwrap(), &api_key_wasm, user_id_wasm, &pod_values_clone).await {
                        Ok(success) => {
                            if success {
                                dispatch_wasm.reduce_mut(|state| state.info_message = Option::from("Podcast successfully added".to_string()));
                                app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                                is_added_inner.set(true);
                            } else {
                                dispatch_wasm.reduce_mut(|state| state.error_message = Option::from("Failed to add podcast".to_string()));
                                app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                            }
                        },
                        Err(e) => {
                            dispatch_wasm.reduce_mut(|state| state.error_message = Option::from(format!("Error adding podcast: {:?}", e)));
                            app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                        }
                    }
                });
            })
        }
    };

    let button_content = if *is_added {
        trash_icon()
    } else {
        add_icon()
    };
    
    let button_class = if *is_added { "bg-red-500" } else { "bg-blue-500" };

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window)]
        fn toggle_description(guid: &str);
    }

    
    html! {
        <div class="main-container">
            <Search_nav />
            <UseScrollToTop />
            <h1 class="page_header text-2xl font-bold my-4 text-center">{ "Podcast Episode Results" }</h1>
        {
            if let Some(podcast_info) = clicked_podcast_info {
                let sanitized_title = podcast_info.podcast_title.replace(|c: char| !c.is_alphanumeric(), "-");
                let desc_id = format!("desc-{}", sanitized_title);
                let toggle_description = {
                    let desc_id = desc_id.clone();
                    Callback::from(move |_: MouseEvent| {
                        let desc_id = desc_id.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let window = web_sys::window().expect("no global `window` exists");
                            let function = window
                                .get("toggle_description")
                                .expect("should have `toggle_description` as a function")
                                .dyn_into::<js_sys::Function>()
                                .unwrap();
                            let this = JsValue::NULL;
                            let guid = JsValue::from_str(&desc_id);
                            function.call1(&this, &guid).unwrap();
                        });
                    })
                };
                

                
                web_sys::console::log_1(&format!("Error: {}", desc_id).into());
                let toggle_description = {
                    let desc_id = desc_id.clone();
                    Callback::from(move |_| {
                        let desc_id = desc_id.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let window = web_sys::window().expect("no global `window` exists");
                            let function = window
                                .get("toggle_description")
                                .expect("should have `toggle_description` as a function")
                                .dyn_into::<js_sys::Function>()
                                .unwrap();
                            let this = JsValue::NULL;
                            let guid = JsValue::from_str(&desc_id);
                            function.call1(&this, &guid).unwrap();
                        });
                    })
                };
                let sanitized_description = sanitize_html(&podcast_info.podcast_description);

                html! {
                    <div class="item-header">
                        <img src={podcast_info.podcast_artwork.clone()} alt={format!("Cover for {}", &podcast_info.podcast_title)} class="item-header-cover"/>
                        <div class="item-header-info">
                            <div class="title-button-container">
                                <h2 class="item-header-title">{ &podcast_info.podcast_title }</h2>
                                <button onclick={toggle_podcast} title="Click to add or remove podcast from feed" class={"item-container-button selector-button font-bold py-2 px-4 rounded-full self-center mr-8"} style="width: 60px; height: 60px;">
                                    { button_content }
                                </button>
                            </div>

                            // <p class="item-header-description">{ &podcast_info.podcast_description }</p>
                            <div class="item-header-description desc-collapsed" id={desc_id.clone()} onclick={toggle_description.clone()}>
                                { sanitized_description }
                                <button class="toggle-desc-btn" onclick={toggle_description}>{ "" }</button>
                            </div>
                            <div class="item-header-info">
                                <p class="header-text">{ format!("Episode Count: {}", &podcast_info.podcast_episode_count) }</p>
                                <p class="header-text">{ format!("Authors: {}", &podcast_info.podcast_author) }</p>
                                <p class="header-text">{ format!("Explicit: {}", if podcast_info.podcast_explicit { "Yes" } else { "No" }) }</p>

                                <div>
                                    {
                                        if let Some(categories) = &podcast_info.podcast_categories {
                                            html! {
                                                for categories.values().map(|category_name| {
                                                    html! { <span class="category-box">{ category_name }</span> }
                                                })
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>

                            </div>
                        </div>
                    </div>
                }
            } else {
                html! {}
            }
        }
        {
                if let Some(results) = podcast_feed_results {
                    html! {
                        <div>
                            { for results.episodes.iter().map(|episode| {
                                let dispatch = _dispatch.clone();
                                let search_dispatch = _search_dispatch.clone();
                                let search_state_clone = search_state.clone(); // Clone search_state

                                // Clone the variables outside the closure
                                let episode_url_clone = episode.enclosure_url.clone().unwrap_or_default();
                                let episode_title_clone = episode.title.clone().unwrap_or_default();
                                let episode_artwork_clone = episode.artwork.clone().unwrap_or_default();
                                // let episode_duration_clone = episode.duration.clone().unwrap_or_default();
                                let episode_duration_clone = episode.duration.clone().unwrap_or_default();
                                let episode_duration_in_seconds = match convert_time_to_seconds(&episode_duration_clone) {
                                    Ok(seconds) => seconds as i32,
                                    Err(e) => {
                                        eprintln!("Failed to convert time to seconds: {}", e);
                                        0
                                    }
                                };
                                let episode_id_clone = episode.episode_id.unwrap_or(0);
                                let server_name_play = server_name.clone();
                                let user_id_play = user_id.clone();
                                let api_key_play = api_key.clone();

                                let is_expanded = search_state.expanded_descriptions.contains(
                                    &episode.guid.clone().unwrap()
                                );
                                

                                let sanitized_description = sanitize_html_with_blank_target(&episode.description.clone().unwrap_or_default());

                                let (description, _is_truncated) = if is_expanded {
                                    (sanitized_description, false)
                                } else {
                                    truncate_description(sanitized_description, 300)
                                };

                                let search_state_toggle = search_state_clone.clone();
                                let toggle_expanded = {
                                    let search_dispatch_clone = search_dispatch.clone();
                                    let episode_guid = episode.guid.clone().unwrap();
                                    Callback::from(move |_: MouseEvent| {
                                        let guid_clone = episode_guid.clone();
                                        let search_dispatch_call = search_dispatch_clone.clone();

                                        if search_state_toggle.expanded_descriptions.contains(&guid_clone) {
                                            search_dispatch_call.apply(AppStateMsg::CollapseEpisode(guid_clone));
                                        } else {
                                            search_dispatch_call.apply(AppStateMsg::ExpandEpisode(guid_clone));
                                        }

                                    })
                                };


                                let state = state.clone();
                                let on_play_click = on_play_click(
                                    episode_url_clone.clone(),
                                    episode_title_clone.clone(),
                                    episode_artwork_clone.clone(),
                                    episode_duration_in_seconds,
                                    episode_id_clone.clone(),
                                    Some(0),
                                    api_key_play.unwrap().unwrap(),
                                    user_id_play.unwrap(),
                                    server_name_play.unwrap(),
                                    dispatch.clone(),
                                    state.clone(),
                                    None,
                                );

                                let description_class = if is_expanded {
                                    "desc-expanded".to_string()
                                } else {
                                    "desc-collapsed".to_string()
                                };

                                let date_format = match_date_format(search_state_clone.date_format.as_deref());
                                let datetime = parse_date(&episode.pub_date.clone().unwrap_or_default(), &search_state_clone.user_tz);
                                let format_release = format!("{}", format_datetime(&datetime, &search_state_clone.hour_preference, date_format));
                                let boxed_episode = Box::new(episode.clone()) as Box<dyn EpisodeTrait>;
                                let duration = episode.duration.clone().unwrap().parse::<f64>().unwrap_or(0.0);
                                let formatted_duration = format_time(duration);
                                let episode_url_for_ep_item = episode_url_clone.clone();
                                let should_show_buttons = !episode_url_for_ep_item.is_empty();
                                html! {
                                    <div class="item-container flex items-center mb-4 shadow-md rounded-lg">
                                        <img src={episode.artwork.clone().unwrap_or_default()} alt={format!("Cover for {}", &episode.title.clone().unwrap_or_default())} class="object-cover align-top-cover w-full item-container img"/>
                                        <div class="flex flex-col p-4 space-y-2 flex-grow md:w-7/12">
                                            <p class="item_container-text text-xl font-semibold">{ &episode.title.clone().unwrap_or_default() }</p>
                                            // <p class="text-gray-600">{ &episode.description.clone().unwrap_or_default() }</p>
                                            {
                                                html! {
                                                    <div class="item-container-text hidden md:block">
                                                        <div class={format!("item_container-text episode-description-container {}", description_class)}>
                                                            <SafeHtml html={description} />
                                                        </div>
                                                        <a class="link hover:underline cursor-pointer mt-4" onclick={toggle_expanded}>
                                                            { if is_expanded { "See Less" } else { "See More" } }
                                                        </a>
                                                    </div>
                                                }
                                            }
                                            <span class="episode-time-badge inline-flex items-center px-2.5 py-0.5 rounded me-2">
                                                <svg class="time-icon w-2.5 h-2.5 me-1.5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 20 20">
                                                    <path d="M10 0a10 10 0 1 0 10 10A10.011 10.011 0 0 0 10 0Zm3.982 13.982a1 1 0 0 1-1.414 0l-3.274-3.274A1.012 1.012 0 0 1 9 10V6a1 1 0 0 1 2 0v3.586l2.982 2.982a1 1 0 0 1 0 1.414Z"/>
                                                </svg>
                                                { format_release }
                                            </span>
                                            {
                                                // if formatted_listen_duration.is_some() {
                                                //     html! {
                                                //         <div class="flex items-center space-x-2">
                                                //             <span class="item_container-text">{ formatted_listen_duration.clone() }</span>
                                                //             <div class="progress-bar-container">
                                                //                 <div class="progress-bar" style={ format!("width: {}%;", listen_duration_percentage) }></div>
                                                //             </div>
                                                //             <span class="item_container-text">{ formatted_duration }</span>
                                                //         </div>
                                                //     }
                                                    
                                                // } else {
                                                    html! {
                                                        <span class="item_container-text">{ format!("{}", formatted_duration) }</span>
                                                    }
                                                // }
                                            }
                                        </div>
                                        {
                                            html! {
                                                <div class="flex flex-col items-center h-full w-2/12 px-2 space-y-4 md:space-y-8 button-container" style="align-self: center;"> // Add align-self: center; heren medium and larger screens
                                                    if should_show_buttons {
                                                        <button
                                                            class="item-container-button border-solid border selector-button font-bold py-2 px-4 rounded-full flex items-center justify-center md:w-16 md:h-16 w-10 h-10"
                                                            onclick={on_play_click}
                                                        >
                                                        <span class="material-bonus-color material-icons large-material-icons md:text-6xl text-4xl">{"play_arrow"}</span>
                                                        </button>
                                                        {
                                                            if podcast_added {
                                                                let page_type = "episode_layout".to_string();

                                                                let context_button = html! {
                                                                    <ContextButton episode={boxed_episode} page_type={page_type.clone()} />
                                                                };


                                                                context_button

                                                            } else {
                                                                html! {}
                                                            }
                                                        }
                                                    }
                                                </div>
                                            }
                                        }


                                    </div>
                                }
                            })}
                        </div>
                    }
                } else {
                    html! {
                        <div class="empty-episodes-container" id="episode-container">
                            <img src="static/assets/favicon.png" alt="Logo" class="logo"/>
                            <h1 class="page-subtitles">{ "No Episodes Found" }</h1>
                            <p class="page-paragraphs">{"This podcast strangely doesn't have any episodes. Try a more mainstream one maybe?"}</p>
                        </div>
                    }
                }
            }
        <App_drawer />
        // Conditional rendering for the error banner
        {
            if state.error_message.as_ref().map_or(false, |msg| !msg.is_empty()) {
                html! { <div class="error-snackbar">{ &state.error_message }</div> }
            } else {
                html! {}
            }
        }
        //     if !state.error_message.is_empty() {
        //         html! { <div class="error-snackbar">{ &state.error_message }</div> }
        //     } else {
        //         html! {}
        //     }
        // }
        //     // Conditional rendering for the info banner
        {
        if state.info_message.as_ref().map_or(false, |msg| !msg.is_empty()) {
                html! { <div class="info-snackbar">{ &state.info_message }</div> }
            } else {
                html! {}
            }
        }
        // {
        //     if !state.info_message.is_empty() {
        //         html! { <div class="info-snackbar">{ &state.info_message }</div> }
        //     } else {
        //         html! {}
        //     }
        // }
        {
            if let Some(audio_props) = &state.currently_playing {
                html! { <AudioPlayer src={audio_props.src.clone()} title={audio_props.title.clone()} artwork_url={audio_props.artwork_url.clone()} duration={audio_props.duration.clone()} episode_id={audio_props.episode_id.clone()} duration_sec={audio_props.duration_sec.clone()} start_pos_sec={audio_props.start_pos_sec.clone()} /> }
            } else {
                html! {}
            }
        }
        </div>

    }
}

