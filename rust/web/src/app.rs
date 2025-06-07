use leptos::prelude::*;
use leptos::html;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes, A},
    StaticSegment, ParamSegment,
};

use crate::auth::{GetCurrentUser, Logout};
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
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/web.css"/>

        // sets the document title
        <Title text="brdg.me"/>

        // content for this welcome page
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
fn UserMenu() -> impl IntoView {
    let get_user_action = ServerAction::<GetCurrentUser>::new();
    let logout_action = ServerAction::<Logout>::new();
    
    let current_user = get_user_action.value();
    
    // Fetch current user on component mount
    Effect::new(move |_| {
        get_user_action.dispatch(GetCurrentUser {});
    });
    
    let on_logout = move |_| {
        logout_action.dispatch(Logout {});
    };
    
    view! {
        <div class="user-menu">
            <Suspense fallback=move || view! { <span>"Loading..."</span> }>
                {move || {
                    current_user.get().map(|result| match result {
                        Ok(Some(user)) => view! {
                            <span class="user-greeting">"Welcome, " {user.name}</span>
                            <button class="btn btn-small" on:click=on_logout>"Logout"</button>
                        }.into_any(),
                        Ok(None) => view! {
                            <A href="/login">"Login"</A>
                        }.into_any(),
                        Err(_) => view! {
                            <A href="/login">"Login"</A>
                        }.into_any(),
                    })
                }}
            </Suspense>
        </div>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <MainLayout>
            <h1>"Home blah"</h1>
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
                            <a on:click=show_code_link>"I already have a login code"</a>
                        </div>
                    </form>
                </div>
            </Show>
            
            <Show when=move || show_code_input.get()>
                <div>
                    <Show when=move || !email.get().is_empty()>
                        <div>"Logging in as" <a>{email.get()}</a></div>
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
                    <p>"No active games. " <A href="/games">"Start a new game!"</A></p>
                </section>
                
                <section class="game-history">
                    <h2>"Recent Games"</h2>
                    <p>"No completed games yet."</p>
                </section>
                
                <section class="statistics">
                    <h2>"Statistics"</h2>
                    <p>"Games played: 0"</p>
                    <p>"Games won: 0"</p>
                    <p>"Win rate: N/A"</p>
                </section>
            </div>
        </MainLayout>
    }
}

