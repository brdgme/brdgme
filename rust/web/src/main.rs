
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use web::app::*;
    use web::db::create_pool;
    use web::auth::session::create_session_layer;
    use web::state::AppState;
    use web::websocket::GameBroadcaster;


    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Create database connection pool
    let pool = create_pool().await.expect("Failed to create database pool");
    
    // Create broadcaster
    let broadcaster = GameBroadcaster::new(1024);

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    
    let state = AppState {
        leptos_options: leptos_options.clone(),
        pool: pool.clone(),
        broadcaster: broadcaster.clone(),
    };

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes_with_context(
            &state,
            routes,
            {
                let pool = pool.clone();
                let broadcaster = broadcaster.clone();
                move || {
                    provide_context(pool.clone());
                    provide_context(broadcaster.clone());
                }
            },
            {
                let leptos_options = state.leptos_options.clone();
                move || shell(leptos_options.clone())
            }
        )
        .nest("/api", web::game::server::api_routes())
        .route("/ws", axum::routing::get(web::websocket::ws_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>({
            let leptos_options = leptos_options.clone();
            move |_| shell(leptos_options.clone())
        }))
        .layer(create_session_layer())
        .with_state(state);

    // run our app with hyper
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
