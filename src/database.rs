use std::fmt::Write;
use std::path::Path;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use serde::{Deserialize, Serialize};
use serenity::model::{
    channel::Message,
    id::{ChannelId, GuildId, MessageId, UserId},
    misc::Mention,
};
use tokio::sync::Mutex;

use super::{Config, FeedbackChannel, SETTINGS};

const DB_PATH: &str = "config/database.db";

lazy_static! {
    pub static ref DB: Database = Database::open(Path::new(DB_PATH), &SETTINGS);
}

#[derive(Deserialize, Serialize)]
pub struct FbEntry {
    pub last_reply: DateTime<Utc>,
}

#[derive(Deserialize, Serialize)]
pub struct PromoEntry {
    pub last_activity: DateTime<Utc>,
}

#[derive(Deserialize, Serialize)]
pub struct OpenMessage {
    pub user: u64,
    pub msg: u64,
}

impl OpenMessage {
    pub fn new(msg: &Message) -> OpenMessage {
        OpenMessage {
            user: msg.author.id.0,
            msg: msg.id.0,
        }
    }
    pub(crate) fn display(&self, out: &mut String, channel: &FeedbackChannel) {
        let link =
            MessageId(self.msg).link(ChannelId(channel.channel), Some(GuildId(channel.guild)));
        let mention = Mention::from(UserId(self.user));
        writeln!(out, "- {link} from {mention}").unwrap();
    }
}

pub struct Database {
    db: Mutex<PickleDb>,
}

impl Database {
    fn open(path: &Path, config: &Config) -> Self {
        Self {
            db: Mutex::new({
                let mut db = PickleDb::load(
                    path,
                    PickleDbDumpPolicy::AutoDump,
                    SerializationMethod::Json,
                )
                .unwrap_or_else(|_| {
                    PickleDb::new(
                        path,
                        PickleDbDumpPolicy::AutoDump,
                        SerializationMethod::Json,
                    )
                });

                for channel in &config.channels {
                    let key = channel.to_string();
                    if !db.lexists(&key) {
                        db.lcreate(&key).unwrap();
                    }
                }

                db
            }),
        }
    }

    pub async fn take_feedback(&self, author: UserId, guild: GuildId) -> Option<FbEntry> {
        let uid = format!("f_{author},{guild}");
        let mut db = self.db.lock().await;
        if let Some(permit) = db.get::<FbEntry>(&uid) {
            db.rem(&uid).expect("Failed to drop entry");
            Some(permit)
        } else {
            None
        }
    }

    pub async fn allow_feedback(&self, author: UserId, guild: GuildId, fb: &FbEntry) {
        let uid = format!("f_{author},{guild}");
        let mut db = self.db.lock().await;
        db.set(&uid, fb).unwrap();
    }

    pub(crate) async fn add_open_msg(&self, channel: &FeedbackChannel, msg: &Message) {
        let mut db = self.db.lock().await;
        db.ladd(&channel.to_string(), &OpenMessage::new(&msg));
    }

    pub(crate) async fn remove_open_msg(&self, channel: &FeedbackChannel, msg: &Message) {
        let mut db = self.db.lock().await;
        db.lrem_value(&channel.to_string(), &OpenMessage::new(&msg)).unwrap();
    }

    pub(crate) async fn open_msgs<F: FnMut(OpenMessage)>(
        &self,
        channel: &FeedbackChannel,
        mut f: F,
    ) {
        let db = self.db.lock().await;
        for item in db.liter(&channel.to_string()) {
            f(item.get_item().unwrap());
        }
    }
}
