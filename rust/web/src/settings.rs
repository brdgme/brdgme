//! The /settings page: username, preferred colours, theme picker, email
//! placeholder. Logged-in only - anonymous visitors are sent to /login.
//! See docs/superpowers/specs/2026-07-16-35-settings-page-design.md.

use leptos::prelude::*;
use leptos_router::{NavigateOptions, hooks::use_navigate};

use crate::app::{local_data_theme, set_theme_client};
use crate::components::MainLayout;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();

    // Logged-in only: once the user resource resolves to anonymous, bounce
    // to /login. SSR/hydration render normally (resource is None there).
    let navigate = use_navigate();
    Effect::new(move |_| {
        if matches!(current_user.get(), Some(Ok(None))) {
            navigate("/login", NavigateOptions::default());
        }
    });

    // One round trip for everything the page prefills (name, email, colour
    // prefs). LocalResource, matching current_user/active_games: fetched on
    // the client after hydration.
    let settings: LocalResource<Result<crate::auth::SettingsData, ServerFnError>> =
        LocalResource::new(crate::auth::get_settings);

    view! {
        <MainLayout>
            <div class="settings content-page">
                <h1>"Settings"</h1>
                <UsernameSection settings=settings/>
                <ColorsSection settings=settings/>
                <ThemeSection/>
                <EmailSection settings=settings/>
            </div>
        </MainLayout>
    }
}

/// Explicit-save username form. Server-side rejections (format or "That
/// name is taken") come back as Ok(Some(message)) from set_username and
/// render as a field error; transport errors render a generic one.
#[component]
fn UsernameSection(
    settings: LocalResource<Result<crate::auth::SettingsData, ServerFnError>>,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let name_input = NodeRef::<html::Input>::new();
    let error = RwSignal::new(None::<String>);

    let save_action = Action::new(|name: &String| {
        let name = name.clone();
        async move { crate::auth::set_username(name).await }
    });
    Effect::new(move |_| {
        if let Some(result) = save_action.value().get() {
            match result {
                Ok(field_error) => error.set(field_error),
                Err(_) => error.set(Some("Failed to save. Please try again.".to_string())),
            }
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(el) = name_input.get() {
            save_action.dispatch(el.value());
        }
    };

    view! {
        <h2>"Username"</h2>
        <form on:submit=on_submit>
            <FormField
                label="Username"
                help="1-16 characters: letters, numbers, - and _. Must be unique."
                error=Signal::derive(move || error.get())
            >
                <input
                    type="text"
                    node_ref=name_input
                    pattern="[a-zA-Z0-9_-]{1,16}"
                    required
                    prop:value=move || {
                        settings.get().and_then(|r| r.ok()).map(|s| s.name).unwrap_or_default()
                    }
                />
            </FormField>
            <div class="form-actions">
                <input type="submit" value="Save" disabled=move || save_action.pending().get()/>
                <Show when=move || {
                    save_action.value().get().is_some_and(|r| matches!(r, Ok(None)))
                        && !save_action.pending().get()
                }>
                    <span class="form-help">"Saved."</span>
                </Show>
            </div>
        </form>
    }
}

/// Exactly three ordered selects over the 8-colour palette; picking a colour
/// already used in another box swaps the two, so the trio is always valid.
/// Saves immediately on change (fire-and-forget, like the theme tiles).
#[component]
fn ColorsSection(
    settings: LocalResource<Result<crate::auth::SettingsData, ServerFnError>>,
) -> impl IntoView {
    use crate::components::{ColorChip, FormField};

    let colors = RwSignal::new(vec![
        "Green".to_string(),
        "Red".to_string(),
        "Blue".to_string(),
    ]);
    // Adopt the stored prefs exactly once; after that the signal is the
    // source of truth (get_settings always returns a valid trio).
    let initialized = RwSignal::new(false);
    Effect::new(move |_| {
        if let Some(Ok(s)) = settings.get()
            && !initialized.get_untracked()
        {
            initialized.set(true);
            colors.set(s.pref_colors);
        }
    });

    let save_action = ServerAction::<crate::auth::SetPrefColors>::new();
    let pick = move |i: usize, val: String| {
        colors.update(|c| {
            if let Some(j) = c.iter().position(|x| *x == val)
                && j != i
            {
                c[j] = c[i].clone();
            }
            c[i] = val;
        });
        save_action.dispatch(crate::auth::SetPrefColors {
            colors: colors.get_untracked(),
        });
    };

    view! {
        <h2>"Preferred colours"</h2>
        {["1st choice", "2nd choice", "3rd choice"]
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                view! {
                    <FormField label=label>
                        <select
                            on:change=move |ev| pick(i, event_target_value(&ev))
                            prop:value=move || colors.get().get(i).cloned().unwrap_or_default()
                        >
                            {crate::theme::PLAYER_COLOR_NAMES
                                .into_iter()
                                .map(|name| {
                                    view! { <option value=name>{name}</option> }
                                })
                                .collect_view()}
                        </select>
                        <ColorChip color=Signal::derive(move || {
                            colors.get().get(i).cloned().unwrap_or_default()
                        })/>
                    </FormField>
                }
            })
            .collect_view()}
    }
}

