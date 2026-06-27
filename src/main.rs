use dotenv::dotenv;
use mail_parser::MessageParser;
use std::env;

extern crate imap;

fn main() {
    if let Ok(msg) = fetch_inbox_top() {
        print!("{:?}", msg);
    }
}

fn fetch_inbox_top() -> imap::error::Result<Option<String>> {
    dotenv().ok();
    let email = env::var("EMAIL").unwrap();
    let app_pw = env::var("APP_PASSWORD").unwrap();

    let client = imap::ClientBuilder::new("imap.gmail.com", 993).connect()?;

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let mut imap_session = client.login(email, app_pw).map_err(|e| e.0)?;

    // we want to fetch the first email in the INBOX mailbox
    imap_session.select("INBOX")?;

    // fetch message number 1 in this mailbox, along with its RFC822 field.
    // RFC 822 dictates the format of the body of e-mails
    let messages = imap_session.fetch("1", "RFC822")?;
    let message = if let Some(m) = messages.iter().next() {
        m
    } else {
        return Ok(None);
    };

    // extract the message's body
    let body = message.body().expect("message did not have a body!");
    let body = std::str::from_utf8(body).expect("message was not valid utf-8");

    let body = MessageParser::default()
        .parse(body)
        .unwrap()
        .body_text(0)
        .unwrap()
        .to_string();

    // be nice to the server and log out
    imap_session.logout()?;

    Ok(Some(body))
}
