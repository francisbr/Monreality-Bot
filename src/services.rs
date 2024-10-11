pub mod unmute {
    use std::{sync::Arc, time::Duration};

    use chrono::Utc;
    use poise::serenity_prelude::{EditMember, Http};
    use tokio::sync::mpsc::unbounded_channel;

    use super::persistance::PersistanceClient;

    pub async fn start_workers(
        persistance: &PersistanceClient,
        guild_id: u64,
        framework_client: Arc<Http>,
    ) {
        let (sender, mut receiver) = unbounded_channel::<Vec<u64>>();

        tokio::spawn({
            let persistance = persistance.clone();

            async move {
                loop {
                    if let Ok(muted_users) = persistance.get_all_muted_user_id().await {
                        let _ = sender.send(muted_users);
                    }

                    let _ = tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        });

        tokio::spawn({
            let persistance = persistance.clone();

            async move {
                while let Some(user_ids) = receiver.recv().await {
                    for user_id in user_ids {
                        if let Some(unmute_time) = persistance.get_unmute_time(&user_id).await {
                            if unmute_time < Utc::now()
                                && unmute(&framework_client, guild_id, user_id).await.is_ok()
                            {
                                let _ = persistance.clear_unmute_time(&user_id).await;
                            }
                        }
                    }
                }
            }
        });
    }

    async fn unmute(
        framework_client: &Http,
        guild_id: u64,
        user_id: u64,
    ) -> Result<(), anyhow::Error> {
        framework_client
            .edit_member(
                guild_id.into(),
                user_id.into(),
                &EditMember::new().mute(false),
                Some("Timeout done!"),
            )
            .await?;

        Ok(())
    }
}

pub mod persistance {
    use std::{sync::LazyLock, time::Duration};

    use chrono::{DateTime, Utc};
    use r2d2::Pool;
    use redis::Commands;

    static LAST_YEAR: LazyLock<DateTime<Utc>> =
        LazyLock::new(|| Utc::now() - Duration::from_secs(60 * 60 * 24 * 365));

    #[derive(Debug, Clone)]
    pub struct PersistanceClient {
        client: Pool<redis::Client>,
    }

    impl From<&Pool<redis::Client>> for PersistanceClient {
        fn from(value: &Pool<redis::Client>) -> Self {
            Self {
                client: value.clone(),
            }
        }
    }

    impl PersistanceClient {
        pub async fn get_all_muted_user_id(&self) -> Result<Vec<u64>, anyhow::Error> {
            let mut conn = self.client.get()?;

            Ok(conn.keys::<_, Vec<u64>>("*")?)
        }

        pub async fn get_unmute_time(&self, user_id: &u64) -> Option<DateTime<Utc>> {
            let mut conn = self.client.get().ok()?;

            conn.get::<_, i64>(user_id)
                .map_or(None, |t| DateTime::from_timestamp(t, 0))
        }

        pub async fn update_unmute_time(
            &self,
            user_id: &u64,
            unmute_time: DateTime<Utc>,
        ) -> Result<DateTime<Utc>, anyhow::Error> {
            let mut conn = self.client.get()?;

            let t = redis::transaction(&mut conn, &[user_id], |con, pipe| {
                let prev_unmute_time = con
                    .get::<_, i64>(user_id)
                    .ok()
                    .map(|old_val| DateTime::from_timestamp(old_val, 0))
                    .flatten()
                    .unwrap_or(*LAST_YEAR);

                if prev_unmute_time < unmute_time {
                    pipe.set(user_id, unmute_time.timestamp()).ignore();
                }

                pipe.get(user_id).query(con)
            })
            .map(|(t,): (i64,)| DateTime::from_timestamp(t, 0).unwrap())?;

            Ok(t)
        }

        pub async fn clear_unmute_time(&self, user_id: &u64) -> Result<(), anyhow::Error> {
            let mut conn = self.client.get()?;

            let _ = conn.del::<_, u64>(user_id);

            Ok(())
        }
    }
}

pub mod quotes {
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
