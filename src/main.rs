use std::ops::Add;

use anyhow::Context as _;
use chrono::{DateTime, Duration, Utc};
use poise::{
    samples::register_in_guild,
    serenity_prelude::{self as serenity},
};
use profanity_filter_bot::{constants, quote::random_quote, UnmuteWorker};
use shuttle_persist::PersistInstance;
use shuttle_poise::ShuttlePoise;
use shuttle_secrets::SecretStore;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, StateData, Error>;

/// Temporarly mute a user because he said something bad
#[poise::command(
    slash_command,
    required_bot_permissions = "ADMINISTRATOR",
    required_permissions = "ADMINISTRATOR"
)]
async fn mute(
    ctx: Context<'_>,
    #[description = "User that said something offending deserving of a temporary mute."]
    offender: serenity::Member,
) -> Result<(), Error> {
    static MUTE_DURATION: i64 = 15;

    let muted = offender.edit(ctx.http(), |e| e.mute(true)).await;
    let unmute_time = Utc::now().add(Duration::minutes(MUTE_DURATION));

    let user_id = offender.user.id;
    ctx.send(|r| {
        r.allowed_mentions(|m| m.users(vec![offender.user]));

        if muted.is_ok() {
            r.content(random_quote(user_id, unmute_time, MUTE_DURATION as u64));
        }

        r
    })
    .await?;

    let user_id = &user_id.to_string();
    let state = ctx.data();
    let should_save = state
        .persist
        .load::<DateTime<Utc>>(user_id)
        .map(|prev_unmute| prev_unmute < unmute_time)
        .unwrap_or(true);

    if should_save {
        let _ = state.persist.save(user_id, unmute_time);
    }

    Ok(())
}

struct StateData {
    pub persist: PersistInstance,
}

#[shuttle_runtime::main]
async fn poise(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_persist::Persist] persist: PersistInstance,
) -> ShuttlePoise<StateData, Error> {
    // Get the discord token set in `Secrets.toml`
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![mute()],
            ..Default::default()
        })
        .token(discord_token)
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            UnmuteWorker::start(ctx.http.clone(), persist.clone());

            Box::pin(async move {
                register_in_guild(
                    &ctx.http,
                    &framework.options().commands,
                    serenity::GuildId(constants::GUILD_ID),
                )
                .await?;
                // poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                Ok(StateData { persist })
            })
        })
        .build()
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    Ok(framework.into())
}