/// Placeholder until #22d (multi-email management) lands: current login
/// email read-only plus a muted coming-soon note.
#[component]
fn EmailSection(
    settings: LocalResource<Result<crate::auth::SettingsData, ServerFnError>>,
) -> impl IntoView {
    use crate::components::FormField;

    view! {
        <h2>"Email addresses"</h2>
        <FormField label="Login email">
            <div>{move || {
                settings.get().and_then(|r| r.ok()).map(|s| s.email).unwrap_or_default()
            }}</div>
        </FormField>
        <div class="form-help">"Additional email addresses are coming soon."</div>
    }
}

/// The theme picker: one block per category (h3 heading + wrapping tile
/// row), selected tile outlined with a thicker highlight border. Applies
/// immediately on click; profile sync is fire-and-forget for logged-in
/// users (same pattern as the old /theme page).
#[component]
fn ThemeSection() -> impl IntoView {
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();
    let set_theme_action = ServerAction::<crate::auth::SetTheme>::new();

    // Drives the .selected highlight; None = System. Initialized from the
    // <html data-theme> attribute on hydrate (Effects are inert during SSR,
    // so SSR renders no selection - class-only change, no structural
    // mismatch).
    let current_theme = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        current_theme.set(local_data_theme());
    });

    // Handles are Copy, so this is callable from any number of move
    // closures without Rc.
    fn select(
        slug: Option<String>,
        current_theme: RwSignal<Option<String>>,
        current_user: LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>,
        set_theme_action: ServerAction<crate::auth::SetTheme>,
    ) {
        set_theme_client(slug.as_deref());
        current_theme.set(slug.clone());
        if matches!(current_user.get_untracked(), Some(Ok(Some(_)))) {
            set_theme_action.dispatch(crate::auth::SetTheme { theme: slug });
        }
    }

    fn tile(
        slug: &'static str,
        name: &'static str,
        current_theme: RwSignal<Option<String>>,
        current_user: LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>,
        set_theme_action: ServerAction<crate::auth::SetTheme>,
    ) -> impl IntoView {
        let sample_html = crate::theme::SAMPLE_HTML.clone();
        let on_click = move |_| {
            select(
                Some(slug.to_string()),
                current_theme,
                current_user,
                set_theme_action,
            )
        };
        view! {
            <div
                class="theme-tile"
                class:selected=move || current_theme.get().as_deref() == Some(slug)
                data-theme=slug
                style="background-color: var(--mk-background); color: var(--mk-foreground);"
                on:click=on_click
            >
                <div class="theme-tile-label">{name}</div>
                <div class="theme-tile-sample" inner_html=sample_html></div>
            </div>
        }
    }

    view! {
        <h2>"Theme"</h2>
        <div class="theme-category">
            <div class="theme-tiles">
                <div
                    class="theme-tile"
                    class:selected=move || current_theme.get().is_none()
                    style="background-color: var(--mk-background); color: var(--mk-foreground);"
                    on:click=move |_| select(None, current_theme, current_user, set_theme_action)
                >
                    <div class="theme-tile-label">"System"</div>
                </div>
                {crate::theme::grouped_themes()
                    .into_iter()
                    .filter(|(c, _)| *c == brdgme_color::ThemeCategory::Default)
                    .flat_map(|(_, group)| group)
                    .map(|(slug, name)| tile(slug, name, current_theme, current_user, set_theme_action))
                    .collect_view()}
            </div>
        </div>
        {crate::theme::grouped_themes().into_iter().filter_map(|(category, group)| {
            let heading = match category {
                brdgme_color::ThemeCategory::Default => None,
                brdgme_color::ThemeCategory::Light => Some("Light"),
                brdgme_color::ThemeCategory::Dark => Some("Dark"),
                brdgme_color::ThemeCategory::Deutan => Some("Deuteranopia"),
                brdgme_color::ThemeCategory::Protan => Some("Protanopia"),
                brdgme_color::ThemeCategory::Tritan => Some("Tritanopia"),
            }?;
            Some(view! {
                <div class="theme-category">
                    <h3>{heading}</h3>
                    <div class="theme-tiles">
                        {group.into_iter().map(|(slug, name)| {
                            tile(slug, name, current_theme, current_user, set_theme_action)
                        }).collect_view()}
                    </div>
                </div>
            })
        }).collect_view()}
    }
}
