use std::ops::Add as _;

use chrono::{Duration, Utc};
use poise::{
    serenity_prelude::{self as serenity, CreateAllowedMentions, CreateMessage, EditMember},
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
    context_menu_command = "Mute",
    slash_command,
    required_bot_permissions = "ADMINISTRATOR",
    required_permissions = "ADMINISTRATOR"
)]
pub async fn mute(
    ctx: Context<'_>,
    #[description = "User that said something offending deserving of a temporary mute."]
    offender: serenity::User,
) -> Result<(), Error> {
    let settings = &ctx.data().settings;
    let persistance = &ctx.data().persistance;

    let has_muted = ctx
        .http()
        .edit_member(
            ctx.guild_id().unwrap(),
            offender.id,
            &EditMember::new().mute(true),
            Some("Said some profanity!"),
        )
        .await
        .is_ok();

    let unmute_time = persistance
        .update_unmute_time(
            &offender.id.get(),
            Utc::now().add(Duration::minutes(settings.mute_duration as i64)),
        )
        .await?;

    ctx.http()
        .send_message(
            ctx.channel_id(),
            vec![],
            &CreateMessage::new()
                .allowed_mentions(CreateAllowedMentions::new().users([offender.id]))
                .content(match has_muted {
                    true => random_quote(offender.id, unmute_time, settings.mute_duration),
                    false => format!("<@{}> dodged the bullet this time...", offender.id),
                }),
        )
        .await?;

    Ok(())
}
