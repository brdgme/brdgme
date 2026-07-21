//! #30 friends: server fns and the /friends page.
//! Spec: docs/superpowers/specs/2026-07-08-30-friends-design.md

use leptos::prelude::*;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::error::internal;

pub const INVITE_POLICIES: [(&str, &str); 3] = [
    ("open", "Anyone can add me to a game"),
    ("friends", "Only friends can add me to a game"),
    ("none", "Nobody can add me to a game"),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriendEntry {
    pub user_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriendRequestEntry {
    pub request_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriendsOverview {
    pub friends: Vec<FriendEntry>,
    pub incoming: Vec<FriendRequestEntry>,
    /// Pending AND declined outgoing requests, indistinguishable by design
    /// (D1 silent shield).
    pub outgoing: Vec<FriendEntry>,
    pub blocked: Vec<FriendEntry>,
    pub invite_policy: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpponentSuggestion {
    pub user_id: Uuid,
    pub name: String,
    pub is_friend: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSearchResult {
    pub user_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriendActiveGame {
    pub game_id: Uuid,
    pub game_type: String,
    pub player_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriendRecentResult {
    pub game_id: Uuid,
    pub game_type: String,
    pub finished_at: Option<time::PrimitiveDateTime>,
    /// Ordered by place; places[i] belongs to player_names[i], 0 = unplaced.
    pub player_names: Vec<String>,
    pub places: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriendActivity {
    pub active: Vec<FriendActiveGame>,
    pub results: Vec<FriendRecentResult>,
}

#[cfg(feature = "ssr")]
pub(crate) async fn require_user() -> Result<crate::auth::AuthUser, ServerFnError> {
    crate::auth::server::get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))
}

#[server(GetFriendsOverview, "/api")]
pub async fn get_friends_overview() -> Result<FriendsOverview, ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    let friends = crate::db::list_friends(&pool, user.id)
        .await
        .map_err(internal("get_friends_overview: friends"))?;
    let incoming = crate::db::list_incoming_friend_requests(&pool, user.id)
        .await
        .map_err(internal("get_friends_overview: incoming"))?;
    let outgoing = crate::db::list_outgoing_friend_requests(&pool, user.id)
        .await
        .map_err(internal("get_friends_overview: outgoing"))?;
    let blocked = crate::db::list_blocked(&pool, user.id)
        .await
        .map_err(internal("get_friends_overview: blocked"))?;
    let invite_policy = crate::db::get_invite_policy(&pool, user.id)
        .await
        .map_err(internal("get_friends_overview: policy"))?;
    let entry = |(user_id, name): (Uuid, String)| FriendEntry { user_id, name };
    Ok(FriendsOverview {
        friends: friends.into_iter().map(entry).collect(),
        incoming: incoming
            .into_iter()
            .map(|(request_id, user_id, name)| FriendRequestEntry {
                request_id,
                user_id,
                name,
            })
            .collect(),
        outgoing: outgoing.into_iter().map(entry).collect(),
        blocked: blocked.into_iter().map(entry).collect(),
        invite_policy,
    })
}

/// D3: by user id (game-page button) or by exact username (friends page).
/// Silent-shield semantics live in db::send_friend_request; the only honest
/// errors here are the caller's own mistakes.
#[server(SendFriendRequest, "/api")]
pub async fn send_friend_request(
    user_id: Option<Uuid>,
    name: Option<String>,
) -> Result<(), ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    let target = match (user_id, name) {
        (Some(id), _) => crate::db::get_user(&pool, id)
            .await
            .map_err(internal("send_friend_request: find user"))?
            .map(|u| u.id)
            .ok_or_else(|| ServerFnError::new("User not found"))?,
        (None, Some(name)) => crate::db::get_user_by_name(&pool, name.trim())
            .await
            .map_err(internal("send_friend_request: find user by name"))?
            .map(|(id, _)| id)
            .ok_or_else(|| ServerFnError::new(format!("No user named {}", name.trim())))?,
        (None, None) => return Err(ServerFnError::new("No user specified")),
    };
    if target == user.id {
        return Err(ServerFnError::new("You cannot friend yourself"));
    }
    if crate::db::has_block(&pool, user.id, target)
        .await
        .map_err(internal("send_friend_request: check block"))?
    {
        return Err(ServerFnError::new(
            "You have blocked this user - unblock them first",
        ));
    }
    crate::db::send_friend_request(&pool, user.id, target)
        .await
        .map_err(internal("send_friend_request: send"))
}

/// accept=true/block=false accepts; accept=false/block=false declines
/// (keeps the row - D1 shield); block=true declines AND blocks in one step
/// (D7 - the block itself deletes the request row and becomes the shield).
#[server(RespondToFriendRequest, "/api")]
pub async fn respond_to_friend_request(
    request_id: Uuid,
    accept: bool,
    block: bool,
) -> Result<(), ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    if accept && block {
        return Err(ServerFnError::new("Cannot accept and block"));
    }
    if block {
        let source = crate::db::get_pending_request_source(&pool, request_id, user.id)
            .await
            .map_err(internal("respond_to_friend_request: find request"))?
            .ok_or_else(|| ServerFnError::new("Request not found"))?;
        return crate::db::block_user(&pool, user.id, source)
            .await
            .map_err(internal("respond_to_friend_request: block"));
    }
    let updated = crate::db::respond_to_friend_request(&pool, request_id, user.id, accept)
        .await
        .map_err(internal("respond_to_friend_request: respond"))?;
    if !updated {
        return Err(ServerFnError::new("Request not found"));
    }
    Ok(())
}

#[server(Unfriend, "/api")]
pub async fn unfriend(user_id: Uuid) -> Result<(), ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    crate::db::unfriend(&pool, user.id, user_id)
        .await
        .map_err(internal("unfriend: delete"))
}

#[server(BlockUser, "/api")]
pub async fn block_user(user_id: Uuid) -> Result<(), ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    if user_id == user.id {
        return Err(ServerFnError::new("You cannot block yourself"));
    }
    crate::db::block_user(&pool, user.id, user_id)
        .await
        .map_err(internal("block_user: block"))
}

#[server(UnblockUser, "/api")]
pub async fn unblock_user(user_id: Uuid) -> Result<(), ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    crate::db::unblock_user(&pool, user.id, user_id)
        .await
        .map_err(internal("unblock_user: unblock"))
}

