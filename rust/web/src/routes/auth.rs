use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    let mut email = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut message = use_signal(|| String::new());

    let handle_submit = move |_| {
        spawn(async move {
            loading.set(true);
            message.set("".to_string());
            
            // TODO: Implement login server function
            // For now, just simulate
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            
            if email().is_empty() {
                message.set("Please enter an email address".to_string());
            } else {
                message.set("Login link sent to your email!".to_string());
            }
            
            loading.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8",
            div { class: "max-w-md w-full space-y-8",
                div {
                    h2 { class: "mt-6 text-center text-3xl font-extrabold text-gray-900",
                        "Sign in to your account"
                    }
                    p { class: "mt-2 text-center text-sm text-gray-600",
                        "Enter your email to receive a login link"
                    }
                }
                form { 
                    class: "mt-8 space-y-6", 
                    onsubmit: move |evt| {
                        evt.prevent_default();
                        handle_submit(evt);
                    },
                    div { class: "rounded-md shadow-sm -space-y-px",
                        div {
                            label { r#for: "email-address", class: "sr-only", "Email address" }
                            input {
                                id: "email-address",
                                name: "email",
                                r#type: "email",
                                autocomplete: "email",
                                required: true,
                                class: "appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 focus:z-10 sm:text-sm",
                                placeholder: "Email address",
                                value: email(),
                                oninput: move |evt| email.set(evt.value())
                            }
                        }
                    }

                    if !message().is_empty() {
                        div { class: "text-sm text-center",
                            if message().contains("sent") {
                                p { class: "text-green-600", "{message}" }
                            } else {
                                p { class: "text-red-600", "{message}" }
                            }
                        }
                    }

                    div {
                        button {
                            r#type: "submit",
                            disabled: loading(),
                            class: "group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50",
                            if loading() {
                                "Sending..."
                            } else {
                                "Send Login Link"
                            }
                        }
                    }

                    div { class: "text-center",
                        Link { 
                            to: "/",
                            class: "text-indigo-600 hover:text-indigo-500",
                            "← Back to Home"
                        }
                    }
                }
            }
        }
    }
}