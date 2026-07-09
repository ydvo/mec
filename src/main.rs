use dotenv::dotenv;
use imap::{ImapConnection, Session};
use mail_parser::MessageParser;
use std::env;

extern crate imap;

type ImapSession = imap::Session<Box<dyn ImapConnection>>;

pub struct Connection {
    pub session: ImapSession,
    pub current_mailbox: String,
    pub mailbox_info: Option<imap::types::Mailbox>,
}

impl Connection {
    pub fn new() -> anyhow::Result<Self> {
        let mut session = make_connection()?;
        let default_mailbox = "INBOX".to_string(); // default to Inbox
        let mailbox_info = session.select(&default_mailbox)?;

        Ok(Self {
            session,
            current_mailbox: default_mailbox,
            mailbox_info: Some(mailbox_info),
        })
    }
}

fn make_connection() -> imap::error::Result<ImapSession> {
    dotenv().ok();
    let email = env::var("EMAIL").unwrap();
    let app_pw = env::var("APP_PASSWORD").unwrap();

    let client = imap::ClientBuilder::new("imap.gmail.com", 993).connect()?;

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let imap_session = client.login(email, app_pw).map_err(|e| e.0)?;

    Ok(imap_session)
}

/// List the first x messages in current mailbox
///
/// * `session`: Current Imap Session
/// * `num_messages`: number of msgs to list
fn list_messages(c: &mut Connection, num_messages: u32) -> Result<(), Box<dyn std::error::Error>> {
    let total = c.mailbox_info.as_ref().expect("No mailbox selected").exists;
    if total == 0 {
        return Ok(());
    }
    let range = format!("{}:*", total.saturating_sub(num_messages - 1));

    let messages = c.session.fetch(&range, "RFC822.HEADER")?;

    for msg in messages.iter() {
        let raw_header = match msg.header() {
            Some(h) => h,
            None => continue,
        };

        let parsed = MessageParser::default()
            .parse(raw_header)
            .expect("failed to parse headers");

        // subject() returns Option<&str> — already decoded, no RFC 2047 tokens
        let subject = parsed.subject().unwrap_or("(no subject)");

        // from() returns Option<&Address> — typed, not a raw string
        let from = parsed
            .from()
            .and_then(|a| a.first())
            .map(|a| match (a.name(), a.address()) {
                (Some(name), Some(addr)) => format!("{} <{}>", name, addr),
                (None, Some(addr)) => addr.to_string(),
                _ => "(unknown)".to_string(),
            })
            .unwrap_or_default();

        // date() returns Option<&DateTime> — a structured type, not a string
        let date = parsed
            .date()
            .map(|d| format!("{}-{:02}-{:02}", d.year, d.month, d.day))
            .unwrap_or_default();

        println!("From:    {}", from);
        println!("Subject: {}", subject);
        println!("Date:    {}", date);
        println!("---");
    }
    Ok(())
}

fn list_mailboxes() -> anyhow::Result<()> {
    print!("Mailboxes");
    Ok(())
}

fn select_mailbox(c: &mut Connection, mailbox: &str) -> anyhow::Result<()> {
    print!(" select box");
    Ok(())
}

fn read_full_message(uid: u32) -> anyhow::Result<()> {
    print!("msg");
    Ok(())
}

fn set_flag(uid: u32, f: Option<String>) -> anyhow::Result<()> {
    print!("flag");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut connection = Connection::new()?;
    let mut args = std::env::args();
    args.next(); // skip program name
    println!(
        "Connected. Commands: \n
        list x: List x most recent messages and uid \n
        listm: List possible mailboxes \n
        select x: Select mailbox x \n
        read x: Print message from uid \n
        flag x f: toggle flag f on msg uid x "
    );

    let stdin = std::io::stdin();
    let mut input = String::new();

    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;

        input.clear();
        stdin.read_line(&mut input)?;
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        match parts.as_slice() {
            ["quit"] | ["q"] => {
                connection.session.logout()?;
                break;
            }
            ["select", mailbox] => {
                if let Err(e) = select_mailbox(&mut connection, mailbox) {
                    eprintln!("error: {}", e);
                }
            }
            ["list", n] => {
                let count = n.parse().unwrap_or(10);
                if let Err(e) = list_messages(&mut connection, count) {
                    eprintln!("error: {}", e);
                }
            }
            _ => eprintln!("unknown command"),
        }
    }

    Ok(())
}
