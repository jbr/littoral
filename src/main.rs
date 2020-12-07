use async_std::sync::{Arc, RwLock};
use broadcaster::BroadcastChannel;
use futures_util::future::Either;
use futures_util::StreamExt;
use gh_emoji::Replacer;
use petname::Petnames;
use serde_derive::Serialize;
use serde_json::json;
use tide::http::format_err;
use tide::{Body, Request};
use tide_websockets::{Message as WSMessage, WebSocket};

#[derive(Clone, Debug, Serialize)]
enum Message {
    Chat { user: String, message: String },
    Userlist(Vec<String>),
}

#[derive(Clone, Debug)]
struct State {
    broadcaster: BroadcastChannel<Message>,
    users: Arc<RwLock<Vec<String>>>,
    petnames: Petnames<'static>,
}

impl State {
    fn new() -> Self {
        Self {
            broadcaster: BroadcastChannel::new(),
            users: Arc::new(RwLock::new(vec![])),
            petnames: Petnames::default(),
        }
    }

    async fn add_user(&self, user: String) -> tide::Result<()> {
        self.users.write().await.push(user.clone());
        self.send_chat(
            String::from("system"),
            format!("{} has entered the chat", user),
        )
        .await?;
        self.send_userlist().await
    }

    async fn send_message(&self, message: Message) -> tide::Result<()> {
        self.broadcaster.send(&message).await?;
        Ok(())
    }

    async fn send_chat(&self, user: String, message: String) -> tide::Result<()> {
        self.send_message(Message::Chat { user, message }).await
    }

    async fn send_userlist(&self) -> tide::Result<()> {
        self.send_message(Message::Userlist(self.users.read().await.clone()))
            .await
    }

    async fn remove_user(&self, user: String) -> tide::Result<()> {
        self.users.write().await.retain(|u| u != &user);
        self.send_chat(String::from("system"), format!("{} left the chat", user))
            .await?;
        self.send_userlist().await
    }

    fn current_user(&self) -> String {
        self.petnames.generate_one(2, ".")
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    let mut app = tide::with_state(State::new());
    app.with(tide::sessions::SessionMiddleware::new(
        tide::sessions::MemoryStore::new(),
        std::env::var("SESSION_SECRET").unwrap().as_bytes(),
    ));

    let mut state = app.state().clone();
    async_std::task::spawn(async move {
        while let Some(message) = state.broadcaster.next().await {
            match message {
                Message::Chat { user, message } => println!("{}: {}", user, message),
                Message::Userlist(userlist) => println!("{:?}", userlist),
            };
        }
        tide::Result::Ok(())
    });

    app.at("/")
        .with(WebSocket::new(|request: Request<State>, wsc| async move {
            let state = request.state().clone();
            let current_user = request.state().current_user();
            let broadcaster = state.broadcaster.clone();

            let mut combined_stream = futures_util::stream::select(
                wsc.clone().map(|l| Either::Left(l)),
                broadcaster.clone().map(|r| Either::Right(r)),
            );

            state.add_user(current_user.clone()).await?;
            let replacer = Replacer::new();

            while let Some(item) = combined_stream.next().await {
                match item {
                    Either::Left(Ok(WSMessage::Text(message))) => {
                        state
                            .send_chat(current_user.clone(), replacer.replace_all(&message).into())
                            .await?;
                    }

                    Either::Right(Message::Chat { user, message }) => {
                        wsc.send_json(
                            &json!({ "type": "message", "user": user, "message": message }),
                        )
                        .await?;
                    }

                    Either::Right(Message::Userlist(userlist)) => {
                        wsc.send_json(&json!({ "type": "userlist", "users": userlist }))
                            .await?;
                    }

                    o => {
                        state.remove_user(current_user.clone()).await?;
                        log::debug!("{:?}", o);
                        return Err(format_err!("no idea"));
                    }
                }
            }

            Ok(())
        }))
        .get(|_| async { Ok(Body::from_file("./client/build/index.html").await?) });

    app.at("/").serve_dir("./client/build")?;

    app.listen("127.0.0.1:8080").await?;

    Ok(())
}
