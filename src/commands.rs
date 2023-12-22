use std::ops::Add as _;

use chrono::{Duration, Utc};
use poise::{
    serenity_prelude::{self as serenity},
    Command,
};

use crate::{services::quotes::random_quote, State};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, State, Error>;

pub fn all() -> Vec<Command<State, Error>> {
    vec![mute()]
}

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
    let settings = &ctx.data().settings;
    let persistance = &ctx.data().persistance;

    let user_id = offender.user.id;
    let has_muted = offender.edit(ctx.http(), |e| e.mute(true)).await.is_ok();

    let unmute_time = persistance
        .update_unmute_time(
            user_id.as_u64(),
            Utc::now().add(Duration::minutes(settings.mute_duration as i64)),
        )
        .await?;

    ctx.send(|r| {
        r.allowed_mentions(|m| m.users(vec![offender.user]));

        if has_muted {
            r.content(random_quote(user_id, unmute_time, settings.mute_duration));
        } else {
            r.content(format!("<@{user_id}> dodged the bullet this time..."));
        }

        r
    })
    .await?;

    Ok(())
}
