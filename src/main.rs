use std::time::Duration;

use broadcaster::BroadcastChannel;
use futures_util::future::Either;
use futures_util::StreamExt;
use tide::http::format_err;
use tide::Body;
use tide_websockets::WebsocketMiddleware;

#[derive(Clone, Debug)]
struct ChatMessage {
    user: String,
    message: String,
}

#[derive(Clone, Debug)]
struct State {
    broadcaster: BroadcastChannel<ChatMessage>,
}

impl State {
    fn new() -> Self {
        Self {
            broadcaster: BroadcastChannel::new(),
        }
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
        while let Some(ChatMessage { user, message }) = state.broadcaster.next().await {
            println!("{}: {}", user, message);
        }
    });

    app.at("/")
        .with(WebsocketMiddleware::new(|request, wsc| async move {
            let petnames = petname::Petnames::default();
            let current_user = petnames.generate_one(2, ".");
            let State { broadcaster } = request.state();
            let broadcaster = broadcaster.clone();

            let mut combined_stream = futures_util::stream::select(
                wsc.clone().map(|l| Either::Left(l)),
                broadcaster.clone().map(|r| Either::Right(r)),
            );

            let message = format!("{} has entered the room", current_user.clone());
            let cloned_broadcaster = broadcaster.clone();
            async_std::task::spawn(async move {
                async_std::task::sleep(Duration::from_millis(1)).await;
                cloned_broadcaster
                    .send(&ChatMessage {
                        user: String::from("system"),
                        message,
                    })
                    .await
            });

            while let Some(item) = combined_stream.next().await {
                match item {
                    Either::Left(Ok(message)) => {
                        broadcaster
                            .clone()
                            .send(&ChatMessage {
                                user: current_user.clone(),
                                message: message.into_text()?,
                            })
                            .await?;
                    }

                    Either::Right(ChatMessage { user, message }) => {
                        wsc.send_string(format!("{}: {}", user, message)).await?;
                    }

                    o => {
                        dbg!(o);
                        return Err(format_err!("no idea"));
                    }
                }
            }

            Ok(())
        }))
        .get(|_| async { Ok(Body::from_file("./static/build/index.html").await?) })
        .serve_dir("./static/build")?;

    app.listen("127.0.0.1:8080").await?;

    Ok(())
}