#[server(SetInvitePolicy, "/api")]
pub async fn set_invite_policy(policy: String) -> Result<(), ServerFnError> {
    use sqlx::PgPool;
    if !INVITE_POLICIES.iter().any(|(slug, _)| *slug == policy) {
        return Err(ServerFnError::new("Unknown invite policy"));
    }
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    crate::db::set_invite_policy(&pool, user.id, &policy)
        .await
        .map_err(internal("set_invite_policy: update"))
}

#[server(GetOpponentSuggestions, "/api")]
pub async fn get_opponent_suggestions() -> Result<Vec<OpponentSuggestion>, ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    let rows = crate::db::opponent_suggestions(&pool, user.id)
        .await
        .map_err(internal("get_opponent_suggestions: query"))?;
    Ok(rows
        .into_iter()
        .map(|(user_id, name, is_friend)| OpponentSuggestion {
            user_id,
            name,
            is_friend,
        })
        .collect())
}

/// #44 new game page typeahead: display-name substring search. Login
/// required; under 2 trimmed characters returns empty; capped at 10;
/// excludes the caller (all enforced in db::search_users).
#[server(SearchUsers, "/api")]
pub async fn search_users(query: String) -> Result<Vec<UserSearchResult>, ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    let rows = crate::db::search_users(&pool, user.id, &query)
        .await
        .map_err(internal("search_users: query"))?;
    Ok(rows
        .into_iter()
        .map(|(user_id, name)| UserSearchResult { user_id, name })
        .collect())
}

#[server(GetFriendActivity, "/api")]
pub async fn get_friend_activity() -> Result<FriendActivity, ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    let active = crate::db::friends_active_games(&pool, user.id, 10)
        .await
        .map_err(internal("get_friend_activity: active"))?;
    let results = crate::db::friends_recent_results(&pool, user.id, 10)
        .await
        .map_err(internal("get_friend_activity: results"))?;
    Ok(FriendActivity {
        active: active
            .into_iter()
            .map(|(game_id, game_type, player_names)| FriendActiveGame {
                game_id,
                game_type,
                player_names,
            })
            .collect(),
        results: results
            .into_iter()
            .map(
                |(game_id, game_type, finished_at, player_names, places)| FriendRecentResult {
                    game_id,
                    game_type,
                    finished_at,
                    player_names,
                    places,
                },
            )
            .collect(),
    })
}

