use std::collections::HashSet;

use chrono::Utc;
use lazy_static::lazy_static;
use serde::Deserialize;
use serenity::{
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
        id::{ChannelId, GuildId},
        interactions::{Interaction, InteractionResponseType},
    },
    prelude::*,
};

mod util;
use util::*;

mod database;
use database::*;

#[derive(Deserialize)]
struct Config {
    client_id: u64,
    channels: HashSet<FeedbackChannel>,
    discord_token: String,
    min_msg_len: usize,
    permission_timeout_days: u64,
}

#[derive(Deserialize, PartialEq, Eq, Hash)]
struct FeedbackChannel {
    guild: u64,
    channel: u64,
}

impl FeedbackChannel {
    fn new(guild: GuildId, channel: ChannelId) -> Self {
        Self {
            guild: guild.0,
            channel: channel.0,
        }
    }
    fn to_string(&self) -> String {
        format!("fc_{},{}", self.guild, self.channel)
    }
}

lazy_static! {
    static ref SETTINGS: Config =
        toml::de::from_str(&std::fs::read_to_string("config/config.toml").unwrap()).unwrap();
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let guild = if let Some(guild) = command.guild_id {
                guild
            } else {
                return;
            };
            let channel = FeedbackChannel::new(guild, command.channel_id);

            if !SETTINGS.channels.contains(&channel) {
                return;
            }

            let content = match command.data.name.as_str() {
                "open" => {
                    let mut out = String::new();

                    DB.open_msgs(&channel, |msg| {
                        msg.display(&mut out, &channel);
                    })
                    .await;

                    if out.is_empty() {
                        format!("No open messages...")
                    } else {
                        format!("Here is a list of posts that still need feedback:\n{out}")
                    }
                }
                _ => "not implemented, yikes".to_string(),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                println!("Cannot respond to slash command: {why}");
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.id.0 == SETTINGS.client_id {
            return;
        };

        let guild = if let Some(guild) = msg.guild_id {
            guild
        } else {
            return;
        };
        let channel = FeedbackChannel::new(guild, msg.channel_id);

        if !SETTINGS.channels.contains(&channel) {
            return;
        }

        if msg.content == "ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "pong").await {
                println!("Error sending message: {why:?}");
            }
        }

        if is_feedback_request(&msg) {
            handle_feedback_request(ctx, msg).await;
        } else if is_feedback_reply(&msg, SETTINGS.min_msg_len) {
            handle_feedback_reply(ctx, msg).await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        for FeedbackChannel { guild, channel: _ } in &SETTINGS.channels {
            let commands =
                GuildId::set_application_commands(&GuildId(*guild), &ctx.http, |commands| {
                    commands.create_application_command(|command| {
                        command
                            .name("open")
                            .description("Lists posts in need of feedback")
                    })
                })
                .await
                .unwrap();

            println!("Available guild commands for {guild:?}: {commands:?}");
        }
    }
}

async fn handle_feedback_request(ctx: Context, msg: Message) {
    if let Some(permit) = DB.take_feedback(msg.author.id, msg.guild_id.unwrap()).await {
        let t_days = (Utc::now() - permit.last_reply).num_days();
        if t_days <= SETTINGS.permission_timeout_days as i64 {
            let channel = FeedbackChannel::new(msg.guild_id.unwrap(), msg.channel_id);
            DB.add_open_msg(&channel, &msg).await;
            if let Err(why) = msg
                .reply_ping(
                    &ctx.http,
                    "Successfully spent your permission to ask for feedback.",
                )
                .await
            {
                println!("Error replying to feedback request: {why:?}");
            }
            return;
        }
    }

    msg.reply_mention(
        &ctx.http,
        "We *highly encourage* you to give feedback before you ask for it yourself. YEET!",
    )
    .await
    .unwrap();
    msg.delete(&ctx.http).await.unwrap();
}

async fn handle_feedback_reply(ctx: Context, msg: Message) {
    let ref_msg = msg.referenced_message.as_ref().unwrap();

    let channel = FeedbackChannel::new(msg.guild_id.unwrap(), msg.channel_id);
    DB.remove_open_msg(&channel, &ref_msg).await;
    DB.allow_feedback(
        msg.author.id,
        msg.guild_id.unwrap(),
        &FbEntry {
            last_reply: Utc::now(),
        },
    )
    .await;

    if let Err(why) = msg
        .reply_ping(
            &ctx.http,
            "Your feedback has been observed by forces unknown...",
        )
        .await
    {
        println!("Error replying to feedback: {why:?}");
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with Discord bot token in the config file.
    let token = &SETTINGS.discord_token;
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .application_id(SETTINGS.client_id)
        .await
        .expect("Err creating client");

    client.start().await.unwrap();
}
