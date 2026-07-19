use dotenv::dotenv;
use imap::extensions::sort::{SortCharset, SortCriterion};
use imap::types::Fetches;
use imap::{ImapConnection, Session};
use mail_parser::MessageParser;
use std::env;

extern crate imap;

type ImapSession = imap::Session<Box<dyn ImapConnection>>;

/// Imap connection info
///
/// * `session`: the imap session
/// * `current_mailbox`: what mailbox currently connected to
/// * `mailbox_info`: details about mailbox e.g. total num msgs
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

/// Create new imap session
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
fn list_messages(c: &mut Connection, num_messages: u32) -> anyhow::Result<()> {
    let messages: Fetches;

    if !c.session.capabilities()?.has_str("SORT") {
        let uid_hashset = c.session.uid_search("ALL")?;
        let mut uid_vec: Vec<_> = uid_hashset.into_iter().collect();
        uid_vec.sort_unstable_by_key(|uid| std::cmp::Reverse(*uid));

        let uids_to_fetch: Vec<String> = uid_vec
            .into_iter()
            .take(num_messages as usize)
            .map(|uid| uid.to_string())
            .collect();

        let uid_set_str = uids_to_fetch.join(",");

        messages = c.session.uid_fetch(&uid_set_str, "RFC822.HEADER")?;
    } else {
        let sorted_session = c.session.sort(
            &[SortCriterion::Reverse(&SortCriterion::Arrival)],
            SortCharset::Utf8,
            "ALL",
        )?;

        let uids_to_fetch: Vec<String> = sorted_session
            .into_iter()
            .take(num_messages as usize)
            .map(|uid| uid.to_string())
            .collect();

        let uid_set = uids_to_fetch.join(",");

        messages = c.session.uid_fetch(&uid_set, "RFC822.HEADER")?;
    }

    // print header for each msg
    for msg in messages.iter() {
        let uid = msg.uid.unwrap_or_default();

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

        println!("UID:     {}", uid);
        println!("From:    {}", from);
        println!("Subject: {}", subject);
        println!("Date:    {}", date);
        println!("---");
    }
    Ok(())
}

fn list_mailboxes(c: &mut Connection) -> anyhow::Result<()> {
    let mailboxes = c.session.list(None, Some("*"))?;
    println!("Available mailboxes:");
    for mb in mailboxes.iter() {
        println!("  - {}", mb.name());
    }

    Ok(())
}

fn select_mailbox(c: &mut Connection, mailbox: &str) -> anyhow::Result<()> {
    c.mailbox_info = Some(c.session.select(mailbox)?);
    c.current_mailbox = mailbox.to_string();
    println!("Selected mailbox: {}", mailbox);
    Ok(())
}

fn read_full_message(c: &mut Connection, uid: u32) -> anyhow::Result<()> {
    let fetch = c.session.uid_fetch(&uid.to_string(), "RFC822")?;
    if let Some(msg) = fetch
        .iter()
        .next()
        .and_then(|f| f.body())
        .and_then(|f| MessageParser::default().parse(f))
    {
        let count = msg.text_body_count();
        for i in 0..count {
            if let Some(txt) = msg.body_text(i) {
                println!("{}", txt);
            }
        }
    } else {
        println!("No body for uid {}", uid);
    }

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
            ["listm"] => {
                if let Err(e) = list_mailboxes(&mut connection) {
                    eprintln!("error: {}", e);
                }
            }
            ["read", n] => {
                let uid = n.parse()?;
                if let Err(e) = read_full_message(&mut connection, uid) {
                    eprintln!("error: {}", e);
                }
            }
            _ => eprintln!("unknown command"),
        }
    }

    Ok(())
}
