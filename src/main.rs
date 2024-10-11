use dotenv::dotenv;
use poise::serenity_prelude::{self as serenity, ClientBuilder};
use services::{persistance::PersistanceClient, unmute};
use settings::AppSettings;

mod commands;
mod services;
mod settings;

struct State {
    settings: AppSettings,
    persistance: PersistanceClient,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let settings = AppSettings::new().unwrap();
    let token = settings.discord_token.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let redis_client = redis::Client::open(settings.redis_url.clone())?;
                let client = r2d2::Pool::new(redis_client)?;

                let persistance = PersistanceClient::from(&client);

                unmute::start_workers(&persistance, settings.guild_id, ctx.http.clone()).await;

                poise::builtins::register_in_guild(
                    &ctx.http,
                    &framework.options().commands,
                    settings.guild_id.into(),
                )
                .await?;

                println!("Framework setup done");
                Ok(State {
                    settings,
                    persistance,
                })
            })
        })
        .build();

    let client = ClientBuilder::new(&token, serenity::GatewayIntents::non_privileged())
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
