use dioxus::prelude::*;

enum Mode {
    EnteringEmail,
    SubmittingEmail,
    EnteringCode,
    SubmittingCode,
}

#[component]
fn LoginEnteringEmail(
    onsubmit: EventHandler<String>,
    onclickhavecode: EventHandler<()>,
) -> Element {
    let mut email = use_signal(|| "".to_string());
    rsx! {
        div { "Enter your email address to start" }
        form {
            onsubmit: move |event| {
                event.prevent_default();
                onsubmit(email.read().clone())
            },
            div {
                input {
                    type: "email",
                    required: true,
                    autofocus: true,
                    placeholder: "Email address",
                    value: email,
                    oninput: move |event| email.set(event.value()),
                }
                input {
                    type: "submit",
                    value: "Get code",
                }
            }
            div {
                class: "hasCode",
                a {
                    href: "#",
                    onclick: move |event| {
                        event.prevent_default();
                        onclickhavecode(())
                    },
                    "I already have a login code"
                }
            }
        }
    }
}

#[component]
fn LoginSubmittingEmail() -> Element {
    rsx! {
        div { "Submitting email..." }
    }
}

#[server]
async fn submit_login_email(email: String) -> Result<(), ServerFnError> {
    let code: i32 = "3214".parse()?;
    Ok(())
}

#[component]
pub fn Login() -> Element {
    let mut mode = use_signal(|| Mode::EnteringEmail);
    let submit_email = move |email| async move {
        mode.set(Mode::SubmittingEmail);
        submit_login_email(email).await.unwrap();
        mode.set(Mode::SubmittingCode);
    };
    let click_have_code = move |_| {
        let code: i32 = "3214".parse()?;
        mode.set(Mode::EnteringCode);
        Ok(())
    };
    rsx! {
        div {
            class: "login",
            h1 { "brdg.me" }
            div {
                class: "subtitle",
                "Lo-fi board games, email / web"
            }
            match *mode.read() {
                Mode::EnteringEmail => rsx! {
                    LoginEnteringEmail {
                        onsubmit: submit_email,
                        onclickhavecode: click_have_code,
                    },
                },
                Mode::SubmittingEmail => rsx! { "Submitting email..." },
                Mode::EnteringCode => rsx! { "Enter the code we sent you" },
                Mode::SubmittingCode => rsx! { "Submitting code..." },
            }
        },
    }
}