#[component]
pub fn FriendsPage() -> impl IntoView {
    use crate::components::layout::MainLayout;

    let (refresh, set_refresh) = signal(0u32);
    let overview = LocalResource::new(move || {
        refresh.track();
        get_friends_overview()
    });

    let add_action = ServerAction::<SendFriendRequest>::new();
    let respond_action = ServerAction::<RespondToFriendRequest>::new();
    let unfriend_action = ServerAction::<Unfriend>::new();
    let unblock_action = ServerAction::<UnblockUser>::new();
    let policy_action = ServerAction::<SetInvitePolicy>::new();

    let (add_name, set_add_name) = signal(String::new());

    // Any successful mutation refetches the overview.
    Effect::new(move |_| {
        if let Some(Ok(())) = add_action.value().get() {
            set_add_name.set(String::new());
            set_refresh.update(|n| *n += 1);
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = respond_action.value().get() {
            set_refresh.update(|n| *n += 1);
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = unfriend_action.value().get() {
            set_refresh.update(|n| *n += 1);
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = unblock_action.value().get() {
            set_refresh.update(|n| *n += 1);
        }
    });

    view! {
        <MainLayout>
            <div class="friends content-page">
                <h1>"Friends"</h1>
                {move || match overview.get() {
                    None => view! { <p>"Loading..."</p> }.into_any(),
                    Some(Err(e)) => view! { <p class="error">"Error: " {e.to_string()}</p> }.into_any(),
                    Some(Ok(o)) => view! {
                        <section class="friends-add">
                            <h2>"Add a friend"</h2>
                            <form on:submit=move |ev: leptos::ev::SubmitEvent| {
                                ev.prevent_default();
                                let name = add_name.get_untracked();
                                if !name.trim().is_empty() {
                                    add_action.dispatch(SendFriendRequest { user_id: None, name: Some(name) });
                                }
                            }>
                                <input
                                    type="text"
                                    placeholder="Exact username"
                                    prop:value=add_name
                                    on:input=move |ev| set_add_name.set(event_target_value(&ev))
                                />
                                <input type="submit" value="Send request"
                                    disabled=move || add_action.pending().get() />
                            </form>
                            {move || add_action.value().get().and_then(|r| r.err()).map(|e| view! {
                                <p class="error">{e.to_string()}</p>
                            })}
                        </section>

                        <section class="friends-incoming">
                            <h2>"Incoming requests"</h2>
                            {if o.incoming.is_empty() {
                                view! { <p>"No incoming requests."</p> }.into_any()
                            } else {
                                o.incoming.iter().map(|r| {
                                    let id = r.request_id;
                                    let name = r.name.clone();
                                    view! {
                                        <div class="friend-row">
                                            <span>
                                                <A href=format!("/players/{}", crate::players::encode_path_segment(&name))>
                                                    {name.clone()}
                                                </A>
                                            </span>
                                            " "
                                            <a href="#" on:click=move |ev| {
                                                ev.prevent_default();
                                                respond_action.dispatch(RespondToFriendRequest { request_id: id, accept: true, block: false });
                                            }>"Accept"</a>
                                            " | "
                                            <a href="#" on:click=move |ev| {
                                                ev.prevent_default();
                                                respond_action.dispatch(RespondToFriendRequest { request_id: id, accept: false, block: false });
                                            }>"Decline"</a>
                                            " | "
                                            <a href="#" on:click=move |ev| {
                                                ev.prevent_default();
                                                let confirmed = web_sys::window()
                                                    .and_then(|w| w.confirm_with_message("Decline and block? They will no longer be able to send you friend requests or add you to games.").ok())
                                                    .unwrap_or(false);
                                                if confirmed {
                                                    respond_action.dispatch(RespondToFriendRequest { request_id: id, accept: false, block: true });
                                                }
                                            }>"Decline and block"</a>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }}
                        </section>

                        <section class="friends-list">
                            <h2>"Friends"</h2>
                            {if o.friends.is_empty() {
                                view! { <p>"No friends yet. Add the people you play with!"</p> }.into_any()
                            } else {
                                o.friends.iter().map(|f| {
                                    let uid = f.user_id;
                                    let name = f.name.clone();
                                    view! {
                                        <div class="friend-row">
                                            <span>
                                                <A href=format!("/players/{}", crate::players::encode_path_segment(&name))>
                                                    {name.clone()}
                                                </A>
                                            </span>
                                            " "
                                            <a href="#" on:click=move |ev| {
                                                ev.prevent_default();
                                                unfriend_action.dispatch(Unfriend { user_id: uid });
                                            }>"Unfriend"</a>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }}
                        </section>

                        <section class="friends-outgoing">
                            <h2>"Sent requests"</h2>
                            {if o.outgoing.is_empty() {
                                view! { <p>"No pending sent requests."</p> }.into_any()
                            } else {
                                o.outgoing.iter().map(|f| {
                                    let name = f.name.clone();
                                    view! {
                                        <div class="friend-row">
                                            <span>
                                                <A href=format!("/players/{}", crate::players::encode_path_segment(&name))>
                                                    {name.clone()}
                                                </A>
                                            </span>
                                            " - pending"
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }}
                        </section>

                        <section class="friends-blocked">
                            <h2>"Blocked"</h2>
                            {if o.blocked.is_empty() {
                                view! { <p>"Nobody is blocked."</p> }.into_any()
                            } else {
                                o.blocked.iter().map(|f| {
                                    let uid = f.user_id;
                                    let name = f.name.clone();
                                    view! {
                                        <div class="friend-row">
                                            <span>
                                                <A href=format!("/players/{}", crate::players::encode_path_segment(&name))>
                                                    {name.clone()}
                                                </A>
                                            </span>
                                            " "
                                            <a href="#" on:click=move |ev| {
                                                ev.prevent_default();
                                                unblock_action.dispatch(UnblockUser { user_id: uid });
                                            }>"Unblock"</a>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }}
                        </section>

                        <section class="friends-policy">
                            <h2>"Who can add me to games"</h2>
                            <select on:change=move |ev| {
                                policy_action.dispatch(SetInvitePolicy { policy: event_target_value(&ev) });
                            }>
                                {INVITE_POLICIES.iter().map(|(slug, label)| {
                                    let selected = o.invite_policy == *slug;
                                    view! { <option value=*slug selected=selected>{*label}</option> }
                                }).collect_view()}
                            </select>
                        </section>
                    }.into_any(),
                }}
            </div>
        </MainLayout>
    }
}
