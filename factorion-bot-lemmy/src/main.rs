use std::{env, panic};

use dotenv::dotenv;
use factorion_lib::{Commands, Comment, Consts, comment::CommentConstructed, rug::az::Az};
use lemmy_client::{
    LemmyClient, LemmyRequest,
    lemmy_api_common::{
        comment::{CreateComment, GetComments},
        lemmy_db_schema::{
            newtypes::{CommentId, PersonId, PostId, PrivateMessageId},
            sensitive::SensitiveString,
            source::language::Language,
        },
        person::{GetPersonMentions, Login},
        post::GetPosts,
        private_message::{CreatePrivateMessage, GetPrivateMessages},
    },
};
use log::{error, info};
use tokio::join;

// TODOS:
//  - persist already seen
//  - load/use community config
//  - load Consts
//  - influxdb

#[tokio::main]
async fn main() {
    init();

    let domain = env::var("LEMMY_DOMAIN").expect("No lemmy domain set!");
    let secure = env::var("LEMMY_DOMAIN_SECURE")
        .map_or(true, |s| matches!(s.as_str(), "y" | "Y" | "1" | ""));
    let user =
        SensitiveString::from(env::var("LEMMY_USER").expect("No lemmy username or email set!"));
    let pass = SensitiveString::from(env::var("LEMMY_PASS").expect("No lemmy password set!"));
    let token = env::var("LEMMY_2FA").ok();

    let client = LemmyClient::new(lemmy_client::ClientOptions { domain, secure });

    let jwt = client
        .login(Login {
            username_or_email: user,
            password: pass,
            totp_2fa_token: token,
        })
        .await
        .expect("Failed to login!")
        .jwt
        .expect("Got no token back after login!");
    let jwt = Some(jwt.into_inner());

    let site = client
        .get_site(LemmyRequest {
            body: (),
            jwt: jwt.clone(),
        })
        .await
        .expect("Failed to get site");

    let languages = site.all_languages;

    let mut already_seen_posts = vec![];
    let mut already_seen_comments = vec![];
    let mut already_seen_messages = vec![];

    let consts = Consts::default();

    for i in (0usize..).cycle() {
        let comments = get_comments(
            &client,
            &jwt,
            &languages,
            &mut already_seen_posts,
            &mut already_seen_comments,
            &mut already_seen_messages,
        )
        .await;

        let comments = comments
            .into_iter()
            .filter_map(|c| {
                let id = c.meta.id.clone();
                match std::panic::catch_unwind(|| Comment::extract(c, &consts)) {
                    Ok(c) => Some(c),
                    Err(_) => {
                        error!("Failed to extract comment {id:?}!");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        let comments = comments
            .into_iter()
            .filter_map(|c| {
                let id = c.meta.id.clone();
                match std::panic::catch_unwind(|| Comment::calc(c, &consts)) {
                    Ok(c) => Some(c),
                    Err(_) => {
                        error!("Failed to extract comment {id:?}!");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        for comment in comments {
            if comment.status.no_factorial || comment.status.already_replied_or_rejected {
                continue;
            }

            let reply = comment.get_reply(&consts);
            match comment.meta.id {
                Id::Post(id) => {
                    client
                        .create_comment(LemmyRequest {
                            body: CreateComment {
                                content: reply,
                                post_id: id,
                                parent_id: None,
                                language_id: None,
                            },
                            jwt: jwt.clone(),
                        })
                        .await
                        .unwrap();
                }
                Id::Comment(id, pid) => {
                    client
                        .create_comment(LemmyRequest {
                            body: CreateComment {
                                content: reply,
                                post_id: pid,
                                parent_id: Some(id),
                                language_id: None,
                            },
                            jwt: jwt.clone(),
                        })
                        .await
                        .unwrap();
                }
                Id::Message(id) => {
                    client
                        .create_private_message(LemmyRequest {
                            body: CreatePrivateMessage {
                                content: reply,
                                recipient_id: id,
                            },
                            jwt: jwt.clone(),
                        })
                        .await
                        .unwrap();
                }
            };
        }
    }
}

#[derive(Debug, Clone)]
enum Id {
    Post(PostId),
    Comment(CommentId, PostId),
    Message(PersonId),
}

struct Meta {
    id: Id,
    creator: Option<String>,
    comm: String,
}

async fn get_comments(
    client: &LemmyClient,
    jwt: &Option<String>,
    languages: &[Language],
    already_seen_posts: &mut Vec<PostId>,
    already_seen_comments: &mut Vec<CommentId>,
    already_seen_messages: &mut Vec<PrivateMessageId>,
) -> Vec<CommentConstructed<Meta>> {
    let (posts, comments, mentions, messages) = join!(
        client.list_posts(LemmyRequest {
            body: GetPosts {
                ..Default::default()
            },
            jwt: jwt.clone(),
        }),
        client.list_comments(LemmyRequest {
            body: GetComments {
                ..Default::default()
            },
            jwt: jwt.clone()
        }),
        client.list_mentions(LemmyRequest {
            body: GetPersonMentions {
                ..Default::default()
            },
            jwt: jwt.clone()
        }),
        client.list_private_messages(LemmyRequest {
            body: GetPrivateMessages {
                ..Default::default()
            },
            jwt: jwt.clone()
        })
    );
    let mut res = Vec::new();
    for post in posts.expect("Failed to get Posts!").posts {
        let id = post.post.id;
        let mut text = post.post.name;
        if let Some(alt) = post.post.alt_text {
            text.push_str(" ");
            text.push_str(&alt);
        }
        if let Some(body) = post.post.body {
            text.push_str(" ");
            text.push_str(&body);
        }
        let creator = post.creator.display_name;
        let comm = post.community.name;
        let lang = languages
            .iter()
            .find_map(|lang| (lang.id == post.post.language_id).then_some(lang.code.as_str()))
            .unwrap_or("en");
        if already_seen_posts.contains(&id) {
            continue;
        }

        res.push(Comment::new(
            &text,
            Meta {
                id: Id::Post(id),
                creator,
                comm,
            },
            Commands::NONE,
            10000,
            lang,
        ));

        already_seen_posts.push(id);
    }
    for comment in comments.expect("Failed to get Comments!").comments {
        let id = comment.comment.id;
        let pid = comment.post.id;
        let text = comment.comment.content;
        let creator = comment.creator.display_name;
        let comm = comment.community.name;
        let lang = languages
            .iter()
            .find_map(|lang| (lang.id == comment.comment.language_id).then_some(lang.code.as_str()))
            .unwrap_or("en");
        if already_seen_comments.contains(&id) {
            continue;
        }

        res.push(Comment::new(
            &text,
            Meta {
                id: Id::Comment(id, pid),
                creator,
                comm,
            },
            Commands::NONE,
            10000,
            lang,
        ));

        already_seen_comments.push(id);
    }
    for mention in mentions.expect("Failed to get Mentions!").mentions {
        let id = mention.comment.id;
        let pid = mention.post.id;
        let text = mention.comment.content;
        let creator = mention.creator.display_name;
        let comm = mention.community.name;
        let lang = languages
            .iter()
            .find_map(|lang| (lang.id == mention.comment.language_id).then_some(lang.code.as_str()))
            .unwrap_or("en");
        if already_seen_comments.contains(&id) {
            continue;
        }

        let comment = Comment::new(
            &text,
            Meta {
                id: Id::Comment(id, pid),
                creator,
                comm,
            },
            Commands::NONE,
            10000,
            lang,
        );
        if comment.status.no_factorial {
            todo!()
        }
        res.push(comment);
        already_seen_comments.push(id);
    }
    for message in messages.expect("Failed to get Messages!").private_messages {
        let id = message.private_message.id;
        let text = message.private_message.content;
        let creator = message.creator.display_name;
        let comm = String::from("[PM]");
        let lang = "en";
        if already_seen_messages.contains(&id) {
            continue;
        }

        res.push(Comment::new(
            &text,
            Meta {
                id: Id::Message(message.creator.id),
                creator,
                comm,
            },
            Commands::NONE,
            10000,
            lang,
        ));

        already_seen_messages.push(id);
    }
    res
}

fn init() {
    dotenv().ok();
    env_logger::builder()
        .format(|buf, record| {
            use std::io::Write;
            let style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "{style}{} | {} | {} | {}",
                record.level(),
                record.target(),
                buf.timestamp(),
                record.args()
            )
        })
        .init();

    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| format!("Unknown panic payload: {panic_info:?}"));

        error!("Thread panicked at {location} with message: {message}");
    }));

    info!("factorion-lib initialized successfully");
}
