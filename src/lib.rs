use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use poise::serenity_prelude::Http;
use shuttle_persist::PersistInstance;
use tokio::sync::mpsc::unbounded_channel;

pub mod constants {
    pub const GUILD_ID: u64 = 175408004693229568;
}

pub mod quote {
    use chrono::{DateTime, Utc};
    use poise::serenity_prelude::UserId;
    use rand::seq::SliceRandom;

    pub fn random_quote(user_id: UserId, unmute_time: DateTime<Utc>, mute_duration: u64) -> String {
        let user_id = format!("<@{user_id}>");

        let quote_pool = vec![
            format!(
                "Ruh roh! {} said a bad bad word. They'll be muted for the next {} minutes.",
                user_id, mute_duration
            ),
            format!(
                "BOY, AIN'T NO WAY HE SAID THAT. That's gotta be a {} for {}!",
                mute_duration, user_id
            ),
            format!(
                "Reformed??? More like relapse. Come back at <t:{}:t> when you're done reflecting {}.",
                unmute_time.timestamp(),
                user_id
            ),
            format!("{}, that's crazy. Unmute <t:{}:R>", user_id, unmute_time.timestamp()),
            format!("That was uncalled for {}. Unmute <t:{}:R>", user_id, unmute_time.timestamp()),
            format!("Which Ayad was it this time...? {}.", user_id),
        ];

        quote_pool.choose(&mut rand::thread_rng()).map_or_else(
            || format!("WHAT DID {} SAY??????", user_id),
            |quote| quote.to_owned(),
        )
    }
}

pub struct UnmuteWorker;

impl UnmuteWorker {
    pub fn start(http: Arc<Http>, persist: PersistInstance) {
        let (sender, mut receiver) = unbounded_channel();

        let p = persist.clone();
        tokio::spawn(async move {
            loop {
                match p.clone().list() {
                    Ok(keys) => {
                        let _ = sender.send(keys);
                    }
                    Err(_) => {}
                };

                let _ = tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        tokio::spawn(async move {
            let http = http.as_ref();
            while let Some(user_ids) = receiver.recv().await {
                for user_id in user_ids {
                    match persist.load::<DateTime<Utc>>(&user_id) {
                        Ok(unmute_time) => {
                            if unmute_time <= Utc::now() {
                                let user_id_u64 = user_id.parse::<u64>().unwrap();

                                let member = http
                                    .get_member(175408004693229568, user_id_u64)
                                    .await
                                    .unwrap();

                                let is_unmuted = member.edit(http, |e| e.mute(false)).await;

                                if is_unmuted.is_ok() {
                                    let _ = persist.remove(&user_id);
                                }
                            }
                        }
                        Err(_error) => {}
                    };
                }
            }
        });
    }
}
