use anyhow::{Context, Result};
use email::MimeMessage;
use lettre::smtp::authentication::Credentials;
use lettre::smtp::SmtpClient;
use lettre::{ClientSecurity, SendableEmail, Transport};
use log::error;

use crate::config::{Mail, CONFIG};

pub fn send(email: SendableEmail) -> Result<()> {
    match CONFIG.mail {
        Mail::Log => {
            error!("{}", email.message_to_string()?);
            Ok(())
        }
        Mail::Relay {} => Ok(SmtpClient::new("smtp:25", ClientSecurity::None)?
            .transport()
            .send(email)
            .map(|_| ())
            .context("unable to send email")?),
        Mail::Smtp {
            ref addr,
            ref user,
            ref pass,
        } => Ok(SmtpClient::new_simple(addr.as_ref())?
            .credentials(Credentials::new(user.to_string(), pass.to_string()))
            .transport()
            .send(email)
            .map(|_| ())
            .context("unable to send email")?),
    }
}

pub fn html_layout(content: &str) -> String {
    format!(
        "
        <link
            href=\"https://fonts.googleapis.com/css?family=Source+Code+Pro:400,700\"
            rel=\"stylesheet\"
        >
        <pre
            style=\"
                background-color: white;
                color: black;
                font-family: 'Source Code Pro', 'Lucida Console', monospace;
            \"
        >{}</pre>",
        content
    )
}

pub fn handle_inbound_email(e: &str) {
    // TODO handle error
    let parsed = MimeMessage::parse(e).unwrap();
    let bodies = extract_bodies(&parsed);
    println!("{} {:?}", bodies.len(), bodies);
}

fn extract_bodies(mm: &MimeMessage) -> Vec<String> {
    let mut bodies: Vec<String> = vec![mm.body.clone()];
    for c in &mm.children {
        bodies.extend(extract_bodies(c));
    }
    bodies
}