#[component]
fn GamePage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let _game_id = move || params.get().get("id").map(|s| s.clone()).unwrap_or_default();
    
    let (is_my_turn, _set_is_my_turn) = signal(true); // Mock data for now
    
    view! {
        <MainLayout is_my_turn=is_my_turn.get() has_sub_menu=true has_next_game=is_my_turn.get()>
            <div class="game-container">
                <div class="game-main">
                    <div class="game-render">
                        <pre>"    " <span style="background-color:#dcdcdc;"> </span><span style="background-color:#dcdcdc;"><span style="color:#505050;">"A1"</span></span><span style="background-color:#dcdcdc;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#bebebe;"> </span><span style="background-color:#bebebe;"><span style="color:#505050;">"A2"</span></span><span style="background-color:#bebebe;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#dcdcdc;"> </span><span style="background-color:#dcdcdc;"><span style="color:#505050;">"A3"</span></span><span style="background-color:#dcdcdc;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#bebebe;"> </span><span style="background-color:#bebebe;"><span style="color:#505050;">"A4"</span></span><span style="background-color:#bebebe;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#dcdcdc;"> </span><span style="background-color:#dcdcdc;"><span style="color:#505050;">"A5"</span></span><span style="background-color:#dcdcdc;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#bebebe;"> </span><span style="background-color:#bebebe;"><span style="color:#505050;">"A6"</span></span><span style="background-color:#bebebe;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#dcdcdc;"> </span><span style="background-color:#dcdcdc;"><span style="color:#505050;">"A7"</span></span><span style="background-color:#dcdcdc;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#bebebe;"> </span><span style="background-color:#bebebe;"><span style="color:#505050;">"A8"</span></span><span style="background-color:#bebebe;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#dcdcdc;"> </span><span style="background-color:#dcdcdc;"><span style="color:#505050;">"A9"</span></span><span style="background-color:#dcdcdc;"><span style="color:#505050;"></span>"  "</span><span style="background-color:#bebebe;"> </span><span style="background-color:#bebebe;"><span style="color:#505050;">"A10"</span></span><span style="background-color:#bebebe;"><span style="color:#505050;"></span>" "</span><span style="background-color:#dcdcdc;"> </span><span style="background-color:#dcdcdc;"><span style="color:#505050;">"A11"</span></span><span style="background-color:#dcdcdc;"><span style="color:#505050;"></span>" "</span><span style="background-color:#f8bbd0;"> </span><span style="background-color:#f8bbd0;"><b><span style="color:#000000;">"A12"</span></b></span><span style="background-color:#f8bbd0;"><b><span style="color:#000000;"></span></b>" "</span>"    " "\n"
"               " <span style="color:#616161;">"No corporations have been founded yet"</span>"                " "\n"
"                      " <span style="color:#616161;">"Draw tiles remaining: " <b>"93"</b></span>"                      " "\n"
"\n"
"        " <b>"Corporation"</b>"   " <b>"Size"</b>"   " <b>"Value"</b>"   " <b>"Shares"</b>"    " <b>"Minor"</b>"   " <b>"Major"</b>"        " "\n"
"        " <b><span style="color:#7b1fa2;">"Worldwide"</span></b>"     0      $200    25 left   $1000   $2000        " "\n"
"        " <b><span style="color:#e64a19;">"Sackson"</span></b>"       0      $200    25 left   $1000   $2000        " "\n"
"        " <b><span style="color:#388e3c;">"Festival"</span></b>"      0      $300    25 left   $1500   $3000        " "\n"
"        " <b><span style="color:#fbc02d;">"Imperial"</span></b>"      0      $300    25 left   $1500   $3000        " "\n"
"        " <b><span style="color:#1976d2;">"American"</span></b>"      0      $300    25 left   $1500   $3000        " "\n"
"        " <b><span style="color:#d32f2f;">"Continental"</span></b>"   0      $400    25 left   $2000   $4000        " "\n"
"        " <b><span style="color:#000000;">"Tower"</span></b>"         0      $400    25 left   $2000   $4000        " "\n"
"\n"
<b>"Player"</b>"                      " <b>"Cash"</b>"    " <span style="color:#7b1fa2;"><b>"WO"</b></span>"   " <span style="color:#e64a19;"><b>"SA"</b></span>"   " <span style="color:#388e3c;"><b>"FE"</b></span>"   " <span style="color:#fbc02d;"><b>"IM"</b></span>"   " <span style="color:#1976d2;"><b>"AM"</b></span>"   " <span style="color:#d32f2f;"><b>"CO"</b></span>"   " <span style="color:#000000;"><b>"TO"</b></span> "\n"
<b><span style="color:#388e3c;">"<beefsack>"</span></b>"                  $6000   0    0    0    0    0    0    0 " "\n"
<b><span style="color:#ffa000;">"<beefsack+test@gmail.com>"</span></b>"   $6000   0    0    0    0    0    0    0 "</pre>
                    </div>
                    <div class="recent-logs-container">
                        <div class="recent-logs-header">"Recent logs"</div>
                        <div class="recent-logs">
                            <div><b><span style="color:#ffa000;">"<beefsack+test@gmail.com>"</span></b>" played " <b>"E10"</b></div>
                        </div>
                    </div>
                    <Show when=move || is_my_turn.get()>
                        <div class="suggestions-container">
                            <div class="suggestions-content">
                                <div>
                                    <div class="suggestion-doc-item">
                                        <a>"play"</a><span class="suggestion-doc-desc">" - " "play a tile to the board"</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div class="game-command-input">
                            <form>
                                <input type="text" placeholder="Enter command..." autocomplete="off" autocorrect="off" autocapitalize="none" spellcheck="false"/>
                                <input type="submit" value="Send"/>
                            </form>
                        </div>
                    </Show>
                    <Show when=move || !is_my_turn.get()>
                        <div class="game-current-turn">
                            <span>"Waiting on" <span>" " <strong class="brdgme-red">"<baconheist>"</strong></span></span>
                        </div>
                    </Show>
                </div>
                <div class="game-meta">
                    <div class="game-meta-main">
                        <div>
                            <h2>"Acquire"</h2>
                            <div>
                                <div><strong class="brdgme-amber">"<beefsack+test@gmail.com>"</strong></div>
                                <div style="margin-left: 1em;">
                                    <div><abbr title="ELO rating, new players start at 1200" style="cursor: help;">"Rating"</abbr>": 1200"</div>
                                    <div>"Points: 6000"</div>
                                </div>
                            </div>
                            <div>
                                <div><strong class="brdgme-green">"<beefsack>"</strong></div>
                                <div style="margin-left: 1em;">
                                    <div><abbr title="ELO rating, new players start at 1200" style="cursor: help;">"Rating"</abbr>": 1359"</div>
                                    <div>"Points: 6000"</div>
                                </div>
                            </div>
                            <div>
                                <h3>"Actions"</h3>
                                <div><a>"Concede"</a></div>
                            </div>
                        </div>
                    </div>
                    <div class="game-meta-logs">
                        <h2>"Logs"</h2>
                        <div class="game-meta-logs-content">
                            <div>
                                <div class="game-log-entry">
                                    <div class="log-time">"- 10:22 PM -"</div>
                                    <div><b>"2 player special rule: a dummy player is added for shareholder bonuses. A dice (D6) is rolled to determine the dummy player's shares. The money for the dummy player is not tracked and it is not able to win the game."</b></div>
                                </div>
                                <div class="game-log-entry">
                                    <div></div>
                                    <div><b><span style="color:#ffa000;">"<beefsack+test@gmail.com>"</span></b>" will start the game"</div>
                                </div>
                                <div class="game-log-entry">
                                    <div></div>
                                    <div><b><span style="color:#ffa000;">"<beefsack+test@gmail.com>"</span></b>" played " <b>"E10"</b></div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </MainLayout>
    }
}
