use leptos::prelude::*;
use leptos::html;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes, A},
    StaticSegment, ParamSegment,
};
use uuid::Uuid;

use crate::components::MainLayout;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="apple-mobile-web-app-capable" content="yes"/>
                <meta name="mobile-web-app-capable" content="yes"/>
                <link href="https://fonts.googleapis.com/css2?family=Source+Code+Pro:wght@400;700&display=swap" rel="stylesheet"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    
    let (last_update, set_last_update) = signal(0u64);
    provide_context(crate::websocket_client::WebSocketTrigger {
        last_update,
        set_last_update,
    });
    crate::websocket_client::use_websocket();

    view! {
        <Stylesheet id="leptos" href="/pkg/web.css"/>
        <Title text="brdg.me"/>

        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=HomePage/>
                <Route path=StaticSegment("login") view=LoginPage/>
                <Route path=StaticSegment("games") view=GamesPage/>
                <Route path=StaticSegment("dashboard") view=DashboardPage/>
                <Route path=(StaticSegment("games"), ParamSegment("id")) view=GamePage/>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <MainLayout>
            <h1>"Welcome to brdg.me"</h1>
            <p>"Lo-fi board games by email and web."</p>
            <A href="/dashboard">"Go to Dashboard"</A>
        </MainLayout>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    let (show_code_input, set_show_code_input) = signal(false);
    let (email, set_email) = signal(String::new());
    
    let email_input = NodeRef::<html::Input>::new();
    let code_input = NodeRef::<html::Input>::new();
    
    let on_email_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let email_value = email_input.get().unwrap().value();
        set_email.set(email_value);
        set_show_code_input.set(true);
    };
    
    let on_code_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        // TODO: Handle code submission
    };
    
    let show_code_link = move |_| {
        set_show_code_input.set(true);
    };

    view! {
        <div class="login">
            <h1>"brdg.me"</h1>
            <div class="subtitle">"Lo-fi board games, email / web"</div>
            
            <Show when=move || !show_code_input.get()>
                <div>
                    <div>"Enter your email address to start"</div>
                    <form on:submit=on_email_submit>
                        <div>
                            <input 
                                type="email" 
                                node_ref=email_input
                                placeholder="Email address"
                                required
                            />
                            <input type="submit" value="Get code"/>
                        </div>
                        <div class="hasCode">
                            <a on:click=show_code_link style="cursor:pointer">"I already have a login code"</a>
                        </div>
                    </form>
                </div>
            </Show>
            
            <Show when=move || show_code_input.get()>
                <div>
                    <Show when=move || !email.get().is_empty()>
                        <div>"Logging in as " <a>{email.get()}</a></div>
                    </Show>
                    <div>
                        <div>"A login code has been sent to your email, please enter it here"</div>
                        <form on:submit=on_code_submit>
                            <input 
                                type="tel" 
                                pattern="[0-9]*"
                                node_ref=code_input
                                placeholder="Login code"
                                required
                            />
                            <input type="submit" value="Play!"/>
                        </form>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn GamesPage() -> impl IntoView {
    view! {
        <MainLayout>
            <h1>"Games"</h1>
            <p>"Browse active games and create new ones."</p>
        </MainLayout>
    }
}

#[component]
fn DashboardPage() -> impl IntoView {
    view! {
        <MainLayout>
            <h1>"Dashboard"</h1>
            <p>"View your active games and statistics."</p>
            
            <div class="dashboard-sections">
                <section class="active-games">
                    <h2>"Active Games"</h2>
                    <p>"Use the sidebar to navigate your games."</p>
                </section>
            </div>
        </MainLayout>
    }
}

#[component]
fn GamePage() -> impl IntoView {
    use crate::game::server_fns::get_game_details;
    use crate::components::game::*;
    use std::str::FromStr;

    let params = leptos_router::hooks::use_params_map();
    let game_id = move || params.get().get("id").as_deref().and_then(|id| Uuid::from_str(id).ok());
    
    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    
    let game_data = Resource::new(
        move || (game_id(), trigger.last_update.get()),
        |(id, _)| async move {
            match id {
                Some(id) => get_game_details(id).await,
                None => Err(ServerFnError::new("Invalid Game ID")),
            }
        }
    );
    
    view! {
        <Suspense fallback=move || view! { <MainLayout><div>"Loading game..."</div></MainLayout> }>
                        {move || {
                            game_data.get().map(|res| match res {
                                Ok(data) => {
                                    let is_my_turn = data.is_my_turn;
                                    let id = data.id;
                                    let html = data.html.clone();
                                    let command_spec = data.command_spec.clone();
                                    let player_names: Vec<String> = data.players.iter().map(|p| p.name.clone()).collect();
                                    
                                    view! {
                                        <MainLayout is_my_turn=is_my_turn has_sub_menu=true has_next_game=is_my_turn>
                                            <div class="game-container">
                                                <div class="game-main">
                                                    <GameBoard html=html />
                                                    <GameLogs />
                                                    <Show when=move || is_my_turn>
                                                        <GameCommandInput 
                                                            game_id=id 
                                                            command_spec=command_spec.clone() 
                                                            player_names=player_names.clone()
                                                        />
                                                    </Show>
                                                    <Show when=move || !is_my_turn>
                                                        <div class="game-current-turn">
                                                            <span>"Waiting on opponents..."</span>
                                                        </div>
                                                    </Show>
                                                </div>
                                                <GameMeta data=data />
                                            </div>
                                        </MainLayout>
                                    }.into_any()
                                },
            
                    Err(e) => view! { <MainLayout><div class="error">"Error: " {e.to_string()}</div></MainLayout> }.into_any(),
                })
            }}
        </Suspense>
    }
}